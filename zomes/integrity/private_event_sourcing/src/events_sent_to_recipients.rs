use hdi::prelude::*;

pub use private_event_sourcing_types::EventSentToRecipients;

pub fn validate_create_events_sent_to_recipients(
    _action: EntryCreationAction,
    _events_sent_to_recipients: EventSentToRecipients,
) -> ExternResult<ValidateCallbackResult> {
    Ok(ValidateCallbackResult::Valid)
}
pub fn validate_update_events_sent_to_recipients(
    _action: Update,
    _events_sent_to_recipients: EventSentToRecipients,
) -> ExternResult<ValidateCallbackResult> {
    Ok(ValidateCallbackResult::Invalid(format!(
        "EventsSentToRecipients cannot be updated"
    )))
}
pub fn validate_delete_events_sent_to_recipients(
    _action: Delete,
) -> ExternResult<ValidateCallbackResult> {
    Ok(ValidateCallbackResult::Invalid(format!(
        "EventsSentToRecipients cannot be deleted"
    )))
}
