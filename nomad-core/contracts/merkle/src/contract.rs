#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use cw2::set_contract_version;
use ethers_core::types::H256;

use crate::error::ContractError;
use crate::merkle_tree::IncrementalMerkle;
use crate::state::MERKLE;
use common::merkle::{CountResponse, ExecuteMsg, InstantiateMsg, QueryMsg, RootResponse};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:merkle";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let merkle = IncrementalMerkle::default();
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    MERKLE.save(deps.storage, &merkle)?;

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
        ExecuteMsg::Insert { element } => try_insert(deps, element),
    }
}

pub fn try_insert(deps: DepsMut, element: H256) -> Result<Response, ContractError> {
    let mut merkle = MERKLE.load(deps.storage)?;
    merkle.ingest(element);
    MERKLE.save(deps.storage, &merkle)?;
    Ok(Response::new().add_attribute("element", format!("{:?}", element)))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Root {} => to_binary(&query_root(deps)?),
        QueryMsg::Count {} => to_binary(&query_count(deps)?),
        // QueryMsg::Tree {} => to_binary(&query_tree(deps)?),
    }
}

pub fn query_root(deps: Deps) -> StdResult<RootResponse> {
    let merkle = MERKLE.load(deps.storage)?;
    Ok(RootResponse {
        root: merkle.root(),
    })
}

pub fn query_count(deps: Deps) -> StdResult<CountResponse> {
    let merkle = MERKLE.load(deps.storage)?;
    Ok(CountResponse {
        count: merkle.count(),
    })
}

// pub fn query_tree(deps: Deps) -> StdResult<TreeResponse> {
//     let tree = MERKLE.load(deps.storage)?;
//     Ok(TreeResponse { tree })
// }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::merkle_tree::INITIAL_ROOT;
    use cosmwasm_std::testing::{mock_dependencies_with_balance, mock_env, mock_info};
    use cosmwasm_std::{coins, from_binary};

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies_with_balance(&coins(2, "token"));

        let msg = InstantiateMsg {};
        let info = mock_info("owner", &coins(100, "earth"));

        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // Initial root valid
        let res = query(deps.as_ref(), mock_env(), QueryMsg::Root {}).unwrap();
        let value: RootResponse = from_binary(&res).unwrap();
        assert_eq!(*INITIAL_ROOT, value.root);
    }
}
