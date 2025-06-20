use std::collections::BTreeMap;

use hdk::prelude::*;

#[derive(Serialize, Deserialize, Debug)]
pub struct ReceivePrivateEventsInput {
    pub private_events: BTreeMap<EntryHashB64, PrivateEventEntry>,
    pub provenance: AgentPubKey,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SignedContent<T> {
    pub timestamp: Timestamp,
    pub event_type: String,
    pub content: T,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SignedEvent<T> {
    pub author: AgentPubKey,
    pub signature: Signature,
    pub event: SignedContent<T>,
}

#[hdk_entry_helper]
#[derive(Clone)]
pub struct PrivateEventEntry(pub SignedEvent<SerializedBytes>);
