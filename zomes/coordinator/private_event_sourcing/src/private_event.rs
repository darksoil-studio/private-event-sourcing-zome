use hdk::prelude::*;
use private_event_sourcing_integrity::{PrivateEventContent, *};
use serde::de::DeserializeOwned;
use std::collections::BTreeMap;

use crate::{
    linked_devices::query_my_linked_devices, query_event_histories,
    send_acknowledgement_for_event_to_recipient, utils::create_relaxed, Signal,
};

pub trait EventType {
    fn event_type(&self) -> String;
}

pub trait PrivateEvent:
    EventType
    + Clone
    + DeserializeOwned
    + Serialize
    + std::fmt::Debug
    + TryFrom<SerializedBytes>
    + TryInto<SerializedBytes>
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

    /// Whether creating this event adds new recipients to old events
    /// When this is true, the recipients for all existing events will be recalculated,
    /// and the events with new recipients will be sent to these new recipients
    fn adds_new_recipients_for_other_events(
        &self,
        event_hash: EntryHash,
        author: AgentPubKey,
        timestamp: Timestamp,
    ) -> ExternResult<bool>;
}

pub fn create_private_event<T: PrivateEvent>(private_event: T) -> ExternResult<EntryHash> {
    let event_bytes: SerializedBytes = private_event
        .clone()
        .try_into()
        .map_err(|_err| wasm_error!("Failed to serialize."))?;
    let signed = SignedEntry::build(PrivateEventContent {
        event_type: private_event.event_type(),
        event: event_bytes,
    })?;
    let author = signed.author.clone();
    let timestamp = signed.payload.timestamp.clone();
    let private_event_entry = PrivateEventEntry(signed);

    let entry_hash = hash_entry(&private_event_entry)?;
    let validation_outcome = private_event.validate(entry_hash, author, timestamp)?;

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

    let entry_hash = hash_entry(&private_event_entry)?;
    let app_entry = EntryTypes::PrivateEvent(private_event_entry.clone());
    create_relaxed(app_entry)?;
    emit_signal(Signal::NewPrivateEvent {
        event_hash: entry_hash.clone(),
        private_event_entry: private_event_entry.clone(),
    })?;

    Ok(entry_hash)
}

pub fn validate_private_event_entry<T: PrivateEvent>(
    private_event_entry: &PrivateEventEntry,
) -> ExternResult<ValidateCallbackResult> {
    let signed_valid = private_event_entry.0.verify()?;

    if !signed_valid {
        return Ok(ValidateCallbackResult::Invalid(String::from(
            "Invalid private event entry: invalid signature.",
        )));
    }

    let private_event = T::try_from(private_event_entry.0.payload.content.event.clone())
        .map_err(|_err| wasm_error!("Failed to deserialize the private event."))?;

    if private_event
        .event_type()
        .ne(&private_event_entry.0.payload.content.event_type)
    {
        return Ok(ValidateCallbackResult::Invalid(format!(
            "Invalid event type: expected '{}', but got '{}'.",
            private_event_entry.0.payload.content.event_type,
            private_event.event_type()
        )));
    }

    let entry_hash = hash_entry(private_event_entry)?;

    private_event.validate(
        entry_hash,
        private_event_entry.0.author.clone(),
        private_event_entry.0.payload.timestamp,
    )
}

pub fn receive_private_events<T: PrivateEvent>(
    my_private_event_entries: &BTreeMap<EntryHashB64, PrivateEventEntry>,
    provenance: AgentPubKey,
    private_event_entries: Vec<PrivateEventEntry>,
) -> ExternResult<BTreeMap<EntryHashB64, PrivateEventEntry>> {
    debug!("[receive_private_events/start]");
    // check_is_linked_device(provenance)?;

    let mut ordered_their_private_event_entries: Vec<PrivateEventEntry> = private_event_entries;
    ordered_their_private_event_entries.sort_by_key(|e| e.0.payload.timestamp);

    let my_pub_key = agent_info()?.agent_initial_pubkey;

    let mut new_entries: BTreeMap<EntryHashB64, PrivateEventEntry> = BTreeMap::new();

    for private_event_entry in ordered_their_private_event_entries {
        let entry_hash = EntryHashB64::from(hash_entry(&private_event_entry)?);
        if let Some(event) = my_private_event_entries.get(&entry_hash) {
            // We already have this event
            if event.0.author.ne(&my_pub_key) {
                send_acknowledgement_for_event_to_recipient::<T>(&entry_hash, &provenance)?;
            }
            continue;
        }

        let outcome = validate_private_event_entry::<T>(&private_event_entry);

        match outcome {
            Ok(ValidateCallbackResult::Valid) => {
                let app_entry = EntryTypes::PrivateEvent(private_event_entry.clone());
                create_relaxed(app_entry)?;
                info!("Received a PrivateEvent {entry_hash}.");
                new_entries.insert(entry_hash, private_event_entry);
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
                warn!(
                    "Received a PrivateEvent {entry_hash} but we don't have all its dependencies: adding it to the awaiting dependencies queue."
                );
                create_relaxed(EntryTypes::AwaitingDependencies(
                    AwaitingDependencies::Event {
                        event: private_event_entry,
                        unresolved_dependencies,
                    },
                ))?;
            }
            Err(_) => {
                warn!(
                    "Received a PrivateEvent {entry_hash} but its validation failed: adding it to the awaiting dependencies queue."
                );
                create_relaxed(EntryTypes::AwaitingDependencies(
                    AwaitingDependencies::Event {
                        event: private_event_entry,
                        unresolved_dependencies: UnresolvedDependencies::Hashes(vec![]),
                    },
                ))?;
            }
        }
    }
    Ok(new_entries)
}

pub fn query_private_event_entries_by_type(
    event_type: &String,
) -> ExternResult<BTreeMap<EntryHashB64, PrivateEventEntry>> {
    let all_entries = query_private_event_entries(())?;

    let entries_of_this_type = all_entries
        .into_iter()
        .filter(|(_hash, entry)| entry.0.payload.content.event_type.eq(event_type))
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
) -> ExternResult<SignedEntry<PrivateEventContent<T>>> {
    let private_event = T::try_from(private_event_entry.0.payload.content.event)
        .map_err(|_err| wasm_error!("Failed to deserialize private event."))?;
    Ok(SignedEntry {
        author: private_event_entry.0.author,
        signature: private_event_entry.0.signature,
        payload: SignedContent {
            timestamp: private_event_entry.0.payload.timestamp,
            content: PrivateEventContent {
                event_type: private_event_entry.0.payload.content.event_type,
                event: private_event,
            },
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
