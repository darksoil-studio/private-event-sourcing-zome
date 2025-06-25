use std::collections::BTreeMap;

use hdk::prelude::*;
use private_event_sourcing_integrity::*;

use crate::{
    query_event_histories, query_private_event_entries, query_private_event_entry,
    send_async_message, utils::create_relaxed, PrivateEvent, PrivateEventSourcingRemoteSignal,
};

pub fn create_acknowledgements<T: PrivateEvent>(
    events_hashes: BTreeSet<EntryHashB64>,
) -> ExternResult<()> {
    for event_hash in events_hashes {
        create_acknowledgement::<T>(event_hash)?;
    }

    Ok(())
}

pub fn query_acknowledgement_for(
    event_hash: &EntryHashB64,
) -> ExternResult<Option<Acknowledgement>> {
    let acknowledgements = query_acknowledgement_entries(())?;
    Ok(acknowledgements
        .iter()
        .find(|a| {
            a.0.payload
                .content
                .private_event_hash
                .eq(&EntryHash::from(event_hash.clone()))
        })
        .cloned())
}

pub fn send_acknowledgement_for_event_to_recipient<T: PrivateEvent>(
    event_hash: &EntryHashB64,
    recipient: &AgentPubKey,
) -> ExternResult<()> {
    if let Some(acknowledgement) = query_acknowledgement_for(event_hash)? {
        let message = Message {
            private_events: vec![],
            events_sent_to_recipients: vec![],
            acknowledgements: vec![acknowledgement.clone()],
        };

        send_remote_signal(
            SerializedBytes::try_from(PrivateEventSourcingRemoteSignal::SendMessage(
                message.clone(),
            ))
            .map_err(|err| wasm_error!(err))?,
            vec![recipient.clone()],
        )?;

        send_async_message(vec![recipient.clone()].into_iter().collect(), message)?;
    } else {
        warn!("Received an event I already have but have not created an acknowledgement for.");
    }

    Ok(())
}

pub fn create_acknowledgement<T: PrivateEvent>(event_hash: EntryHashB64) -> ExternResult<()> {
    let Some(private_event_entry) = query_private_event_entry(event_hash.clone().into())? else {
        return Err(wasm_error!("Could not find private event"));
    };

    if private_event_entry
        .0
        .author
        .eq(&agent_info()?.agent_initial_pubkey)
    {
        return Ok(()); // We are the author, no need to create acknowledgement
    }

    let acknowledgement = if let Some(ack) = query_acknowledgement_for(&event_hash)? {
        ack.clone()
    } else {
        let acknowledgement_content = AcknowledgementContent {
            private_event_hash: event_hash.clone().into(),
        };
        let signed_entry = SignedEntry::build(acknowledgement_content)?;
        let acknowledgement = Acknowledgement(signed_entry);

        info!("Creating acknowledgement for entry {}.", event_hash);
        create_relaxed(EntryTypes::Acknowledgement(acknowledgement.clone()))?;
        acknowledgement
    };

    let private_event = T::try_from(private_event_entry.0.payload.content.event.clone())
        .map_err(|_err| wasm_error!("Failed to deserialize the private event."))?;

    let mut recipients = private_event.recipients(
        event_hash.clone().into(),
        private_event_entry.0.author.clone(),
        private_event_entry.0.payload.timestamp,
    )?;

    recipients.insert(private_event_entry.0.author);

    let my_pub_key = agent_info()?.agent_initial_pubkey;

    let recipients: BTreeSet<AgentPubKey> = recipients
        .into_iter()
        .filter(|agent| agent.ne(&my_pub_key))
        .collect();

    let message = Message {
        private_events: vec![],
        acknowledgements: vec![acknowledgement],
        events_sent_to_recipients: vec![],
    };

    info!(
        "Sending acknowledgement for {} to {:?}.",
        event_hash, recipients
    );

    send_remote_signal(
        SerializedBytes::try_from(PrivateEventSourcingRemoteSignal::SendMessage(
            message.clone(),
        ))
        .map_err(|err| wasm_error!(err))?,
        recipients.clone().into_iter().collect(),
    )?;

    send_async_message(recipients, message)?;

    Ok(())
}

pub fn receive_acknowledgements<T: PrivateEvent>(
    provenance: AgentPubKey,
    acknowledgements: Vec<Acknowledgement>,
) -> ExternResult<()> {
    let current_acknowledgements = query_acknowledgement_entries(())?;
    let current_events = query_private_event_entries(())?;

    for acknowledgement in acknowledgements {
        if current_acknowledgements
            .iter()
            .find(|a| a.eq(&&acknowledgement))
            .is_some()
        {
            // We already have this acknowledgement committed
            continue;
        }

        let valid = acknowledgement.0.verify()?;

        if !valid {
            return Err(wasm_error!("Invalid acknowledgement: invalid signature."));
        }

        let event_hash = acknowledgement.0.payload.content.private_event_hash.clone();

        if current_events.contains_key(&EntryHashB64::from(event_hash.clone())) {
            info!(
                "Received acknowledgement for entry {} from agent {}. {}",
                event_hash,
                provenance,
                agent_info()?.agent_initial_pubkey
            );
            create_relaxed(EntryTypes::Acknowledgement(acknowledgement))?;
        } else {
            create_relaxed(EntryTypes::AwaitingDependencies(
                AwaitingDependencies::Acknowledgement { acknowledgement },
            ))?;
        }
    }

    Ok(())
}

pub fn query_acknowledgements_by_agents() -> ExternResult<BTreeMap<AgentPubKey, BTreeSet<EntryHash>>>
{
    let acknowledgements = query_acknowledgement_entries(())?;

    let mut all_acknowledgements: BTreeMap<AgentPubKey, BTreeSet<EntryHash>> = BTreeMap::new();

    for acknowledgement in acknowledgements {
        all_acknowledgements
            .entry(acknowledgement.0.author)
            .or_insert(BTreeSet::new())
            .insert(acknowledgement.0.payload.content.private_event_hash);
    }

    Ok(all_acknowledgements)
}

#[hdk_extern]
pub fn query_acknowledgement_entries() -> ExternResult<Vec<Acknowledgement>> {
    let filter = ChainQueryFilter::new()
        .entry_type(UnitEntryTypes::Acknowledgement.try_into()?)
        .include_entries(true)
        .action_type(ActionType::Create);
    let records = query(filter)?;
    let mut acknowledgements = records
        .into_iter()
        .map(|r| {
            let Some(entry) = r.entry().as_option().clone() else {
                return Err(wasm_error!("PrivateEvents record contained no entry."));
            };
            let entry = Acknowledgement::try_from(entry)?;
            Ok(entry)
        })
        .collect::<ExternResult<Vec<Acknowledgement>>>()?;

    let mut histories = query_event_histories()?;

    for history in &mut histories {
        acknowledgements.append(&mut history.acknowledgements);
    }

    Ok(acknowledgements)
}
