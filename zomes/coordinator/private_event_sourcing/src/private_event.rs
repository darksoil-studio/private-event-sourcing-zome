use hdk::prelude::*;
use private_event_sourcing_integrity::*;
use std::collections::BTreeMap;

use crate::{linked_devices::query_my_linked_devices, send_events, utils::create_relaxed, Signal};

pub trait EventType {
    fn event_type(&self) -> String;
}

pub trait PrivateEvent:
    EventType + Clone + TryFrom<SerializedBytes> + TryInto<SerializedBytes>
{
    /// Whether the given entry is to be accepted in to our source chain
    fn validate(
        &self,
        event_hash: EntryHash,
        author: AgentPubKey,
        timestamp: Timestamp,
    ) -> ExternResult<ValidateCallbackResult>;

    /// The agents other than the linked devices for the author that are suposed to receive this entry
    fn recipients(
        &self,
        event_hash: EntryHash,
        author: AgentPubKey,
        timestamp: Timestamp,
    ) -> ExternResult<BTreeSet<AgentPubKey>>;

    /// Code to run after an event has been committed
    fn post_commit(
        &self,
        entry_hash: EntryHash,
        author: AgentPubKey,
        timestamp: Timestamp,
    ) -> ExternResult<()> {
        Ok(())
    }
}

fn build_private_event_entry<T: PrivateEvent>(
    private_event: T,
    timestamp: Timestamp,
) -> ExternResult<PrivateEventEntry> {
    let event_type = private_event.event_type();
    let bytes: SerializedBytes = private_event
        .try_into()
        .map_err(|_err| wasm_error!("Failed to serialize private event."))?;

    let signed: SignedContent<SerializedBytes> = SignedContent {
        content: bytes,
        timestamp,
        event_type,
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
    let timestamp = sys_time()?;
    let entry = build_private_event_entry(private_event.clone(), timestamp)?;
    let entry_hash = hash_entry(&entry)?;
    let validation_outcome = private_event.validate(
        entry_hash,
        agent_info()?.agent_latest_pubkey,
        timestamp.clone(),
    )?;

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

    internal_create_private_event::<T>(entry)
}

pub fn send_private_events_to_new_recipients<T: PrivateEvent>(
    events_hashes: BTreeSet<EntryHash>,
) -> ExternResult<()> {
    send_events::<T>(events_hashes)
}

pub fn validate_private_event_entry<T: PrivateEvent>(
    entry_hash: EntryHash,
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
    let expected_entry_hash = hash_entry(private_event_entry)?;

    if expected_entry_hash.ne(&entry_hash) {
        return Ok(ValidateCallbackResult::Invalid(String::from(
            "Invalid private event entry: invalid entry hash.",
        )));
    }

    let private_event = T::try_from(private_event_entry.0.event.content.clone())
        .map_err(|err| wasm_error!("Failed to deserialize the private event."))?;

    if private_event
        .event_type()
        .ne(&private_event_entry.0.event.event_type)
    {
        return Ok(ValidateCallbackResult::Invalid(format!(
            "Invalid event type: expected '{}', but got '{}'.",
            private_event_entry.0.event.event_type,
            private_event.event_type()
        )));
    }

    private_event.validate(
        entry_hash,
        private_event_entry.0.author.clone(),
        private_event_entry.0.event.timestamp,
    )
}

pub fn receive_private_events<T: PrivateEvent>(
    provenance: AgentPubKey,
    private_event_entries: BTreeMap<EntryHashB64, PrivateEventEntry>,
) -> ExternResult<()> {
    debug!("[receive_private_events/start]");
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

        let outcome =
            validate_private_event_entry::<T>(entry_hash.clone().into(), &private_event_entry);

        match outcome {
            Ok(ValidateCallbackResult::Valid) => {
                info!("Received a PrivateEvent {entry_hash}.");
                internal_create_private_event::<T>(private_event_entry)?;
            }
            Ok(ValidateCallbackResult::Invalid(reason)) => {
                warn!("Received an invalid PrivateEvent {entry_hash}: discarding.");
                return Err(wasm_error!(
                    "Invalid PrivateEvent {}: {}.",
                    entry_hash,
                    reason
                ));
            }
            Ok(ValidateCallbackResult::UnresolvedDependencies(unresolved_dependencies)) => {
                info!(
                    "Received a PrivateEvent {entry_hash} but we don't have all its dependencies: adding it to the awaiting dependencies queue."
                );
                create_relaxed(EntryTypes::AwaitingDependencies(AwaitingDependencies {
                    event: private_event_entry,
                    unresolved_dependencies,
                }))?;
            }
            Err(_) => {
                info!(
                    "Received a PrivateEvent {entry_hash} but its validation failed: adding it to the awaiting dependencies queue."
                );
                create_relaxed(EntryTypes::AwaitingDependencies(AwaitingDependencies {
                    event: private_event_entry,
                    unresolved_dependencies: UnresolvedDependencies::Hashes(vec![]),
                }))?;
            }
        }
    }
    Ok(())
}

pub(crate) fn internal_create_private_event<T: PrivateEvent>(
    private_event_entry: PrivateEventEntry,
) -> ExternResult<EntryHash> {
    let entry_hash = hash_entry(&private_event_entry)?;
    let app_entry = EntryTypes::PrivateEvent(private_event_entry);
    let action_hash = create_relaxed(app_entry.clone())?;
    let Some(record) = get(action_hash, GetOptions::local())? else {
        return Err(wasm_error!(
            "Unreachable: could not get the record that was just created."
        ));
    };
    emit_signal(Signal::EntryCreated {
        action: record.signed_action,
        app_entry,
    })?;
    // send_private_event_to_linked_devices_and_recipients::<T>(
    //     entry_hash.clone(),
    //     private_event_entry.clone(),
    // )?;

    // let private_event = T::try_from(private_event_entry.0.event.content.clone())
    //     .map_err(|err| wasm_error!("Failed to deserialize private event."))?;
    // private_event.post_commit(
    //     entry_hash.clone(),
    //     private_event_entry.0.author,
    //     private_event_entry.0.event.timestamp,
    // )?;

    Ok(entry_hash)
}

pub fn query_private_event_entries_by_type(
    event_type: &String,
) -> ExternResult<BTreeMap<EntryHashB64, PrivateEventEntry>> {
    let all_entries = query_private_event_entries(())?;

    let entries_of_this_type = all_entries
        .into_iter()
        .filter(|(_hash, entry)| entry.0.event.event_type.eq(event_type))
        .collect();
    Ok(entries_of_this_type)
}

pub fn query_private_events_by_type<T: PrivateEvent>(
    event_type: &String,
) -> ExternResult<BTreeMap<EntryHashB64, SignedEvent<T>>> {
    let private_events_entries = query_private_event_entries_by_type(event_type)?;

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
    let mut private_event_entries = records
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

    let mut histories = query_event_histories()?;

    for history in &mut histories {
        private_event_entries.append(&mut history.events);
    }

    Ok(private_event_entries)
}

pub fn query_event_histories() -> ExternResult<Vec<EventHistory>> {
    let filter = ChainQueryFilter::new()
        .entry_type(UnitEntryTypes::EventHistory.try_into()?)
        .include_entries(true)
        .action_type(ActionType::Create);
    let records = query(filter)?;
    let event_histories = records
        .into_iter()
        .map(|r| {
            let Some(entry) = r.entry().as_option().clone() else {
                return Err(wasm_error!("PrivateEvents record contained no entry."));
            };
            let entry = EventHistory::try_from(entry)?;
            Ok(entry)
        })
        .collect::<ExternResult<Vec<EventHistory>>>()?;

    Ok(event_histories)
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

pub fn private_event_entry_to_signed_event<T: PrivateEvent>(
    private_event_entry: PrivateEventEntry,
) -> ExternResult<SignedEvent<T>> {
    let private_event = T::try_from(private_event_entry.0.event.content)
        .map_err(|err| wasm_error!("Failed to deserialize private event."))?;
    Ok(SignedEvent {
        author: private_event_entry.0.author,
        signature: private_event_entry.0.signature,
        event: SignedContent {
            event_type: private_event_entry.0.event.event_type,
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

fn check_is_linked_device(agent: AgentPubKey) -> ExternResult<()> {
    let my_devices = query_my_linked_devices()?;
    if my_devices.contains(&agent) {
        Ok(())
    } else {
        Err(wasm_error!("Given agent is not a linked device."))
    }
}
