use hdk::prelude::*;
use private_event_sourcing_integrity::{
    EntryTypes, EventSentToRecipients, EventSentToRecipientsContent, Message, SignedEntry,
};

use crate::{
    acknowledgements::query_acknowledgements_by_agents, attempt_commit_awaiting_deps_entries, create_acknowledgements, events_sent_to_recipients::{
        query_events_sent_to_recipients, query_events_sent_to_recipients_entries,
    }, query_acknowledgement_entries, query_my_linked_devices, query_private_event_entries, query_private_event_entry, send_async_message, utils::create_relaxed, PrivateEvent, PrivateEventSourcingRemoteSignal
};

const INTERVAL_RESEND_MS: i64 = 1000 * 60 * 60 * 24 * 1000; // 1000 days

pub fn resend_events_if_necessary<T: PrivateEvent>() -> ExternResult<()> {
    debug!("[send_events] Sending events to linked devices and recipients if necessary.");

    let entries = query_private_event_entries(())?;
    let events_sent_to_recipients = query_events_sent_to_recipients()?;
    let acknowledgements = query_acknowledgements_by_agents()?;
    let events_sent_to_recipients_entries = query_events_sent_to_recipients_entries(())?;
    let acknowledgements_entries = query_acknowledgement_entries(())?;

    let my_linked_devices = query_my_linked_devices()?;

    let now = sys_time()?;

    let my_pub_key = agent_info()?.agent_initial_pubkey;

    for (event_hash, private_event_entry) in entries {
        let private_event = T::try_from(private_event_entry.0.payload.content.event.clone())
            .map_err(|_err| wasm_error!("Failed to deserialize private event"))?;

        // For each event, get the recipients
        let recipients_result = private_event.recipients(
            event_hash.clone().into(),
            private_event_entry.0.author.clone(),
            private_event_entry.0.payload.timestamp,
        );
        let Ok(mut recipients) = recipients_result else {
            warn!("Error calling PrivateEvent::recipients()");
            continue;
        };
        recipients.append(&mut my_linked_devices.clone());

        // Filter out the events with acknowledgements from all recipients
        let recipients_without_acknowledgement: BTreeSet<AgentPubKey> = recipients
            .into_iter()
            .filter(|recipient| my_pub_key.ne(recipient)) // Filter me out
            .filter(|recipient| private_event_entry.0.author.ne(recipient)) // Filter authors out
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
                    Some(last_sent) => now.as_millis() - last_sent.as_millis() > INTERVAL_RESEND_MS,
                    None => true,
                }
            })
            .collect();

        if !recipients_to_send.is_empty() {
            info!(
                "Sending private event entry {} to recipients: {:?}.",
                event_hash, recipients_to_send
            );

            let content = EventSentToRecipientsContent {
                event_hash: event_hash.clone().into(),
                recipients: recipients_to_send.clone(),
            };
            let signed = SignedEntry::build(content)?;
            let event_sent_to_recipients = EventSentToRecipients(signed);

            let acknowledgements_for_this_entry = acknowledgements_entries
                .iter()
                .filter(|ack| {
                    ack.0
                        .payload
                        .content
                        .private_event_hash
                        .eq(&EntryHash::from(event_hash.clone()))
                })
                .cloned()
                .collect();

            let mut events_sent_to_recipients_for_this_entry: Vec<EventSentToRecipients> =
                events_sent_to_recipients_entries
                    .iter()
                    .filter(|event_sent_to_recipients| {
                        event_sent_to_recipients
                            .0
                            .payload
                            .content
                            .event_hash
                            .eq(&EntryHash::from(event_hash.clone()))
                    })
                    .cloned()
                    .collect();

            events_sent_to_recipients_for_this_entry.push(event_sent_to_recipients.clone());

            let message = Message {
                private_events: vec![private_event_entry],
                events_sent_to_recipients: events_sent_to_recipients_for_this_entry,
                acknowledgements: acknowledgements_for_this_entry,
            };

            send_remote_signal(
                SerializedBytes::try_from(PrivateEventSourcingRemoteSignal::SendMessage(
                    message.clone(),
                ))
                .map_err(|err| wasm_error!(err))?,
                recipients_to_send.clone().into_iter().collect(),
            )?;

            if let Ok(()) = send_async_message(
                recipients_to_send.clone(),
                EntryHashB64::from(event_hash).to_string(),
                message,
            ) {
                create_relaxed(EntryTypes::EventSentToRecipients(event_sent_to_recipients))?;
            }
        }
    }

    Ok(())
}

pub fn send_new_events<T: PrivateEvent>(event_hashes: BTreeSet<EntryHash>) -> ExternResult<()> {
    info!("[send_events] Sending new events: {:?}.", event_hashes);

    let my_linked_devices = query_my_linked_devices()?;

    let my_pub_key = agent_info()?.agent_initial_pubkey;

    for event_hash in event_hashes {
        let Some(private_event_entry) = query_private_event_entry(event_hash.clone())? else {
            error!("Could not find private event entry: {}.", event_hash);
            continue;
        };

        if private_event_entry.0.author.ne(&my_pub_key) {
            // We don't need to directly send to all recipients another author's event
            continue;
        };

        let private_event = T::try_from(private_event_entry.0.payload.content.event.clone())
            .map_err(|_err| wasm_error!("Failed to deserialize private event"))?;

        // For each event, get the recipients
        let recipients_result = private_event.recipients(
            event_hash.clone().into(),
            private_event_entry.0.author.clone(),
            private_event_entry.0.payload.timestamp,
        );
        let Ok(mut recipients) = recipients_result else {
            warn!("Error calling PrivateEvent::recipients()");
            continue;
        };
        recipients.append(&mut my_linked_devices.clone());

        let recipients: BTreeSet<AgentPubKey> = recipients
            .into_iter()
            .filter(|recipient| my_pub_key.ne(recipient)) // Filter me out
            .collect();

        if !recipients.is_empty() {
            info!(
                "Sending private event entry {} to recipients: {:?}.",
                event_hash, recipients
            );

            let content = EventSentToRecipientsContent {
                event_hash: event_hash.clone().into(),
                recipients: recipients.clone(),
            };
            let signed = SignedEntry::build(content)?;
            let event_sent_to_recipients = EventSentToRecipients(signed);

            let message = Message {
                private_events: vec![private_event_entry],
                events_sent_to_recipients: vec![event_sent_to_recipients.clone()],
                acknowledgements: vec![],
            };

            send_remote_signal(
                SerializedBytes::try_from(PrivateEventSourcingRemoteSignal::SendMessage(
                    message.clone(),
                ))
                .map_err(|err| wasm_error!(err))?,
                recipients.clone().into_iter().collect(),
            )?;

            if let Ok(()) = send_async_message(
                recipients.clone(),
                EntryHashB64::from(event_hash).to_string(),
                message,
            ) {
                create_relaxed(EntryTypes::EventSentToRecipients(event_sent_to_recipients))?;
            }
        }
    }

    create_acknowledgements::<T>()?;

    // This makes the stress test fail
    // Because over time a bunch of queries accumulate in memory
    // attempt_commit_awaiting_deps_entries::<T>()?;

    Ok(())
}

#[hdk_extern]
pub fn synchronize_with_linked_device(linked_device: AgentPubKey) -> ExternResult<()> {
    let private_events = query_private_event_entries(())?
        .into_iter()
        .map(|(_, e)| e)
        .collect();
    let events_sent_to_recipients = query_events_sent_to_recipients_entries(())?;
    let acknowledgements = query_acknowledgement_entries(())?;

    let message = Message {
        private_events,
        events_sent_to_recipients,
        acknowledgements,
    };

    send_remote_signal(
        SerializedBytes::try_from(PrivateEventSourcingRemoteSignal::SendMessage(message))
            .map_err(|err| wasm_error!(err))?,
        vec![linked_device],
    )?;

    Ok(())
}
