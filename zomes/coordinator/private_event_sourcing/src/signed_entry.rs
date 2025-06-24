use hdk::prelude::*;
use private_event_sourcing_integrity::{SignedEntry, *};

// #[hdk_entry_helper]
// pub struct HashableSignedEntry(pub SignedContent<SerializedBytes>);

pub fn build_signed_entry<T>(content: T) -> ExternResult<SignedEntry<T>> {
    let timestamp = sys_time()?;
    let payload: SignedContent<T> = SignedContent { timestamp, content };
    let signed_hash = hash_entry(&signed.clone())?;
    let my_pub_key = agent_info()?.agent_initial_pubkey;
    let signature = sign(my_pub_key.clone(), &signed_hash)?;
    Ok(SignedEntry {
        author: my_pub_key,
        signature,
        payload,
    })
}

pub fn validate_signed_entry<T>(signed: SignedEntry<T>) -> ExternResult<bool> {
    let signed_hash = hash_entry(&signed.payload)?;
    verify_signature(
        signed.author.clone(),
        signed.signature.clone(),
        &signed_hash,
    )
}
