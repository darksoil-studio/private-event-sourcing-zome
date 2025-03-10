use std::collections::BTreeMap;

use hdk::prelude::*;
use private_event_sourcing::*;

#[private_event]
#[serde(tag = "type")]
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
        author: AgentPubKey,
        timestamp: Timestamp,
    ) -> ExternResult<ValidateCallbackResult> {
        Ok(ValidateCallbackResult::Valid)
    }

    fn recipients(
        &self,
        author: AgentPubKey,
        timestamp: Timestamp,
    ) -> ExternResult<Vec<AgentPubKey>> {
        match self {
            Event::SharedEntry { recipient, .. } => {
                let mut recipients = query_friends()?;
                recipients.insert(recipient.clone());

                Ok(recipients.into_iter().collect())
            }
            _ => Ok(vec![]),
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
        let Event::NewFriend { friend } = private_event.event.content else {
            continue;
        };
        friends.insert(friend);
    }

    Ok(friends)
}

#[hdk_extern]
pub fn attempt_commit_awaiting_deps_entries() -> ExternResult<()> {
    private_event_sourcing::attempt_commit_awaiting_deps_entries::<Event>()?;

    Ok(())
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

#[hdk_extern(infallible)]
fn scheduled_tasks(_: Option<Schedule>) -> Option<Schedule> {
    if let Err(err) = private_event_sourcing::scheduled_tasks::<Event>() {
        error!("Failed to perform scheduled tasks: {err:?}");
    }

    Some(Schedule::Persisted("*/30 * * * * * *".into())) // Every 30 seconds
}

#[hdk_extern]
pub fn migrate_from_old_cell(old_cell: CellId) -> ExternResult<()> {
    let response = call(
        CallTargetCell::OtherCell(old_cell),
        zome_info()?.name,
        "query_private_event_entries".into(),
        None,
        (),
    )?;

    let ZomeCallResponse::Ok(result) = response else {
        return Err(wasm_error!(
            "Error quering the old private event entries: {:?}.",
            response
        ));
    };
    let private_event_entries: BTreeMap<EntryHashB64, PrivateEventEntry> =
        result.decode().map_err(|err| wasm_error!(err))?;

    import_events(private_event_entries)?;

    Ok(())
}
