#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Attribute, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
};
use cw2::set_contract_version;
use lib::Bytes32;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, ContainsResponse, LastItemResponse, PeekResponse, IsEmptyResponse, LengthResponse};
use crate::state::STATE;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:queue";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Enqueue { item } => try_enqueue(deps, item),
        ExecuteMsg::Dequeue {} => try_dequeue(deps),
        ExecuteMsg::EnqueueBatch { items } => try_enqueue_batch(deps, items),
        ExecuteMsg::DequeueBatch { number } => try_dequeue_batch(deps, number),
    }
}

pub fn try_enqueue(deps: DepsMut, item: Bytes32) -> Result<Response, ContractError> {
    let mut queue = STATE.load(deps.storage)?;
    queue.push_back(item);
    STATE.save(deps.storage, &queue)?;
    Ok(Response::new().add_attribute("action", "enqueue"))
}

pub fn try_dequeue(deps: DepsMut) -> Result<Response, ContractError> {
    let mut queue = STATE.load(deps.storage)?;
    let item = queue.pop_front().ok_or(ContractError::QueueEmpty {})?;
    STATE.save(deps.storage, &queue)?;
    Ok(Response::new()
        .add_attribute("action", "dequeue")
        .add_attribute("item", std::str::from_utf8(&item).unwrap()))
}

pub fn try_enqueue_batch(deps: DepsMut, items: Vec<Bytes32>) -> Result<Response, ContractError> {
    let mut queue = STATE.load(deps.storage)?;
    queue.extend(items.iter());
    STATE.save(deps.storage, &queue)?;
    Ok(Response::new().add_attribute("action", "enqueue_batch"))
}

pub fn try_dequeue_batch(deps: DepsMut, number: u128) -> Result<Response, ContractError> {
    let mut queue = STATE.load(deps.storage)?;
    let drained: Vec<Bytes32> = queue.drain(0..(number as usize)).collect();
    STATE.save(deps.storage, &queue)?;

    let attributes: Vec<Attribute> = drained
        .iter()
        .enumerate()
        .map(|(i, item)| {
            Attribute::new(
                format!("item_{}", i),
                std::str::from_utf8(item).unwrap().to_owned(),
            )
        })
        .collect();

    Ok(Response::new().add_attributes(attributes))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Contains { item } => to_binary(&query_contains(deps, item)?),
        QueryMsg::LastItem {} => to_binary(&query_last_item(deps)?),
        QueryMsg::Peek {} => to_binary(&query_peek(deps)?),
        QueryMsg::IsEmpty {} => to_binary(&query_is_empty(deps)?),
        QueryMsg::Length {} => to_binary(&query_length(deps)?),
    }
}

pub fn query_contains(deps: Deps, item: Bytes32) -> StdResult<ContainsResponse> {
    let queue = STATE.load(deps.storage)?;
    Ok(ContainsResponse {
        contains: queue.contains(&item),
    })
}

pub fn query_last_item(deps:Deps) -> StdResult<LastItemResponse> {
    let queue = STATE.load(deps.storage)?;
    Ok(LastItemResponse {
        item: queue.back().expect("queue empty").to_owned(),
    })
}

pub fn query_peek(deps: Deps) -> StdResult<PeekResponse> {
    let queue = STATE.load(deps.storage)?;
    Ok(PeekResponse {
        item: queue.front().expect("queue empty").to_owned(),
    })
}

pub fn query_is_empty(deps: Deps) -> StdResult<IsEmptyResponse> {
    let queue = STATE.load(deps.storage)?;
    Ok(IsEmptyResponse {
        is_empty: queue.is_empty(),
    })
}

pub fn query_length(deps: Deps) -> StdResult<LengthResponse> {
    let queue = STATE.load(deps.storage)?;
    Ok(LengthResponse {
        length: queue.len(),
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
    }
}
