use hdi::prelude::*;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct AcknowledgementContent {
    pub private_event_hash: EntryHash,
    pub timestamp: Timestamp,
}

#[hdk_entry_helper]
#[derive(Clone, PartialEq, Eq)]
pub struct Acknowledgement {
    pub author: AgentPubKey,
    pub signature: Signature,
    pub content: AcknowledgementContent,
}

pub fn validate_create_acknowledgement(
    _action: EntryCreationAction,
    _acknowledgement: Acknowledgement,
) -> ExternResult<ValidateCallbackResult> {
    Ok(ValidateCallbackResult::Valid)
}
pub fn validate_update_acknowledgement(
    _action: Update,
    _acknowledgement: Acknowledgement,
) -> ExternResult<ValidateCallbackResult> {
    Ok(ValidateCallbackResult::Invalid(format!(
        "Acknowledgements cannot be updated"
    )))
}
pub fn validate_delete_acknowledgement(_action: Delete) -> ExternResult<ValidateCallbackResult> {
    Ok(ValidateCallbackResult::Invalid(format!(
        "Acknowledgements cannot be deleted"
    )))
}
