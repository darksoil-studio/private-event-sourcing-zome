use hdk::prelude::*;
use serde::de::DeserializeOwned;

#[derive(Serialize, Clone, Deserialize, Debug, SerializedBytes)]
pub struct Message {
    pub private_events: Vec<PrivateEventEntry>,
    pub events_sent_to_recipients: Vec<EventSentToRecipients>,
    pub acknowledgements: Vec<Acknowledgement>,
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

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct SignedContent<T> {
    pub timestamp: Timestamp,
    pub content: T,
}

impl<T: std::fmt::Debug + Serialize + DeserializeOwned> std::convert::TryFrom<&SignedContent<T>>
    for SerializedBytes
{
    type Error = SerializedBytesError;
    fn try_from(
        t: &SignedContent<T>,
    ) -> std::result::Result<SerializedBytes, SerializedBytesError> {
        encode(t).map(|v| SerializedBytes::from(UnsafeBytes::from(v)))
    }
}

impl<T: std::fmt::Debug + Serialize + DeserializeOwned> std::convert::TryFrom<SignedContent<T>>
    for SerializedBytes
{
    type Error = SerializedBytesError;
    fn try_from(t: SignedContent<T>) -> std::result::Result<SerializedBytes, SerializedBytesError> {
        SerializedBytes::try_from(&t)
    }
}

impl<T: std::fmt::Debug + Serialize + DeserializeOwned> std::convert::TryFrom<SerializedBytes>
    for SignedContent<T>
{
    type Error = SerializedBytesError;
    fn try_from(
        sb: SerializedBytes,
    ) -> std::result::Result<SignedContent<T>, SerializedBytesError> {
        decode(sb.bytes())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct SignedEntry<T> {
    pub author: AgentPubKey,
    pub signature: Signature,
    pub payload: SignedContent<T>,
}

impl<T> SignedEntry<T>
where
    T: Clone + std::fmt::Debug + Serialize + DeserializeOwned,
{
    pub fn build(content: T) -> ExternResult<Self> {
        let timestamp = sys_time()?;
        let payload: SignedContent<T> = SignedContent {
            timestamp,
            content: content.clone(),
        };
        let bytes = SerializedBytes::try_from(payload)
            .clone()
            .map_err(|_err| wasm_error!("Failed to serialize content."))?;
        let hash = hash_blake2b(bytes.bytes().to_vec(), 32)?;
        let my_pub_key = agent_info()?.agent_initial_pubkey;
        let signature = sign(my_pub_key.clone(), &hash)?;
        Ok(SignedEntry {
            author: my_pub_key,
            signature,
            payload: SignedContent { timestamp, content },
        })
    }

    pub fn verify(&self) -> ExternResult<bool> {
        let bytes: SerializedBytes = self
            .payload
            .clone()
            .try_into()
            .map_err(|_err| wasm_error!("Failed to serialize content."))?;
        let hash = hash_blake2b(bytes.bytes().to_vec(), 32)?;
        verify_signature(self.author.clone(), self.signature.clone(), &hash)
    }
}

#[hdk_entry_helper]
#[derive(Clone)]
pub struct PrivateEventEntry(pub SignedEntry<PrivateEventContent<SerializedBytes>>);

pub type SignedEvent<T> = SignedEntry<PrivateEventContent<T>>;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct EventSentToRecipientsContent {
    pub event_hash: EntryHash,
    pub recipients: BTreeSet<AgentPubKey>,
}

#[hdk_entry_helper]
#[derive(Clone, PartialEq, Eq)]
pub struct EventSentToRecipients(pub SignedEntry<EventSentToRecipientsContent>);

#[derive(Serialize, Deserialize, SerializedBytes, Debug, Clone, PartialEq, Eq)]
pub struct AcknowledgementContent {
    pub private_event_hash: EntryHash,
}

#[hdk_entry_helper]
#[derive(Clone, PartialEq, Eq)]
pub struct Acknowledgement(pub SignedEntry<AcknowledgementContent>);
