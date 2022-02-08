use common::home;
use common::nomad_base::LocalDomainResponse;
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use cw2::set_contract_version;
use ethers_core::{types::H256, utils::keccak256};

use crate::error::ContractError;
use crate::state::{DOMAIN_TO_REPLICA, HOME, REPLICA_TO_DOMAIN, WATCHER_PERMISSIONS};
use common::connection_manager::{
    DomainToReplicaResponse, ExecuteMsg, InstantiateMsg, IsReplicaResponse, QueryMsg,
    ReplicaToDomainResponse, WatcherPermissionResponse,
};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:connection-manager";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    ownable::instantiate(deps.branch(), env, info, common::ownable::InstantiateMsg {})?;
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

pub fn addr_domain_hash(addr: Addr, domain: u32) -> H256 {
    let domain_addr = addr.to_string() + &domain.to_string();
    keccak256(domain_addr.as_bytes()).into()
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::DomainToReplica { domain } => to_binary(&query_domain_to_replica(deps, domain)?),
        QueryMsg::ReplicaToDomain { replica } => {
            to_binary(&query_replica_to_domain(deps, replica)?)
        }
        QueryMsg::WatcherPermission { domain, watcher } => {
            to_binary(&query_watcher_permission(deps, domain, watcher)?)
        }
        QueryMsg::IsReplica { replica } => to_binary(&query_is_replica(deps, replica)?),
        QueryMsg::LocalDomain {} => to_binary(&query_local_domain(deps)?),
        QueryMsg::Owner {} => to_binary(&ownable::query_owner(deps)?),
    }
}

pub fn query_domain_to_replica(deps: Deps, domain: u32) -> StdResult<DomainToReplicaResponse> {
    let replica_addr = DOMAIN_TO_REPLICA
        .may_load(deps.storage, domain)?
        .unwrap_or(Addr::unchecked("0x0"));
    Ok(DomainToReplicaResponse {
        replica: replica_addr.to_string(),
    })
}

pub fn query_replica_to_domain(deps: Deps, replica: String) -> StdResult<ReplicaToDomainResponse> {
    let replica_addr = deps.api.addr_validate(&replica)?;
    let domain = REPLICA_TO_DOMAIN
        .may_load(deps.storage, replica_addr)?
        .unwrap_or_default();
    Ok(ReplicaToDomainResponse { domain })
}

pub fn query_watcher_permission(
    deps: Deps,
    domain: u32,
    replica: String,
) -> StdResult<WatcherPermissionResponse> {
    let replica_addr = deps.api.addr_validate(&replica)?;
    let addr_domain_hash = addr_domain_hash(replica_addr, domain);

    let has_permission = WATCHER_PERMISSIONS
        .may_load(deps.storage, addr_domain_hash.as_bytes())?
        .unwrap_or(false);

    Ok(WatcherPermissionResponse { has_permission })
}

pub fn query_is_replica(deps: Deps, replica: String) -> StdResult<IsReplicaResponse> {
    let replica_addr = deps.api.addr_validate(&replica)?;
    let is_replica = REPLICA_TO_DOMAIN
        .may_load(deps.storage, replica_addr)?
        .unwrap_or_default()
        == 0;
    Ok(IsReplicaResponse { is_replica })
}

pub fn query_local_domain(deps: Deps) -> StdResult<LocalDomainResponse> {
    let home_addr = HOME.load(deps.storage)?;

    let query_msg = home::QueryMsg::LocalDomain {};
    let local_domain_resp: LocalDomainResponse =
        deps.querier.query_wasm_smart(home_addr, &query_msg)?;
    Ok(local_domain_resp)
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::ownable::OwnerResponse;
    use cosmwasm_std::testing::{mock_dependencies_with_balance, mock_env, mock_info};
    use cosmwasm_std::{coins, from_binary};

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies_with_balance(&coins(2, "token"));

        let msg = InstantiateMsg {};
        let info = mock_info("owner", &coins(100, "earth"));

        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // Owner
        let owner_res = query(deps.as_ref(), mock_env(), QueryMsg::Owner {}).unwrap();
        let value: OwnerResponse = from_binary(&owner_res).unwrap();
        assert_eq!("owner", value.owner);
    }
}
