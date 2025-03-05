use std::collections::BTreeMap;

use hdk::prelude::*;
use private_event_sourcing_integrity::*;

use crate::{
    internal_create_private_event, query_private_event_entries, validate_private_event_entry,
    PrivateEvent,
};

pub fn attempt_commit_awaiting_deps_entries<T: PrivateEvent>() -> ExternResult<()> {
    let mut entries: Vec<(EntryHashB64, PrivateEventEntry)> =
        query_awaiting_deps_entries()?.into_iter().collect();

    entries.sort_by(|e1, e2| e1.1 .0.event.timestamp.cmp(&e2.1 .0.event.timestamp));

    let private_messenger_entries = query_private_event_entries(())?;
    for (_action_hash, private_event_entry) in entries {
        let entry_hash = hash_entry(&private_event_entry)?;

        if !private_messenger_entries.contains_key(&entry_hash.clone().into()) {
            let valid = validate_private_event_entry::<T>(&private_event_entry)?;

            match valid {
                ValidateCallbackResult::Valid => {
                    internal_create_private_event::<T>(private_event_entry, true)?;
                }
                ValidateCallbackResult::Invalid(reason) => {
                    error!("Invalid awaiting dependencies entry: {reason}");
                }
                ValidateCallbackResult::UnresolvedDependencies(_) => {}
            }
        }
    }

    Ok(())
}

pub fn query_awaiting_deps_entries() -> ExternResult<BTreeMap<EntryHashB64, PrivateEventEntry>> {
    let existing_private_messenger_entries = query_private_event_entries(())?;

    let filter = ChainQueryFilter::new()
        .entry_type(UnitEntryTypes::AwaitingDependencies.try_into()?)
        .include_entries(true)
        .action_type(ActionType::Create);
    let create_records = query(filter)?;

    let mut entries: BTreeMap<EntryHashB64, PrivateEventEntry> = BTreeMap::new();

    for record in create_records {
        let Ok(Some(private_messenger_entry)) = record
            .entry()
            .to_app_option::<PrivateEventEntry>()
            .map_err(|err| wasm_error!(err))
        else {
            continue;
        };
        let Some(entry_hash) = record.action().entry_hash() else {
            continue;
        };
        if existing_private_messenger_entries.contains_key(&EntryHashB64::from(entry_hash.clone()))
        {
            continue;
        }
        entries.insert(entry_hash.clone().into(), private_messenger_entry);
    }

    Ok(entries)
}
