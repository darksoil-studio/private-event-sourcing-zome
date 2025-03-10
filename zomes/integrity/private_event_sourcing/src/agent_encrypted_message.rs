use hdi::prelude::*;

#[hdk_entry_helper]
#[derive(Clone)]
pub struct EncryptedMessage(pub SerializedBytes);

pub fn validate_create_encrypted_message(
    _action: EntryCreationAction,
    _encrypted_message: EncryptedMessage,
) -> ExternResult<ValidateCallbackResult> {
    Ok(ValidateCallbackResult::Valid)
}
pub fn validate_update_encrypted_message(
    _action: Update,
    _encrypted_message: EncryptedMessage,
) -> ExternResult<ValidateCallbackResult> {
    Ok(ValidateCallbackResult::Invalid(format!(
        "EncryptedMessages cannot be updated"
    )))
}
pub fn validate_delete_encrypted_message(_action: Delete) -> ExternResult<ValidateCallbackResult> {
    Ok(ValidateCallbackResult::Invalid(format!(
        "EncryptedMessages cannot be deleted"
    )))
}

pub fn validate_create_link_agent_encrypted_message(
    _action: CreateLink,
    base_address: AnyLinkableHash,
    target_address: AnyLinkableHash,
    _tag: LinkTag,
) -> ExternResult<ValidateCallbackResult> {
    // Check the entry type for the given action hash
    let Some(_agent) = base_address.clone().into_agent_pub_key() else {
        return Ok(ValidateCallbackResult::Invalid(format!(
            "Base of an AgentEncryptedMessage link must be a profile ActionHash"
        )));
    };

    // Check the entry type for the given action hash
    let Some(_encrypted_message_hash) = target_address.into_entry_hash() else {
        return Ok(ValidateCallbackResult::Invalid(
            "No action hash associated with link".to_string(),
        ));
    };
    // let entry = must_get_entry(encrypted_message_hash.clone())?;
    // let Ok(message) = EncryptedMessage::try_from(entry.content) else {
    //     return Ok(ValidateCallbackResult::Invalid(
    //         "Linked action must reference an entry of type EncryptedMessage".to_string(),
    //     ));
    // };
    // if message.recipient.ne(&agent) {
    //     return Ok(ValidateCallbackResult::Invalid(
    //         "Recipient for an EncryptedMessage must be the base address of an AgentToEncryptedMessage entry".to_string(),
    //     ));
    // }

    Ok(ValidateCallbackResult::Valid)
}

pub fn validate_delete_link_agent_encrypted_message(
    _action_hash: ActionHash,
    action: DeleteLink,
    _original_action: CreateLink,
    base_address: AnyLinkableHash,
    _target: AnyLinkableHash,
    _tag: LinkTag,
) -> ExternResult<ValidateCallbackResult> {
    // Check the entry type for the given action hash
    let Some(agent) = base_address.into_agent_pub_key() else {
        return Ok(ValidateCallbackResult::Invalid(format!(
            "Base of an AgentEncryptedMessage link must be a profile ActionHash"
        )));
    };

    if agent.ne(&action.author) {
        return Ok(ValidateCallbackResult::Invalid(String::from(
            "Encrypted messages can only be deleted by their recipients",
        )));
    }

    Ok(ValidateCallbackResult::Valid)
}
