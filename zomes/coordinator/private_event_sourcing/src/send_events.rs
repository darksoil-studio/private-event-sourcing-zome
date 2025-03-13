use std::collections::BTreeMap;

use hdk::prelude::*;
use private_event_sourcing_integrity::{
    EncryptedMessage, EventHistorySummary, PrivateEventEntry, UnitEntryTypes,
};

use crate::{
    agent_encrypted_message::{get_agent_encrypted_messages, get_message},
    create_encrypted_message, query_my_linked_devices, query_private_event_entries, PrivateEvent,
    PrivateEventSourcingRemoteSignal,
};

pub fn send_events<T: PrivateEvent>(events_hashes: BTreeSet<EntryHash>) -> ExternResult<()> {
    let entries = query_private_event_entries(())?;

    let my_pub_key = agent_info()?.agent_latest_pubkey;
    let now = sys_time()?.as_millis();

    let filtered_entries: BTreeMap<EntryHashB64, PrivateEventEntry> = entries
        .into_iter()
        .filter(|(key, _value)| events_hashes.contains(&EntryHash::from(key.clone())))
        .filter(|(_key, value)| {
            let elapsed = now - value.0.event.timestamp.as_millis();
            let entry_was_committed_less_than_10_seconds_ago = elapsed < 10 * 1000;
            value.0.author.eq(&my_pub_key) || !entry_was_committed_less_than_10_seconds_ago
        })
        .collect();

    send_events_to_linked_devices_and_recipients::<T>(filtered_entries)
}

pub fn send_events_to_linked_devices_and_recipients<T: PrivateEvent>(
    events: BTreeMap<EntryHashB64, PrivateEventEntry>,
) -> ExternResult<()> {
    debug!(
        "[send_events] Sending events to linked devices and recipients: {:?}",
        events.keys()
    );

    if events.is_empty() {
        return Ok(());
    }

    let my_linked_devices = query_my_linked_devices()?;

    let mut events_for_recipients: BTreeMap<AgentPubKey, BTreeSet<EntryHashB64>> = BTreeMap::new();

    for (event_hash, private_event_entry) in events.iter() {
        let private_event = T::try_from(private_event_entry.0.event.content.clone())
            .map_err(|err| wasm_error!("Failed to deserialize private event."))?;

        let recipients_result = private_event.recipients(
            event_hash.clone().into(),
            private_event_entry.0.author.clone(),
            private_event_entry.0.event.timestamp,
        );
        let Ok(mut recipients) = recipients_result else {
            warn!("Error calling PrivateEvent::recipients()");
            continue;
        };
        recipients.append(&mut my_linked_devices.clone());

        for recipient in recipients {
            events_for_recipients
                .entry(recipient)
                .or_insert(BTreeSet::new())
                .insert(event_hash.clone());
        }
    }

    for (recipient, events_hashes_to_send) in events_for_recipients {
        let existing_private_event_entries_hashes: BTreeSet<EntryHash> =
            get_private_events_already_sent_to(&recipient)?;

        let missing_private_entry_hashes: BTreeSet<EntryHashB64> = events_hashes_to_send
            .into_iter()
            .filter(|entry_hash| {
                !existing_private_event_entries_hashes.contains(&entry_hash.clone().into())
            })
            .collect();

        let events_to_send: BTreeMap<EntryHashB64, PrivateEventEntry> =
            missing_private_entry_hashes
                .into_iter()
                .filter_map(|event_hash| {
                    let Some(event) = events.get(&event_hash) else {
                        return None;
                    };
                    Some((event_hash, event.clone()))
                })
                .collect();

        info!("Sending private events entry to recipient: {recipient:?}.");

        send_remote_signal(
            SerializedBytes::try_from(PrivateEventSourcingRemoteSignal::SendPrivateEvents(
                events_to_send.clone(),
            ))
            .map_err(|err| wasm_error!(err))?,
            vec![recipient.clone()],
        )?;

        create_encrypted_message(recipient, events_to_send)?;
    }

    Ok(())
}

pub fn synchronize_all_entries<T: PrivateEvent>() -> ExternResult<()> {
    let private_event_entries = query_private_event_entries(())?;
    send_events::<T>(private_event_entries.into_keys().map(Into::into).collect())
}

fn get_private_events_already_sent_to(agent: &AgentPubKey) -> ExternResult<BTreeSet<EntryHash>> {
    let filter = ChainQueryFilter::new().entry_type(UnitEntryTypes::PrivateEvent.try_into()?);
    let activity = get_agent_activity(agent.clone(), filter, ActivityRequest::Full)?;

    let get_inputs: Vec<GetInput> = activity
        .valid_activity
        .into_iter()
        .map(|(_, action_hash)| GetInput::new(action_hash.into(), GetOptions::network()))
        .collect();
    let maybe_actions = HDK.with(|hdk| hdk.borrow().get(get_inputs))?;
    let mut entry_hashes: BTreeSet<EntryHash> = maybe_actions
        .into_iter()
        .filter_map(|r| r)
        .filter_map(|r| match r.action() {
            Action::Create(create) => Some(create.entry_hash.clone()),
            _ => None,
        })
        .collect();

    let filter =
        ChainQueryFilter::new().entry_type(UnitEntryTypes::EventHistorySummary.try_into()?);
    let activity = get_agent_activity(agent.clone(), filter, ActivityRequest::Full)?;

    let get_inputs: Vec<GetInput> = activity
        .valid_activity
        .into_iter()
        .map(|(_, action_hash)| GetInput::new(action_hash.into(), GetOptions::network()))
        .collect();
    let maybe_records = HDK.with(|hdk| hdk.borrow().get(get_inputs))?;
    let mut entry_hashes_from_summaries: BTreeSet<EntryHash> = maybe_records
        .into_iter()
        .filter_map(|r| r)
        .filter_map(|r| {
            let entry = r.entry().as_option().clone()?;
            let event_history_summary = EventHistorySummary::try_from(entry).ok()?;
            Some(event_history_summary.events_hashes)
        })
        .flatten()
        .collect();

    entry_hashes.append(&mut entry_hashes_from_summaries);

    let links = get_agent_encrypted_messages(agent.clone())?;

    let encrypted_messages: Vec<EncryptedMessage> = links
        .into_iter()
        .map(|link| get_message(&link))
        .collect::<ExternResult<Vec<Option<EncryptedMessage>>>>()?
        .into_iter()
        .filter_map(|maybe_encrypted_message| maybe_encrypted_message)
        .collect();

    let mut messages_sent_entry_hashes: BTreeSet<EntryHash> = encrypted_messages
        .into_iter()
        .map(|message| message.entry_hashes)
        .flatten()
        .map(|entry_hash| EntryHash::from(entry_hash))
        .collect();
    entry_hashes.append(&mut messages_sent_entry_hashes);

    Ok(entry_hashes)
}
