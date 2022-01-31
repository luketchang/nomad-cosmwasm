#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_binary, to_binary, Addr, Binary, Deps, DepsMut, Env, Event, MessageInfo, Response,
    StdResult,
};
use cw2::set_contract_version;
use lib::{addr_to_bytes32, NomadMessage};

use crate::error::ContractError;
use crate::msg::{
    ExecuteMsg, InstantiateMsg, NoncesResponse, QueryMsg, SuggestUpdateResponse,
    UpdaterManagerResponse,
};
use crate::state::{NONCES, UPDATER_MANAGER};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:home";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
const MAX_MESSAGE_BODY_BYTES: u128 = 2 * 2 ^ 10;

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
    match msg {
        ExecuteMsg::Dispatch {
            destination,
            recipient,
            message,
        } => try_dispatch(deps, info, destination, recipient, message),
        ExecuteMsg::Update {
            committed_root,
            new_root,
            signature,
        } => try_update(deps, committed_root, new_root, signature),
        ExecuteMsg::DoubleUpdate {
            old_root,
            new_roots,
            signature,
            signature_2,
        } => Ok(nomad_base::contract::try_double_update(
            deps,
            old_root,
            new_roots,
            signature,
            signature_2,
            fail,
        )?),
        ExecuteMsg::ImproperUpdate {
            old_root,
            new_root,
            signature,
        } => try_improper_update(deps, old_root, new_root, &signature),
        ExecuteMsg::SetUpdater { updater } => {
            Ok(nomad_base::contract::set_updater(deps, info, updater)?)
        }
        ExecuteMsg::SetUpdaterManager { updater_manager } => {
            try_set_updater_manager(deps, info, updater_manager)
        }
        ExecuteMsg::RenounceOwnership {} => {
            Ok(ownable::contract::try_renounce_ownership(deps, info)?)
        }
        ExecuteMsg::TransferOwnership { new_owner } => Ok(
            ownable::contract::try_transfer_ownership(deps, info, new_owner)?,
        ),
    }
}

pub fn try_dispatch(
    mut deps: DepsMut,
    info: MessageInfo,
    destination: u32,
    recipient: String,
    message: Vec<u8>,
) -> Result<Response, ContractError> {
    let length = message.len() as u128;
    if length > MAX_MESSAGE_BODY_BYTES {
        return Err(ContractError::MsgTooLong { length });
    }

    let nonce = query_nonces(deps.as_ref(), destination)?.next_nonce;
    NONCES.save(deps.storage, destination, &(nonce + 1))?;

    let origin = nomad_base::contract::query_local_domain(deps.as_ref())?.local_domain;
    let recipient_addr = deps.api.addr_validate(&recipient)?;

    let message = NomadMessage {
        origin,
        sender: addr_to_bytes32(info.sender),
        nonce,
        destination,
        recipient: addr_to_bytes32(recipient_addr),
        body: message,
    };

    let hash: [u8; 32] = message.to_leaf().into();
    merkle::contract::try_insert(deps.branch(), hash)?;

    let root = merkle::contract::query_root(deps.as_ref())?.root;
    queue::contract::try_enqueue(deps, root)?;

    Ok(Response::new())
}

pub fn try_update(
    mut deps: DepsMut,
    committed_root: [u8; 32],
    new_root: [u8; 32],
    signature: Vec<u8>,
) -> Result<Response, ContractError> {
    if try_improper_update(deps.branch(), committed_root, new_root, &signature).is_ok() {
        return Ok(Response::new()); // kludge?
    }

    loop {
        let next_res = queue::contract::try_dequeue(deps.branch())?;
        let next: [u8; 32] = from_binary(&next_res.data.unwrap())?;
        if next == new_root {
            break;
        }
    }

    nomad_base::contract::set_committed_root(deps.branch(), new_root)?;

    let local_domain = nomad_base::contract::query_local_domain(deps.as_ref())?.local_domain;

    Ok(Response::new().add_event(
        Event::new("Update")
            .add_attribute("local_domain", local_domain.to_string())
            .add_attribute(
                "committed_root",
                std::str::from_utf8(&committed_root).unwrap(),
            )
            .add_attribute("new_root", std::str::from_utf8(&new_root).unwrap())
            .add_attribute("signature", String::from_utf8_lossy(&signature)),
    ))
}

pub fn try_improper_update(
    deps: DepsMut,
    old_root: [u8; 32],
    new_root: [u8; 32],
    signature: &[u8],
) -> Result<Response, ContractError> {
    if !nomad_base::contract::is_updater_signature(deps.as_ref(), old_root, new_root, signature)? {
        return Err(ContractError::NotUpdaterSignature);
    }

    if !queue::contract::query_contains(deps.as_ref(), new_root)?.contains {
        fail(deps)?;
        return Ok(Response::new().add_event(
            Event::new("ImproperUpdate")
                .add_attribute("old_root", std::str::from_utf8(&old_root).unwrap())
                .add_attribute("new_root", std::str::from_utf8(&new_root).unwrap())
                .add_attribute("signature", String::from_utf8_lossy(signature)),
        ));
    }

    Err(ContractError::NotImproperUpdate)
}

pub fn try_set_updater_manager(
    deps: DepsMut,
    info: MessageInfo,
    updater_manager: String,
) -> Result<Response, ContractError> {
    ownable::contract::only_owner(deps.as_ref(), info)?;
    let updater_manager_addr = deps.api.addr_validate(&updater_manager)?;

    UPDATER_MANAGER.save(deps.storage, &updater_manager_addr)?;

    Ok(Response::new().add_event(
        Event::new("SetUpdaterManager").add_attribute("updater_manager", updater_manager),
    ))
}

fn fail(deps: DepsMut) -> Result<Response, nomad_base::ContractError> {
    nomad_base::contract::set_failed(deps)?;

    // TODO: queue submessage to slash updater manager updater
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