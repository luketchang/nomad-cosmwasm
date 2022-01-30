#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use cw2::set_contract_version;
use std::collections::HashMap;

use crate::error::ContractError;
use crate::msg::{
    ExecuteMsg, InstantiateMsg, NoncesResponse, QueryMsg, SuggestUpdateResponse,
    UpdaterManagerResponse,
};
use crate::state::{State, NONCES, UPDATER_MANAGER};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:home";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    ownable::contract::instantiate(deps.branch(), env.clone(), info.clone(), msg.clone().into())?;
    queue::contract::instantiate(deps.branch(), env.clone(), info.clone(), msg.clone().into())?;
    merkle::contract::instantiate(deps.branch(), env.clone(), info.clone(), msg.clone().into())?;
    nomad_base::contract::instantiate(
        deps.branch(),
        env.clone(),
        info.clone(),
        msg.clone().into(),
    )?;

    println!("Initialized child contracts!");

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    UPDATER_MANAGER.save(deps.storage, &Addr::unchecked("0x0"))?;
    println!("Initialized own state");

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Nonces { domain } => to_binary(&query_nonces(deps, domain)?),
        QueryMsg::SuggestUpdate {} => to_binary(&query_suggest_update(deps)?),
        QueryMsg::UpdaterManager {} => to_binary(&query_updater_manager(deps)?),
        QueryMsg::State {} => to_binary(&nomad_base::contract::query_state(deps)?),
        QueryMsg::CommittedRoot {} => to_binary(&nomad_base::contract::query_committed_root(deps)?),
        QueryMsg::HomeDomainHash {} => {
            to_binary(&nomad_base::contract::query_home_domain_hash(deps)?)
        }
        QueryMsg::LocalDomain {} => to_binary(&nomad_base::contract::query_local_domain(deps)?),
        QueryMsg::Updater {} => to_binary(&nomad_base::contract::query_updater(deps)?),
        QueryMsg::Count {} => to_binary(&merkle::contract::query_count(deps)?),
        QueryMsg::Root {} => to_binary(&merkle::contract::query_root(deps)?),
        QueryMsg::Tree {} => to_binary(&merkle::contract::query_tree(deps)?),
        QueryMsg::QueueContains { item } => {
            to_binary(&queue::contract::query_contains(deps, item)?)
        }
        QueryMsg::QueueEnd {} => to_binary(&queue::contract::query_last_item(deps)?),
        QueryMsg::QueueLength {} => to_binary(&queue::contract::query_length(deps)?),
        QueryMsg::Owner {} => to_binary(&ownable::contract::query_owner(deps)?),
    }
}

pub fn query_nonces(deps: Deps, domain: u32) -> StdResult<NoncesResponse> {
    Ok(NoncesResponse {
        next_nonce: NONCES.may_load(deps.storage, domain)?.unwrap_or_default(),
    })
}

pub fn query_suggest_update(deps: Deps) -> StdResult<SuggestUpdateResponse> {
    let committed_root = nomad_base::contract::query_committed_root(deps)?.committed_root;
    let new_root = queue::contract::query_last_item(deps)?.item;
    Ok(SuggestUpdateResponse {
        committed_root,
        new_root,
    })
}

pub fn query_updater_manager(deps: Deps) -> StdResult<UpdaterManagerResponse> {
    let updater_manager = UPDATER_MANAGER.load(deps.storage)?;
    Ok(UpdaterManagerResponse {
        updater_manager: updater_manager.into_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies_with_balance, mock_env, mock_info};
    use cosmwasm_std::{coins, from_binary};
    use merkle::merkle_tree::INITIAL_ROOT;
    use merkle::msg::RootResponse;
    use nomad_base::msg::{LocalDomainResponse, StateResponse, UpdaterResponse};
    use queue::msg::{LastItemResponse, LengthResponse};

    const LOCAL_DOMAIN: u32 = 1000;
    const UPDATER_PRIVKEY: &str =
        "1111111111111111111111111111111111111111111111111111111111111111";
    const UPDATER_PUBKEY: &str = "0x19e7e376e7c213b7e7e7e46cc70a5dd086daff2a";

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies_with_balance(&coins(2, "token"));

        let msg = InstantiateMsg {
            local_domain: LOCAL_DOMAIN,
            updater: UPDATER_PUBKEY.to_owned(),
        };
        let info = mock_info("creator", &coins(100, "earth"));

        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // ------ HOME ------
        // Nonces
        let res = query(deps.as_ref(), mock_env(), QueryMsg::Nonces { domain: 100 }).unwrap();
        let value: NoncesResponse = from_binary(&res).unwrap();
        assert_eq!(0, value.next_nonce);

        // Empty updater manager
        let res = query(deps.as_ref(), mock_env(), QueryMsg::UpdaterManager {}).unwrap();
        let value: UpdaterManagerResponse = from_binary(&res).unwrap();
        assert_eq!("0x0", value.updater_manager);

        // Suggested update 0x0 and 0x0
        let res = query(deps.as_ref(), mock_env(), QueryMsg::SuggestUpdate {}).unwrap();
        let value: SuggestUpdateResponse = from_binary(&res).unwrap();
        assert_eq!([0u8; 32], value.committed_root);
        assert_eq!([0u8; 32], value.new_root);

        // ------ NOMAD_BASE ------
        // State
        let res = query(deps.as_ref(), mock_env(), QueryMsg::State {}).unwrap();
        let value: StateResponse = from_binary(&res).unwrap();
        assert_eq!(1, value.state);

        // Local domain
        let res = query(deps.as_ref(), mock_env(), QueryMsg::LocalDomain {}).unwrap();
        let value: LocalDomainResponse = from_binary(&res).unwrap();
        assert_eq!(LOCAL_DOMAIN, value.local_domain);

        // Updater
        let res = query(deps.as_ref(), mock_env(), QueryMsg::Updater {}).unwrap();
        let value: UpdaterResponse = from_binary(&res).unwrap();
        assert_eq!(UPDATER_PUBKEY.to_owned(), value.updater);

        // ------ MERKLE ------
        // Initial root valid
        let res = query(deps.as_ref(), mock_env(), QueryMsg::Root {}).unwrap();
        let value: RootResponse = from_binary(&res).unwrap();
        assert_eq!(*INITIAL_ROOT, value.root);

        // ------ QUEUE ------
        // Length 0
        let res = query(deps.as_ref(), mock_env(), QueryMsg::QueueLength {}).unwrap();
        let value: LengthResponse = from_binary(&res).unwrap();
        assert_eq!(0, value.length);

        // Last item defaults to 0x0
        let res = query(deps.as_ref(), mock_env(), QueryMsg::QueueEnd {}).unwrap();
        let value: LastItemResponse = from_binary(&res).unwrap();
        assert_eq!([0u8; 32], value.item);
    }
}
