#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use cw2::set_contract_version;
use ethers::core::types::{RecoveryMessage, Signature, H160, H256};
use sha3::{digest::Update, Digest, Keccak256};
use std::str::FromStr;

use crate::error::ContractError;
use crate::msg::{
    CommittedRootResponse, ExecuteMsg, HomeDomainHashResponse, InstantiateMsg, LocalDomainResponse,
    QueryMsg, StateResponse, UpdaterResponse,
};
use crate::state::{State, STATE};
use ownable::contract::{query_owner, try_renounce_ownership, try_transfer_ownership};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:ownable";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let updater = deps.api.addr_validate(&msg.updater)?;

    let state = State {
        local_domain: msg.local_domain,
        updater,
        state: crate::state::States::Active,
        committed_root: [0u8; 32],
    };

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    STATE.save(deps.storage, &state)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("local_domain", msg.local_domain.to_string())
        .add_attribute("updater", msg.updater))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::DoubleUpdate {
            old_root,
            new_root,
            signature,
            signature_2,
        } => try_double_update(deps, old_root, new_root, signature, signature_2),
        ExecuteMsg::RenounceOwnership {} => Ok(try_renounce_ownership(deps, info)?),
        ExecuteMsg::TransferOwnership { new_owner } => {
            Ok(try_transfer_ownership(deps, info, new_owner)?)
        }
    }
}

pub fn try_double_update(
    deps: DepsMut,
    old_root: [u8; 32],
    new_root: [u8; 32],
    signature: String,
    signature_2: String,
) -> Result<Response, ContractError> {
    if is_updater_signature(deps.as_ref(), old_root, new_root, signature)?
        && is_updater_signature(deps.as_ref(), old_root, new_root, signature_2)?
        && new_root != old_root
    {}

    Ok(Response::new())
}

fn is_updater_signature(
    deps: Deps,
    old_root: [u8; 32],
    new_root: [u8; 32],
    signature: String,
) -> Result<bool, ContractError> {
    let home_domain_hash = query_home_domain_hash(deps)?.home_domain_hash;
    let updater = query_updater(deps)?.updater;

    let digest = H256::from_slice(
        Keccak256::new()
            .chain(home_domain_hash)
            .chain(old_root)
            .chain(new_root)
            .finalize()
            .as_slice(),
    );

    let sig = Signature::from_str(&signature)?;
    let recovered_address = sig.recover(RecoveryMessage::Hash(digest))?;
    Ok(H160::from_str(&updater).unwrap() == recovered_address)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::CommittedRoot {} => to_binary(&query_committed_root(deps)?),
        QueryMsg::HomeDomainHash {} => to_binary(&query_home_domain_hash(deps)?),
        QueryMsg::LocalDomain {} => to_binary(&query_local_domain(deps)?),
        QueryMsg::State {} => to_binary(&query_state(deps)?),
        QueryMsg::Updater {} => to_binary(&query_updater(deps)?),
        QueryMsg::Owner {} => to_binary(&query_owner(deps)?),
    }
}

fn query_committed_root(deps: Deps) -> StdResult<CommittedRootResponse> {
    let state = STATE.load(deps.storage)?;
    Ok(CommittedRootResponse {
        committed_root: state.committed_root,
    })
}

fn query_home_domain_hash(deps: Deps) -> StdResult<HomeDomainHashResponse> {
    let state = STATE.load(deps.storage)?;
    let domain = state.local_domain;

    let home_domain_hash = <[u8; 32]>::from(
        Keccak256::new()
            .chain(domain.to_be_bytes())
            .chain("NOMAD".as_bytes())
            .finalize(),
    );

    Ok(HomeDomainHashResponse { home_domain_hash })
}

fn query_local_domain(deps: Deps) -> StdResult<LocalDomainResponse> {
    let state = STATE.load(deps.storage)?;
    Ok(LocalDomainResponse {
        local_domain: state.local_domain,
    })
}

fn query_state(deps: Deps) -> StdResult<StateResponse> {
    let state = STATE.load(deps.storage)?;
    Ok(StateResponse {
        state: state.state as u8,
    })
}

fn query_updater(deps: Deps) -> StdResult<UpdaterResponse> {
    let state = STATE.load(deps.storage)?;
    Ok(UpdaterResponse {
        updater: state.updater.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies_with_balance, mock_env, mock_info};
    use cosmwasm_std::{coins, from_binary};

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies_with_balance(&coins(2, "token"));

        let msg = InstantiateMsg {};
        let info = mock_info("creator", &coins(100, "earth"));

        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        let res = query(deps.as_ref(), mock_env(), QueryMsg::Owner {}).unwrap();
        let value: OwnerResponse = from_binary(&res).unwrap();
        assert_eq!("creator", value.owner);
    }

    #[test]
    fn renounce_ownership() {
        let mut deps = mock_dependencies_with_balance(&coins(100, "token"));

        let msg = InstantiateMsg {};
        let info = mock_info("creator", &coins(100, "token"));
        let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        let msg = ExecuteMsg::RenounceOwnership {};
        let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        let res = query(deps.as_ref(), mock_env(), QueryMsg::Owner {}).unwrap();
        let value: OwnerResponse = from_binary(&res).unwrap();
        assert_eq!("0x0", value.owner);
    }

    #[test]
    fn transfer_ownership() {
        let mut deps = mock_dependencies_with_balance(&coins(100, "token"));

        let msg = InstantiateMsg {};
        let info = mock_info("creator", &coins(100, "token"));
        let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        let msg = ExecuteMsg::TransferOwnership {
            new_owner: "new_owner".to_owned(),
        };
        let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        let res = query(deps.as_ref(), mock_env(), QueryMsg::Owner {}).unwrap();
        let value: OwnerResponse = from_binary(&res).unwrap();
        assert_eq!("new_owner", value.owner);
    }

    #[test]
    fn access_control() {
        let mut deps = mock_dependencies_with_balance(&coins(100, "token"));

        let msg = InstantiateMsg {};
        let info = mock_info("creator", &coins(100, "token"));
        let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        let no_auth_info = mock_info("not_auth", &coins(100, "token"));
        let renounce_msg = ExecuteMsg::RenounceOwnership {};
        let is_error = execute(
            deps.as_mut(),
            mock_env(),
            no_auth_info.clone(),
            renounce_msg,
        )
        .is_err();
        assert!(is_error);

        let transfer_msg = ExecuteMsg::TransferOwnership {
            new_owner: "new_owner".to_owned(),
        };
        let is_error = execute(deps.as_mut(), mock_env(), no_auth_info, transfer_msg).is_err();
        assert!(is_error);

        let res = query(deps.as_ref(), mock_env(), QueryMsg::Owner {}).unwrap();
        let value: OwnerResponse = from_binary(&res).unwrap();
        assert_eq!("creator", value.owner);
    }
}
