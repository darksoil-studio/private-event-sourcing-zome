use std::collections::BTreeMap;

use hdi::prelude::*;

use crate::{Acknowledgement, AwaitingDependencies, EventSentToRecipients, PrivateEventEntry};

#[hdk_entry_helper]
#[derive(Clone)]
pub struct EventHistory {
    pub awaiting_deps: Vec<AwaitingDependencies>,
    pub events: BTreeMap<EntryHashB64, PrivateEventEntry>,
    pub events_sent_to_recipients: Vec<EventSentToRecipients>,
    pub acknowledgements: Vec<Acknowledgement>,
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
