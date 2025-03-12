# Setup

> [!WARNING]
> This guide assumes that you have scaffolded a hApp with the [TNESH stack template](https://darksoil.studio/tnesh-stack).

1. Add the `github:darksoil-studio/private-event-sourcing-zome` flake input in your `flake.nix`.
2. Add the UI package for `@darksoil-studio/private-event-sourcing-zome` as a dependency of your UI package.
3. Add the `private_event_sourcing` coordinator and integrity crates to your integrity and coordinator zomes.
4. In your coordinator zome, add a `private_event.rs` file with the following content (change all references to `ZOME_NAME` to your actual zome name):


```rust
use std::collections::BTreeMap;

use hdk::prelude::*;
use private_event_sourcing::*;

#[private_event]
#[serde(tag = "type")]
pub enum ZOME_NAMEEvent {}

impl PrivateEvent for ZOME_NAMEEvent {
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

pub fn query_ZOME_NAME_events() -> ExternResult<BTreeMap<EntryHashB64, SignedEvent<ZOME_NAMEEvent>>> {
    query_private_events()
}

#[hdk_extern]
pub fn recv_remote_signal(signal_bytes: SerializedBytes) -> ExternResult<()> {
    if let Ok(private_event_sourcing_remote_signal) =
        PrivateEventSourcingRemoteSignal::try_from(signal_bytes)
    {
        recv_private_events_remote_signal::<ZOME_NAMEEvent>(private_event_sourcing_remote_signal)
    } else {
        Ok(())
    }
}

#[hdk_extern]
pub fn attempt_commit_awaiting_deps_entries() -> ExternResult<()> {
    private_event_sourcing::attempt_commit_awaiting_deps_entries::<ZOME_NAMEEvent>()
}

#[hdk_extern]
pub fn send_event(event_hash: EntryHash) -> ExternResult<()> {
    private_event_sourcing::send_event::<ZOME_NAMEEvent>(event_hash)
}

#[hdk_extern(infallible)]
fn scheduled_tasks(_: Option<Schedule>) -> Option<Schedule> {
    if let Err(err) = private_event_sourcing::scheduled_tasks::<ZOME_NAMEEvent>() {
        error!("Failed to perform scheduled tasks: {err:?}");
    }

    Some(Schedule::Persisted("*/30 * * * * * *".into())) // Every 30 seconds
}

```

That's it! You have now integrated the `private_event_sourcing` coordinator and integrity zomes and their UI into your app!


