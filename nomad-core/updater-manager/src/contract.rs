#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Env, Event, MessageInfo, Response, StdResult,
};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, UpdaterResponse};
use crate::state::{HOME, UPDATER};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:updater-manager";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let updater = deps.api.addr_validate(&msg.updater)?;

    ownable::contract::instantiate(
        deps.branch(),
        env.clone(),
        info.clone(),
        ownable::msg::InstantiateMsg {},
    )?;

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    UPDATER.save(deps.storage, &updater)?;

    Ok(Response::new())
}

pub fn only_home(deps: Deps, info: MessageInfo) -> Result<Response, ContractError> {
    let home = HOME.load(deps.storage)?;
    if info.sender != home {
        return Err(ContractError::SlashNotHome);
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
        ExecuteMsg::SetHome { home } => try_set_home(deps, info, home),
        ExecuteMsg::SetUpdater { updater } => try_set_updater(deps, info, updater),
        ExecuteMsg::SlashUpdater { reporter } => try_slash_updater(deps, info, reporter),
        ExecuteMsg::RenounceOwnership {} => {
            Ok(ownable::contract::try_renounce_ownership(deps, info)?)
        }
        ExecuteMsg::TransferOwnership { new_owner } => Ok(
            ownable::contract::try_transfer_ownership(deps, info, new_owner)?,
        ),
    }
}

pub fn try_set_home(
    deps: DepsMut,
    info: MessageInfo,
    home: String,
) -> Result<Response, ContractError> {
    ownable::contract::only_owner(deps.as_ref(), info)?;

    let home_addr = deps.api.addr_validate(&home)?;
    HOME.save(deps.storage, &home_addr)?;

    Ok(Response::new().add_event(Event::new("SetHome").add_attribute("home", home)))
}

pub fn try_set_updater(
    deps: DepsMut,
    info: MessageInfo,
    updater: String,
) -> Result<Response, ContractError> {
    ownable::contract::only_owner(deps.as_ref(), info)?;

    let updater_addr = deps.api.addr_validate(&updater)?;
    UPDATER.save(deps.storage, &updater_addr)?;

    Ok(Response::new().add_event(Event::new("SetUpdater").add_attribute("updater", updater)))
}

pub fn try_slash_updater(
    deps: DepsMut,
    info: MessageInfo,
    reporter: String,
) -> Result<Response, ContractError> {
    only_home(deps.as_ref(), info)?;

    // TODO: implement updater slashing
    Ok(Response::new().add_event(Event::new("SlashUpdater").add_attribute("reporter", reporter)))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Updater {} => to_binary(&query_updater(deps)?),
        QueryMsg::Owner {} => to_binary(&ownable::contract::query_owner(deps)?),
    }
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

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies_with_balance(&coins(2, "token"));

        let msg = InstantiateMsg {
            updater: "updater".to_owned(),
        };
        let info = mock_info("owner", &coins(100, "earth"));

        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // Updater
        let res = query(deps.as_ref(), mock_env(), QueryMsg::Updater {}).unwrap();
        let value: UpdaterResponse = from_binary(&res).unwrap();
        assert_eq!("updater".to_owned(), value.updater);

        // Owner
        let res = query(deps.as_ref(), mock_env(), QueryMsg::Owner {}).unwrap();
        let value: ownable::msg::OwnerResponse = from_binary(&res).unwrap();
        assert_eq!("owner", value.owner);

        // Set home
        let info = mock_info("owner", &coins(100, "earth"));
        let msg = ExecuteMsg::SetHome {
            home: "home".to_owned(),
        };
        let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    }
}
