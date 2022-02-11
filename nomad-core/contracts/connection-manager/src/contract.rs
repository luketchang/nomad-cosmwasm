use common::nomad_base::{HomeDomainHashResponse, LocalDomainResponse, UpdaterResponse};
use common::{addr_to_h256, h256_to_n_byte_addr, home, replica};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, Deps, DepsMut, Env, Event, MessageInfo, Response, StdResult,
};
use cw2::set_contract_version;
use ethers_core::types::{RecoveryMessage, Signature, H160, H256};
use ethers_core::utils::keccak256;
use sha3::{digest::Update, Digest, Keccak256};
use std::convert::TryFrom;

use crate::error::ContractError;
use crate::state::{
    CHAIN_ADDR_LENGTH_BYTES, DOMAIN_TO_REPLICA, HOME, REPLICA_TO_DOMAIN, WATCHER_PERMISSIONS,
};
use common::connection_manager::{
    DomainToReplicaResponse, ExecuteMsg, HomeResponse, InstantiateMsg, IsReplicaResponse, QueryMsg,
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
    CHAIN_ADDR_LENGTH_BYTES.save(deps.storage, &msg.chain_addr_length_bytes)?;

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
        } => execute_unenroll_replica(deps, domain, updater, signature),
        ExecuteMsg::OwnerEnrollReplica { domain, replica } => {
            execute_owner_enroll_replica(deps, info, domain, replica)
        }
        ExecuteMsg::OwnerUnenrollReplica { replica } => {
            execute_owner_unenroll_replica(deps, info, replica)
        }
        ExecuteMsg::SetWatcherPermission {
            watcher,
            domain,
            access,
        } => execute_set_watcher_permission(deps, info, watcher, domain, access),
        ExecuteMsg::SetHome { home } => execute_set_home(deps, info, home),
        ExecuteMsg::RenounceOwnership {} => Ok(ownable::execute_renounce_ownership(deps, info)?),
        ExecuteMsg::TransferOwnership { new_owner } => {
            Ok(ownable::execute_transfer_ownership(deps, info, new_owner)?)
        }
    }
}

pub fn execute_unenroll_replica(
    deps: DepsMut,
    domain: u32,
    updater: H256,
    signature: Vec<u8>,
) -> Result<Response, ContractError> {
    let replica = query_domain_to_replica(deps.as_ref(), domain)?.replica;
    let replica_addr = deps.api.addr_validate(&replica)?;
    if replica_addr == Addr::unchecked("0x0") {
        return Err(ContractError::NotReplicaExists { domain });
    }
    let replica_h256 = addr_to_h256(replica_addr.clone());

    let replica_updater_resp: UpdaterResponse = deps
        .querier
        .query_wasm_smart(replica, &replica::QueryMsg::Updater {})?;
    let replica_updater = replica_updater_resp.updater;

    let provided_updater_addr: H160 = updater.into();
    if replica_updater != provided_updater_addr {
        return Err(ContractError::NotCurrentUpdater {
            address: provided_updater_addr.to_string(),
        });
    }

    let watcher =
        recover_watcher_from_sig(deps.as_ref(), domain, replica_h256, updater, &signature)?;

    let watcher_permission =
        query_watcher_permission(deps.as_ref(), watcher, domain)?.has_permission;
    if !watcher_permission {
        return Err(ContractError::NotWatcherPermission {
            watcher,
            replica: replica_h256,
            domain,
        });
    }

    _unenroll_replica(deps, replica_addr)
}

pub fn execute_owner_enroll_replica(
    mut deps: DepsMut,
    info: MessageInfo,
    domain: u32,
    replica: String,
) -> Result<Response, ContractError> {
    ownable::only_owner(deps.as_ref(), info)?;

    let replica_addr = deps.api.addr_validate(&replica)?;

    // Unenroll existing
    _unenroll_replica(deps.branch(), replica_addr.clone())?;

    // Enroll new
    _enroll_replica(deps, domain, replica_addr)
}

pub fn execute_owner_unenroll_replica(
    mut deps: DepsMut,
    info: MessageInfo,
    replica: String,
) -> Result<Response, ContractError> {
    ownable::only_owner(deps.as_ref(), info)?;

    let replica_addr = deps.api.addr_validate(&replica)?;

    _unenroll_replica(deps.branch(), replica_addr.clone())
}

pub fn execute_set_watcher_permission(
    deps: DepsMut,
    info: MessageInfo,
    watcher: H160,
    domain: u32,
    access: bool,
) -> Result<Response, ContractError> {
    ownable::only_owner(deps.as_ref(), info)?;

    let watcher_domain_hash = watcher_domain_hash(watcher, domain);
    WATCHER_PERMISSIONS.save(deps.storage, watcher_domain_hash.as_bytes(), &access)?;

    Ok(Response::new().add_event(
        Event::new("WatcherPermissionSet")
            .add_attribute("domain", domain.to_string())
            .add_attribute("watcher", format!("{:?}", watcher))
            .add_attribute("permission", (access as u32).to_string()),
    ))
}

pub fn execute_set_home(
    deps: DepsMut,
    info: MessageInfo,
    home: String,
) -> Result<Response, ContractError> {
    ownable::only_owner(deps.as_ref(), info)?;

    let home_addr = deps.api.addr_validate(&home)?;

    HOME.save(deps.storage, &home_addr)?;

    Ok(Response::new().add_event(Event::new("SetHome").add_attribute("new_home", &home)))
}

pub fn _enroll_replica(
    deps: DepsMut,
    domain: u32,
    replica: Addr,
) -> Result<Response, ContractError> {
    DOMAIN_TO_REPLICA.save(deps.storage, domain, &replica)?;
    REPLICA_TO_DOMAIN.save(deps.storage, replica.clone(), &domain)?;

    Ok(Response::new().add_event(
        Event::new("ReplicaEnrolled")
            .add_attribute("domain", domain.to_string())
            .add_attribute("replica", replica.to_string()),
    ))
}

pub fn _unenroll_replica(deps: DepsMut, replica: Addr) -> Result<Response, ContractError> {
    let domain = REPLICA_TO_DOMAIN
        .may_load(deps.storage, replica.clone())?
        .unwrap_or_default();
    DOMAIN_TO_REPLICA.save(deps.storage, domain, &Addr::unchecked("0x0"))?;
    REPLICA_TO_DOMAIN.save(deps.storage, replica.clone(), &0u32)?;

    Ok(Response::new().add_event(
        Event::new("ReplicaUnenrolled")
            .add_attribute("domain", domain.to_string())
            .add_attribute("replica", replica.to_string()),
    ))
}

pub fn recover_watcher_from_sig(
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
            .chain(domain.to_be_bytes())
            .chain(updater)
            .finalize()
            .as_slice(),
    );

    let sig = Signature::try_from(signature)?;
    Ok(sig.recover(RecoveryMessage::Data(digest.as_bytes().to_vec()))?)
}

pub fn watcher_domain_hash(watcher: H160, domain: u32) -> H256 {
    let mut buf = watcher.to_fixed_bytes().to_vec();
    buf.append(&mut domain.to_be_bytes().to_vec());
    keccak256(buf).into()
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
            .chain(domain.to_be_bytes())
            .chain(updater)
            .finalize()
            .as_slice(),
    );

    let sig = Signature::try_from(signature)?;
    Ok(sig.recover(RecoveryMessage::Data(digest.as_bytes().to_vec()))?)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Home {} => to_binary(&query_home(deps)?),
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
        QueryMsg::ChainAddrLengthBytes {} => to_binary(&query_chain_addr_length_bytes(deps)?),
    }
}

pub fn query_home(deps: Deps) -> StdResult<HomeResponse> {
    let home = HOME
        .may_load(deps.storage)?
        .unwrap_or(Addr::unchecked("0x0"));
    Ok(HomeResponse {
        home: home.to_string(),
    })
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
    watcher: H160,
    domain: u32,
) -> StdResult<WatcherPermissionResponse> {
    let watcher_domain_hash = watcher_domain_hash(watcher, domain);
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
        != 0;
    Ok(IsReplicaResponse { is_replica })
}

pub fn query_local_domain(deps: Deps) -> StdResult<LocalDomainResponse> {
    let home_addr = HOME.load(deps.storage)?;

    let query_msg = home::QueryMsg::LocalDomain {};
    let local_domain_resp: LocalDomainResponse =
        deps.querier.query_wasm_smart(home_addr, &query_msg)?;
    Ok(local_domain_resp)
}

pub fn query_chain_addr_length_bytes(deps: Deps) -> StdResult<usize> {
    CHAIN_ADDR_LENGTH_BYTES.load(deps.storage)
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::ownable::OwnerResponse;
    use cosmwasm_std::testing::{mock_dependencies_with_balance, mock_env, mock_info};
    use cosmwasm_std::{coins, from_binary};
    use ethers_signers::{LocalWallet, Signer};

    const CHAIN_ADDR_LENGTH_BYTES: usize = 42;
    const REPLICA_DOMAIN: u32 = 2000;
    const WATCHER_PRIVKEY: &str =
        "1111111111111111111111111111111111111111111111111111111111111111";

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies_with_balance(&coins(2, "token"));

        let msg = InstantiateMsg {
            chain_addr_length_bytes: CHAIN_ADDR_LENGTH_BYTES,
        };
        let info = mock_info("owner", &coins(100, "earth"));

        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // Owner
        let owner_res = query(deps.as_ref(), mock_env(), QueryMsg::Owner {}).unwrap();
        let value: OwnerResponse = from_binary(&owner_res).unwrap();
        assert_eq!("owner", value.owner);

        // Home 0x0
        let home_res = query(deps.as_ref(), mock_env(), QueryMsg::Home {}).unwrap();
        let value: HomeResponse = from_binary(&home_res).unwrap();
        assert_eq!("0x0", value.home);
    }

    #[test]
    fn only_owner_restricts_access() {
        let mut deps = mock_dependencies_with_balance(&coins(2, "token"));

        let msg = InstantiateMsg {
            chain_addr_length_bytes: CHAIN_ADDR_LENGTH_BYTES,
        };
        let info = mock_info("owner", &coins(100, "earth"));

        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        let not_owner_info = mock_info("not_owner", &coins(100, "earth"));
        let msg = ExecuteMsg::SetHome {
            home: "home".to_owned(),
        };
        let res = execute(deps.as_mut(), mock_env(), not_owner_info, msg);
        assert!(res.is_err());
        assert!(res.err().unwrap().to_string().contains("Unauthorized"));
    }

    #[test]
    fn owner_sets_home() {
        let mut deps = mock_dependencies_with_balance(&coins(2, "token"));

        let msg = InstantiateMsg {
            chain_addr_length_bytes: CHAIN_ADDR_LENGTH_BYTES,
        };
        let info = mock_info("owner", &coins(100, "earth"));

        let res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
        assert_eq!(0, res.messages.len());

        // Owner executes set home
        let msg = ExecuteMsg::SetHome {
            home: "home".to_owned(),
        };
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // Check home is now "home"
        let home_res = query(deps.as_ref(), mock_env(), QueryMsg::Home {}).unwrap();
        let value: HomeResponse = from_binary(&home_res).unwrap();
        assert_eq!("home", value.home);
    }

    #[test]
    fn onwer_enrolls_and_unenrolls_replica() {
        let mut deps = mock_dependencies_with_balance(&coins(2, "token"));

        let msg = InstantiateMsg {
            chain_addr_length_bytes: CHAIN_ADDR_LENGTH_BYTES,
        };
        let info = mock_info("owner", &coins(100, "earth"));

        let res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
        assert_eq!(0, res.messages.len());

        let replica_addr = Addr::unchecked("replica");

        // Check replica not already enrolled
        let msg = QueryMsg::IsReplica {
            replica: replica_addr.to_string(),
        };
        let res = query(deps.as_ref(), mock_env(), msg).unwrap();
        let value: IsReplicaResponse = from_binary(&res).unwrap();
        assert!(!value.is_replica);

        // Owner enrolls replica
        let msg = ExecuteMsg::OwnerEnrollReplica {
            domain: REPLICA_DOMAIN,
            replica: replica_addr.to_string(),
        };
        execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        // Check replica enrolled
        let res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::DomainToReplica {
                domain: REPLICA_DOMAIN,
            },
        )
        .unwrap();
        let value: DomainToReplicaResponse = from_binary(&res).unwrap();
        assert_eq!("replica", value.replica);

        let res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::ReplicaToDomain {
                replica: replica_addr.to_string(),
            },
        )
        .unwrap();
        let value: ReplicaToDomainResponse = from_binary(&res).unwrap();
        assert_eq!(2000, value.domain);

        // Owner unenrolls replica
        let msg = ExecuteMsg::OwnerUnenrollReplica {
            replica: replica_addr.to_string(),
        };
        execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        // Check replica unenrolled
        let res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::DomainToReplica {
                domain: REPLICA_DOMAIN,
            },
        )
        .unwrap();
        let value: DomainToReplicaResponse = from_binary(&res).unwrap();
        assert_eq!("0x0", value.replica);

        let res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::ReplicaToDomain {
                replica: replica_addr.to_string(),
            },
        )
        .unwrap();
        let value: ReplicaToDomainResponse = from_binary(&res).unwrap();
        assert_eq!(0, value.domain);
    }

    #[test]
    fn owner_sets_watcher_permissions() {
        let watcher: LocalWallet = WATCHER_PRIVKEY.parse().unwrap();

        let mut deps = mock_dependencies_with_balance(&coins(2, "token"));

        let msg = InstantiateMsg {
            chain_addr_length_bytes: CHAIN_ADDR_LENGTH_BYTES,
        };
        let info = mock_info("owner", &coins(100, "earth"));

        let res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
        assert_eq!(0, res.messages.len());

        // Watcher starts with no access
        let res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::WatcherPermission {
                watcher: watcher.address(),
                domain: REPLICA_DOMAIN,
            },
        )
        .unwrap();
        let value: WatcherPermissionResponse = from_binary(&res).unwrap();
        assert!(!value.has_permission);

        // Set watcher permission
        let msg = ExecuteMsg::SetWatcherPermission {
            watcher: watcher.address(),
            domain: REPLICA_DOMAIN,
            access: true,
        };
        execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        // Check watcher has permission
        let res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::WatcherPermission {
                watcher: watcher.address(),
                domain: REPLICA_DOMAIN,
            },
        )
        .unwrap();
        let value: WatcherPermissionResponse = from_binary(&res).unwrap();
        assert!(value.has_permission);
    }
}
