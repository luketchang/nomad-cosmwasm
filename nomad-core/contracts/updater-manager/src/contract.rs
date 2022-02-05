#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, ContractResult, CosmosMsg, Deps, DepsMut, Env, Event, MessageInfo, Reply,
    ReplyOn, Response, StdResult, SubMsg, WasmMsg,
};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::state::{HOME, UPDATER};
use common::updater_manager::{ExecuteMsg, InstantiateMsg, QueryMsg, UpdaterResponse};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:updater-manager";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const SET_UPDATER_ID: u64 = 1;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let updater = deps.api.addr_validate(&msg.updater)?;

    ownable::instantiate(
        deps.branch(),
        env.clone(),
        info.clone(),
        common::ownable::InstantiateMsg {},
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
        ExecuteMsg::RenounceOwnership {} => Ok(ownable::try_renounce_ownership(deps, info)?),
        ExecuteMsg::TransferOwnership { new_owner } => {
            Ok(ownable::try_transfer_ownership(deps, info, new_owner)?)
        }
    }
}

pub fn try_set_home(
    deps: DepsMut,
    info: MessageInfo,
    home: String,
) -> Result<Response, ContractError> {
    ownable::only_owner(deps.as_ref(), info)?;

    let home_addr = deps.api.addr_validate(&home)?;
    HOME.save(deps.storage, &home_addr)?;

    Ok(Response::new().add_event(Event::new("SetHome").add_attribute("home", home)))
}

pub fn try_set_updater(
    deps: DepsMut,
    info: MessageInfo,
    updater: String,
) -> Result<Response, ContractError> {
    ownable::only_owner(deps.as_ref(), info)?;

    let updater_addr = deps.api.addr_validate(&updater.clone())?;
    UPDATER.save(deps.storage, &updater_addr)?;

    let home_addr = HOME.load(deps.storage)?;

    let set_updater_msg = common::home::ExecuteMsg::SetUpdater { updater };
    let wasm_msg = WasmMsg::Execute {
        contract_addr: home_addr.to_string(),
        msg: to_binary(&set_updater_msg)?,
        funds: vec![],
    };
    let cosmos_msg = CosmosMsg::Wasm(wasm_msg);

    let sub_msg = SubMsg {
        id: SET_UPDATER_ID,
        msg: cosmos_msg,
        gas_limit: None,
        reply_on: ReplyOn::Always,
    };

    Ok(Response::new().add_submessage(sub_msg))
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
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        SET_UPDATER_ID => reply_set_updater(deps.as_ref(), msg),
        _ => Err(ContractError::UnknownReplyMessage { id: msg.id }),
    }
}

pub fn reply_set_updater(_deps: Deps, msg: Reply) -> Result<Response, ContractError> {
    match msg.result {
        ContractResult::Ok(_) => Ok(Response::new().add_attribute("action", "set_updater")),
        ContractResult::Err(e) => Err(ContractError::FailedSetUpdaterCall(e)),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Updater {} => to_binary(&query_updater(deps)?),
        QueryMsg::Owner {} => to_binary(&ownable::query_owner(deps)?),
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
        let value: common::ownable::OwnerResponse = from_binary(&res).unwrap();
        assert_eq!("owner", value.owner);

        // Set home
        let info = mock_info("owner", &coins(100, "earth"));
        let msg = ExecuteMsg::SetHome {
            home: "home".to_owned(),
        };
        let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    }
}
