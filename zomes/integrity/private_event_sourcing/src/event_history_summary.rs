use hdi::prelude::*;

#[hdk_entry_helper]
#[derive(Clone)]
pub struct EventHistorySummary {
    pub events_hashes: BTreeSet<EntryHash>,
}

pub fn validate_create_event_history_summary(
    _action: EntryCreationAction,
    _event: EventHistorySummary,
) -> ExternResult<ValidateCallbackResult> {
    Ok(ValidateCallbackResult::Valid)
}

pub fn validate_update_event_history_summary(
    _action: Update,
) -> ExternResult<ValidateCallbackResult> {
    Ok(ValidateCallbackResult::Invalid(format!(
        "EventHistorySummaries cannot be updated."
    )))
}

pub fn validate_delete_event_history_summary(
    _action: Delete,
) -> ExternResult<ValidateCallbackResult> {
    Ok(ValidateCallbackResult::Invalid(format!(
        "EventHistorySummaries cannot be deleted."
    )))
}
