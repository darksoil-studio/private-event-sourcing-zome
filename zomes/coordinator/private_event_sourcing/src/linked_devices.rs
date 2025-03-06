use hdk::prelude::*;
use serde::de::DeserializeOwned;

pub fn call_local_zome<P, R>(
    zome_name: ZomeName,
    fn_name: FunctionName,
    payload: P,
) -> ExternResult<Option<R>>
where
    P: serde::Serialize + std::fmt::Debug,
    R: DeserializeOwned + std::fmt::Debug,
{
    let call_result = call(
        CallTargetCell::Local,
        zome_name.clone(),
        fn_name.clone(),
        None,
        payload,
    );

    match call_result {
        Ok(response) => match response {
            ZomeCallResponse::Ok(result) => {
                let result: R = result.decode().map_err(|err| wasm_error!(err))?;
                Ok(Some(result))
            }
            _ => Err(wasm_error!(WasmErrorInner::Guest(format!(
                "Failed to call {zome_name}/{fn_name}: {response:?}"
            )))),
        },
        Err(err) => {
            if format!("{err:?}").contains("Zome not found") {
                return Ok(None);
            }
            return Err(err);
        }
    }
}

pub fn linked_devices_zome_name() -> Option<ZomeName> {
    match std::option_env!("LINKED_DEVICES_COORDINATOR_ZOME_NAME") {
        Some(zome_name) => Some(zome_name.into()),
        None => None,
    }
}

pub fn get_linked_devices_for(agent: AgentPubKey) -> ExternResult<Vec<AgentPubKey>> {
    let Some(zome_name) = linked_devices_zome_name() else {
        return Ok(Vec::new());
    };
    let Some(links): Option<Vec<Link>> =
        call_local_zome(zome_name, "get_linked_devices_for_agent".into(), agent)?
    else {
        return Ok(vec![]);
    };

    let linked_devices: Vec<AgentPubKey> = links
        .into_iter()
        .filter_map(|link| link.target.into_agent_pub_key())
        .collect();

    Ok(linked_devices)
}

pub fn get_all_agents_for(agent: AgentPubKey) -> ExternResult<Vec<AgentPubKey>> {
    let mut agents = get_linked_devices_for(agent.clone())?;
    agents.push(agent);
    Ok(agents)
}

pub fn query_all_my_agents() -> ExternResult<Vec<AgentPubKey>> {
    let mut agents = query_my_linked_devices()?;
    agents.push(agent_info()?.agent_latest_pubkey);
    Ok(agents)
}

pub fn query_my_linked_devices() -> ExternResult<Vec<AgentPubKey>> {
    let Some(zome_name) = linked_devices_zome_name() else {
        return Ok(Vec::new());
    };
    let Some(links): Option<Vec<Link>> =
        call_local_zome(zome_name, "query_my_linked_devices".into(), ())?
    else {
        return Ok(vec![]);
    };

    let agents = links
        .into_iter()
        .filter_map(|link| link.target.into_agent_pub_key())
        .collect();

    Ok(agents)
}
