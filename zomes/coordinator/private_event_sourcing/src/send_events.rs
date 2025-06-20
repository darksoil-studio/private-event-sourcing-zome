use std::collections::BTreeMap;

use hdk::prelude::*;
use private_event_sourcing_integrity::{EntryTypes, EventsSentToRecipients, PrivateEventEntry};

use crate::{
    acknowledgements::query_acknowledgements_by_agents,
    events_sent_to_recipients::query_events_sent_to_recipients, query_my_linked_devices,
    query_private_event_entries, utils::create_relaxed, PrivateEvent,
    PrivateEventSourcingRemoteSignal,
};

pub fn send_events<T: PrivateEvent>() -> ExternResult<()> {
    debug!("[send_events] Sending events to linked devices and recipients if necessary.");

    let entries = query_private_event_entries(())?;
    let events_sent_to_recipients = query_events_sent_to_recipients()?;
    let acknowledgements = query_acknowledgements_by_agents()?;

    let my_linked_devices = query_my_linked_devices()?;

    let now = sys_time()?;

    let mut new_events_sent_to_recipients: BTreeMap<EntryHash, BTreeSet<AgentPubKey>> =
        BTreeMap::new();

    for (event_hash, private_event_entry) in entries {
        let private_event = T::try_from(private_event_entry.0.event.content.clone())
            .map_err(|_err| wasm_error!("Failed to deserialize private event"))?;

        // For each event, get the recipients
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

        // Filter out the events with acknowledgements from all recipients
        let recipients_without_acknowledgement: BTreeSet<AgentPubKey> = recipients
            .into_iter()
            .filter(|recipient| {
                !acknowledgements
                    .get(recipient)
                    .cloned()
                    .unwrap_or_default()
                    .contains(&EntryHash::from(event_hash.clone()))
            })
            .collect();

        // If the event was never sent or the last time it was sent was more than X, send it
        let recipients_to_send: BTreeSet<AgentPubKey> = recipients_without_acknowledgement
            .into_iter()
            .filter(|recipient| {
                let events_sent_for_recipient = events_sent_to_recipients
                    .get(&EntryHash::from(event_hash.clone()))
                    .cloned()
                    .unwrap_or_default();

                match events_sent_for_recipient.get(recipient) {
                    Some(last_sent) => {
                        now.as_millis() - last_sent.as_millis() > 1000 * 60 * 60 * 24
                    }
                    None => true,
                }
            })
            .collect();

        if !recipients_to_send.is_empty() {
            info!(
                "Sending private events entry to recipients: {:?}.",
                recipients_to_send
            );

            let mut events_to_send: BTreeMap<EntryHashB64, PrivateEventEntry> = BTreeMap::new();
            events_to_send.insert(event_hash.clone(), private_event_entry.clone());

            send_remote_signal(
                SerializedBytes::try_from(PrivateEventSourcingRemoteSignal::SendPrivateEvents(
                    events_to_send.clone(),
                ))
                .map_err(|err| wasm_error!(err))?,
                recipients_to_send.clone().into_iter().collect(),
            )?;

            T::send_message(recipients_to_send.clone(), private_event_entry)?;

            new_events_sent_to_recipients.insert(EntryHash::from(event_hash), recipients_to_send);
        }
    }

    if new_events_sent_to_recipients.len() > 0 {
        create_relaxed(EntryTypes::EventsSentToRecipients(EventsSentToRecipients {
            events_sent_to_recipients: new_events_sent_to_recipients,
            timestamp: now,
        }))?;
    }

    Ok(())
}

#[hdk_extern]
pub fn synchronize_with_linked_device(linked_device: AgentPubKey) -> ExternResult<()> {
    let entries = query_private_event_entries(())?;

    send_remote_signal(
        SerializedBytes::try_from(PrivateEventSourcingRemoteSignal::SendPrivateEvents(entries))
            .map_err(|err| wasm_error!(err))?,
        vec![linked_device],
    )?;

    Ok(())
}
