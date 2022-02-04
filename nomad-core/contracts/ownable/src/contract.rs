#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::state::OWNER;
use msg::ownable::{ExecuteMsg, InstantiateMsg, OwnerResponse, QueryMsg};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:ownable";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    OWNER.save(deps.storage, &info.sender.clone())?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender))
}

pub fn only_owner(deps: Deps, info: MessageInfo) -> Result<Response, ContractError> {
    let owner = OWNER.load(deps.storage)?;

    if info.sender != owner {
        return Err(ContractError::Unauthorized {});
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
        ExecuteMsg::RenounceOwnership {} => try_renounce_ownership(deps, info),
        ExecuteMsg::TransferOwnership { new_owner } => {
            try_transfer_ownership(deps, info, new_owner)
        }
    }
}

pub fn try_renounce_ownership(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    only_owner(deps.as_ref(), info)?;
    OWNER.save(deps.storage, &Addr::unchecked("0x0"))?;
    Ok(Response::new().add_attribute("action", "renounce_ownership"))
}

pub fn try_transfer_ownership(
    deps: DepsMut,
    info: MessageInfo,
    new_owner: String,
) -> Result<Response, ContractError> {
    only_owner(deps.as_ref(), info)?;

    let new_owner = deps.api.addr_validate(&new_owner)?;
    OWNER.save(deps.storage, &new_owner.clone())?;

    Ok(Response::new()
        .add_attribute("action", "transfer_ownership")
        .add_attribute("new_owner", new_owner))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Owner {} => to_binary(&query_owner(deps)?),
    }
}

pub fn query_owner(deps: Deps) -> StdResult<OwnerResponse> {
    let owner = OWNER.load(deps.storage)?.to_string();
    Ok(OwnerResponse { owner })
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
        let info = mock_info("owner", &coins(100, "earth"));

        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        let res = query(deps.as_ref(), mock_env(), QueryMsg::Owner {}).unwrap();
        let value: OwnerResponse = from_binary(&res).unwrap();
        assert_eq!("owner", value.owner);
    }

    #[test]
    fn renounce_ownership() {
        let mut deps = mock_dependencies_with_balance(&coins(100, "token"));

        let msg = InstantiateMsg {};
        let info = mock_info("owner", &coins(100, "token"));
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
        let info = mock_info("owner", &coins(100, "token"));
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
        let info = mock_info("owner", &coins(100, "token"));
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
        assert_eq!("owner", value.owner);
    }
}
