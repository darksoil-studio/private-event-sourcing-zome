# Setup

> [!WARNING]
> This guide assumes that you have scaffolded a hApp with the [TNESH stack template](https://darksoil.studio/tnesh-stack).

1. Add the `github:darksoil-studio/private-event-sourcing-zome` flake input in your `flake.nix`.
2. Add the UI package for `@darksoil-studio/private-event-sourcing-zome` as a dependency of your UI package.
3. Add the `private_event_sourcing` coordinator and integrity crates to your integrity and coordinator zomes.
4. In your coordinator zome, add a `private_event.rs` file with the following content (change all references to `YOURZOME` to your actual zome name):


```rust
use std::collections::BTreeMap;

use hdk::prelude::*;
use private_event_sourcing::*;

#[derive(Serialize, Deserialize, Debug, Clone, SerializedBytes)]
#[serde(tag = "type")]
pub enum YOURZOMEEvent {}

impl PrivateEvent for YOURZOMEEvent {
    fn validate(
        &self,
        _author: AgentPubKey,
        _timestamp: Timestamp,
    ) -> ExternResult<ValidateCallbackResult> {
        Ok(ValidateCallbackResult::Valid)
    }

    fn recipients(
        &self,
        _author: AgentPubKey,
        _timestamp: Timestamp,
    ) -> ExternResult<Vec<AgentPubKey>> {
        Ok(vec![])
    }
}

pub fn query_YOURZOME_events() -> ExternResult<BTreeMap<EntryHashB64, SignedEvent<YOURZOMEEvent>>> {
    query_private_events()
}

#[hdk_extern]
pub fn recv_remote_signal(signal_bytes: SerializedBytes) -> ExternResult<()> {
    if let Ok(private_event_sourcing_remote_signal) =
        PrivateEventSourcingRemoteSignal::try_from(signal_bytes)
    {
        recv_private_events_remote_signal::<YOURZOMEEvent>(private_event_sourcing_remote_signal)
    } else {
        Ok(())
    }
}

#[hdk_extern]
pub fn attempt_commit_awaiting_deps_entries() -> ExternResult<()> {
    private_event_sourcing::attempt_commit_awaiting_deps_entries::<YOURZOMEEvent>()?;

    Ok(())
}

#[hdk_extern(infallible)]
fn scheduled_synchronize_with_linked_devices(_: Option<Schedule>) -> Option<Schedule> {
    if let Err(err) = commit_my_pending_encrypted_messages::<YOURZOMEEvent>() {
        error!("Failed to commit my encrypted messages: {err:?}");
    }
    if let Err(err) = synchronize_with_linked_devices(()) {
        error!("Failed to synchronize with other agents: {err:?}");
    }

    Some(Schedule::Persisted("*/30 * * * * * *".into())) // Every 30 seconds
}
```

That's it! You have now integrated the `private_event_sourcing` coordinator and integrity zomes and their UI into your app!


