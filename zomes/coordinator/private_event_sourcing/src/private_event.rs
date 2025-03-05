use hdk::prelude::*;
use private_event_sourcing_integrity::*;
use std::collections::BTreeMap;

use crate::{
    agent_encrypted_message::create_encrypted_message, linked_devices::query_my_linked_devices,
    utils::create_relaxed, PrivateEventSourcingRemoteSignal, Signal,
};

pub trait PrivateEvent: TryFrom<SerializedBytes> + TryInto<SerializedBytes> + Clone {
    /// Whether the given entry is to be accepted in to our source chain
    fn validate(
        &self,
        author: AgentPubKey,
        timestamp: Timestamp,
    ) -> ExternResult<ValidateCallbackResult>;

    /// The agents other than the linked devices for the author that are suposed to receive this entry
    fn recipients(
        &self,
        author: AgentPubKey,
        timestamp: Timestamp,
    ) -> ExternResult<Vec<AgentPubKey>>;

    /// Code to run after an event has been committed
    fn post_commit(&self, author: AgentPubKey, timestamp: Timestamp) -> ExternResult<()> {
        Ok(())
    }
}

fn build_private_event_entry<T: PrivateEvent>(private_event: T) -> ExternResult<PrivateEventEntry> {
    let bytes: SerializedBytes = private_event
        .try_into()
        .map_err(|_err| wasm_error!("Failed to serialize private event."))?;

    let signed: SignedContent<SerializedBytes> = SignedContent {
        content: bytes,
        timestamp: sys_time()?,
    };
    let my_pub_key = agent_info()?.agent_latest_pubkey;
    let signature = sign(my_pub_key.clone(), &signed)?;
    Ok(PrivateEventEntry(SignedEvent {
        author: my_pub_key,
        signature,
        event: signed,
    }))
}

pub fn create_private_event<T: PrivateEvent>(private_event: T) -> ExternResult<EntryHash> {
    let entry = build_private_event_entry(private_event.clone())?;
    let validation_outcome =
        private_event.validate(agent_info()?.agent_latest_pubkey, entry.0.event.timestamp)?;

    match validation_outcome {
        ValidateCallbackResult::Valid => {}
        ValidateCallbackResult::Invalid(reason) => Err(wasm_error!(
            "Validation for private event failed: {}.",
            reason
        ))?,
        ValidateCallbackResult::UnresolvedDependencies(_) => Err(wasm_error!(
            "Could not create private event because of unresolved dependencies."
        ))?,
    };
    let entry_hash = hash_entry(&entry)?;

    internal_create_private_event::<T>(entry, false)?;

    Ok(entry_hash)
}

pub fn send_private_event_to_new_recipients(
    event_hash: EntryHash,
    recipients: Vec<AgentPubKey>,
) -> ExternResult<()> {
    let Some(private_event_entry) = query_private_event_entry(event_hash)? else {
        return Err(wasm_error!(
            "PrivateEventEntry with hash {event_hash} not found."
        ));
    };

    // Send to recipients
    info!("Sending private event entry to new recipients: {recipients:?}.");

    send_remote_signal(
        SerializedBytes::try_from(PrivateEventSourcingRemoteSignal::NewPrivateEvent(
            private_event_entry.clone(),
        ))
        .map_err(|err| wasm_error!(err))?,
        recipients.clone(),
    )?;
    for recipient in recipients {
        create_encrypted_message(recipient, private_event_entry.clone())?;
    }
    Ok(())
}

fn check_is_linked_device(agent: AgentPubKey) -> ExternResult<()> {
    let my_devices = query_my_linked_devices()?;
    if my_devices.contains(&agent) {
        Ok(())
    } else {
        Err(wasm_error!("Given agent is not a linked device."))
    }
}

pub fn validate_private_event_entry<T: PrivateEvent>(
    private_event_entry: &PrivateEventEntry,
) -> ExternResult<ValidateCallbackResult> {
    let valid = verify_signature(
        private_event_entry.0.author.clone(),
        private_event_entry.0.signature.clone(),
        &private_event_entry.0.event,
    )?;

    if !valid {
        return Ok(ValidateCallbackResult::Invalid(String::from(
            "Invalid private event entry: invalid signature.",
        )));
    }

    let private_event = T::try_from(private_event_entry.0.event.content.clone())
        .map_err(|err| wasm_error!("Failed to deserialize the private event: {err:?}."))?;

    private_event.validate(
        private_event_entry.0.author.clone(),
        private_event_entry.0.event.timestamp,
    )
}

pub fn receive_private_event<T: PrivateEvent>(
    provenance: AgentPubKey,
    private_event_entry: PrivateEventEntry,
) -> ExternResult<()> {
    debug!("[receive_private_event/start]");

    // check_is_linked_device(provenance)?;

    let outcome = validate_private_event_entry::<T>(&private_event_entry)?;

    match outcome {
        ValidateCallbackResult::Valid => {
            info!("Received a PrivateEvent.");
            internal_create_private_event::<T>(private_event_entry, true)?;
        }
        ValidateCallbackResult::UnresolvedDependencies(unresolved_dependencies) => {
            create_relaxed(EntryTypes::AwaitingDependencies(AwaitingDependencies {
                event: private_event_entry,
                unresolved_dependencies,
            }))?;
        }
        ValidateCallbackResult::Invalid(reason) => {
            return Err(wasm_error!("Invalid PrivateEvent: {:?}.", reason));
        }
    }
    Ok(())
}

pub fn receive_private_events<T: PrivateEvent>(
    provenance: AgentPubKey,
    private_event_entries: BTreeMap<EntryHashB64, PrivateEventEntry>,
) -> ExternResult<()> {
    // check_is_linked_device(provenance)?;

    let my_private_event_entries = query_private_event_entries(())?;

    let mut ordered_their_private_messenger_entries: Vec<(EntryHashB64, PrivateEventEntry)> =
        private_event_entries.into_iter().collect();

    ordered_their_private_messenger_entries
        .sort_by(|e1, e2| e1.1 .0.event.timestamp.cmp(&e2.1 .0.event.timestamp));

    for (entry_hash, private_event_entry) in ordered_their_private_messenger_entries {
        if my_private_event_entries.contains_key(&entry_hash) {
            // We already have this message committed
            continue;
        }

        let outcome = validate_private_event_entry::<T>(&private_event_entry)?;

        match outcome {
            ValidateCallbackResult::Valid => {
                info!("Received a PrivateEvent.");
                internal_create_private_event::<T>(private_event_entry, true)?;
            }
            ValidateCallbackResult::UnresolvedDependencies(unresolved_dependencies) => {
                create_relaxed(EntryTypes::AwaitingDependencies(AwaitingDependencies {
                    event: private_event_entry,
                    unresolved_dependencies,
                }))?;
            }
            ValidateCallbackResult::Invalid(reason) => {
                return Err(wasm_error!("Invalid PrivateEvent: {}.", reason));
            }
        }
    }
    Ok(())
}

pub(crate) fn internal_create_private_event<T: PrivateEvent>(
    private_event_entry: PrivateEventEntry,
    relaxed: bool,
) -> ExternResult<()> {
    let app_entry = EntryTypes::PrivateEvent(private_event_entry.clone());
    let action_hash = match relaxed {
        true => create_relaxed(app_entry.clone())?,
        false => create_entry(app_entry.clone())?,
    };
    let Some(record) = get(action_hash, GetOptions::local())? else {
        return Err(wasm_error!(
            "Unreachable: could not get the record that was just created."
        ));
    };
    emit_signal(Signal::EntryCreated {
        action: record.signed_action,
        app_entry: app_entry.clone(),
    })?;
    send_private_event_to_linked_devices_and_recipients::<T>(private_event_entry.clone())?;

    let private_event = T::try_from(private_event_entry.0.event.content)
        .map_err(|err| wasm_error!("Failed to deserialize private event: {err:?}."))?;
    private_event.post_commit(
        private_event_entry.0.author,
        private_event_entry.0.event.timestamp,
    )?;

    Ok(())
}

pub fn send_private_event_to_linked_devices_and_recipients<T: PrivateEvent>(
    private_event_entry: PrivateEventEntry,
) -> ExternResult<()> {
    let my_pub_key = agent_info()?.agent_latest_pubkey;

    // We are not the author, do nothing
    if private_event_entry.0.author.ne(&my_pub_key) {
        return Ok(());
    }

    let my_linked_devices = query_my_linked_devices()?;

    let private_event = T::try_from(private_event_entry.0.event.content.clone())
        .map_err(|err| wasm_error!("Failed to deserialize private event: {err:?}."))?;

    let recipients = private_event.recipients(
        private_event_entry.0.author.clone(),
        private_event_entry.0.event.timestamp,
    )?;

    send_remote_signal(
        SerializedBytes::try_from(PrivateEventSourcingRemoteSignal::NewPrivateEvent(
            private_event_entry.clone(),
        ))
        .map_err(|err| wasm_error!(err))?,
        my_linked_devices.clone(),
    )?;

    for linked_device in my_linked_devices {
        create_encrypted_message(linked_device, private_event_entry.clone())?;
    }

    // Send to recipients
    info!("Sending private event entry to recipients: {recipients:?}.");

    send_remote_signal(
        SerializedBytes::try_from(PrivateEventSourcingRemoteSignal::NewPrivateEvent(
            private_event_entry.clone(),
        ))
        .map_err(|err| wasm_error!(err))?,
        recipients.clone(),
    )?;
    for recipient in recipients {
        create_encrypted_message(recipient, private_event_entry.clone())?;
    }

    Ok(())
}

pub fn query_private_events<T: PrivateEvent>(
) -> ExternResult<BTreeMap<EntryHashB64, SignedEvent<T>>> {
    let private_events_entries = query_private_event_entries(())?;

    let private_events = private_events_entries
        .into_iter()
        .filter_map(|(entry_hash, entry)| {
            private_event_entry_to_signed_event(entry)
                .ok()
                .map(|e| (entry_hash, e))
        })
        .collect();

    Ok(private_events)
}

#[hdk_extern]
pub fn query_private_event_entries() -> ExternResult<BTreeMap<EntryHashB64, PrivateEventEntry>> {
    let filter = ChainQueryFilter::new()
        .entry_type(UnitEntryTypes::PrivateEvent.try_into()?)
        .include_entries(true)
        .action_type(ActionType::Create);
    let records = query(filter)?;
    let private_event_entries = records
        .into_iter()
        .map(|r| {
            let Some(entry_hash) = r.action().entry_hash() else {
                return Err(wasm_error!("PrivateEvents record contained no entry hash."));
            };
            let Some(entry) = r.entry().as_option().clone() else {
                return Err(wasm_error!("PrivateEvents record contained no entry."));
            };
            let entry = PrivateEventEntry::try_from(entry)?;
            Ok((entry_hash.clone().into(), entry))
        })
        .collect::<ExternResult<BTreeMap<EntryHashB64, PrivateEventEntry>>>()?;

    Ok(private_event_entries)
}

pub fn query_private_event_entry(event_hash: EntryHash) -> ExternResult<Option<PrivateEventEntry>> {
    let Some(record) = get(event_hash, GetOptions::local())? else {
        return Ok(None);
    };

    let Some(entry) = record.entry().as_option().clone() else {
        return Err(wasm_error!("PrivateEvents record contained no entry."));
    };
    let entry = PrivateEventEntry::try_from(entry)?;
    Ok(Some(entry))
}

fn private_event_entry_to_signed_event<T: PrivateEvent>(
    private_event_entry: PrivateEventEntry,
) -> ExternResult<SignedEvent<T>> {
    let private_event = T::try_from(private_event_entry.0.event.content)
        .map_err(|err| wasm_error!("Failed to deserialize private event: {err:?}."))?;
    Ok(SignedEvent {
        author: private_event_entry.0.author,
        signature: private_event_entry.0.signature,
        event: SignedContent {
            timestamp: private_event_entry.0.event.timestamp,
            content: private_event,
        },
    })
}

pub fn query_private_event<T: PrivateEvent>(
    event_hash: EntryHash,
) -> ExternResult<Option<SignedEvent<T>>> {
    let Some(private_event_entry) = query_private_event_entry(event_hash)? else {
        return Ok(None);
    };
    let signed_event = private_event_entry_to_signed_event(private_event_entry)?;
    Ok(Some(signed_event))
}
