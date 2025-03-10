use hdi::prelude::*;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SignedContent<T> {
    pub timestamp: Timestamp,
    pub event_type: String,
    pub content: T,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SignedEvent<T> {
    pub author: AgentPubKey,
    pub signature: Signature,
    pub event: SignedContent<T>,
}

#[hdk_entry_helper]
#[derive(Clone)]
pub struct PrivateEventEntry(pub SignedEvent<SerializedBytes>);

pub fn validate_create_private_event(
    _action: EntryCreationAction,
    _event: PrivateEventEntry,
) -> ExternResult<ValidateCallbackResult> {
    Ok(ValidateCallbackResult::Valid)
}

pub fn validate_update_private_event(
    _action: Update,
    _event: PrivateEventEntry,
) -> ExternResult<ValidateCallbackResult> {
    Ok(ValidateCallbackResult::Invalid(format!(
        "PrivateEvents cannot be updated"
    )))
}

pub fn validate_delete_private_event(_action: Delete) -> ExternResult<ValidateCallbackResult> {
    Ok(ValidateCallbackResult::Invalid(format!(
        "PrivateEvents cannot be deleted"
    )))
}
