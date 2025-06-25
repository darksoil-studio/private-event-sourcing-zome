use hdi::prelude::*;
pub use private_event_sourcing_types::Acknowledgement;

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
