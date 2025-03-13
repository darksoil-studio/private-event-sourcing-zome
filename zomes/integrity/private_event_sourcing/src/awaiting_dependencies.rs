use hdi::prelude::*;

use crate::PrivateEventEntry;

#[hdk_entry_helper]
#[derive(Clone)]
pub struct AwaitingDependencies {
    pub event: PrivateEventEntry,
    pub unresolved_dependencies: UnresolvedDependencies,
}

pub fn validate_create_awaiting_dependencies(
    _action: EntryCreationAction,
    _event: AwaitingDependencies,
) -> ExternResult<ValidateCallbackResult> {
    Ok(ValidateCallbackResult::Valid)
}

pub fn validate_update_awaiting_dependencies(
    _action: Update,
    _event: AwaitingDependencies,
) -> ExternResult<ValidateCallbackResult> {
    Ok(ValidateCallbackResult::Invalid(format!(
        "AwaitingDependencies cannot be updated"
    )))
}

pub fn validate_delete_awaiting_dependencies(
    action: Delete,
) -> ExternResult<ValidateCallbackResult> {
    let create = must_get_action(action.deletes_address)?;
    if action.author.ne(create.hashed.content.author()) {
        return Ok(ValidateCallbackResult::Invalid(format!(
            "AwaitingDependencies can only be deleted by their authors"
        )));
    }
    Ok(ValidateCallbackResult::Valid)
}
