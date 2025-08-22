use hdk::prelude::*;
use private_event_sourcing_integrity::Message;
use send_async_message_zome_trait::SendAsyncMessageInput;

use crate::{
    events_sent_to_recipients::receive_events_sent_to_recipients, query_private_event_entries,
    receive_acknowledgements, receive_private_events, PrivateEvent,
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
    debug!("[receive_message] start.");

    let mut private_event_entries = query_private_event_entries(())?;

    let mut new_events = receive_private_events::<T>(
        &private_event_entries,
        provenance.clone(),
        message.private_events,
    )?;
    debug!(
        "[receive_message] received {} new private events.",
        new_events.len()
    );

    private_event_entries.append(&mut new_events);

    let count = message.events_sent_to_recipients.len();
    receive_events_sent_to_recipients::<T>(
        &private_event_entries,
        provenance.clone(),
        message.events_sent_to_recipients,
    )?;
    debug!(
        "[receive_message] received {} events_sent_to_recipients.",
        count
    );

    let count = message.acknowledgements.len();
    receive_acknowledgements::<T>(
        &private_event_entries,
        provenance.clone(),
        message.acknowledgements,
    )?;
    debug!("[receive_message] received {} acknowledgements.", count);

    Ok(())
}
