use std::collections::VecDeque;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Attribute, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{
    ContainsResponse, ExecuteMsg, InstantiateMsg, IsEmptyResponse, LastItemResponse,
    LengthResponse, PeekResponse, QueryMsg,
};
use crate::state::QUEUE;

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
    let queue = VecDeque::<[u8; 32]>::new();
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    QUEUE.save(deps.storage, &queue)?;

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

pub fn try_enqueue(deps: DepsMut, item: [u8; 32]) -> Result<Response, ContractError> {
    let mut queue = QUEUE.load(deps.storage)?;
    queue.push_back(item);
    QUEUE.save(deps.storage, &queue)?;
    Ok(Response::new().add_attribute("action", "enqueue"))
}

pub fn try_dequeue(deps: DepsMut) -> Result<Response, ContractError> {
    let mut queue = QUEUE.load(deps.storage)?;
    let item = queue.pop_front().ok_or(ContractError::QueueEmpty {})?;
    QUEUE.save(deps.storage, &queue)?;
    Ok(Response::new().set_data(to_binary(&item)?))
}

pub fn try_enqueue_batch(deps: DepsMut, items: Vec<[u8; 32]>) -> Result<Response, ContractError> {
    let mut queue = QUEUE.load(deps.storage)?;
    queue.extend(items.iter());
    QUEUE.save(deps.storage, &queue)?;
    Ok(Response::new().add_attribute("action", "enqueue_batch"))
}

pub fn try_dequeue_batch(deps: DepsMut, number: u128) -> Result<Response, ContractError> {
    let mut queue = QUEUE.load(deps.storage)?;
    let drained: Vec<[u8; 32]> = queue.drain(0..(number as usize)).collect();
    QUEUE.save(deps.storage, &queue)?;

    let attributes: Vec<Attribute> = drained
        .iter()
        .enumerate()
        .map(|(i, item)| {
            Attribute::new(
                format!("item_{}", i + 1),
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

pub fn query_contains(deps: Deps, item: [u8; 32]) -> StdResult<ContainsResponse> {
    let queue = QUEUE.load(deps.storage)?;
    Ok(ContainsResponse {
        contains: queue.contains(&item),
    })
}

pub fn query_last_item(deps: Deps) -> StdResult<LastItemResponse> {
    let queue = QUEUE.load(deps.storage)?;
    Ok(LastItemResponse {
        item: queue.back().map_or([0u8; 32], |item| item.clone()),
    })
}

pub fn query_peek(deps: Deps) -> StdResult<PeekResponse> {
    let queue = QUEUE.load(deps.storage)?;
    Ok(PeekResponse {
        item: queue.front().expect("queue empty").to_owned(),
    })
}

pub fn query_is_empty(deps: Deps) -> StdResult<IsEmptyResponse> {
    let queue = QUEUE.load(deps.storage)?;
    Ok(IsEmptyResponse {
        is_empty: queue.is_empty(),
    })
}

pub fn query_length(deps: Deps) -> StdResult<LengthResponse> {
    let queue = QUEUE.load(deps.storage)?;
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

        // Length 0
        let res = query(deps.as_ref(), mock_env(), QueryMsg::Length {}).unwrap();
        let value: LengthResponse = from_binary(&res).unwrap();
        assert_eq!(0, value.length);

        // Last item defaults to 0x0
        let res = query(deps.as_ref(), mock_env(), QueryMsg::LastItem {}).unwrap();
        let value: LastItemResponse = from_binary(&res).unwrap();
        assert_eq!([0u8; 32], value.item);
    }

    #[test]
    fn enqueues_dequeues_and_query() {
        let mut deps = mock_dependencies_with_balance(&coins(2, "token"));

        let msg = InstantiateMsg {};
        let info = mock_info("creator", &coins(100, "earth"));

        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // Is empty
        let res = query(deps.as_ref(), mock_env(), QueryMsg::IsEmpty {}).unwrap();
        let value: IsEmptyResponse = from_binary(&res).unwrap();
        assert_eq!(true, value.is_empty);

        // Enqueue single
        let single_item = [0u8; 32];
        try_enqueue(deps.as_mut(), single_item).unwrap();

        // Is empty
        let res = query(deps.as_ref(), mock_env(), QueryMsg::Length {}).unwrap();
        let value: LengthResponse = from_binary(&res).unwrap();
        assert_eq!(1, value.length);

        // Dequeue single
        let res = try_dequeue(deps.as_mut()).unwrap();
        let item: [u8; 32] = from_binary(&res.data.unwrap()).unwrap();
        assert_eq!(single_item, item);

        // Enqueue batch 3
        let items = vec![[0u8; 32], [1u8; 32], [2u8; 32]];
        try_enqueue_batch(deps.as_mut(), items).unwrap();

        // Length
        let res = query(deps.as_ref(), mock_env(), QueryMsg::Length {}).unwrap();
        let value: LengthResponse = from_binary(&res).unwrap();
        assert_eq!(3, value.length);

        // Is empty
        let res = query(deps.as_ref(), mock_env(), QueryMsg::IsEmpty {}).unwrap();
        let value: IsEmptyResponse = from_binary(&res).unwrap();
        assert_eq!(false, value.is_empty);

        // Peek
        let res = query(deps.as_ref(), mock_env(), QueryMsg::Peek {}).unwrap();
        let value: PeekResponse = from_binary(&res).unwrap();
        assert_eq!([0u8; 32], value.item);

        // Last item
        let res = query(deps.as_ref(), mock_env(), QueryMsg::LastItem {}).unwrap();
        let value: LastItemResponse = from_binary(&res).unwrap();
        assert_eq!([2u8; 32], value.item);

        // Contains
        let res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::Contains { item: [1u8; 32] },
        )
        .unwrap();
        let value: ContainsResponse = from_binary(&res).unwrap();
        assert_eq!(true, value.contains);

        // Dequeue batch 2
        let res = try_dequeue_batch(deps.as_mut(), 2).unwrap();
        let item_1_attr = res
            .attributes
            .iter()
            .find(|&item| item.key == "item_1")
            .unwrap();
        let item_2_attr = res
            .attributes
            .iter()
            .find(|&item| item.key == "item_2")
            .unwrap();
        assert_eq!(std::str::from_utf8(&[0u8; 32]).unwrap(), item_1_attr.value);
        assert_eq!(std::str::from_utf8(&[1u8; 32]).unwrap(), item_2_attr.value);

        // Length
        let res = query(deps.as_ref(), mock_env(), QueryMsg::Length {}).unwrap();
        let value: LengthResponse = from_binary(&res).unwrap();
        assert_eq!(1, value.length);
    }
}
