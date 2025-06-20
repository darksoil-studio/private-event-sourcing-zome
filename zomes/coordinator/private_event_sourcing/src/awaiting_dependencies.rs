use std::collections::BTreeMap;

use hdk::prelude::*;
use private_event_sourcing_integrity::*;

use crate::{
    acknowledgements::query_acknowledgement_entries, internal_create_private_event,
    query_private_event_entries, utils::create_relaxed, validate_private_event_entry, PrivateEvent,
};

pub fn attempt_commit_awaiting_deps_entries<T: PrivateEvent>() -> ExternResult<()> {
    let mut entries: Vec<(EntryHashB64, PrivateEventEntry)> =
        query_awaiting_deps_entries()?.into_iter().collect();

    entries.sort_by(|e1, e2| e1.1 .0.event.timestamp.cmp(&e2.1 .0.event.timestamp));

    let private_event_entries = query_private_event_entries(())?;
    for (_action_hash, private_event_entry) in entries {
        let entry_hash = hash_entry(&private_event_entry)?;

        if !private_event_entries.contains_key(&entry_hash.clone().into()) {
            let valid = validate_private_event_entry::<T>(entry_hash, &private_event_entry)?;

            match valid {
                ValidateCallbackResult::Valid => {
                    internal_create_private_event::<T>(private_event_entry)?;
                }
                ValidateCallbackResult::Invalid(reason) => {
                    error!("Invalid awaiting dependencies entry: {reason}");
                }
                ValidateCallbackResult::UnresolvedDependencies(_) => {}
            }
        }
    }

    let private_event_entries = query_private_event_entries(())?;
    let acknowledgements = query_awaiting_deps_acknowledgements()?;

    for acknowledgement in acknowledgements {
        if private_event_entries.contains_key(&EntryHashB64::from(
            acknowledgement.content.private_event_hash.clone(),
        )) {
            create_relaxed(EntryTypes::Acknowledgement(acknowledgement))?;
        }
    }

    Ok(())
}

pub fn query_awaiting_deps_entries() -> ExternResult<BTreeMap<EntryHashB64, PrivateEventEntry>> {
    let existing_private_event_entries = query_private_event_entries(())?;

    let filter = ChainQueryFilter::new()
        .entry_type(UnitEntryTypes::AwaitingDependencies.try_into()?)
        .include_entries(true)
        .action_type(ActionType::Create);
    let create_records = query(filter)?;

    let mut entries: BTreeMap<EntryHashB64, PrivateEventEntry> = BTreeMap::new();

    for record in create_records {
        let Ok(Some(awaiting_dependencies)) = record
            .entry()
            .to_app_option::<AwaitingDependencies>()
            .map_err(|err| wasm_error!(err))
        else {
            continue;
        };
        let AwaitingDependencies::Event { event, .. } = awaiting_dependencies else {
            continue;
        };
        let Some(entry_hash) = record.action().entry_hash() else {
            continue;
        };
        if existing_private_event_entries.contains_key(&EntryHashB64::from(entry_hash.clone())) {
            continue;
        }
        entries.insert(entry_hash.clone().into(), event);
    }

    Ok(entries)
}

pub fn query_awaiting_deps_acknowledgements() -> ExternResult<Vec<Acknowledgement>> {
    let existing_acknowledgements = query_acknowledgement_entries()?;

    let filter = ChainQueryFilter::new()
        .entry_type(UnitEntryTypes::AwaitingDependencies.try_into()?)
        .include_entries(true)
        .action_type(ActionType::Create);
    let create_records = query(filter)?;

    let mut acknowledgements: Vec<Acknowledgement> = Vec::new();

    for record in create_records {
        let Ok(Some(awaiting_dependencies)) = record
            .entry()
            .to_app_option::<AwaitingDependencies>()
            .map_err(|err| wasm_error!(err))
        else {
            continue;
        };
        let AwaitingDependencies::Acknowledgement { acknowledgement } = awaiting_dependencies
        else {
            continue;
        };
        if existing_acknowledgements
            .iter()
            .find(|a| a.eq(&&acknowledgement))
            .is_some()
        {
            continue;
        }
        acknowledgements.push(acknowledgement);
    }

    Ok(acknowledgements)
}
