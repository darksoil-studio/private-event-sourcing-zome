use hdk::prelude::*;
use private_event_sourcing_integrity::*;

use crate::{
    agent_encrypted_message::create_encrypted_message, linked_devices::query_my_linked_devices,
    private_event::query_private_event_entries, PrivateEventSourcingRemoteSignal,
};

#[hdk_extern]
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

        for (_, private_event_entry) in missing_private_event_entries {
            send_remote_signal(
                SerializedBytes::try_from(PrivateEventSourcingRemoteSignal::NewPrivateEvent(
                    private_event_entry.clone(),
                ))
                .map_err(|err| wasm_error!(err))?,
                vec![agent.clone()],
            )?;

            create_encrypted_message(agent.clone(), private_event_entry)?;
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
