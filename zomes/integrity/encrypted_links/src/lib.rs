use hdi::prelude::*;

mod agent_encrypted_message;
pub use agent_encrypted_message::*;

#[derive(Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    EncryptedMessage(EncryptedMessage),
}

#[derive(Serialize, Deserialize)]
#[hdk_link_types]
pub enum LinkTypes {
    AgentEncryptedMessage,
}

/// Validation you perform during the genesis process. Nobody else on the network performs it, only you.
/// There *is no* access to network calls in this callback
#[hdk_extern]
pub fn genesis_self_check(_data: GenesisSelfCheckData) -> ExternResult<ValidateCallbackResult> {
    Ok(ValidateCallbackResult::Valid)
}

/// Validation the network performs when you try to join, you can't perform this validation yourself as you are not a member yet.
/// There *is* access to network calls in this function
pub fn validate_agent_joining(
    _agent_pub_key: AgentPubKey,
    _membrane_proof: &Option<MembraneProof>,
) -> ExternResult<ValidateCallbackResult> {
    Ok(ValidateCallbackResult::Valid)
}
pub fn action_hash(op: &Op) -> &ActionHash {
    match op {
        Op::StoreRecord(StoreRecord { record }) => record.action_address(),
        Op::StoreEntry(StoreEntry { action, .. }) => &action.hashed.hash,
        Op::RegisterUpdate(RegisterUpdate { update, .. }) => &update.hashed.hash,
        Op::RegisterDelete(RegisterDelete { delete, .. }) => &delete.hashed.hash,
        Op::RegisterAgentActivity(RegisterAgentActivity { action, .. }) => &action.hashed.hash,
        Op::RegisterCreateLink(RegisterCreateLink { create_link }) => &create_link.hashed.hash,
        Op::RegisterDeleteLink(RegisterDeleteLink { delete_link, .. }) => &delete_link.hashed.hash,
    }
}
/// This is the unified validation callback for all entries and link types in this integrity zome
/// Below is a match template for all of the variants of `DHT Ops` and entry and link types
///
/// Holochain has already performed the following validation for you:
/// - The action signature matches on the hash of its content and is signed by its author
/// - The previous action exists, has a lower timestamp than the new action, and incremented sequence number
/// - The previous action author is the same as the new action author
/// - The timestamp of each action is after the DNA's origin time
/// - AgentActivity authorities check that the agent hasn't forked their chain
/// - The entry hash in the action matches the entry content
/// - The entry type in the action matches the entry content
/// - The entry size doesn't exceed the maximum entry size (currently 4MB)
/// - Private entry types are not included in the Op content, and public entry types are
/// - If the `Op` is an update or a delete, the original action exists and is a `Create` or `Update` action
/// - If the `Op` is an update, the original entry exists and is of the same type as the new one
/// - If the `Op` is a delete link, the original action exists and is a `CreateLink` action
/// - Link tags don't exceed the maximum tag size (currently 1KB)
/// - Countersigned entries include an action from each required signer
///
/// You can read more about validation here: https://docs.rs/hdi/latest/hdi/index.html#data-validation
#[hdk_extern]
pub fn validate(op: Op) -> ExternResult<ValidateCallbackResult> {
    match op.flattened::<EntryTypes, LinkTypes>()? {
        FlatOp::StoreEntry(store_entry) => match store_entry {
            OpEntry::CreateEntry { app_entry, action } => match app_entry {
                EntryTypes::EncryptedMessage(encrypted_message) => {
                    validate_create_encrypted_message(
                        EntryCreationAction::Create(action),
                        encrypted_message,
                    )
                }
            },
            OpEntry::UpdateEntry {
                app_entry, action, ..
            } => match app_entry {
                EntryTypes::EncryptedMessage(encrypted_message) => {
                    validate_create_encrypted_message(
                        EntryCreationAction::Update(action),
                        encrypted_message,
                    )
                }
            },
            _ => Ok(ValidateCallbackResult::Valid),
        },
        FlatOp::RegisterUpdate(update_entry) => match update_entry {
            OpUpdate::Entry { app_entry, action } => match app_entry {
                EntryTypes::EncryptedMessage(encrypted_message) => {
                    validate_update_encrypted_message(action, encrypted_message)
                }
            },
            _ => Ok(ValidateCallbackResult::Valid),
        },
        FlatOp::RegisterDelete(delete_entry) => {
            let action = delete_entry.action;
            let original_action_hash = action.deletes_address.clone();
            let original_record = must_get_valid_record(original_action_hash)?;
            let original_action = original_record.action().clone();
            let original_action = match original_action {
                Action::Create(create) => EntryCreationAction::Create(create),
                Action::Update(update) => EntryCreationAction::Update(update),
                _ => {
                    return Ok(ValidateCallbackResult::Invalid(
                        "Original action for a delete must be a Create or Update action"
                            .to_string(),
                    ));
                }
            };
            let app_entry_type = match original_action.entry_type() {
                EntryType::App(app_entry_type) => app_entry_type,
                _ => {
                    return Ok(ValidateCallbackResult::Valid);
                }
            };
            let entry = match original_record.entry().as_option() {
                Some(entry) => entry,
                None => {
                    return Ok(ValidateCallbackResult::Invalid(
                        "Original record for a delete must contain an entry".to_string(),
                    ));
                }
            };
            let original_app_entry = match EntryTypes::deserialize_from_type(
                app_entry_type.zome_index,
                app_entry_type.entry_index,
                entry,
            )? {
                Some(app_entry) => app_entry,
                None => {
                    return Ok(ValidateCallbackResult::Invalid(
                        "Original app entry must be one of the defined entry types for this zome"
                            .to_string(),
                    ));
                }
            };
            match original_app_entry {
                EntryTypes::EncryptedMessage(_) => validate_delete_encrypted_message(action),
            }
        }
        FlatOp::RegisterCreateLink {
            link_type,
            base_address,
            target_address,
            tag,
            action,
        } => match link_type {
            LinkTypes::AgentEncryptedMessage => validate_create_link_agent_encrypted_message(
                action,
                base_address,
                target_address,
                tag,
            ),
        },
        FlatOp::RegisterDeleteLink {
            link_type,
            base_address,
            target_address,
            tag,
            original_action,
            action,
        } => match link_type {
            LinkTypes::AgentEncryptedMessage => validate_delete_link_agent_encrypted_message(
                action_hash(&op).clone(),
                action,
                original_action,
                base_address,
                target_address,
                tag,
            ),
        },
        FlatOp::StoreRecord(store_record) => match store_record {
            OpRecord::CreateEntry { app_entry, action } => match app_entry {
                EntryTypes::EncryptedMessage(encrypted_message) => {
                    validate_create_encrypted_message(
                        EntryCreationAction::Create(action),
                        encrypted_message,
                    )
                }
            },
            OpRecord::UpdateEntry {
                app_entry, action, ..
            } => match app_entry {
                EntryTypes::EncryptedMessage(encrypted_message) => {
                    let result = validate_create_encrypted_message(
                        EntryCreationAction::Update(action.clone()),
                        encrypted_message.clone(),
                    )?;
                    let ValidateCallbackResult::Valid = result else {
                        return Ok(result);
                    };
                    validate_update_encrypted_message(action, encrypted_message)
                }
            },
            OpRecord::DeleteEntry {
                original_action_hash,
                action,
                ..
            } => {
                let original_record = must_get_valid_record(original_action_hash)?;
                let original_action = original_record.action().clone();
                let original_action = match original_action {
                    Action::Create(create) => EntryCreationAction::Create(create),
                    Action::Update(update) => EntryCreationAction::Update(update),
                    _ => {
                        return Ok(ValidateCallbackResult::Invalid(
                            "Original action for a delete must be a Create or Update action"
                                .to_string(),
                        ));
                    }
                };
                let app_entry_type = match original_action.entry_type() {
                    EntryType::App(app_entry_type) => app_entry_type,
                    _ => {
                        return Ok(ValidateCallbackResult::Valid);
                    }
                };
                let entry = match original_record.entry().as_option() {
                    Some(entry) => entry,
                    None => {
                        return Ok(ValidateCallbackResult::Invalid(
                            "Original record for a delete must contain an entry".to_string(),
                        ));
                    }
                };
                let original_app_entry = match EntryTypes::deserialize_from_type(
                    app_entry_type.zome_index,
                    app_entry_type.entry_index,
                    entry,
                )? {
                    Some(app_entry) => app_entry,
                    None => {
                        return Ok(
                            ValidateCallbackResult::Invalid(
                                "Original app entry must be one of the defined entry types for this zome"
                                    .to_string(),
                            ),
                        );
                    }
                };
                match original_app_entry {
                    EntryTypes::EncryptedMessage(_) => validate_delete_encrypted_message(action),
                }
            }
            OpRecord::CreateLink {
                base_address,
                target_address,
                tag,
                link_type,
                action,
            } => match link_type {
                LinkTypes::AgentEncryptedMessage => validate_create_link_agent_encrypted_message(
                    action,
                    base_address,
                    target_address,
                    tag,
                ),
            },
            OpRecord::DeleteLink {
                original_action_hash,
                base_address,
                action,
            } => {
                let record = must_get_valid_record(original_action_hash)?;
                let create_link = match record.action() {
                    Action::CreateLink(create_link) => create_link.clone(),
                    _ => {
                        return Ok(ValidateCallbackResult::Invalid(
                            "The action that a DeleteLink deletes must be a CreateLink".to_string(),
                        ));
                    }
                };
                let link_type =
                    match LinkTypes::from_type(create_link.zome_index, create_link.link_type)? {
                        Some(lt) => lt,
                        None => {
                            return Ok(ValidateCallbackResult::Valid);
                        }
                    };
                match link_type {
                    LinkTypes::AgentEncryptedMessage => {
                        validate_delete_link_agent_encrypted_message(
                            action_hash(&op).clone(),
                            action,
                            create_link.clone(),
                            base_address,
                            create_link.target_address,
                            create_link.tag,
                        )
                    }
                }
            }
            OpRecord::CreatePrivateEntry { .. } => Ok(ValidateCallbackResult::Valid),
            OpRecord::UpdatePrivateEntry { .. } => Ok(ValidateCallbackResult::Valid),
            OpRecord::CreateCapClaim { .. } => Ok(ValidateCallbackResult::Valid),
            OpRecord::CreateCapGrant { .. } => Ok(ValidateCallbackResult::Valid),
            OpRecord::UpdateCapClaim { .. } => Ok(ValidateCallbackResult::Valid),
            OpRecord::UpdateCapGrant { .. } => Ok(ValidateCallbackResult::Valid),
            OpRecord::Dna { .. } => Ok(ValidateCallbackResult::Valid),
            OpRecord::OpenChain { .. } => Ok(ValidateCallbackResult::Valid),
            OpRecord::CloseChain { .. } => Ok(ValidateCallbackResult::Valid),
            OpRecord::InitZomesComplete { .. } => Ok(ValidateCallbackResult::Valid),
            _ => Ok(ValidateCallbackResult::Valid),
        },
        FlatOp::RegisterAgentActivity(agent_activity) => match agent_activity {
            OpActivity::CreateAgent { agent, action } => {
                let previous_action = must_get_action(action.prev_action)?;
                match previous_action.action() {
					Action::AgentValidationPkg(AgentValidationPkg { membrane_proof, .. }) => {
						validate_agent_joining(agent, membrane_proof)
					}
					_ => Ok(ValidateCallbackResult::Invalid(
						"The previous action for a `CreateAgent` action must be an `AgentValidationPkg`"
							.to_string(),
					)),
				}
            }
            _ => Ok(ValidateCallbackResult::Valid),
        },
    }
}
