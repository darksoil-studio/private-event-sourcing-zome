use hdk::prelude::*;

#[derive(Serialize, Deserialize, Debug, SerializedBytes)]
pub struct Message {
    pub private_events: Vec<PrivateEventEntry>,
    pub events_sent_to_recipients: Vec<EventSentToRecipients>,
    pub acknowledgments: Vec<Acknowledgement>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ReceiveMessageInput {
    pub provenance: AgentPubKey,
    pub message: Message,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PrivateEventContent<T> {
    pub event_type: String,
    pub event: T,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SignedContent<T> {
    pub timestamp: Timestamp,
    pub content: T,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SignedEntry<T> {
    pub author: AgentPubKey,
    pub signature: Signature,
    pub payload: SignedContent<T>,
}

#[hdk_entry_helper]
#[derive(Clone)]
pub struct PrivateEventEntry(pub SignedEntry<PrivateEventContent<SerializedBytes>>);

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct EventSentToRecipientsContent {
    pub event_hash: EntryHash,
    pub recipients: BTreeSet<AgentPubKey>,
}

#[hdk_entry_helper]
#[derive(Clone)]
pub struct EventSentToRecipients(pub SignedEntry<EventSentToRecipientsContent>);

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct AcknowledgementContent {
    pub private_event_hash: EntryHash,
}

#[hdk_entry_helper]
#[derive(Clone)]
pub struct Acknowledgement(pub SignedEntry<AcknowledgementContent>);
