use hdk::prelude::*;
use private_event_sourcing_integrity::{EntryTypes, EventHistory, UnitEntryTypes};

use crate::{
    acknowledgements::query_acknowledgement_entries, awaiting_dependencies::query_awaiting_deps,
    events_sent_to_recipients::query_events_sent_to_recipients_entries,
    query_private_event_entries, utils::create_relaxed,
};

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

pub fn export_event_history() -> ExternResult<EventHistory> {
    let acknowledgements = query_acknowledgement_entries()?;
    let awaiting_deps = query_awaiting_deps()?;
    let events_sent_to_recipients = query_events_sent_to_recipients_entries()?;
    let events = query_private_event_entries(())?;

    Ok(EventHistory {
        awaiting_deps,
        events,
        events_sent_to_recipients,
        acknowledgements,
    })
}

pub fn import_event_history(history: EventHistory) -> ExternResult<()> {
    // TODO: what to do about validation?

    create_relaxed(EntryTypes::EventHistory(history))?;

    Ok(())
}
