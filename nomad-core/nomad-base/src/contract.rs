#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Env, Event, MessageInfo, Response, StdResult,
};
use cw2::set_contract_version;
use ethers_core::types::{RecoveryMessage, Signature, H160, H256};
use sha3::{digest::Update, Digest, Keccak256};
use std::{convert::TryFrom, str::FromStr};

use crate::error::ContractError;
use crate::msg::{
    CommittedRootResponse, ExecuteMsg, HomeDomainHashResponse, InstantiateMsg, LocalDomainResponse,
    QueryMsg, StateResponse, UpdaterResponse,
};
use crate::state::{States, LOCAL_DOMAIN, UPDATER, STATE, COMMITTED_ROOT};
use ownable::contract::{
    instantiate as ownable_instantiate, query_owner, try_renounce_ownership, try_transfer_ownership,
};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:nomad-base";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    ownable_instantiate(deps.branch(), env, info, msg.clone().into())?;

    let updater = deps.api.addr_validate(&msg.updater)?;

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    LOCAL_DOMAIN.save(deps.storage, &msg.local_domain)?;
    UPDATER.save(deps.storage, &updater)?;
    STATE.save(deps.storage, &States::Active)?;
    COMMITTED_ROOT.save(deps.storage, &H256::zero())?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("local_domain", msg.local_domain.to_string())
        .add_attribute("updater", msg.updater))
}

pub fn not_failed(deps: Deps) -> Result<Response, ContractError> {
    let state = STATE.load(deps.storage)?;
    if state == States::Failed {
        return Err(ContractError::NotFailedError {});
    }

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
        ExecuteMsg::RenounceOwnership {} => Ok(try_renounce_ownership(deps, info)?),
        ExecuteMsg::TransferOwnership { new_owner } => {
            Ok(try_transfer_ownership(deps, info, new_owner)?)
        }
    }
}

pub fn try_double_update(
    deps: DepsMut,
    info: MessageInfo,
    old_root: H256,
    new_roots: [H256; 2],
    signature: Vec<u8>,
    signature_2: Vec<u8>,
    fail: fn(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError>,
) -> Result<Response, ContractError> {
    not_failed(deps.as_ref())?;

    if is_updater_signature(deps.as_ref(), old_root, new_roots[0], &signature)?
        && is_updater_signature(deps.as_ref(), old_root, new_roots[1], &signature_2)?
        && new_roots[0] != new_roots[1]
    {
        fail(deps, info)?;
        return Ok(Response::new().add_event(
            Event::new("DoubleUpdate")
                .add_attribute("old_root", format!("{:?}", old_root))
                .add_attribute("new_roots", format!("{:?}", new_roots))
                .add_attribute("signature", format!("{:?}", signature))
                .add_attribute("signature_2", format!("{:?}", signature_2)),
        ));
    }

    Err(ContractError::InvalidDoubleUpdate {})
}

pub fn is_updater_signature(
    deps: Deps,
    old_root: H256,
    new_root: H256,
    signature: &[u8],
) -> Result<bool, ContractError> {
    let home_domain_hash = query_home_domain_hash(deps)?.home_domain_hash;
    let updater = query_updater(deps)?.updater;

    let digest = H256::from_slice(
        Keccak256::new()
            .chain(H256::from(home_domain_hash))
            .chain(old_root)
            .chain(new_root)
            .finalize()
            .as_slice(),
    );

    let sig = Signature::try_from(signature)?;
    let recovered_address = sig.recover(RecoveryMessage::Data(digest.as_bytes().to_vec()))?;
    Ok(H160::from_str(&updater).unwrap() == recovered_address)
}

pub fn _set_failed(deps: DepsMut) -> Result<Response, ContractError> {
    STATE.save(deps.storage, &States::Failed)?;
    Ok(Response::new())
}

pub fn _set_updater(deps: DepsMut, updater: String) -> Result<Response, ContractError> {
    let updater_addr = deps.api.addr_validate(&updater)?;
    UPDATER.save(deps.storage, &updater_addr)?;

    Ok(Response::new())
}

pub fn _set_committed_root(deps: DepsMut, root: H256) -> Result<Response, ContractError> {
    COMMITTED_ROOT.save(deps.storage, &root)?;

    Ok(Response::new())
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

pub fn query_committed_root(deps: Deps) -> StdResult<CommittedRootResponse> {
    let committed_root = COMMITTED_ROOT.load(deps.storage)?;
    Ok(CommittedRootResponse {
        committed_root,
    })
}

pub fn query_home_domain_hash(deps: Deps) -> StdResult<HomeDomainHashResponse> {
    let domain = LOCAL_DOMAIN.load(deps.storage)?;
    let home_domain_hash = H256::from_slice(
        Keccak256::new()
            .chain(domain.to_be_bytes())
            .chain("NOMAD".as_bytes())
            .finalize()
            .as_slice(),
    );

    Ok(HomeDomainHashResponse { home_domain_hash })
}

pub fn query_local_domain(deps: Deps) -> StdResult<LocalDomainResponse> {
    let local_domain = LOCAL_DOMAIN.load(deps.storage)?;
    Ok(LocalDomainResponse { local_domain })
}

pub fn query_state(deps: Deps) -> StdResult<StateResponse> {
    let state = STATE.load(deps.storage)?;
    Ok(StateResponse { state })
}

pub fn query_updater(deps: Deps) -> StdResult<UpdaterResponse> {
    let updater = UPDATER.load(deps.storage)?.to_string();
    Ok(UpdaterResponse { updater })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies_with_balance, mock_env, mock_info};
    use cosmwasm_std::{coins, from_binary};
    use ownable::msg::OwnerResponse;
    use test_utils::Updater;

    const LOCAL_DOMAIN: u32 = 1000;
    const UPDATER_PRIVKEY: &str =
        "1111111111111111111111111111111111111111111111111111111111111111";
    const UPDATER_PUBKEY: &str = "0x19e7e376e7c213b7e7e7e46cc70a5dd086daff2a";

    fn mock_fail_fn(_deps: DepsMut, _info: MessageInfo) -> Result<Response, ContractError> {
        Ok(Response::new())
    }

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies_with_balance(&coins(100, "token"));

        let init_msg = InstantiateMsg {
            local_domain: LOCAL_DOMAIN,
            updater: UPDATER_PUBKEY.to_owned(),
        };
        let info = mock_info("owner", &coins(100, "earth"));

        let res = instantiate(deps.as_mut(), mock_env(), info, init_msg).unwrap();
        assert_eq!(0, res.messages.len());

        // Owner
        let owner_res = query(deps.as_ref(), mock_env(), QueryMsg::Owner {}).unwrap();
        let value: OwnerResponse = from_binary(&owner_res).unwrap();
        assert_eq!("owner", value.owner);

        // State
        let res = query(deps.as_ref(), mock_env(), QueryMsg::State {}).unwrap();
        let value: StateResponse = from_binary(&res).unwrap();
        assert_eq!(States::Active, value.state);

        // Local domain
        let res = query(deps.as_ref(), mock_env(), QueryMsg::LocalDomain {}).unwrap();
        let value: LocalDomainResponse = from_binary(&res).unwrap();
        assert_eq!(LOCAL_DOMAIN, value.local_domain);

        // Updater
        let res = query(deps.as_ref(), mock_env(), QueryMsg::Updater {}).unwrap();
        let value: UpdaterResponse = from_binary(&res).unwrap();
        assert_eq!(UPDATER_PUBKEY.to_owned(), value.updater);
    }

    #[tokio::test]
    async fn accepts_updater_signature() {
        let updater: Updater = Updater::from_privkey(UPDATER_PRIVKEY, LOCAL_DOMAIN);

        let mut deps = mock_dependencies_with_balance(&coins(100, "token"));

        let init_msg = InstantiateMsg {
            local_domain: LOCAL_DOMAIN,
            updater: UPDATER_PUBKEY.to_owned(),
        };
        let info = mock_info("owner", &coins(100, "earth"));

        let res = instantiate(deps.as_mut(), mock_env(), info, init_msg).unwrap();
        assert_eq!(0, res.messages.len());

        let old_root = H256::zero();
        let new_root = H256::repeat_byte(1);
        let update = updater.sign_update(old_root, new_root).await.unwrap();

        let is_updater_sig = is_updater_signature(
            deps.as_ref(),
            old_root,
            new_root,
            &update.signature.to_vec(),
        )
        .unwrap();
        assert!(is_updater_sig)
    }

    #[tokio::test]
    async fn rejects_invalid_updater_signature() {
        let not_updater_privkey =
            "2111111111111111111111111111111111111111111111111111111111111111";
        let not_updater: Updater = Updater::from_privkey(not_updater_privkey, LOCAL_DOMAIN);

        let mut deps = mock_dependencies_with_balance(&coins(100, "token"));

        let init_msg = InstantiateMsg {
            local_domain: LOCAL_DOMAIN,
            updater: UPDATER_PUBKEY.to_owned(),
        };
        let info = mock_info("owner", &coins(100, "earth"));

        let res = instantiate(deps.as_mut(), mock_env(), info, init_msg).unwrap();
        assert_eq!(0, res.messages.len());

        let old_root = H256::zero();
        let new_root = H256::repeat_byte(1);
        let update = not_updater.sign_update(old_root, new_root).await.unwrap();

        let is_updater_sig = is_updater_signature(
            deps.as_ref(),
            old_root,
            new_root,
            &update.signature.to_vec(),
        )
        .unwrap();
        assert!(!is_updater_sig)
    }

    #[tokio::test]
    async fn emits_failure_on_valid_double_update() {
        let updater: Updater = Updater::from_privkey(UPDATER_PRIVKEY, LOCAL_DOMAIN);

        let mut deps = mock_dependencies_with_balance(&coins(100, "token"));

        let init_msg = InstantiateMsg {
            local_domain: LOCAL_DOMAIN,
            updater: UPDATER_PUBKEY.to_owned(),
        };
        let info = mock_info("owner", &coins(100, "earth"));

        let res = instantiate(deps.as_mut(), mock_env(), info.clone(), init_msg).unwrap();
        assert_eq!(0, res.messages.len());

        let old_root = H256::zero();
        let new_root = H256::repeat_byte(1);
        let bad_new_root = H256::repeat_byte(2);
        let update = updater.sign_update(old_root, new_root).await.unwrap();
        let double_update = updater.sign_update(old_root, bad_new_root).await.unwrap();

        let double_update_res = try_double_update(
            deps.as_mut(),
            info.clone(),
            old_root,
            [new_root, bad_new_root],
            update.signature.to_vec(),
            double_update.signature.to_vec(),
            mock_fail_fn,
        );

        assert_eq!("DoubleUpdate", double_update_res.unwrap().events[0].ty);
    }

    #[tokio::test]
    async fn rejects_invalid_double_update() {
        let updater: Updater = Updater::from_privkey(UPDATER_PRIVKEY, LOCAL_DOMAIN);

        let mut deps = mock_dependencies_with_balance(&coins(100, "token"));

        let init_msg = InstantiateMsg {
            local_domain: LOCAL_DOMAIN,
            updater: UPDATER_PUBKEY.to_owned(),
        };
        let info = mock_info("owner", &coins(100, "earth"));

        let res = instantiate(deps.as_mut(), mock_env(), info.clone(), init_msg).unwrap();
        assert_eq!(0, res.messages.len());

        let old_root = H256::zero();
        let new_root = H256::repeat_byte(1);
        let update = updater.sign_update(old_root, new_root).await.unwrap();

        let double_update_res = try_double_update(
            deps.as_mut(),
            info.clone(),
            old_root,
            [new_root, new_root],
            update.signature.to_vec(),
            update.signature.to_vec(),
            mock_fail_fn,
        );

        assert!(double_update_res.is_err());
    }
}
