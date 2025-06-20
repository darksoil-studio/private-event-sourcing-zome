use std::collections::BTreeMap;

use hdk::prelude::*;
use private_event_sourcing_integrity::*;

use crate::{
    query_event_histories, query_private_event_entries, query_private_event_entry,
    utils::create_relaxed, PrivateEvent, PrivateEventSourcingRemoteSignal,
};

pub fn create_acknowledgements<T: PrivateEvent>(
    events_hashes: BTreeSet<EntryHashB64>,
) -> ExternResult<()> {
    for event_hash in events_hashes {
        create_acknowledgement::<T>(event_hash)?;
    }

    Ok(())
}
pub fn create_acknowledgement<T: PrivateEvent>(event_hash: EntryHashB64) -> ExternResult<()> {
    let Some(private_event) = query_private_event_entry(event_hash.clone().into())? else {
        return Err(wasm_error!("Could not find private event"));
    };
    let acknowledgements = query_acknowledgement_entries()?;

    let acknowledgement = if let Some(ack) = acknowledgements.iter().find(|a| {
        a.content
            .private_event_hash
            .eq(&EntryHash::from(event_hash.clone()))
    }) {
        ack.clone()
    } else {
        let acknowledgement_content = AcknowledgementContent {
            private_event_hash: event_hash.into(),
            timestamp: sys_time()?,
        };
        let signature = sign(agent_info()?.agent_initial_pubkey, &acknowledgement_content)?;

        let acknowledgement = Acknowledgement {
            author: agent_info()?.agent_initial_pubkey,
            signature,
            content: acknowledgement_content,
        };

        create_relaxed(EntryTypes::Acknowledgement(acknowledgement.clone()))?;
        acknowledgement
    };

    send_remote_signal(
        PrivateEventSourcingRemoteSignal::SendAcknowledgements(vec![acknowledgement.clone()]),
        vec![private_event.0.author.clone()],
    )?;

    T::send_acknowledgement(private_event.0.author, acknowledgement)?;

    Ok(())
}

pub fn receive_acknowledgements<T: PrivateEvent>(
    acknowledgements: Vec<Acknowledgement>,
) -> ExternResult<()> {
    let current_acknowledgements = query_acknowledgement_entries()?;
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

        let valid = verify_signature(
            acknowledgement.author.clone(),
            acknowledgement.signature.clone(),
            &acknowledgement.content,
        )?;

        if !valid {
            return Err(wasm_error!("Invalid acknowledgement: invalid signature."));
        }

        if current_events.contains_key(&EntryHashB64::from(
            acknowledgement.content.private_event_hash.clone(),
        )) {
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
    let acknowledgements = query_acknowledgement_entries()?;
    let events_sent_to_recipients = acknowledgements
        .into_iter()
        .map(|acknowledgement| {
            Ok((
                acknowledgement.author,
                acknowledgement.content.private_event_hash,
            ))
        })
        .collect::<ExternResult<Vec<(AgentPubKey, EntryHash)>>>()?;

    let mut all_acknowledgements: BTreeMap<AgentPubKey, BTreeSet<EntryHash>> = BTreeMap::new();

    for (agent, received_event) in events_sent_to_recipients {
        all_acknowledgements
            .entry(agent)
            .or_insert(BTreeSet::new())
            .insert(received_event);
    }

    Ok(all_acknowledgements)
}

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
