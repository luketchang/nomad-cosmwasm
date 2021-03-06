use common::MessageStatus;
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdResult};
use cw2::set_contract_version;
use ethers_core::types::H256;

use crate::error::ContractError;
use common::{
    replica::{InstantiateMsg, QueryMsg},
    test::test_replica::ExecuteMsg,
};
use replica::state::{CONFIRM_AT, MESSAGES};

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
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
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
        ExecuteMsg::SetCommittedRoot { root } => _set_committed_root(deps, root),
    }
}

pub fn _set_proven(deps: DepsMut, leaf: H256) -> Result<Response, ContractError> {
    MESSAGES.save(deps.storage, leaf.as_bytes(), &MessageStatus::Pending)?;
    Ok(Response::new())
}

pub fn _set_committed_root(mut deps: DepsMut, root: H256) -> Result<Response, ContractError> {
    nomad_base::_set_committed_root(deps.branch(), root)?;
    CONFIRM_AT.save(deps.storage, root.as_bytes(), &1)?;
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
