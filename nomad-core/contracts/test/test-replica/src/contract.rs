use common::{h256_to_addr, Decode, HandleExecuteMsg, MessageStatus, NomadMessage};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_binary, to_binary, Binary, ContractResult, CosmosMsg, Deps, DepsMut, Env, Event,
    MessageInfo, Reply, ReplyOn, Response, StdResult, SubMsg, WasmMsg,
};
use cw2::set_contract_version;
use ethers_core::types::H256;

use crate::error::ContractError;
use common::{
    replica::{InstantiateMsg, QueryMsg},
    test::test_replica::ExecuteMsg,
};
use replica::state::{CONFIRM_AT, MESSAGES, OPTIMISTIC_SECONDS, REMOTE_DOMAIN};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:test-replica";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const PROCESS_ID: u64 = 1;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg, // This is of type replica::InstantiateMsg
) -> Result<Response, ContractError> {
    Ok(replica::instantiate(
        deps.branch(),
        env,
        info,
        msg.clone().into(),
    )?)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::ReplicaExecuteMsg(msg) => Ok(replica::execute(deps, env, info, msg)?),
        ExecuteMsg::SetProven { leaf } => _set_proven(deps, leaf),
    }
}

pub fn _set_proven(deps: DepsMut, leaf: H256) -> Result<Response, ContractError> {
    MESSAGES.save(deps.storage, leaf.as_bytes(), &MessageStatus::Proven)?;
    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    Ok(replica::reply(deps, env, msg)?)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    replica::query(deps, env, msg)
}
