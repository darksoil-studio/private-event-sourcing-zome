use agent_encrypted_message::{create_encrypted_message, get_my_pending_encrypted_messages};
pub use encrypted_links_integrity::*;
use hc_zome_traits::*;
use hdk::prelude::*;
use private_event_sourcing_types::ReceiveMessageInput;
use send_async_message_zome_trait::SendAsyncMessage;

mod agent_encrypted_message;
mod utils;

pub struct EncryptedMessagesInLinks;

#[implement_zome_trait_as_externs]
impl SendAsyncMessage for EncryptedMessagesInLinks {
    fn send_async_message(
        input: send_async_message_zome_trait::SendAsyncMessageInput,
    ) -> ExternResult<()> {
        for recipient in input.recipients {
            create_encrypted_message(recipient, input.message.clone())?;
        }

        Ok(())
    }
}

#[hdk_extern(infallible)]
fn commit_pending_entries(_: Option<Schedule>) -> Option<Schedule> {
    if let Err(err) = internal_commit_pending_entries() {
        error!("Failed to commite pending entries: {err:?}");
    }

    Some(Schedule::Persisted("*/30 * * * * * *".into())) // Every 30 seconds
}

fn zome_name() -> ExternResult<ZomeName> {
    match std::option_env!("PRIVATE_EVENT_SOURCING_ZOME") {
        Some(z) => Ok(z.to_string().into()),
        None => Err(wasm_error!("No zome name")),
    }
}

pub fn internal_commit_pending_entries() -> ExternResult<()> {
    let messages = get_my_pending_encrypted_messages()?;

    for (provenance, message) in messages {
        call(
            CallTargetCell::Local,
            zome_name()?,
            FunctionName::from("receive_message"),
            None,
            ReceiveMessageInput {
                provenance,
                message,
            },
        )?;
    }

    Ok(())
}

#[hdk_extern]
pub fn init() -> ExternResult<InitCallbackResult> {
    schedule("commit_pending_entries")?;

    Ok(InitCallbackResult::Pass)
}
