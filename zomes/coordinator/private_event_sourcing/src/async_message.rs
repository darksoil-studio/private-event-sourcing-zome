use hdk::prelude::*;
use private_event_sourcing_integrity::Message;
use send_async_message_zome_trait::SendAsyncMessageInput;

use crate::{
    events_sent_to_recipients::receive_events_sent_to_recipients, receive_acknowledgements,
    receive_private_events, PrivateEvent,
};

fn async_message_zome() -> Option<ZomeName> {
    std::option_env!("ASYNC_MESSAGE_ZOME").map(|z| z.to_string().into())
}

pub fn send_async_message(
    recipients: BTreeSet<AgentPubKey>,
    message_id: String,
    message: Message,
) -> ExternResult<()> {
    let Some(zome) = async_message_zome() else {
        return Ok(());
    };

    let bytes = SerializedBytes::try_from(message)
        .map_err(|_err| wasm_error!("Failed to serialize bytes"))?;

    call(
        CallTargetCell::Local,
        zome,
        FunctionName::from("send_async_message"),
        None,
        SendAsyncMessageInput {
            recipients,
            zome_name: zome_info()?.name,
            message_id,
            message: bytes.bytes().to_vec(),
        },
    )?;

    Ok(())
}

pub fn receive_message<T: PrivateEvent>(
    provenance: AgentPubKey,
    message: Message,
) -> ExternResult<()> {
    receive_private_events::<T>(provenance.clone(), message.private_events)?;

    receive_events_sent_to_recipients::<T>(provenance.clone(), message.events_sent_to_recipients)?;

    receive_acknowledgements::<T>(provenance.clone(), message.acknowledgements)?;

    Ok(())
}
