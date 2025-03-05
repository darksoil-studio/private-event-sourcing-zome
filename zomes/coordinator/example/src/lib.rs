use example_integrity::*;
use hdk::prelude::*;
use private_event_sourcing::*;

#[derive(Serialize, Deserialize, Debug, SerializedBytes)]
pub struct SharedEntry {
    recipient: AgentPubKey,
    content: String,
}

impl PrivateEvent for SharedEntry {
    fn validate(&self, author: AgentPubKey) -> ExternResult<ValidateCallbackResult> {
        Ok(ValidateCallbackResult::Valid)
    }
    fn recipients(&self) -> ExternResult<Vec<AgentPubKey>> {
        Ok(vec![self.recipient.clone()])
    }
}

#[hdk_extern]
pub fn create_private_shared_entry(entry: SharedEntry) -> ExternResult<()> {
    create_private_event(entry)?;
    Ok(())
}

#[hdk_extern]
pub fn attempt_commit_awaiting_deps_entries() -> ExternResult<()> {
    private_event_sourcing::attempt_commit_awaiting_deps_entries::<SharedEntry>()?;

    Ok(())
}

#[hdk_extern]
pub fn recv_remote_signal(signal_bytes: SerializedBytes) -> ExternResult<()> {
    if let Ok(private_event_sourcing_remote_signal) =
        PrivateEventSourcingRemoteSignal::try_from(signal_bytes)
    {
        recv_private_events_remote_signal::<SharedEntry>(private_event_sourcing_remote_signal)
    } else {
        Ok(())
    }
}
