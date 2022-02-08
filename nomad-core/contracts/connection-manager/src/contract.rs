use common::nomad_base::{HomeDomainHashResponse, LocalDomainResponse, UpdaterResponse};
use common::{h256_to_n_byte_addr, home, replica};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use cw2::set_contract_version;
use ethers_core::types::{RecoveryMessage, Signature, H160, H256};
use ethers_core::utils::keccak256;
use sha3::{digest::Update, Digest, Keccak256};

use crate::error::ContractError;
use crate::state::{
    CHAIN_ADDR_LENGTH_BYTES, DOMAIN_TO_REPLICA, HOME, REPLICA_TO_DOMAIN, WATCHER_PERMISSIONS,
};
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
    match msg {
        ExecuteMsg::UnenrollReplica {
            domain,
            updater,
            signature,
        } => try_unenroll_replica(deps, domain, updater, signature),
    }
}

pub fn try_unenroll_replica(
    deps: DepsMut,
    domain: u32,
    updater: H256,
    signature: Vec<u8>,
) -> Result<Response, ContractError> {
    let replica = query_domain_to_replica(deps.as_ref(), domain)?.replica;
    if &replica == "0x0" {
        return Err(ContractError::ReplicaNotExists { domain });
    }

    let replica_updater_resp: UpdaterResponse = deps
        .querier
        .query_wasm_smart(replica, &replica::QueryMsg::Updater {})?;
    let replica_updater_addr = deps.api.addr_validate(&replica_updater_resp.updater)?;

    let addr_length = CHAIN_ADDR_LENGTH_BYTES.load(deps.storage)?;
    let provided_updater_addr = h256_to_n_byte_addr(deps.as_ref(), addr_length, updater);

    if replica_updater_addr != provided_updater_addr {
        return Err(ContractError::NotCurrentUpdater { address: provided_updater_addr.to_string() });
    }

    let watcher = recover_from_watcher_sig(deps.as_ref(), domain, replica, updater, &signature)?;
    let watcher_perms_key = watcher_domain_hash(deps.as_ref(), watcher, domain);

    Ok(Response::new())
}

pub fn recover_from_watcher_sig(
    deps: Deps,
    domain: u32,
    replica: H256,
    updater: H256,
    signature: &[u8],
) -> Result<H160, ContractError> {
    let addr_length = CHAIN_ADDR_LENGTH_BYTES.load(deps.storage)?;
    let replica_addr = h256_to_n_byte_addr(deps.clone(), addr_length, replica);

    let home_domain_hash_resp: HomeDomainHashResponse = deps
        .querier
        .query_wasm_smart(replica_addr, &replica::QueryMsg::HomeDomainHash {})?;
    let home_domain_hash = home_domain_hash_resp.home_domain_hash;

    let digest = H256::from_slice(
        Keccak256::new()
            .chain(home_domain_hash)
            .chain(domain)
            .chain(updater)
            .finalize()
            .as_slice(),
    );

    let sig = Signature::try_from(signature)?;
    Ok(sig.recover(RecoveryMessage::Data(digest.as_bytes().to_vec()))?)
}

pub fn watcher_domain_hash(deps: Deps, watcher: H256, domain: u32) -> H256 {
    let addr_length = CHAIN_ADDR_LENGTH_BYTES.load(deps.storage)?;
    let watcher_domain_concat =
        h256_to_n_byte_addr(deps, addr_length, watcher).to_string() + &domain.to_string();
    keccak256(watcher_domain_concat.as_bytes()).into()
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::DomainToReplica { domain } => to_binary(&query_domain_to_replica(deps, domain)?),
        QueryMsg::ReplicaToDomain { replica } => {
            to_binary(&query_replica_to_domain(deps, replica)?)
        }
        QueryMsg::WatcherPermission { watcher, domain } => {
            to_binary(&query_watcher_permission(deps, watcher, domain)?)
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
    watcher: H256,
    domain: u32,
) -> StdResult<WatcherPermissionResponse> {
    let watcher_domain_hash = watcher_domain_hash(deps.clone(), watcher, domain);
    let has_permission = WATCHER_PERMISSIONS
        .may_load(deps.storage, watcher_domain_hash.as_bytes())?
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
