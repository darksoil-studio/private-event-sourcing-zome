use agent_encrypted_message::{create_encrypted_message, get_my_pending_encrypted_messages};
pub use encrypted_links_integrity::*;
use hdk::prelude::*;
use private_event_sourcing_types::ReceiveMessageInput;
use send_async_message_zome_trait::SendAsyncMessage;

mod agent_encrypted_message;
mod utils;

pub struct EncryptedMessagesInLinks;

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

pub fn internal_commit_pending_entries() -> ExternResult<Sche> {
    let messages = get_my_pending_encrypted_messages()?;

    for (provenance, message) in messages {
        call(
            CallTargetCell::Local,
            ZomeName::from("private_event_sourcing"),
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
