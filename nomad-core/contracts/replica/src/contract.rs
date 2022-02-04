#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_binary, to_binary, Binary, ContractResult, CosmosMsg, Deps, DepsMut, Env, Event,
    MessageInfo, Reply, ReplyOn, Response, StdResult, SubMsg, WasmMsg,
};
use cw2::set_contract_version;
use ethers_core::types::H256;
use lib::{bytes32_to_addr, Decode, HandleMsg, MessageStatus, NomadMessage};

use crate::error::ContractError;
use crate::state::{CONFIRM_AT, MESSAGES, OPTIMISTIC_SECONDS, REMOTE_DOMAIN};
use msg::replica::{
    AcceptableRootResponse, ConfirmAtResponse, ExecuteMsg, InstantiateMsg, MessageStatusResponse,
    OptimisticSecondsResponse, QueryMsg, RemoteDomainResponse,
};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:replica";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const PROCESS_ID: u64 = 1;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    nomad_base::instantiate(deps.branch(), env, info, msg.clone().into())?;

    REMOTE_DOMAIN.save(deps.storage, &msg.remote_domain)?;
    OPTIMISTIC_SECONDS.save(deps.storage, &msg.optimistic_seconds)?;
    nomad_base::_set_committed_root(deps.branch(), msg.committed_root)?;

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Update {
            committed_root,
            new_root,
            signature,
        } => try_update(deps, env, committed_root, new_root, signature),
        ExecuteMsg::DoubleUpdate {
            old_root,
            new_roots,
            signature,
            signature_2,
        } => Ok(nomad_base::try_double_update(
            deps,
            info,
            old_root,
            new_roots,
            signature,
            signature_2,
            _fail,
        )?),
        ExecuteMsg::Prove { leaf, proof, index } => try_prove(deps, env, leaf, proof, index),
        ExecuteMsg::Process { message } => try_process(deps, info, message),
        ExecuteMsg::ProveAndProcess {
            message,
            proof,
            index,
        } => try_prove_and_process(deps, env, info, message, proof, index),
        ExecuteMsg::SetConfirmation { root, confirm_at } => {
            try_set_confirmation(deps, info, root, confirm_at)
        }
        ExecuteMsg::SetOptimisticTimeout { optimistic_seconds } => {
            try_set_optimistic_timeout(deps, info, optimistic_seconds)
        }
        ExecuteMsg::SetUpdater { updater } => try_set_updater(deps, info, updater),
        ExecuteMsg::RenounceOwnership {} => Ok(ownable::try_renounce_ownership(deps, info)?),
        ExecuteMsg::TransferOwnership { new_owner } => {
            Ok(ownable::try_transfer_ownership(deps, info, new_owner)?)
        }
    }
}

pub fn try_update(
    mut deps: DepsMut,
    env: Env,
    old_root: H256,
    new_root: H256,
    signature: Vec<u8>,
) -> Result<Response, ContractError> {
    nomad_base::not_failed(deps.as_ref())?;

    let committed_root = nomad_base::query_committed_root(deps.as_ref())?.committed_root;
    if old_root != committed_root {
        return Err(ContractError::NotCurrentCommittedRoot { old_root });
    }

    if !nomad_base::is_updater_signature(deps.as_ref(), old_root, new_root, &signature)? {
        return Err(ContractError::NotUpdaterSignature);
    }

    // TODO: _beforeUpdate hook?

    let optimistic_seconds = query_optimistic_seconds(deps.as_ref())?.optimistic_seconds;
    let confirm_at = env.block.time.seconds() + optimistic_seconds;
    CONFIRM_AT.save(deps.storage, new_root.as_bytes(), &confirm_at)?;

    nomad_base::_set_committed_root(deps.branch(), new_root)?;

    let remote_domain = query_remote_domain(deps.as_ref())?.remote_domain;

    Ok(Response::new().add_event(
        Event::new("Update")
            .add_attribute("local_domain", remote_domain.to_string())
            .add_attribute("committed_root", format!("{:?}", committed_root))
            .add_attribute("new_root", format!("{:?}", new_root))
            .add_attribute("signature", format!("{:?}", signature)),
    ))
}

pub fn try_prove(
    deps: DepsMut,
    env: Env,
    leaf: H256,
    proof: [H256; 32],
    index: u64,
) -> Result<Response, ContractError> {
    let message_status = query_message_status(deps.as_ref(), leaf)?.status;
    if message_status != MessageStatus::None {
        return Err(ContractError::MessageAlreadyProven { leaf });
    }

    let calculated_root = merkle::merkle_tree::merkle_root_from_branch(
        leaf,
        &proof[..],
        merkle::merkle_tree::TREE_DEPTH,
        index as usize,
    );

    let acceptable_root = query_acceptable_root(deps.as_ref(), env, calculated_root)?.acceptable;
    if acceptable_root {
        MESSAGES.save(deps.storage, leaf.as_bytes(), &MessageStatus::Proven)?;
        return Ok(Response::new().set_data(to_binary(&true)?));
    }

    Ok(Response::new().set_data(to_binary(&false)?))
}

pub fn try_process(
    deps: DepsMut,
    info: MessageInfo,
    message: Vec<u8>,
) -> Result<Response, ContractError> {
    let nomad_message =
        NomadMessage::read_from(&mut message.as_slice()).expect("!message conversion");

    let local_domain = nomad_base::query_local_domain(deps.as_ref())?.local_domain;
    if nomad_message.destination != local_domain {
        return Err(ContractError::WrongDestination {
            destination: nomad_message.destination,
        });
    }

    let leaf = nomad_message.to_leaf();
    let message_status = query_message_status(deps.as_ref(), leaf)?.status;
    if message_status != MessageStatus::Proven {
        return Err(ContractError::MessageNotYetProven { leaf });
    }

    MESSAGES.save(deps.storage, leaf.as_bytes(), &MessageStatus::Processed)?;

    // TODO: check gas limit to ensure rest of tx doesn't fail for gas

    let handle_msg: HandleMsg = nomad_message.clone().into();
    let wasm_msg = WasmMsg::Execute {
        contract_addr: bytes32_to_addr(deps.as_ref(), nomad_message.recipient).to_string(),
        msg: to_binary(&handle_msg)?,
        funds: info.funds,
    };
    let cosmos_msg = CosmosMsg::Wasm(wasm_msg);

    let sub_msg = SubMsg {
        id: PROCESS_ID,
        msg: cosmos_msg,
        gas_limit: None,
        reply_on: ReplyOn::Always,
    };

    Ok(Response::new().add_submessage(sub_msg))
}

pub fn try_prove_and_process(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    message: Vec<u8>,
    proof: [H256; 32],
    index: u64,
) -> Result<Response, ContractError> {
    let leaf = NomadMessage::read_from(&mut message.as_slice())
        .expect("!message conversion")
        .to_leaf();
    let ret = try_prove(deps.branch(), env, leaf, proof, index)?.data;
    let prove_success: bool = from_binary(&ret.unwrap())?;

    if !prove_success {
        return Err(ContractError::FailedProveCall { leaf, index });
    }

    try_process(deps.branch(), info, message)
}

pub fn try_set_confirmation(
    deps: DepsMut,
    info: MessageInfo,
    root: H256,
    confirm_at: u64,
) -> Result<Response, ContractError> {
    ownable::only_owner(deps.as_ref(), info)?;

    let prev_confirm_at = CONFIRM_AT
        .may_load(deps.storage, root.as_bytes())?
        .unwrap_or_default();
    CONFIRM_AT.save(deps.storage, root.as_bytes(), &confirm_at)?;

    Ok(Response::new().add_event(
        Event::new("SetConfirmation")
            .add_attribute("root", format!("{:?}", root))
            .add_attribute("previous_confirm_at", prev_confirm_at.to_string())
            .add_attribute("new_confirm_at", confirm_at.to_string()),
    ))
}

pub fn try_set_optimistic_timeout(
    deps: DepsMut,
    info: MessageInfo,
    optimistic_seconds: u64,
) -> Result<Response, ContractError> {
    ownable::only_owner(deps.as_ref(), info)?;
    OPTIMISTIC_SECONDS.save(deps.storage, &optimistic_seconds)?;
    Ok(Response::new().add_event(
        Event::new("SetOptimisticTimeout")
            .add_attribute("optimistic_seconds", optimistic_seconds.to_string()),
    ))
}

pub fn try_set_updater(
    deps: DepsMut,
    info: MessageInfo,
    updater: String,
) -> Result<Response, ContractError> {
    ownable::only_owner(deps.as_ref(), info)?;
    Ok(nomad_base::_set_updater(deps, updater)?)
}

fn _fail(mut deps: DepsMut, _info: MessageInfo) -> Result<Response, nomad_base::ContractError> {
    nomad_base::_set_failed(deps.branch())?;
    Ok(Response::new())
}

#[entry_point]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        PROCESS_ID => reply_process(deps.as_ref(), env, msg),
        _ => Err(ContractError::UnknownReplyMessage { id: msg.id }),
    }
}

pub fn reply_process(_deps: Deps, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.result {
        ContractResult::Ok(res) => Ok(Response::new()
            .add_events(res.events)
            .set_data(to_binary(&true)?)),
        ContractResult::Err(e) => Err(ContractError::FailedProcessCall(e)),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::AcceptableRoot { root } => to_binary(&query_acceptable_root(deps, env, root)?),
        QueryMsg::ConfirmAt { root } => to_binary(&query_confirm_at(deps, root)?),
        QueryMsg::MessageStatus { leaf } => to_binary(&query_message_status(deps, leaf)?),
        QueryMsg::OptimisticSeconds {} => {
            let opt = query_optimistic_seconds(deps)?;
            return to_binary(&opt);
        }
        QueryMsg::RemoteDomain {} => to_binary(&query_remote_domain(deps)?),
        QueryMsg::CommittedRoot {} => to_binary(&nomad_base::query_committed_root(deps)?),
        QueryMsg::HomeDomainHash {} => to_binary(&nomad_base::query_home_domain_hash(deps)?),
        QueryMsg::LocalDomain {} => to_binary(&nomad_base::query_local_domain(deps)?),
        QueryMsg::State {} => to_binary(&nomad_base::query_state(deps)?),
        QueryMsg::Updater {} => to_binary(&nomad_base::query_updater(deps)?),
        QueryMsg::Owner {} => to_binary(&ownable::query_owner(deps)?),
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
    let confirm_at = CONFIRM_AT
        .may_load(deps.storage, root.as_bytes())?
        .unwrap_or_default();

    Ok(ConfirmAtResponse { confirm_at })
}

pub fn query_message_status(deps: Deps, leaf: H256) -> StdResult<MessageStatusResponse> {
    let status = MESSAGES
        .may_load(deps.storage, leaf.as_bytes())?
        .unwrap_or_default();
    Ok(MessageStatusResponse { status })
}

pub fn query_optimistic_seconds(deps: Deps) -> StdResult<OptimisticSecondsResponse> {
    let optimistic_seconds = OPTIMISTIC_SECONDS.load(deps.storage)?;
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
    use lib::States;
    use msg::nomad_base::{LocalDomainResponse, StateResponse, UpdaterResponse};

    const LOCAL_DOMAIN: u32 = 2000;
    const REMOTE_DOMAIN: u32 = 1000;
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
        let value: OptimisticSecondsResponse = from_binary(&res).unwrap();
        assert_eq!(optimistic_seconds, value.optimistic_seconds);

        // ------ NOMAD_BASE ------
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
}
