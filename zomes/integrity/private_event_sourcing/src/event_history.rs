use std::collections::BTreeMap;

use hdi::prelude::*;

use crate::PrivateEventEntry;

#[hdk_entry_helper]
#[derive(Clone)]
pub struct EventHistory {
    pub events: BTreeMap<EntryHashB64, PrivateEventEntry>,
}

pub fn validate_create_event_history(
    _action: EntryCreationAction,
    _event: EventHistory,
) -> ExternResult<ValidateCallbackResult> {
    Ok(ValidateCallbackResult::Valid)
}

pub fn validate_update_event_history(_action: Update) -> ExternResult<ValidateCallbackResult> {
    Ok(ValidateCallbackResult::Invalid(format!(
        "EventHistories cannot be updated."
    )))
}

pub fn validate_delete_event_history(_action: Delete) -> ExternResult<ValidateCallbackResult> {
    Ok(ValidateCallbackResult::Invalid(format!(
        "EventHistories cannot be deleted."
    )))
}
