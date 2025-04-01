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
pub enum ZOME_NAMEEvent {}

impl PrivateEvent for ZOME_NAMEEvent {
    fn validate(
        &self,
        _event_hash: EntryHash,
        _author: AgentPubKey,
        _timestamp: Timestamp,
    ) -> ExternResult<ValidateCallbackResult> {
        Ok(ValidateCallbackResult::Valid)
    }

    fn recipients(
        &self,
        _event_hash: EntryHash,
        _author: AgentPubKey,
        _timestamp: Timestamp,
    ) -> ExternResult<BTreeSet<AgentPubKey>> {
        Ok(BTreeSet::new())
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
```

That's it! You have now integrated the `private_event_sourcing` coordinator and integrity zomes and their UI into your app!


