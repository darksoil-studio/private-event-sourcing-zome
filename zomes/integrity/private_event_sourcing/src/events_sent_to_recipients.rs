use std::collections::BTreeMap;

use hdi::prelude::*;

#[hdk_entry_helper]
#[derive(Clone)]
pub struct EventsSentToRecipients {
    pub events_sent_to_recipients: BTreeMap<EntryHash, BTreeSet<AgentPubKey>>,
    pub timestamp: Timestamp,
}

pub fn validate_create_events_sent_to_recipients(
    _action: EntryCreationAction,
    _events_sent_to_recipients: EventsSentToRecipients,
) -> ExternResult<ValidateCallbackResult> {
    Ok(ValidateCallbackResult::Valid)
}
pub fn validate_update_events_sent_to_recipients(
    _action: Update,
    _events_sent_to_recipients: EventsSentToRecipients,
) -> ExternResult<ValidateCallbackResult> {
    Ok(ValidateCallbackResult::Invalid(format!(
        "EventsSentToRecipients cannot be updated"
    )))
}
pub fn validate_delete_events_sent_to_recipients(
    _action: Delete,
) -> ExternResult<ValidateCallbackResult> {
    Ok(ValidateCallbackResult::Invalid(format!(
        "SentEvents cannot be deleted"
    )))
}
