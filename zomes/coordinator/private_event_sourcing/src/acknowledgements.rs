use std::collections::BTreeMap;

use hdk::prelude::*;
use private_event_sourcing_integrity::*;

use crate::{query_event_histories, utils::create_relaxed, PrivateEvent};

pub fn create_acknowledgements<T: PrivateEvent>(
    events_hashes: BTreeSet<EntryHashB64>,
) -> ExternResult<()> {
    let acknowledgement_content = AcknowledgementContent {
        received_private_events_hashes: events_hashes.into_iter().map(Into::into).collect(),
        timestamp: sys_time()?,
    };
    let signature = sign(agent_info()?.agent_initial_pubkey, &acknowledgement_content)?;

    create_relaxed(EntryTypes::Acknowledgement(Acknowledgement {
        author: agent_info()?.agent_initial_pubkey,
        signature,
        content: acknowledgement_content,
    }))?;

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
                acknowledgement.content.received_private_events_hashes,
            ))
        })
        .collect::<ExternResult<Vec<(AgentPubKey, BTreeSet<EntryHash>)>>>()?;

    let mut all_acknowledgements: BTreeMap<AgentPubKey, BTreeSet<EntryHash>> = BTreeMap::new();

    for (agent, mut received_events) in events_sent_to_recipients {
        all_acknowledgements
            .entry(agent)
            .or_insert(BTreeSet::new())
            .append(&mut received_events);
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
