use std::collections::BTreeMap;

use hdk::prelude::*;
pub use private_event_sourcing_integrity::*;

mod agent_encrypted_message;
pub use agent_encrypted_message::{commit_my_pending_encrypted_messages, create_encrypted_message};
mod awaiting_dependencies;
pub use awaiting_dependencies::attempt_commit_awaiting_deps_entries;
mod linked_devices;
pub use linked_devices::*;
mod private_event;
pub use private_event::*;
mod synchronize;
pub use synchronize::synchronize_with_linked_devices;
mod utils;

pub use strum::IntoStaticStr;

pub use private_event_proc_macro::*;
use synchronize::syncronize_with_recipients;

pub fn scheduled_tasks<T: PrivateEvent>() -> ExternResult<()> {
    commit_my_pending_encrypted_messages::<T>()?;
    synchronize_with_linked_devices()?;
    syncronize_with_recipients::<T>()?;
    Ok(())
}

#[hdk_extern]
pub fn init() -> ExternResult<InitCallbackResult> {
    let mut fns: BTreeSet<GrantedFunction> = BTreeSet::new();
    fns.insert((zome_info()?.name, FunctionName::from("recv_remote_signal")));
    let functions = GrantedFunctions::Listed(fns);
    let cap_grant = ZomeCallCapGrant {
        tag: String::from("receive_messages"),
        access: CapAccess::Unrestricted,
        functions,
    };
    create_cap_grant(cap_grant)?;

    schedule("scheduled_tasks")?;

    Ok(InitCallbackResult::Pass)
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum Signal {
    LinkCreated {
        action: SignedActionHashed,
        link_type: LinkTypes,
    },
    LinkDeleted {
        action: SignedActionHashed,
        create_link_action: SignedActionHashed,
        link_type: LinkTypes,
    },
    EntryCreated {
        action: SignedActionHashed,
        app_entry: EntryTypes,
    },
    EntryUpdated {
        action: SignedActionHashed,
        app_entry: EntryTypes,
        original_app_entry: EntryTypes,
    },
    EntryDeleted {
        action: SignedActionHashed,
        original_app_entry: EntryTypes,
    },
}

#[derive(Serialize, Deserialize, Debug, SerializedBytes)]
pub enum PrivateEventSourcingRemoteSignal {
    NewPrivateEvent(PrivateEventEntry),
    SynchronizeEntriesWithLinkedDevice(BTreeMap<EntryHashB64, PrivateEventEntry>),
}

pub fn recv_private_events_remote_signal<T: PrivateEvent>(
    signal: PrivateEventSourcingRemoteSignal,
) -> ExternResult<()> {
    let provenance = call_info()?.provenance;
    match signal {
        PrivateEventSourcingRemoteSignal::NewPrivateEvent(private_event_entry) => {
            receive_private_event::<T>(provenance, private_event_entry)
        }
        PrivateEventSourcingRemoteSignal::SynchronizeEntriesWithLinkedDevice(
            private_event_entries,
        ) => receive_private_events::<T>(provenance, private_event_entries),
    }
}

#[hdk_extern(infallible)]
pub fn post_commit(committed_actions: Vec<SignedActionHashed>) {
    for action in committed_actions {
        if let Err(err) = signal_action(action) {
            error!("Error signaling new action: {:?}", err);
        }
    }
}
fn signal_action(action: SignedActionHashed) -> ExternResult<()> {
    match action.hashed.content.clone() {
        Action::CreateLink(create_link) => {
            if let Ok(Some(link_type)) =
                LinkTypes::from_type(create_link.zome_index, create_link.link_type)
            {
                emit_signal(Signal::LinkCreated {
                    action: action.clone(),
                    link_type,
                })?;
            }
            Ok(())
        }
        Action::DeleteLink(delete_link) => {
            let record = get(delete_link.link_add_address.clone(), GetOptions::default())?.ok_or(
                wasm_error!(WasmErrorInner::Guest(
                    "Failed to fetch CreateLink action".to_string()
                )),
            )?;
            match record.action() {
                Action::CreateLink(create_link) => {
                    if let Ok(Some(link_type)) =
                        LinkTypes::from_type(create_link.zome_index, create_link.link_type)
                    {
                        emit_signal(Signal::LinkDeleted {
                            action,
                            link_type,
                            create_link_action: record.signed_action.clone(),
                        })?;
                    }
                    Ok(())
                }
                _ => Err(wasm_error!(WasmErrorInner::Guest(
                    "Create Link should exist".to_string()
                ))),
            }
        }
        Action::Create(_create) => {
            if let Ok(Some(app_entry)) = get_entry_for_action(&action.hashed.hash) {
                match app_entry {
                    EntryTypes::PrivateEvent(entry) => {
                        // TODO: change this to only be called once per all actions
                        let result = call_remote(
                            agent_info()?.agent_latest_pubkey,
                            zome_info()?.name,
                            "attempt_commit_awaiting_deps_entries".into(),
                            None,
                            (),
                        )?;
                        let ZomeCallResponse::Ok(_) = result else {
                            return Err(wasm_error!(
                                "Error calling 'attempt_commit_awaiting_deps_entries'"
                            ));
                        };
                    }
                    _ => {}
                };
            }
            Ok(())
        }
        Action::Update(update) => {
            if let Ok(Some(app_entry)) = get_entry_for_action(&action.hashed.hash) {
                if let Ok(Some(original_app_entry)) =
                    get_entry_for_action(&update.original_action_address)
                {
                    emit_signal(Signal::EntryUpdated {
                        action,
                        app_entry,
                        original_app_entry,
                    })?;
                }
            }
            Ok(())
        }
        Action::Delete(delete) => {
            if let Ok(Some(original_app_entry)) = get_entry_for_action(&delete.deletes_address) {
                emit_signal(Signal::EntryDeleted {
                    action,
                    original_app_entry,
                })?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn get_entry_for_action(action_hash: &ActionHash) -> ExternResult<Option<EntryTypes>> {
    let record = match get_details(action_hash.clone(), GetOptions::default())? {
        Some(Details::Record(record_details)) => record_details.record,
        _ => {
            return Ok(None);
        }
    };
    let entry = match record.entry().as_option() {
        Some(entry) => entry,
        None => {
            return Ok(None);
        }
    };
    let (zome_index, entry_index) = match record.action().entry_type() {
        Some(EntryType::App(AppEntryDef {
            zome_index,
            entry_index,
            ..
        })) => (zome_index, entry_index),
        _ => {
            return Ok(None);
        }
    };
    EntryTypes::deserialize_from_type(*zome_index, *entry_index, entry)
}
