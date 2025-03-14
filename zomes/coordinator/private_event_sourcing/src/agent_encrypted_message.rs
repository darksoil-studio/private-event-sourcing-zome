use std::collections::BTreeMap;

use hdk::prelude::*;
use private_event_sourcing_integrity::{
    EncryptedMessage, EntryTypes, LinkTypes, PrivateEventEntry,
};

use crate::{
    private_event::PrivateEvent,
    receive_private_events,
    utils::{create_link_relaxed, create_relaxed, delete_link_relaxed},
};

#[derive(Serialize, Deserialize, Debug, SerializedBytes)]
pub struct PlainPrivateEventEntries(pub BTreeMap<EntryHashB64, PrivateEventEntry>);

pub fn create_encrypted_message(
    recipient: AgentPubKey,
    private_events_entries: BTreeMap<EntryHashB64, PrivateEventEntry>,
) -> ExternResult<()> {
    let entry_hashes: BTreeSet<EntryHashB64> = private_events_entries.keys().cloned().collect();
    let entry_bytes = SerializedBytes::try_from(PlainPrivateEventEntries(private_events_entries))
        .map_err(|err| wasm_error!(err))?;

    let chunks: Vec<XSalsa20Poly1305Data> = entry_bytes
        .bytes()
        .chunks(2_000)
        .map(|c| c.to_vec().into())
        .collect();
    let encrypted_entries = chunks
        .into_iter()
        .map(|chunk| {
            ed_25519_x_salsa20_poly1305_encrypt(
                agent_info()?.agent_latest_pubkey,
                recipient.clone(),
                chunk,
            )
        })
        .collect::<ExternResult<Vec<XSalsa20Poly1305EncryptedData>>>()?;

    let entry = EncryptedMessage {
        encrypted_entries,
        entry_hashes,
    };
    let bytes = SerializedBytes::try_from(entry.clone()).map_err(|err| wasm_error!(err))?;

    if bytes.bytes().len() > 900 {
        let entry_hash = hash_entry(&entry)?;
        create_relaxed(EntryTypes::EncryptedMessage(entry))?;
        create_link_relaxed(
            recipient.clone(),
            entry_hash,
            LinkTypes::AgentEncryptedMessage,
            (),
        )?;
    } else {
        create_link_relaxed(
            recipient.clone(),
            recipient.clone(),
            LinkTypes::AgentEncryptedMessage,
            bytes.bytes().clone(),
        )?;
    }

    Ok(())
}

pub fn get_agent_encrypted_messages(agent: AgentPubKey) -> ExternResult<Vec<Link>> {
    get_links(GetLinksInputBuilder::try_new(agent, LinkTypes::AgentEncryptedMessage)?.build())
}

pub fn get_message(agent_encrypted_message_link: &Link) -> ExternResult<Option<EncryptedMessage>> {
    if agent_encrypted_message_link
        .base
        .eq(&agent_encrypted_message_link.target)
    {
        let tag = agent_encrypted_message_link.tag.clone();
        let bytes = SerializedBytes::from(UnsafeBytes::from(tag.into_inner()));

        let message = EncryptedMessage::try_from(bytes).map_err(|err| wasm_error!(err))?;

        Ok(Some(message))
    } else {
        let Some(entry_hash) = agent_encrypted_message_link
            .target
            .clone()
            .into_entry_hash()
        else {
            return Err(wasm_error!("Invalid EncryptedMessage target"));
        };
        let Some(record) = get(entry_hash, GetOptions::default())? else {
            return Ok(None);
        };

        let Ok(Some(encrypted_message)) = record.entry().to_app_option::<EncryptedMessage>() else {
            return Err(wasm_error!("Invalid EncryptedMessage target"));
        };
        Ok(Some(encrypted_message))
    }
}

pub fn commit_my_pending_encrypted_messages<T: PrivateEvent>() -> ExternResult<()> {
    let my_pub_key = agent_info()?.agent_latest_pubkey;
    let links = get_agent_encrypted_messages(my_pub_key.clone())?;

    for link in links {
        debug!("[commit_my_pending_encrypted_messages] Found an EncryptedMessage link.");
        let Some(message) = get_message(&link)? else {
            continue;
        };
        debug!("[commit_my_pending_encrypted_messages] Found an EncryptedMessage.");

        let decrypted_data = message
            .encrypted_entries
            .into_iter()
            .map(|chunk| {
                ed_25519_x_salsa20_poly1305_decrypt(my_pub_key.clone(), link.author.clone(), chunk)
            })
            .collect::<ExternResult<Vec<XSalsa20Poly1305Data>>>()?;

        let decrypted_bytes: Vec<u8> = decrypted_data
            .into_iter()
            .map(|chunk| chunk.as_ref().to_vec())
            .flatten()
            .collect();
        let decrypted_serialized_bytes = SerializedBytes::from(UnsafeBytes::from(decrypted_bytes));

        if let Ok(private_events_entries) =
            PlainPrivateEventEntries::try_from(decrypted_serialized_bytes)
        {
            if let Err(err) = receive_private_events::<T>(link.author, private_events_entries.0) {
                error!("Failed to receive private event from an encrypted message: {err:?}.")
            }
        }

        delete_link_relaxed(link.create_link_hash)?;
    }

    Ok(())
}
