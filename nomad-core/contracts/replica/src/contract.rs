use common::{h256_to_n_byte_addr, Decode, HandleExecuteMsg, MessageStatus, NomadMessage};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_binary, to_binary, Binary, ContractResult, CosmosMsg, Deps, DepsMut, Env, Event,
    MessageInfo, Reply, ReplyOn, Response, StdResult, SubMsg, WasmMsg,
};
use cw2::set_contract_version;
use ethers_core::types::{H160, H256};

use crate::error::ContractError;
use crate::state::{
    CHAIN_ADDR_LENGTH_BYTES, CONFIRM_AT, MESSAGES, OPTIMISTIC_SECONDS, REMOTE_DOMAIN,
};
use common::replica::{
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

    CHAIN_ADDR_LENGTH_BYTES.save(deps.storage, &msg.chain_addr_length_bytes)?;
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
        _set_message_proven(deps, leaf)?;
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
    if message_status != MessageStatus::Pending {
        return Err(ContractError::MessageNotYetProven { leaf });
    }

    MESSAGES.save(deps.storage, leaf.as_bytes(), &MessageStatus::Processed)?;

    // TODO: check gas limit to ensure rest of tx doesn't fail for gas
    let addr_length = CHAIN_ADDR_LENGTH_BYTES.load(deps.storage)?;

    let handle_msg: HandleExecuteMsg = nomad_message.clone().into();
    let wasm_msg = WasmMsg::Execute {
        contract_addr: h256_to_n_byte_addr(deps.as_ref(), addr_length, nomad_message.recipient)
            .to_string(),
        msg: to_binary(&handle_msg)?,
        funds: info.funds,
    };
    let cosmos_msg = CosmosMsg::Wasm(wasm_msg);

    println!(
        "wasm execute contract addr: {:?}",
        h256_to_n_byte_addr(deps.as_ref(), addr_length, nomad_message.recipient).to_string()
    );

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
    updater: H160,
) -> Result<Response, ContractError> {
    ownable::only_owner(deps.as_ref(), info)?;
    Ok(nomad_base::_set_updater(deps, updater)?)
}

pub fn _set_message_proven(deps: DepsMut, leaf: H256) -> Result<Response, ContractError> {
    MESSAGES.save(deps.storage, leaf.as_bytes(), &MessageStatus::Pending)?;
    Ok(Response::new())
}

pub fn _fail(mut deps: DepsMut, _info: MessageInfo) -> Result<Response, nomad_base::ContractError> {
    Ok(nomad_base::_set_failed(deps.branch())?)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        PROCESS_ID => reply_process(deps.as_ref(), env, msg),
        _ => Err(ContractError::UnknownReplyMessage { id: msg.id }),
    }
}

pub fn reply_process(_deps: Deps, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.result {
        ContractResult::Ok(_) => Ok(Response::new().set_data(to_binary(&true)?)),
        ContractResult::Err(_) => Ok(Response::new().set_data(to_binary(&false)?)),
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
    use common::nomad_base::{
        CommittedRootResponse, LocalDomainResponse, StateResponse, UpdaterResponse,
    };
    use common::States;
    use cosmwasm_std::testing::{mock_dependencies_with_balance, mock_env, mock_info};
    use cosmwasm_std::{coins, from_binary};
    use test_utils::{event_attr_value_by_key, Updater};

    const CHAIN_ADDR_LENGTH_BYTES: usize = 42;
    const LOCAL_DOMAIN: u32 = 2000;
    const REMOTE_DOMAIN: u32 = 1000;
    const UPDATER_PRIVKEY: &str =
        "1111111111111111111111111111111111111111111111111111111111111111";

    #[test]
    fn proper_initialization() {
        let updater: Updater = Updater::from_privkey(UPDATER_PRIVKEY, LOCAL_DOMAIN);

        let mut deps = mock_dependencies_with_balance(&coins(2, "token"));

        let optimistic_seconds = 100u64;

        let msg = InstantiateMsg {
            chain_addr_length_bytes: CHAIN_ADDR_LENGTH_BYTES,
            local_domain: LOCAL_DOMAIN,
            remote_domain: REMOTE_DOMAIN,
            updater: updater.address(),
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
        assert_eq!(updater.address(), value.updater);
    }

    #[tokio::test]
    async fn halts_on_failed_state() {
        let updater: Updater = Updater::from_privkey(UPDATER_PRIVKEY, LOCAL_DOMAIN);

        let mut deps = mock_dependencies_with_balance(&coins(2, "token"));

        let optimistic_seconds = 100u64;

        let msg = InstantiateMsg {
            chain_addr_length_bytes: CHAIN_ADDR_LENGTH_BYTES,
            local_domain: LOCAL_DOMAIN,
            remote_domain: REMOTE_DOMAIN,
            updater: updater.address(),
            committed_root: H256::zero(),
            optimistic_seconds,
        };
        let info = mock_info("owner", &coins(100, "earth"));

        let res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
        assert_eq!(0, res.messages.len());

        // Set failed
        _fail(deps.as_mut(), info).unwrap();

        // Try to submit update to replica
        let committed_root = H256::zero();
        let new_root = H256::repeat_byte(1);
        let update = updater.sign_update(committed_root, new_root).await.unwrap();

        let info = mock_info("submitter", &coins(100, "earth"));
        let msg = ExecuteMsg::Update {
            committed_root,
            new_root,
            signature: update.signature.to_vec(),
        };
        let res = execute(deps.as_mut(), mock_env(), info, msg);
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn adds_pending_updates() {
        let updater: Updater = Updater::from_privkey(UPDATER_PRIVKEY, LOCAL_DOMAIN);

        let mut deps = mock_dependencies_with_balance(&coins(2, "token"));

        let optimistic_seconds = 100u64;

        let msg = InstantiateMsg {
            chain_addr_length_bytes: CHAIN_ADDR_LENGTH_BYTES,
            local_domain: LOCAL_DOMAIN,
            remote_domain: REMOTE_DOMAIN,
            updater: updater.address(),
            committed_root: H256::zero(),
            optimistic_seconds,
        };
        let info = mock_info("owner", &coins(100, "earth"));

        let res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
        assert_eq!(0, res.messages.len());

        // Create two updates
        let committed_root = H256::zero();
        let first_new_root = H256::repeat_byte(1);
        let second_new_root = H256::repeat_byte(2);
        let first_update = updater
            .sign_update(committed_root, first_new_root)
            .await
            .unwrap();
        let second_update = updater
            .sign_update(first_new_root, second_new_root)
            .await
            .unwrap();

        let info = mock_info("submitter", &coins(100, "earth"));

        // Submit first update
        let msg = ExecuteMsg::Update {
            committed_root,
            new_root: first_new_root,
            signature: first_update.signature.to_vec(),
        };
        execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        // Check committed root now first new root
        let res = query(deps.as_ref(), mock_env(), QueryMsg::CommittedRoot {}).unwrap();
        let value: CommittedRootResponse = from_binary(&res).unwrap();
        assert_eq!(first_new_root, value.committed_root);

        // Submit second update
        let msg = ExecuteMsg::Update {
            committed_root: first_new_root,
            new_root: second_new_root,
            signature: second_update.signature.to_vec(),
        };
        execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        // Check committed root now second new root
        let res = query(deps.as_ref(), mock_env(), QueryMsg::CommittedRoot {}).unwrap();
        let value: CommittedRootResponse = from_binary(&res).unwrap();
        assert_eq!(second_new_root, value.committed_root);
    }

    #[tokio::test]
    async fn rejects_invalid_updater_signature() {
        let updater: Updater = Updater::from_privkey(UPDATER_PRIVKEY, LOCAL_DOMAIN);

        let not_updater: Updater = Updater::from_privkey(
            "2111111111111111111111111111111111111111111111111111111111111111",
            LOCAL_DOMAIN,
        );

        let mut deps = mock_dependencies_with_balance(&coins(2, "token"));

        let optimistic_seconds = 100u64;

        let msg = InstantiateMsg {
            chain_addr_length_bytes: CHAIN_ADDR_LENGTH_BYTES,
            local_domain: LOCAL_DOMAIN,
            remote_domain: REMOTE_DOMAIN,
            updater: updater.address(),
            committed_root: H256::zero(),
            optimistic_seconds,
        };
        let info = mock_info("owner", &coins(100, "earth"));

        let res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
        assert_eq!(0, res.messages.len());

        // Sign update with wrong privkey
        let committed_root = H256::zero();
        let new_root = H256::repeat_byte(1);
        let invalid_update = not_updater
            .sign_update(committed_root, new_root)
            .await
            .unwrap();

        let info = mock_info("submitter", &coins(100, "earth"));

        // Submit invalid update
        let msg = ExecuteMsg::Update {
            committed_root,
            new_root,
            signature: invalid_update.signature.to_vec(),
        };
        let res = execute(deps.as_mut(), mock_env(), info.clone(), msg);
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn rejects_update_not_building_off_latest_submitted() {
        let updater: Updater = Updater::from_privkey(UPDATER_PRIVKEY, LOCAL_DOMAIN);

        let mut deps = mock_dependencies_with_balance(&coins(2, "token"));

        let optimistic_seconds = 100u64;

        let msg = InstantiateMsg {
            chain_addr_length_bytes: CHAIN_ADDR_LENGTH_BYTES,
            local_domain: LOCAL_DOMAIN,
            remote_domain: REMOTE_DOMAIN,
            updater: updater.address(),
            committed_root: H256::zero(),
            optimistic_seconds,
        };
        let info = mock_info("owner", &coins(100, "earth"));

        let res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
        assert_eq!(0, res.messages.len());

        // Create two updates (first valid, second invalid)
        let committed_root = H256::zero();
        let new_root = H256::repeat_byte(1);
        let invalid_old_root = H256::repeat_byte(2);
        let invalid_new_root = H256::repeat_byte(3);
        let first_update = updater.sign_update(committed_root, new_root).await.unwrap();
        let second_update = updater
            .sign_update(invalid_old_root, invalid_new_root)
            .await
            .unwrap();

        let info = mock_info("submitter", &coins(100, "earth"));

        // Submit first update
        let msg = ExecuteMsg::Update {
            committed_root,
            new_root,
            signature: first_update.signature.to_vec(),
        };
        execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        // Check committed root now first new root
        let res = query(deps.as_ref(), mock_env(), QueryMsg::CommittedRoot {}).unwrap();
        let value: CommittedRootResponse = from_binary(&res).unwrap();
        assert_eq!(new_root, value.committed_root);

        // Expecting submitting invalid update to fail
        let msg = ExecuteMsg::Update {
            committed_root: invalid_old_root,
            new_root: invalid_new_root,
            signature: second_update.signature.to_vec(),
        };
        let res = execute(deps.as_mut(), mock_env(), info.clone(), msg);
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn accepts_valid_double_update() {
        let updater: Updater = Updater::from_privkey(UPDATER_PRIVKEY, LOCAL_DOMAIN);

        let mut deps = mock_dependencies_with_balance(&coins(2, "token"));

        let optimistic_seconds = 100u64;

        let msg = InstantiateMsg {
            chain_addr_length_bytes: CHAIN_ADDR_LENGTH_BYTES,
            local_domain: LOCAL_DOMAIN,
            remote_domain: REMOTE_DOMAIN,
            updater: updater.address(),
            committed_root: H256::zero(),
            optimistic_seconds,
        };
        let info = mock_info("owner", &coins(100, "earth"));

        let res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
        assert_eq!(0, res.messages.len());

        // Sign double update
        let committed_root = H256::zero();
        let new_root = H256::repeat_byte(1);
        let bad_root = H256::repeat_byte(2);

        let update = updater.sign_update(committed_root, new_root).await.unwrap();
        let bad_update = updater.sign_update(committed_root, bad_root).await.unwrap();

        // Submit double update
        let info = mock_info("submitter", &coins(100, "earth"));
        let msg = ExecuteMsg::DoubleUpdate {
            old_root: committed_root,
            new_roots: [new_root, bad_root],
            signature: update.signature.to_vec(),
            signature_2: bad_update.signature.to_vec(),
        };
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // Check emitted event
        let event = &res.events[0];
        assert_eq!("DoubleUpdate".to_owned(), event.ty);
        assert_eq!(
            format!("{:?}", committed_root),
            event_attr_value_by_key(&event, "old_root").unwrap()
        );
        assert_eq!(
            format!("{:?}", [new_root, bad_root]),
            event_attr_value_by_key(&event, "new_roots").unwrap()
        );
        assert_eq!(
            format!("{:?}", update.signature.to_vec()),
            event_attr_value_by_key(&event, "signature").unwrap()
        );
        assert_eq!(
            format!("{:?}", bad_update.signature.to_vec()),
            event_attr_value_by_key(&event, "signature_2").unwrap()
        );

        // Check replica failed
        let res = query(deps.as_ref(), mock_env(), QueryMsg::State {}).unwrap();
        let state = from_binary::<StateResponse>(&res).unwrap().state;
        assert_eq!(States::Failed, state);
    }
}
