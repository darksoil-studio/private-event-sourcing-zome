use std::collections::BTreeMap;

use hdk::prelude::*;
use private_event_sourcing_integrity::*;

use crate::query_event_histories;

pub fn query_events_sent_to_recipients(
) -> ExternResult<BTreeMap<EntryHash, BTreeMap<AgentPubKey, Timestamp>>> {
    let entries = query_events_sent_to_recipients_entries()?;
    let mut events_sent_to_recipients = entries
        .into_iter()
        .map(|events_sent_to_recipients| {
            Ok((
                events_sent_to_recipients.events_sent_to_recipients,
                events_sent_to_recipients.timestamp,
            ))
        })
        .collect::<ExternResult<Vec<(BTreeMap<EntryHash, BTreeSet<AgentPubKey>>, Timestamp)>>>()?;

    events_sent_to_recipients.sort_by_key(|(_, timestamp)| timestamp.clone());

    let mut all_events: BTreeMap<EntryHash, BTreeMap<AgentPubKey, Timestamp>> = BTreeMap::new();

    for (events, timestamp) in events_sent_to_recipients {
        for (event_hash, agents) in events {
            let mut agents_with_last_sent: BTreeMap<AgentPubKey, Timestamp> = BTreeMap::new();

            for agent in agents {
                agents_with_last_sent.insert(agent, timestamp.clone());
            }

            all_events
                .entry(event_hash)
                .or_insert(BTreeMap::new())
                .append(&mut agents_with_last_sent);
        }
    }

    Ok(all_events)
}

pub fn query_events_sent_to_recipients_entries() -> ExternResult<Vec<EventsSentToRecipients>> {
    let filter = ChainQueryFilter::new()
        .entry_type(UnitEntryTypes::EventsSentToRecipients.try_into()?)
        .include_entries(true)
        .action_type(ActionType::Create);
    let records = query(filter)?;
    let mut events_sent_to_recipients = records
        .into_iter()
        .map(|r| {
            let Some(entry) = r.entry().as_option().clone() else {
                return Err(wasm_error!("PrivateEvents record contained no entry."));
            };
            let entry = EventsSentToRecipients::try_from(entry)?;
            Ok(entry)
        })
        .collect::<ExternResult<Vec<EventsSentToRecipients>>>()?;

    let mut histories = query_event_histories()?;

    for history in &mut histories {
        events_sent_to_recipients.append(&mut history.events_sent_to_recipients);
    }

    Ok(events_sent_to_recipients)
}
