use std::collections::BTreeMap;

use hdk::prelude::*;
use private_event_sourcing_integrity::{
    EntryTypes, EventHistory, EventHistorySummary, PrivateEventEntry,
};

use crate::utils::create_relaxed;

pub fn import_events(events: BTreeMap<EntryHashB64, PrivateEventEntry>) -> ExternResult<()> {
    // TODO: what to do about validation?

    let events_hashes: BTreeSet<EntryHash> = events
        .keys()
        .map(|entry_hash| EntryHash::from(entry_hash.clone()))
        .collect();

    create_relaxed(EntryTypes::EventHistory(EventHistory { events }))?;
    create_relaxed(EntryTypes::EventHistorySummary(EventHistorySummary {
        events_hashes,
    }))?;

    Ok(())
}
