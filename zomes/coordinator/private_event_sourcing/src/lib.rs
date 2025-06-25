use hdk::prelude::*;
pub use strum::IntoStaticStr;

pub use acknowledgements::*;
pub use private_event_sourcing_integrity::*;

mod awaiting_dependencies;
pub use awaiting_dependencies::attempt_commit_awaiting_deps_entries;

mod linked_devices;
pub use linked_devices::*;
mod private_event;
pub use private_event::*;
mod acknowledgements;
mod event_history;
mod utils;
pub use event_history::*;
mod send_events;
pub use send_events::{resend_events_if_necessary, send_new_events};
mod events_sent_to_recipients;

mod async_message;
pub use async_message::*;

pub use private_event_proc_macro::*;

pub fn scheduled_tasks<T: PrivateEvent>() -> ExternResult<()> {
    resend_events_if_necessary::<T>()?;
    attempt_commit_awaiting_deps_entries::<T>()?;
    create_acknowledgements::<T>()?;
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
    // LinkCreated {
    //     action: SignedActionHashed,
    //     link_type: LinkTypes,
    // },
    // LinkDeleted {
    //     action: SignedActionHashed,
    //     create_link_action: SignedActionHashed,
    //     link_type: LinkTypes,
    // },
    NewPrivateEvent {
        event_hash: EntryHash,
        private_event_entry: PrivateEventEntry,
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
    SendMessage(Message),
}

pub fn recv_private_events_remote_signal<T: PrivateEvent>(
    signal: PrivateEventSourcingRemoteSignal,
) -> ExternResult<()> {
    let provenance = call_info()?.provenance;
    match signal {
        PrivateEventSourcingRemoteSignal::SendMessage(message) => {
            receive_message::<T>(provenance, message)
        }
    }
}

pub fn call_send_events(committed_actions: &Vec<SignedActionHashed>) -> ExternResult<()> {
    let private_event_entry_type = UnitEntryTypes::PrivateEvent
        .try_into()
        .expect("Can't convert UnitEntryTypes::PrivateEvent to EntryTypes.");
    let new_private_event_hashes: BTreeSet<EntryHash> = committed_actions
        .iter()
        .into_iter()
        .filter_map(|action| match action.action() {
            Action::Create(create) => Some(create.clone()),
            _ => None,
        })
        .filter(|create| create.entry_type.eq(&private_event_entry_type))
        .map(|create| create.entry_hash)
        .collect();

    if new_private_event_hashes.len() > 0 {
        let result = call(
            CallTargetCell::Local,
            zome_info()?.name,
            "send_new_events".into(),
            None,
            new_private_event_hashes.clone(),
        )?;
        let ZomeCallResponse::Ok(_) = result else {
            return Err(wasm_error!("Error calling 'send_events': {:?}", result));
        };
        let result = call(
            CallTargetCell::Local,
            zome_info()?.name,
            "create_acknowledgements".into(),
            None,
            (),
        )?;
        let ZomeCallResponse::Ok(_) = result else {
            return Err(wasm_error!(
                "Error calling 'create_acknowledgements': {:?}",
                result
            ));
        };
        let result = call(
            CallTargetCell::Local,
            zome_info()?.name,
            "attempt_commit_awaiting_deps_entries".into(),
            None,
            (),
        )?;
        let ZomeCallResponse::Ok(_) = result else {
            return Err(wasm_error!(
                "Error calling 'attempt_commit_awaiting_deps_entries': {:?}",
                result
            ));
        };
        let result = call(
            CallTargetCell::Local,
            zome_info()?.name,
            "resend_events_if_necessary".into(),
            None,
            (),
        )?;
        let ZomeCallResponse::Ok(_) = result else {
            return Err(wasm_error!("Error calling 'send_events': {:?}", result));
        };
    }

    Ok(())
}

#[hdk_extern(infallible)]
pub fn post_commit(committed_actions: Vec<SignedActionHashed>) {
    if let Err(err) = call_send_events(&committed_actions) {
        error!("Error calling send events: {:?}", err);
    }

    for action in committed_actions {
        if let Err(err) = signal_action(action) {
            error!("Error signaling new action: {:?}", err);
        }
    }
}
fn signal_action(action: SignedActionHashed) -> ExternResult<()> {
    match action.hashed.content.clone() {
        // Action::CreateLink(create_link) => {
        //     if let Ok(Some(link_type)) =
        //         LinkTypes::from_type(create_link.zome_index, create_link.link_type)
        //     {
        //         emit_signal(Signal::LinkCreated {
        //             action: action.clone(),
        //             link_type,
        //         })?;
        //     }
        //     Ok(())
        // }
        // Action::DeleteLink(delete_link) => {
        //     let record = get(delete_link.link_add_address.clone(), GetOptions::default())?.ok_or(
        //         wasm_error!(WasmErrorInner::Guest(
        //             "Failed to fetch CreateLink action".to_string()
        //         )),
        //     )?;
        //     match record.action() {
        //         Action::CreateLink(create_link) => {
        //             if let Ok(Some(link_type)) =
        //                 LinkTypes::from_type(create_link.zome_index, create_link.link_type)
        //             {
        //                 emit_signal(Signal::LinkDeleted {
        //                     action,
        //                     link_type,
        //                     create_link_action: record.signed_action.clone(),
        //                 })?;
        //             }
        //             Ok(())
        //         }
        //         _ => Err(wasm_error!(WasmErrorInner::Guest(
        //             "Create Link should exist".to_string()
        //         ))),
        //     }
        // }
        Action::Create(create) => {
            if let Ok(Some(app_entry)) = get_entry_for_action(&action.hashed.hash) {
                match app_entry.clone() {
                    EntryTypes::PrivateEvent(entry) => {
                        emit_signal(Signal::NewPrivateEvent {
                            event_hash: create.entry_hash,
                            private_event_entry: entry,
                        })?;
                    }
                    _ => {}
                };
                emit_signal(Signal::EntryCreated { action, app_entry })?;
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
