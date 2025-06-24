use std::collections::BTreeMap;

use hdk::prelude::*;
use private_event_sourcing_integrity::*;

use crate::{
    query_event_histories, query_private_event_entries, utils::create_relaxed, PrivateEvent,
};

pub fn receive_events_sent_to_recipients<T: PrivateEvent>(
    _provenance: AgentPubKey,
    events_sent_to_recipients: Vec<EventSentToRecipients>,
) -> ExternResult<()> {
    let current_events_sent_to_recipients = query_events_sent_to_recipients_entries()?;
    let current_events = query_private_event_entries(())?;

    for event_sent_to_recipients in events_sent_to_recipients {
        if current_events_sent_to_recipients
            .iter()
            .find(|a| a.eq(&&event_sent_to_recipients))
            .is_some()
        {
            // We already have this event_sent_to_recipients committed, nothing to do
            continue;
        }

        let valid = event_sent_to_recipients.0.verify()?;

        if !valid {
            return Err(wasm_error!(
                "Invalid event_sent_to_recipients: invalid signature."
            ));
        }

        if current_events.contains_key(&EntryHashB64::from(
            event_sent_to_recipients
                .0
                .payload
                .content
                .event_hash
                .clone(),
        )) {
            create_relaxed(EntryTypes::EventSentToRecipients(event_sent_to_recipients))?;
        } else {
            create_relaxed(EntryTypes::AwaitingDependencies(
                AwaitingDependencies::EventsSentToRecipients {
                    event_sent_to_recipients,
                },
            ))?;
        }
    }

    Ok(())
}

pub fn query_events_sent_to_recipients(
) -> ExternResult<BTreeMap<EntryHash, BTreeMap<AgentPubKey, Timestamp>>> {
    let mut events_sent_to_recipients = query_events_sent_to_recipients_entries()?;

    events_sent_to_recipients.sort_by_key(|e| e.0.payload.timestamp.clone());

    let mut all_events: BTreeMap<EntryHash, BTreeMap<AgentPubKey, Timestamp>> = BTreeMap::new();

    for event_sent_to_recipients in events_sent_to_recipients {
        let mut agents_with_last_sent: BTreeMap<AgentPubKey, Timestamp> = BTreeMap::new();

        for agent in event_sent_to_recipients.0.payload.content.recipients {
            agents_with_last_sent
                .insert(agent, event_sent_to_recipients.0.payload.timestamp.clone());
        }

        all_events
            .entry(event_sent_to_recipients.0.payload.content.event_hash)
            .or_insert(BTreeMap::new())
            .append(&mut agents_with_last_sent);
    }

    Ok(all_events)
}

pub fn query_events_sent_to_recipients_entries() -> ExternResult<Vec<EventSentToRecipients>> {
    let filter = ChainQueryFilter::new()
        .entry_type(UnitEntryTypes::EventSentToRecipients.try_into()?)
        .include_entries(true)
        .action_type(ActionType::Create);
    let records = query(filter)?;
    let mut events_sent_to_recipients = records
        .into_iter()
        .map(|r| {
            let Some(entry) = r.entry().as_option().clone() else {
                return Err(wasm_error!("PrivateEvents record contained no entry."));
            };
            let entry = EventSentToRecipients::try_from(entry)?;
            Ok(entry)
        })
        .collect::<ExternResult<Vec<EventSentToRecipients>>>()?;

    let mut histories = query_event_histories()?;

    for history in &mut histories {
        events_sent_to_recipients.append(&mut history.events_sent_to_recipients);
    }

    Ok(events_sent_to_recipients)
}
