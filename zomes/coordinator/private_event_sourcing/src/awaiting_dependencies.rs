use hdk::prelude::*;
use private_event_sourcing_integrity::*;

use crate::{
    acknowledgements::query_acknowledgement_entries,
    events_sent_to_recipients::query_events_sent_to_recipients_entries,
    internal_create_private_event, query_event_histories, query_private_event_entries,
    utils::create_relaxed, validate_private_event_entry, PrivateEvent,
};

pub fn attempt_commit_awaiting_deps_entries<T: PrivateEvent>() -> ExternResult<()> {
    let mut entries: Vec<PrivateEventEntry> = query_awaiting_deps_private_event_entries()?;

    entries.sort_by_key(|e1| e1.0.payload.timestamp);

    let private_event_entries = query_private_event_entries(())?;
    for private_event_entry in entries {
        let entry_hash = hash_entry(&private_event_entry)?;

        if !private_event_entries.contains_key(&entry_hash.clone().into()) {
            let valid = validate_private_event_entry::<T>(&private_event_entry)?;

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

    let events_sent_to_recipients = query_awaiting_deps_events_sent_to_recipients()?;
    for events_sent_to_recipients in events_sent_to_recipients {
        if private_event_entries.contains_key(&EntryHashB64::from(
            events_sent_to_recipients
                .0
                .payload
                .content
                .event_hash
                .clone(),
        )) {
            create_relaxed(EntryTypes::EventSentToRecipients(events_sent_to_recipients))?;
        }
    }

    let acknowledgements = query_awaiting_deps_acknowledgements()?;

    for acknowledgement in acknowledgements {
        if private_event_entries.contains_key(&EntryHashB64::from(
            acknowledgement.0.payload.content.private_event_hash.clone(),
        )) {
            create_relaxed(EntryTypes::Acknowledgement(acknowledgement))?;
        }
    }

    Ok(())
}

pub fn query_awaiting_deps_private_event_entries() -> ExternResult<Vec<PrivateEventEntry>> {
    let existing_private_event_entries = query_private_event_entries(())?;

    let awaiting_deps = query_awaiting_deps()?;

    let entries: Vec<PrivateEventEntry> = awaiting_deps
        .into_iter()
        .filter_map(|awaiting_deps| match awaiting_deps {
            AwaitingDependencies::Event { event, .. } => Some(event),
            _ => None,
        })
        .filter(|event| {
            let Ok(hash) = hash_entry(event) else {
                return false;
            };
            !existing_private_event_entries.contains_key(&EntryHashB64::from(hash.clone()))
        })
        .collect();

    Ok(entries)
}

pub fn query_awaiting_deps_events_sent_to_recipients() -> ExternResult<Vec<EventSentToRecipients>> {
    let existing_events_sent_to_recipients = query_events_sent_to_recipients_entries()?;

    let awaiting_deps = query_awaiting_deps()?;

    let events_sent_to_recipients: Vec<EventSentToRecipients> = awaiting_deps
        .into_iter()
        .filter_map(|awaiting_deps| match awaiting_deps {
            AwaitingDependencies::EventsSentToRecipients {
                event_sent_to_recipients,
            } => Some(event_sent_to_recipients),
            _ => None,
        })
        .filter(|events_sent_to_recipients| {
            !existing_events_sent_to_recipients
                .iter()
                .any(|e| e.eq(events_sent_to_recipients))
        })
        .collect();

    Ok(events_sent_to_recipients)
}

pub fn query_awaiting_deps_acknowledgements() -> ExternResult<Vec<Acknowledgement>> {
    let existing_acknowledgements = query_acknowledgement_entries()?;

    let awaiting_deps = query_awaiting_deps()?;

    let acknowledgements: Vec<Acknowledgement> = awaiting_deps
        .into_iter()
        .filter_map(|awaiting_deps| match awaiting_deps {
            AwaitingDependencies::Acknowledgement { acknowledgement } => Some(acknowledgement),
            _ => None,
        })
        .filter(|acknowledgement| {
            !existing_acknowledgements
                .iter()
                .any(|a| a.eq(acknowledgement))
        })
        .collect();

    Ok(acknowledgements)
}

pub fn query_awaiting_deps() -> ExternResult<Vec<AwaitingDependencies>> {
    let filter = ChainQueryFilter::new()
        .entry_type(UnitEntryTypes::AwaitingDependencies.try_into()?)
        .include_entries(true)
        .action_type(ActionType::Create);
    let create_records = query(filter)?;

    let mut awaiting_dependencies: Vec<AwaitingDependencies> = create_records
        .into_iter()
        .filter_map(|record| {
            let Some(entry) = record.entry.as_option() else {
                return None;
            };
            let Ok(awaiting_deps) = AwaitingDependencies::try_from(entry) else {
                return None;
            };
            Some(awaiting_deps)
        })
        .collect();

    let mut histories = query_event_histories()?;

    for history in &mut histories {
        awaiting_dependencies.append(&mut history.awaiting_deps);
    }

    Ok(awaiting_dependencies)
}
