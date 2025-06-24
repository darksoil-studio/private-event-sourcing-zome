use hdi::prelude::*;

pub use private_event_sourcing_types::*;

mod private_event;
pub use private_event::*;

mod awaiting_dependencies;
pub use awaiting_dependencies::*;

mod events_sent_to_recipients;
pub use events_sent_to_recipients::*;

mod event_history;
pub use event_history::*;

mod acknowledgement;
pub use acknowledgement::*;

#[derive(Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    #[entry_type(visibility = "private")]
    AwaitingDependencies(AwaitingDependencies),
    #[entry_type(visibility = "private")]
    PrivateEvent(PrivateEventEntry),
    #[entry_type(visibility = "private")]
    EventHistory(EventHistory),
    #[entry_type(visibility = "private")]
    Acknowledgement(Acknowledgement),
    #[entry_type(visibility = "private")]
    EventSentToRecipients(EventSentToRecipients),
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
    match op.flattened::<EntryTypes, ()>()? {
        FlatOp::StoreEntry(store_entry) => match store_entry {
            OpEntry::CreateEntry { app_entry, action } => match app_entry {
                EntryTypes::PrivateEvent(private_event) => validate_create_private_event(
                    EntryCreationAction::Create(action),
                    private_event,
                ),
                EntryTypes::AwaitingDependencies(private_event) => {
                    validate_create_awaiting_dependencies(
                        EntryCreationAction::Create(action),
                        private_event,
                    )
                }
                EntryTypes::EventSentToRecipients(events_sent_to_recipients) => {
                    validate_create_events_sent_to_recipients(
                        EntryCreationAction::Create(action),
                        events_sent_to_recipients,
                    )
                }
                EntryTypes::Acknowledgement(acknowledgement) => validate_create_acknowledgement(
                    EntryCreationAction::Create(action),
                    acknowledgement,
                ),
                EntryTypes::EventHistory(event_history) => validate_create_event_history(
                    EntryCreationAction::Create(action),
                    event_history,
                ),
            },
            OpEntry::UpdateEntry {
                app_entry, action, ..
            } => match app_entry {
                EntryTypes::PrivateEvent(private_event) => validate_create_private_event(
                    EntryCreationAction::Update(action),
                    private_event,
                ),
                EntryTypes::AwaitingDependencies(private_event) => {
                    validate_create_awaiting_dependencies(
                        EntryCreationAction::Update(action),
                        private_event,
                    )
                }
                EntryTypes::EventSentToRecipients(events_sent_to_recipients) => {
                    validate_create_events_sent_to_recipients(
                        EntryCreationAction::Update(action),
                        events_sent_to_recipients,
                    )
                }
                EntryTypes::Acknowledgement(acknowledgement) => validate_create_acknowledgement(
                    EntryCreationAction::Update(action),
                    acknowledgement,
                ),
                EntryTypes::EventHistory(event_history) => validate_create_event_history(
                    EntryCreationAction::Update(action),
                    event_history,
                ),
            },
            _ => Ok(ValidateCallbackResult::Valid),
        },
        FlatOp::RegisterUpdate(update_entry) => match update_entry {
            OpUpdate::Entry { app_entry, action } => match app_entry {
                EntryTypes::PrivateEvent(private_event) => {
                    validate_update_private_event(action, private_event)
                }
                EntryTypes::AwaitingDependencies(private_event) => {
                    validate_update_awaiting_dependencies(action, private_event)
                }
                EntryTypes::Acknowledgement(acknowledgement) => {
                    validate_update_acknowledgement(action, acknowledgement)
                }
                EntryTypes::EventSentToRecipients(events_sent_to_recipients) => {
                    validate_update_events_sent_to_recipients(action, events_sent_to_recipients)
                }
                EntryTypes::EventHistory(_event_history) => validate_update_event_history(action),
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
                EntryTypes::PrivateEvent(_) => validate_delete_private_event(action),
                EntryTypes::AwaitingDependencies(_) => {
                    validate_delete_awaiting_dependencies(action)
                }
                EntryTypes::EventSentToRecipients(_) => {
                    validate_delete_events_sent_to_recipients(action)
                }
                EntryTypes::Acknowledgement(_) => validate_delete_acknowledgement(action),
                EntryTypes::EventHistory(_) => validate_delete_event_history(action),
            }
        }
        FlatOp::RegisterCreateLink {
            link_type,
            base_address,
            target_address,
            tag,
            action,
        } => Ok(ValidateCallbackResult::Invalid(String::from(
            "There are no link types in this zome.",
        ))),
        FlatOp::RegisterDeleteLink {
            link_type,
            base_address,
            target_address,
            tag,
            original_action,
            action,
        } => Ok(ValidateCallbackResult::Invalid(String::from(
            "There are no link types in this zome.",
        ))),
        FlatOp::StoreRecord(store_record) => match store_record {
            OpRecord::CreateEntry { app_entry, action } => match app_entry {
                EntryTypes::PrivateEvent(private_event) => validate_create_private_event(
                    EntryCreationAction::Create(action),
                    private_event,
                ),
                EntryTypes::AwaitingDependencies(private_event) => {
                    validate_create_awaiting_dependencies(
                        EntryCreationAction::Create(action),
                        private_event,
                    )
                }
                EntryTypes::EventSentToRecipients(events_sent_to_recipients) => {
                    validate_create_events_sent_to_recipients(
                        EntryCreationAction::Create(action),
                        events_sent_to_recipients,
                    )
                }
                EntryTypes::Acknowledgement(acknowledgement) => validate_create_acknowledgement(
                    EntryCreationAction::Create(action),
                    acknowledgement,
                ),
                EntryTypes::EventHistory(event_history) => validate_create_event_history(
                    EntryCreationAction::Create(action),
                    event_history,
                ),
            },
            OpRecord::UpdateEntry {
                app_entry, action, ..
            } => match app_entry {
                EntryTypes::PrivateEvent(private_event) => {
                    let result = validate_create_private_event(
                        EntryCreationAction::Update(action.clone()),
                        private_event.clone(),
                    )?;
                    let ValidateCallbackResult::Valid = result else {
                        return Ok(result);
                    };
                    validate_update_private_event(action, private_event)
                }
                EntryTypes::AwaitingDependencies(private_event) => {
                    let result = validate_create_awaiting_dependencies(
                        EntryCreationAction::Update(action.clone()),
                        private_event.clone(),
                    )?;
                    let ValidateCallbackResult::Valid = result else {
                        return Ok(result);
                    };
                    validate_update_awaiting_dependencies(action, private_event)
                }
                EntryTypes::EventSentToRecipients(events_sent_to_recipients) => {
                    let result = validate_create_events_sent_to_recipients(
                        EntryCreationAction::Update(action.clone()),
                        events_sent_to_recipients.clone(),
                    )?;
                    let ValidateCallbackResult::Valid = result else {
                        return Ok(result);
                    };
                    validate_update_events_sent_to_recipients(action, events_sent_to_recipients)
                }
                EntryTypes::Acknowledgement(acknowledgement) => {
                    let result = validate_create_acknowledgement(
                        EntryCreationAction::Update(action.clone()),
                        acknowledgement.clone(),
                    )?;
                    let ValidateCallbackResult::Valid = result else {
                        return Ok(result);
                    };
                    validate_update_acknowledgement(action, acknowledgement)
                }
                EntryTypes::EventHistory(event_history) => {
                    let result = validate_create_event_history(
                        EntryCreationAction::Update(action.clone()),
                        event_history,
                    )?;
                    let ValidateCallbackResult::Valid = result else {
                        return Ok(result);
                    };
                    validate_update_event_history(action)
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
                    EntryTypes::PrivateEvent(_) => validate_delete_private_event(action),
                    EntryTypes::AwaitingDependencies(_) => {
                        validate_delete_awaiting_dependencies(action)
                    }
                    EntryTypes::Acknowledgement(_) => validate_delete_acknowledgement(action),
                    EntryTypes::EventSentToRecipients(_) => {
                        validate_delete_events_sent_to_recipients(action)
                    }
                    EntryTypes::EventHistory(_) => validate_delete_event_history(action),
                }
            }
            OpRecord::CreateLink {
                base_address,
                target_address,
                tag,
                link_type,
                action,
            } => Ok(ValidateCallbackResult::Invalid(String::from(
                "There are no link types in this zome",
            ))),
            OpRecord::DeleteLink {
                original_action_hash,
                base_address,
                action,
            } => Ok(ValidateCallbackResult::Invalid(String::from(
                "There are no link types.",
            ))),
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
