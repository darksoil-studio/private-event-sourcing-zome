use hdk::prelude::*;
use private_event_sourcing::*;

#[private_event]
#[serde(tag = "type")]
pub enum Event {
    SharedEntry {
        recipient: AgentPubKey,
        content: String,
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
        let Event::SharedEntry { recipient, .. } = self;
        Ok(vec![recipient.clone()])
    }
}

#[hdk_extern]
pub fn create_private_shared_entry(entry: Event) -> ExternResult<()> {
    create_private_event(entry)?;
    Ok(())
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
fn scheduled_synchronize_with_linked_devices(_: Option<Schedule>) -> Option<Schedule> {
    if let Err(err) = commit_my_pending_encrypted_messages::<Event>() {
        error!("Failed to commit my encrypted messages: {err:?}");
    }
    if let Err(err) = synchronize_with_linked_devices(()) {
        error!("Failed to synchronize with other agents: {err:?}");
    }

    Some(Schedule::Persisted("*/30 * * * * * *".into())) // Every 30 seconds
}
