#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use cw2::set_contract_version;
use ethers_core::types::H256;

use crate::error::ContractError;
use crate::msg::{
    AcceptableRootResponse, ConfirmAtResponse, ExecuteMsg, InstantiateMsg, MessageStatusResponse,
    OptimisticSecondsResponse, QueryMsg, RemoteDomainResponse,
};
use crate::state::{CONFIRM_AT, MESSAGES, OPTIMISTIC_SECONDS, REMOTE_DOMAIN};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:replica";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    nomad_base::contract::instantiate(deps.branch(), env, info, msg.clone().into())?;

    REMOTE_DOMAIN.save(deps.storage, &msg.remote_domain)?;
    OPTIMISTIC_SECONDS.save(deps.storage, &msg.optimistic_seconds)?;
    nomad_base::contract::_set_committed_root(deps.branch(), msg.committed_root)?;

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
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
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::AcceptableRoot { root } => to_binary(&query_acceptable_root(deps, env, root)?),
        QueryMsg::ConfirmAt { root } => to_binary(&query_confirm_at(deps, root)?),
        QueryMsg::MessageStatus { leaf } => to_binary(&query_message_status(deps, leaf)?),
        QueryMsg::OptimisticSeconds {} => {
            let opt = query_optimistic_seconds(deps)?;
            println!("{:?}", opt);
            return to_binary(&opt);
        }
        QueryMsg::RemoteDomain {} => to_binary(&query_remote_domain(deps)?),
        QueryMsg::CommittedRoot {} => to_binary(&nomad_base::contract::query_committed_root(deps)?),
        QueryMsg::HomeDomainHash {} => {
            to_binary(&nomad_base::contract::query_home_domain_hash(deps)?)
        }
        QueryMsg::LocalDomain {} => to_binary(&nomad_base::contract::query_local_domain(deps)?),
        QueryMsg::State {} => to_binary(&nomad_base::contract::query_state(deps)?),
        QueryMsg::Updater {} => to_binary(&nomad_base::contract::query_updater(deps)?),
        QueryMsg::Owner {} => to_binary(&ownable::contract::query_owner(deps)?),
    }
}

pub fn query_acceptable_root(
    deps: Deps,
    env: Env,
    root: H256,
) -> StdResult<AcceptableRootResponse> {
    let confirm_at = query_confirm_at(deps, root)?.confirm_at;
    if confirm_at == 0 {
        return Ok(AcceptableRootResponse { acceptable: false });
    }

    Ok(AcceptableRootResponse {
        acceptable: env.block.time.seconds() as u64 >= confirm_at,
    })
}

pub fn query_confirm_at(deps: Deps, root: H256) -> StdResult<ConfirmAtResponse> {
    let confirm_at = CONFIRM_AT.load(deps.storage, root.as_bytes())?;
    Ok(ConfirmAtResponse { confirm_at })
}

pub fn query_message_status(deps: Deps, leaf: H256) -> StdResult<MessageStatusResponse> {
    let status = MESSAGES.load(deps.storage, leaf.as_bytes())?;
    Ok(MessageStatusResponse { status })
}

pub fn query_optimistic_seconds(deps: Deps) -> StdResult<OptimisticSecondsResponse> {
    let optimistic_seconds = OPTIMISTIC_SECONDS.load(deps.storage)?;
    println!("optimistic seconds: {}", optimistic_seconds);
    Ok(OptimisticSecondsResponse { optimistic_seconds })
}

pub fn query_remote_domain(deps: Deps) -> StdResult<RemoteDomainResponse> {
    let remote_domain = REMOTE_DOMAIN.load(deps.storage)?;
    Ok(RemoteDomainResponse { remote_domain })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies_with_balance, mock_env, mock_info};
    use cosmwasm_std::{coins, from_binary};
    use nomad_base::msg::{LocalDomainResponse, StateResponse, UpdaterResponse};

    const LOCAL_DOMAIN: u32 = 1000;
    const REMOTE_DOMAIN: u32 = 2000;
    const UPDATER_PUBKEY: &str = "0x19e7e376e7c213b7e7e7e46cc70a5dd086daff2a";

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies_with_balance(&coins(2, "token"));

        let optimistic_seconds = 100u64;

        let msg = InstantiateMsg {
            local_domain: LOCAL_DOMAIN,
            remote_domain: REMOTE_DOMAIN,
            updater: UPDATER_PUBKEY.to_owned(),
            committed_root: H256::zero(),
            optimistic_seconds,
        };
        let info = mock_info("owner", &coins(100, "earth"));

        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // ------ REPLICA ------
        // Remote domain
        let res = query(deps.as_ref(), mock_env(), QueryMsg::RemoteDomain {}).unwrap();
        let value: RemoteDomainResponse = from_binary(&res).unwrap();
        assert_eq!(REMOTE_DOMAIN, value.remote_domain);

        // Optimistic seconds
        let res = query(deps.as_ref(), mock_env(), QueryMsg::OptimisticSeconds {}).unwrap();
        println!("{:?}", from_binary::<OptimisticSecondsResponse>(&res));
        let value: OptimisticSecondsResponse = from_binary(&res).unwrap();
        assert_eq!(optimistic_seconds, value.optimistic_seconds);

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
    }
}
