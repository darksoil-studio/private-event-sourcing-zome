use hdk::prelude::*;
use private_event_sourcing::*;

#[private_event]
pub enum Event {
    SharedEntry {
        recipient: AgentPubKey,
        content: String,
    },
    NewFriend {
        friend: AgentPubKey,
    },
}

impl PrivateEvent for Event {
    fn validate(
        &self,
        _entry_hash: EntryHash,
        _author: AgentPubKey,
        _timestamp: Timestamp,
    ) -> ExternResult<ValidateCallbackResult> {
        Ok(ValidateCallbackResult::Valid)
    }

    fn recipients(
        &self,
        _entry_hash: EntryHash,
        _author: AgentPubKey,
        _timestamp: Timestamp,
    ) -> ExternResult<BTreeSet<AgentPubKey>> {
        match self {
            Event::SharedEntry { recipient, .. } => {
                let mut recipients = query_friends()?;
                recipients.insert(recipient.clone());

                Ok(recipients)
            }
            _ => Ok(BTreeSet::new())
        }
    }

    fn adds_new_recipients_for_other_events(
        &self,
        _event_hash: EntryHash,
        _author: AgentPubKey,
        _timestamp: Timestamp,
    ) -> ExternResult<bool> {
        match self {
            Event::NewFriend { .. } => Ok(true),
            _ => Ok(false),
        }
    }
}

#[hdk_extern]
pub fn create_private_shared_entry(entry: Event) -> ExternResult<()> {
    create_private_event(entry)?;
    Ok(())
}

#[hdk_extern]
pub fn add_friend(friend: AgentPubKey) -> ExternResult<()> {
    create_private_event(Event::NewFriend { friend })?;
    Ok(())
}

pub fn query_friends() -> ExternResult<BTreeSet<AgentPubKey>> {
    let private_events = query_private_events::<Event>()?;

    let mut friends: BTreeSet<AgentPubKey> = BTreeSet::new();

    for (_hash, private_event) in private_events {
        let Event::NewFriend { friend } = private_event.payload.content.event else {
            continue;
        };
        friends.insert(friend);
    }

    Ok(friends)
}

#[hdk_extern]
pub fn recv_remote_signal(signal_bytes: SerializedBytes) -> ExternResult<()> {
    if let Ok(private_event_sourcing_remote_signal) =
        PrivateEventSourcingRemoteSignal::try_from(signal_bytes)
    {
        recv_private_events_remote_signal::<Event>(private_event_sourcing_remote_signal)
    } else {
        Ok(())
    }
}

#[hdk_extern]
pub fn migrate_from_old_cell(old_cell: CellId) -> ExternResult<()> {
    let response = call(
        CallTargetCell::OtherCell(old_cell),
        zome_info()?.name,
        "export_event_history".into(),
        None,
        (),
    )?;

    let ZomeCallResponse::Ok(result) = response else {
        return Err(wasm_error!(
            "Error quering the old private event entries: {:?}.",
            response
        ));
    };
    let event_history: EventHistory = result.decode().map_err(|err| wasm_error!(err))?;

    import_event_history(event_history)?;

    Ok(())
}
