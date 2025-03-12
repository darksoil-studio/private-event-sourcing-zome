use std::collections::BTreeMap;

use hdk::prelude::*;
use private_event_sourcing_integrity::*;

use crate::{
    agent_encrypted_message::{create_encrypted_message, get_agent_encrypted_messages},
    linked_devices::query_my_linked_devices,
    private_event::query_private_event_entries,
    private_event_entry_to_signed_event, query_all_my_agents, PrivateEvent,
    PrivateEventSourcingRemoteSignal,
};

pub fn synchronize_with_linked_devices() -> ExternResult<()> {
    let my_pub_key = agent_info()?.agent_latest_pubkey;

    let agents = query_my_linked_devices()?;
    let other_agents = agents.into_iter().filter(|a| a.ne(&my_pub_key));

    let private_event_entries = query_private_event_entries(())?;

    let private_event_entries_unit_entry_types: EntryType =
        UnitEntryTypes::PrivateEvent.try_into()?;

    let my_private_event_entries_hashes = private_event_entries.keys();

    for agent in other_agents {
        let private_event_entries_agent_activity = get_agent_activity(
            agent.clone(),
            ChainQueryFilter::new()
                .entry_type(private_event_entries_unit_entry_types.clone())
                .clone(),
            ActivityRequest::Full,
        )?;

        let actions_get_inputs = private_event_entries_agent_activity
            .valid_activity
            .into_iter()
            .map(|(_, action_hash)| GetInput::new(action_hash.into(), GetOptions::network()))
            .collect();

        let records = HDK.with(|hdk| hdk.borrow().get(actions_get_inputs))?;
        let existing_private_event_entries_hashes: HashSet<EntryHash> = records
            .into_iter()
            .filter_map(|r| r)
            .filter_map(|r| r.action().entry_hash().cloned())
            .collect();

        let missing_private_entry_hashes: Vec<EntryHashB64> = my_private_event_entries_hashes
            .clone()
            .cloned()
            .filter(|entry_hash| {
                !existing_private_event_entries_hashes.contains(&entry_hash.clone().into())
            })
            .collect();

        let mut missing_private_event_entries: Vec<(EntryHashB64, PrivateEventEntry)> = Vec::new();

        for entry_hash in missing_private_entry_hashes {
            let Some(entry) = private_event_entries.get(&entry_hash) else {
                return Err(wasm_error!("Unreachable: QueriedPrivateMessengerEntries entries did not contain one of its entry hashes."));
            };
            missing_private_event_entries.push((entry_hash.clone(), entry.clone()));
        }

        for (entry_hash, private_event_entry) in missing_private_event_entries {
            send_remote_signal(
                SerializedBytes::try_from(PrivateEventSourcingRemoteSignal::NewPrivateEvent(
                    private_event_entry.clone(),
                ))
                .map_err(|err| wasm_error!(err))?,
                vec![agent.clone()],
            )?;

            create_encrypted_message(agent.clone(), entry_hash.into(), private_event_entry)?;
        }
    }

    Ok(())
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

    let mut messages_sent_entry_hashes: BTreeSet<EntryHash> = links
        .into_iter()
        .filter_map(|link| link.target.into_entry_hash())
        .collect();
    entry_hashes.append(&mut messages_sent_entry_hashes);

    Ok(entry_hashes)
}

pub fn syncronize_with_recipients<T: PrivateEvent>() -> ExternResult<()> {
    let private_event_entries = query_private_event_entries(())?;
    let all_my_agents = query_all_my_agents()?;

    let mut agent_private_events: BTreeMap<AgentPubKey, BTreeSet<EntryHash>> = BTreeMap::new();

    for (event_hash_b64, private_event_entry) in private_event_entries {
        let private_event_bytes = SerializedBytes::try_from(
            PrivateEventSourcingRemoteSignal::NewPrivateEvent(private_event_entry.clone()),
        )
        .map_err(|err| wasm_error!(err))?;
        let signed_event = private_event_entry_to_signed_event::<T>(private_event_entry.clone())?;

        let recipients: BTreeSet<AgentPubKey> = signed_event
            .event
            .content
            .recipients(
                event_hash_b64.clone().into(),
                signed_event.author,
                signed_event.event.timestamp,
            )?
            .into_iter()
            .filter(|recipient| !all_my_agents.contains(&recipient))
            .collect();

        let event_hash = EntryHash::from(event_hash_b64);

        for recipient in recipients {
            if !agent_private_events.contains_key(&recipient) {
                agent_private_events.insert(
                    recipient.clone(),
                    get_private_events_already_sent_to(&recipient)?,
                );
            }

            let Some(entry_hashes) = agent_private_events.get(&recipient) else {
                return Err(wasm_error!("Unreachable: agent_private_events is None"));
            };

            if !entry_hashes.contains(&event_hash) {
                info!(
                    "Sending private event of type {} to new recipient {}.",
                    signed_event.event.content.event_type(),
                    recipient
                );
                send_remote_signal(&private_event_bytes, vec![recipient.clone()])?;

                create_encrypted_message(
                    recipient.clone(),
                    event_hash.clone(),
                    private_event_entry.clone(),
                )?;
            }
        }
    }

    Ok(())
}

#[hdk_extern]
pub fn synchronize_with_linked_device(linked_device: AgentPubKey) -> ExternResult<()> {
    let entries = query_private_event_entries(())?;

    send_remote_signal(
        SerializedBytes::try_from(
            PrivateEventSourcingRemoteSignal::SynchronizeEntriesWithLinkedDevice(entries),
        )
        .map_err(|err| wasm_error!(err))?,
        vec![linked_device],
    )?;

    Ok(())
}
