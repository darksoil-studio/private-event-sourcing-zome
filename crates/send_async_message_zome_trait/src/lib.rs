use hc_zome_traits::*;
use hdk::prelude::*;

#[derive(Serialize, Deserialize, Debug)]
pub struct SendAsyncMessageInput {
    pub recipients: BTreeSet<AgentPubKey>,
    pub zome_name: ZomeName,
    pub message: Vec<u8>,
}

#[zome_trait]
pub trait SendAsyncMessage {
    fn send_async_message(input: SendAsyncMessageInput) -> ExternResult<()>;
}
