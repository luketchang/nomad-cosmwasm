#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_binary, to_binary, Addr, Binary, ContractResult, CosmosMsg, Deps, DepsMut, Env, Event,
    MessageInfo, Reply, ReplyOn, Response, StdResult, SubMsg, WasmMsg,
};
use cw2::set_contract_version;
use ethers_core::types::H256;
use lib::{addr_to_bytes32, destination_and_nonce, Encode, NomadMessage};

use crate::error::ContractError;
use crate::msg::{
    ExecuteMsg, InstantiateMsg, NoncesResponse, QueryMsg, SuggestUpdateResponse,
    UpdaterManagerResponse,
};
use crate::state::{NONCES, UPDATER_MANAGER};

const CONTRACT_NAME: &str = "crates.io:home";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const SLASH_UPDATER_ID: u64 = 1;
const MAX_MESSAGE_BODY_BYTES: u64 = 2 * u64::pow(2, 10);

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    queue::contract::instantiate(deps.branch(), env.clone(), info.clone(), msg.clone().into())?;
    merkle::contract::instantiate(deps.branch(), env.clone(), info.clone(), msg.clone().into())?;
    nomad_base::contract::instantiate(
        deps.branch(),
        env.clone(),
        info.clone(),
        msg.clone().into(),
    )?;

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    UPDATER_MANAGER.save(deps.storage, &Addr::unchecked("0x0"))?;

    Ok(Response::new())
}

fn only_updater_manager(deps: Deps, info: MessageInfo) -> Result<Response, ContractError> {
    let updater_manager = UPDATER_MANAGER.load(deps.storage)?;
    if info.sender != updater_manager {
        return Err(ContractError::NotUpdaterManager {
            address: updater_manager.to_string(),
        });
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
        ExecuteMsg::Dispatch {
            destination,
            recipient,
            message_body,
        } => try_dispatch(deps, info, destination, recipient, message_body),
        ExecuteMsg::Update {
            committed_root,
            new_root,
            signature,
        } => try_update(deps, info, committed_root, new_root, signature),
        ExecuteMsg::DoubleUpdate {
            old_root,
            new_roots,
            signature,
            signature_2,
        } => Ok(nomad_base::contract::try_double_update(
            deps,
            info,
            old_root,
            new_roots,
            signature,
            signature_2,
            _fail,
        )?),
        ExecuteMsg::ImproperUpdate {
            old_root,
            new_root,
            signature,
        } => try_improper_update(deps, info, old_root, new_root, &signature),
        ExecuteMsg::SetUpdater { updater } => try_set_updater(deps, info, updater),
        ExecuteMsg::SetUpdaterManager { updater_manager } => {
            try_set_updater_manager(deps, info, updater_manager)
        }
        ExecuteMsg::RenounceOwnership {} => {
            Ok(ownable::contract::try_renounce_ownership(deps, info)?)
        }
        ExecuteMsg::TransferOwnership { new_owner } => Ok(
            ownable::contract::try_transfer_ownership(deps, info, new_owner)?,
        ),
    }
}

pub fn try_dispatch(
    mut deps: DepsMut,
    info: MessageInfo,
    destination: u32,
    recipient: String,
    message: Vec<u8>,
) -> Result<Response, ContractError> {
    nomad_base::contract::not_failed(deps.as_ref())?;

    let length = message.len() as u64;
    if length > MAX_MESSAGE_BODY_BYTES {
        return Err(ContractError::MsgTooLong { length });
    }

    let nonce = query_nonces(deps.as_ref(), destination)?.next_nonce;
    NONCES.save(deps.storage, destination, &(nonce + 1))?;

    let origin = nomad_base::contract::query_local_domain(deps.as_ref())?.local_domain;
    let recipient_addr = deps.api.addr_validate(&recipient)?;

    let nomad_message = NomadMessage {
        origin,
        sender: addr_to_bytes32(info.sender),
        nonce,
        destination,
        recipient: addr_to_bytes32(recipient_addr),
        body: message.clone(),
    };

    // Get state before mutations
    let leaf_index = merkle::contract::query_count(deps.as_ref())?.count;
    let committed_root = nomad_base::contract::query_committed_root(deps.as_ref())?.committed_root;

    // Insert leaf into tree
    let hash: H256 = nomad_message.to_leaf();
    merkle::contract::try_insert(deps.branch(), hash)?;

    // Enqueue merkle root
    let root = merkle::contract::query_root(deps.as_ref())?.root;
    queue::contract::try_enqueue(deps.branch(), root)?;

    Ok(Response::new().add_event(
        Event::new("Dispatch")
            .add_attribute("message_hash", format!("{:?}", hash))
            .add_attribute("leaf_index", leaf_index.to_string())
            .add_attribute(
                "destination_and_nonce",
                destination_and_nonce(destination, nonce).to_string(),
            )
            .add_attribute("committed_root", format!("{:?}", committed_root))
            .add_attribute("message", format!("{:?}", nomad_message.to_vec())),
    ))
}

pub fn try_update(
    mut deps: DepsMut,
    info: MessageInfo,
    committed_root: H256,
    new_root: H256,
    signature: Vec<u8>,
) -> Result<Response, ContractError> {
    nomad_base::contract::not_failed(deps.as_ref())?;

    // TODO: clean up
    let improper_update_res =
        try_improper_update(deps.branch(), info, committed_root, new_root, &signature)?;
    let improper_update: bool = from_binary(&improper_update_res.clone().data.unwrap())?;

    if improper_update {
        return Ok(improper_update_res);
    }

    loop {
        let next_res = queue::contract::try_dequeue(deps.branch())?;
        let next: H256 = from_binary(&next_res.data.unwrap())?;
        if next == new_root {
            break;
        }
    }

    nomad_base::contract::_set_committed_root(deps.branch(), new_root)?;

    let local_domain = nomad_base::contract::query_local_domain(deps.as_ref())?.local_domain;

    Ok(Response::new().add_event(
        Event::new("Update")
            .add_attribute("local_domain", local_domain.to_string())
            .add_attribute("committed_root", format!("{:?}", committed_root))
            .add_attribute("new_root", format!("{:?}", new_root))
            .add_attribute("signature", format!("{:?}", signature)),
    ))
}

pub fn try_improper_update(
    deps: DepsMut,
    info: MessageInfo,
    old_root: H256,
    new_root: H256,
    signature: &[u8],
) -> Result<Response, ContractError> {
    nomad_base::contract::not_failed(deps.as_ref())?;

    if !nomad_base::contract::is_updater_signature(deps.as_ref(), old_root, new_root, signature)? {
        return Err(ContractError::NotUpdaterSignature);
    }

    let committed_root = nomad_base::contract::query_committed_root(deps.as_ref())?.committed_root;
    if old_root != committed_root {
        return Err(ContractError::NotCurrentCommittedRoot { old_root });
    }

    if !queue::contract::query_contains(deps.as_ref(), new_root)?.contains {
        _fail(deps, info)?;
        return Ok(Response::new().set_data(to_binary(&true)?).add_event(
            Event::new("ImproperUpdate")
                .add_attribute("old_root", format!("{:?}", old_root))
                .add_attribute("new_root", format!("{:?}", new_root))
                .add_attribute("signature", format!("{:?}", signature)),
        ));
    }

    Ok(Response::new().set_data(to_binary(&false)?))
}

pub fn try_set_updater(
    deps: DepsMut,
    info: MessageInfo,
    updater: String,
) -> Result<Response, ContractError> {
    only_updater_manager(deps.as_ref(), info.clone())?;
    Ok(nomad_base::contract::_set_updater(deps, updater)?)
}

pub fn try_set_updater_manager(
    deps: DepsMut,
    info: MessageInfo,
    updater_manager: String,
) -> Result<Response, ContractError> {
    ownable::contract::only_owner(deps.as_ref(), info)?;
    let updater_manager_addr = deps.api.addr_validate(&updater_manager)?;

    UPDATER_MANAGER.save(deps.storage, &updater_manager_addr)?;

    Ok(Response::new().add_event(
        Event::new("SetUpdaterManager").add_attribute("updater_manager", updater_manager),
    ))
}

fn _fail(mut deps: DepsMut, info: MessageInfo) -> Result<Response, nomad_base::ContractError> {
    nomad_base::contract::_set_failed(deps.branch())?;

    let slash_updater_msg = updater_manager::msg::ExecuteMsg::SlashUpdater {
        reporter: info.sender.to_string(),
    };
    let wasm_msg = WasmMsg::Execute {
        contract_addr: query_updater_manager(deps.as_ref())?.updater_manager,
        msg: to_binary(&slash_updater_msg)?,
        funds: vec![],
    };
    let cosmos_msg = CosmosMsg::Wasm(wasm_msg);

    let sub_msg = SubMsg {
        id: SLASH_UPDATER_ID,
        msg: cosmos_msg,
        gas_limit: None,
        reply_on: ReplyOn::Always,
    };

    Ok(Response::new().add_submessage(sub_msg))
}

#[entry_point]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        SLASH_UPDATER_ID => reply_slash_updater(deps.as_ref(), env, msg),
        _ => Err(ContractError::UnknownReplyMessage { id: msg.id }),
    }
}

pub fn reply_slash_updater(_deps: Deps, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.result {
        ContractResult::Ok(res) => Ok(Response::new().add_events(res.events)),
        ContractResult::Err(e) => Err(ContractError::FailedSlashUpdaterCall(e)),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Nonces { domain } => to_binary(&query_nonces(deps, domain)?),
        QueryMsg::SuggestUpdate {} => to_binary(&query_suggest_update(deps)?),
        QueryMsg::UpdaterManager {} => to_binary(&query_updater_manager(deps)?),
        QueryMsg::State {} => to_binary(&nomad_base::contract::query_state(deps)?),
        QueryMsg::CommittedRoot {} => to_binary(&nomad_base::contract::query_committed_root(deps)?),
        QueryMsg::HomeDomainHash {} => {
            to_binary(&nomad_base::contract::query_home_domain_hash(deps)?)
        }
        QueryMsg::LocalDomain {} => to_binary(&nomad_base::contract::query_local_domain(deps)?),
        QueryMsg::Updater {} => to_binary(&nomad_base::contract::query_updater(deps)?),
        QueryMsg::Count {} => to_binary(&merkle::contract::query_count(deps)?),
        QueryMsg::Root {} => to_binary(&merkle::contract::query_root(deps)?),
        QueryMsg::Tree {} => to_binary(&merkle::contract::query_tree(deps)?),
        QueryMsg::QueueContains { item } => {
            to_binary(&queue::contract::query_contains(deps, item)?)
        }
        QueryMsg::QueueEnd {} => to_binary(&queue::contract::query_last_item(deps)?),
        QueryMsg::QueueLength {} => to_binary(&queue::contract::query_length(deps)?),
        QueryMsg::Owner {} => to_binary(&ownable::contract::query_owner(deps)?),
    }
}

pub fn query_nonces(deps: Deps, domain: u32) -> StdResult<NoncesResponse> {
    Ok(NoncesResponse {
        next_nonce: NONCES.may_load(deps.storage, domain)?.unwrap_or_default(),
    })
}

pub fn query_suggest_update(deps: Deps) -> StdResult<SuggestUpdateResponse> {
    let committed_root = nomad_base::contract::query_committed_root(deps)?.committed_root;
    let new_root = queue::contract::query_last_item(deps)?.item;
    Ok(SuggestUpdateResponse {
        committed_root,
        new_root,
    })
}

pub fn query_updater_manager(deps: Deps) -> StdResult<UpdaterManagerResponse> {
    let updater_manager = UPDATER_MANAGER.load(deps.storage)?;
    Ok(UpdaterManagerResponse {
        updater_manager: updater_manager.into_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies_with_balance, mock_env, mock_info};
    use cosmwasm_std::{coins, from_binary};
    use merkle::merkle_tree::INITIAL_ROOT;
    use merkle::msg::RootResponse;
    use nomad_base::msg::{
        CommittedRootResponse, LocalDomainResponse, StateResponse, UpdaterResponse,
    };
    use queue::msg::{EndResponse as QueueEndResponse, LengthResponse as QueueLengthResponse};
    use test_utils::Updater;
    use test_utils::{event_attr_value_by_key, h256_to_string};
    use nomad_base::state::States;

    const LOCAL_DOMAIN: u32 = 1000;
    const UPDATER_PRIVKEY: &str =
        "1111111111111111111111111111111111111111111111111111111111111111";
    const UPDATER_PUBKEY: &str = "0x19e7e376e7c213b7e7e7e46cc70a5dd086daff2a";

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies_with_balance(&coins(2, "token"));

        let msg = InstantiateMsg {
            local_domain: LOCAL_DOMAIN,
            updater: UPDATER_PUBKEY.to_owned(),
        };
        let info = mock_info("owner", &coins(100, "earth"));

        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // ------ HOME ------
        // Nonces
        let res = query(deps.as_ref(), mock_env(), QueryMsg::Nonces { domain: 100 }).unwrap();
        let value: NoncesResponse = from_binary(&res).unwrap();
        assert_eq!(0, value.next_nonce);

        // Empty updater manager
        let res = query(deps.as_ref(), mock_env(), QueryMsg::UpdaterManager {}).unwrap();
        let value: UpdaterManagerResponse = from_binary(&res).unwrap();
        assert_eq!("0x0", value.updater_manager);

        // Suggested update 0x0 and 0x0
        let res = query(deps.as_ref(), mock_env(), QueryMsg::SuggestUpdate {}).unwrap();
        let value: SuggestUpdateResponse = from_binary(&res).unwrap();
        assert_eq!(H256::zero(), value.committed_root);
        assert_eq!(H256::zero(), value.new_root);

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

        // ------ MERKLE ------
        // Initial root valid
        let res = query(deps.as_ref(), mock_env(), QueryMsg::Root {}).unwrap();
        let value: RootResponse = from_binary(&res).unwrap();
        assert_eq!(*INITIAL_ROOT, value.root);

        // ------ QUEUE ------
        // Length 0
        let res = query(deps.as_ref(), mock_env(), QueryMsg::QueueLength {}).unwrap();
        let value: QueueLengthResponse = from_binary(&res).unwrap();
        assert_eq!(0, value.length);

        // Last item defaults to 0x0
        let res = query(deps.as_ref(), mock_env(), QueryMsg::QueueEnd {}).unwrap();
        let value: QueueEndResponse = from_binary(&res).unwrap();
        assert_eq!(H256::zero(), value.item);
    }

    #[test]
    fn does_not_dispatch_messages_too_large() {
        let mut deps = mock_dependencies_with_balance(&coins(2, "token"));

        let msg = InstantiateMsg {
            local_domain: LOCAL_DOMAIN,
            updater: UPDATER_PUBKEY.to_owned(),
        };
        let info = mock_info("owner", &coins(100, "earth"));

        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // Dispatch 3000 byte message
        let info = mock_info("dispatcher", &coins(100, "earth"));
        let msg = ExecuteMsg::Dispatch {
            destination: 2000,
            recipient: "recipient".to_owned(),
            message_body: [0u8].repeat(3000),
        };
        let res = execute(deps.as_mut(), mock_env(), info, msg);
        assert!(res.is_err());
    }

    #[test]
    fn dispatches_message() {
        let mut deps = mock_dependencies_with_balance(&coins(2, "token"));

        let msg = InstantiateMsg {
            local_domain: LOCAL_DOMAIN,
            updater: UPDATER_PUBKEY.to_owned(),
        };
        let info = mock_info("owner", &coins(100, "earth"));

        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // Create message
        let sender = H256::repeat_byte(0);
        let destination = 2000;
        let recipient = H256::repeat_byte(1);
        let message_body = [0u8].repeat(100);
        let nonce = query_nonces(deps.as_ref(), destination).unwrap().next_nonce;

        let nomad_message = NomadMessage {
            origin: LOCAL_DOMAIN,
            sender,
            nonce,
            destination,
            recipient,
            body: message_body.clone(),
        };

        // Dispatch message
        let info = mock_info(&h256_to_string(sender), &coins(100, "earth"));
        let msg = ExecuteMsg::Dispatch {
            destination,
            recipient: h256_to_string(recipient),
            message_body,
        };
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        let dispatch_event = &res.events[0];
        assert_eq!("Dispatch", dispatch_event.ty);
        assert_eq!(
            format!("{:?}", nomad_message.to_leaf()),
            event_attr_value_by_key(&dispatch_event, "message_hash").unwrap()
        );
        assert_eq!(
            "0",
            event_attr_value_by_key(&dispatch_event, "leaf_index").unwrap()
        );
        assert_eq!(
            destination_and_nonce(destination, nonce).to_string(),
            event_attr_value_by_key(&dispatch_event, "destination_and_nonce").unwrap()
        );
        assert_eq!(
            format!("{:?}", H256::zero()),
            event_attr_value_by_key(&dispatch_event, "committed_root").unwrap()
        );
        assert_eq!(
            format!("{:?}", nomad_message.to_vec()),
            event_attr_value_by_key(&dispatch_event, "message").unwrap()
        );
    }

    #[test]
    fn suggests_updates() {
        let mut deps = mock_dependencies_with_balance(&coins(2, "token"));

        let msg = InstantiateMsg {
            local_domain: LOCAL_DOMAIN,
            updater: UPDATER_PUBKEY.to_owned(),
        };
        let info = mock_info("owner", &coins(100, "earth"));

        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // Get committed root
        let res = query(deps.as_ref(), mock_env(), QueryMsg::CommittedRoot {}).unwrap();
        let committed_root = from_binary::<CommittedRootResponse>(&res)
            .unwrap()
            .committed_root;

        // Dispatch message
        let info = mock_info("dispatcher", &coins(100, "earth"));
        let msg = ExecuteMsg::Dispatch {
            destination: 2000,
            recipient: "recipient".to_owned(),
            message_body: [0u8].repeat(100),
        };
        let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // Get root at end of queue
        let res = query(deps.as_ref(), mock_env(), QueryMsg::QueueEnd {}).unwrap();
        let latest_root = from_binary::<QueueEndResponse>(&res).unwrap().item;

        // Suggested update contains committed and latest roots
        let res = query(deps.as_ref(), mock_env(), QueryMsg::SuggestUpdate {}).unwrap();
        let suggested_update = from_binary::<SuggestUpdateResponse>(&res).unwrap();

        assert_eq!(committed_root, suggested_update.committed_root);
        assert_eq!(latest_root, suggested_update.new_root);
    }

    #[test]
    fn suggests_zero_update_values_on_empty_queue() {
        let mut deps = mock_dependencies_with_balance(&coins(2, "token"));

        let msg = InstantiateMsg {
            local_domain: LOCAL_DOMAIN,
            updater: UPDATER_PUBKEY.to_owned(),
        };
        let info = mock_info("owner", &coins(100, "earth"));

        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // Queue is empty
        let res = query(deps.as_ref(), mock_env(), QueryMsg::QueueLength {}).unwrap();
        let length = from_binary::<QueueLengthResponse>(&res).unwrap().length;
        assert_eq!(0, length);

        let res = query(deps.as_ref(), mock_env(), QueryMsg::SuggestUpdate {}).unwrap();
        let suggested_update = from_binary::<SuggestUpdateResponse>(&res).unwrap();

        assert_eq!(H256::zero(), suggested_update.committed_root);
        assert_eq!(H256::zero(), suggested_update.new_root);
    }

    #[tokio::test]
    async fn accepts_valid_update() {
        let updater: Updater = Updater::from_privkey(UPDATER_PRIVKEY, LOCAL_DOMAIN);

        let mut deps = mock_dependencies_with_balance(&coins(100, "token"));

        let init_msg = InstantiateMsg {
            local_domain: LOCAL_DOMAIN,
            updater: UPDATER_PUBKEY.to_owned(),
        };
        let info = mock_info("owner", &coins(100, "earth"));

        let res = instantiate(deps.as_mut(), mock_env(), info.clone(), init_msg).unwrap();
        assert_eq!(0, res.messages.len());

        // Dispatch message
        let info = mock_info("dispatcher", &coins(100, "earth"));
        let msg = ExecuteMsg::Dispatch {
            destination: 2000,
            recipient: "recipient".to_owned(),
            message_body: [0u8].repeat(100),
        };
        let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // Get update and sign
        let res = query(deps.as_ref(), mock_env(), QueryMsg::SuggestUpdate {}).unwrap();
        let SuggestUpdateResponse {
            committed_root,
            new_root,
        } = from_binary::<SuggestUpdateResponse>(&res).unwrap();
        let update = updater.sign_update(committed_root, new_root).await.unwrap();

        // Submit update
        let info = mock_info("submitter", &coins(100, "earth"));
        let msg = ExecuteMsg::Update {
            committed_root,
            new_root,
            signature: update.signature.to_vec(),
        };
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // Check update event
        let event = &res.events[0];
        assert_eq!("Update", event.ty);
        assert_eq!(
            format!("{:?}", committed_root),
            event_attr_value_by_key(&event, "committed_root").unwrap()
        );
        assert_eq!(
            format!("{:?}", new_root),
            event_attr_value_by_key(&event, "new_root").unwrap()
        );
        assert_eq!(
            format!("{:?}", update.signature.to_vec()),
            event_attr_value_by_key(&event, "signature").unwrap()
        );

        // Expect queue is empty
        let res = query(deps.as_ref(), mock_env(), QueryMsg::QueueLength {}).unwrap();
        let length = from_binary::<QueueLengthResponse>(&res).unwrap().length;
        assert_eq!(0, length);
    }

    #[tokio::test]
    async fn batch_accepts_updates() {
        let updater: Updater = Updater::from_privkey(UPDATER_PRIVKEY, LOCAL_DOMAIN);

        let mut deps = mock_dependencies_with_balance(&coins(100, "token"));

        let init_msg = InstantiateMsg {
            local_domain: LOCAL_DOMAIN,
            updater: UPDATER_PUBKEY.to_owned(),
        };
        let info = mock_info("owner", &coins(100, "earth"));

        let res = instantiate(deps.as_mut(), mock_env(), info.clone(), init_msg).unwrap();
        assert_eq!(0, res.messages.len());

        // Dispatch several messages
        let info = mock_info("dispatcher", &coins(100, "earth"));
        for i in 1..3 {
            let msg = ExecuteMsg::Dispatch {
                destination: i * 1000,
                recipient: "recipient".to_owned(),
                message_body: [i as u8].repeat(100),
            };

            execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
        }

        // Get update and sign
        let res = query(deps.as_ref(), mock_env(), QueryMsg::SuggestUpdate {}).unwrap();
        let SuggestUpdateResponse {
            committed_root,
            new_root,
        } = from_binary::<SuggestUpdateResponse>(&res).unwrap();
        let update = updater.sign_update(committed_root, new_root).await.unwrap();

        // Submit update
        let info = mock_info("submitter", &coins(100, "earth"));
        let msg = ExecuteMsg::Update {
            committed_root,
            new_root,
            signature: update.signature.to_vec(),
        };
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // Check update event
        let event = &res.events[0];
        assert_eq!("Update", event.ty);
        assert_eq!(
            format!("{:?}", committed_root),
            event_attr_value_by_key(&event, "committed_root").unwrap()
        );
        assert_eq!(
            format!("{:?}", new_root),
            event_attr_value_by_key(&event, "new_root").unwrap()
        );
        assert_eq!(
            format!("{:?}", update.signature.to_vec()),
            event_attr_value_by_key(&event, "signature").unwrap()
        );

        // Expect queue is empty
        let res = query(deps.as_ref(), mock_env(), QueryMsg::QueueLength {}).unwrap();
        let length = from_binary::<QueueLengthResponse>(&res).unwrap().length;
        assert_eq!(0, length);
    }

    #[tokio::test]
    async fn rejects_update_not_building_off_current_committed() {
        let updater: Updater = Updater::from_privkey(UPDATER_PRIVKEY, LOCAL_DOMAIN);

        let mut deps = mock_dependencies_with_balance(&coins(100, "token"));

        let init_msg = InstantiateMsg {
            local_domain: LOCAL_DOMAIN,
            updater: UPDATER_PUBKEY.to_owned(),
        };
        let info = mock_info("owner", &coins(100, "earth"));

        let res = instantiate(deps.as_mut(), mock_env(), info.clone(), init_msg).unwrap();
        assert_eq!(0, res.messages.len());

        // Dispatch several messages
        let info = mock_info("dispatcher", &coins(100, "earth"));
        for i in 1..3 {
            let msg = ExecuteMsg::Dispatch {
                destination: i * 1000,
                recipient: "recipient".to_owned(),
                message_body: [i as u8].repeat(100),
            };

            execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
        }

        // Sign update building off random root
        let res = query(deps.as_ref(), mock_env(), QueryMsg::SuggestUpdate {}).unwrap();
        let SuggestUpdateResponse {
            committed_root: _,
            new_root,
        } = from_binary::<SuggestUpdateResponse>(&res).unwrap();

        let random_root = H256::repeat_byte(1);
        let update = updater.sign_update(random_root, new_root).await.unwrap();

        // Expect update submission to return error
        let info = mock_info("submitter", &coins(100, "earth"));
        let msg = ExecuteMsg::Update {
            committed_root: random_root,
            new_root,
            signature: update.signature.to_vec(),
        };
        let res = execute(deps.as_mut(), mock_env(), info, msg);

        assert!(res.is_err());
    }

    #[tokio::test]
    async fn catches_improper_update() {
        let updater: Updater = Updater::from_privkey(UPDATER_PRIVKEY, LOCAL_DOMAIN);

        let mut deps = mock_dependencies_with_balance(&coins(100, "token"));

        let init_msg = InstantiateMsg {
            local_domain: LOCAL_DOMAIN,
            updater: UPDATER_PUBKEY.to_owned(),
        };
        let info = mock_info("owner", &coins(100, "earth"));

        let res = instantiate(deps.as_mut(), mock_env(), info.clone(), init_msg).unwrap();
        assert_eq!(0, res.messages.len());

        // Dispatch several messages
        let info = mock_info("dispatcher", &coins(100, "earth"));
        for i in 1..3 {
            let msg = ExecuteMsg::Dispatch {
                destination: i * 1000,
                recipient: "recipient".to_owned(),
                message_body: [i as u8].repeat(100),
            };

            execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
        }

        // Sign improper update
        let res = query(deps.as_ref(), mock_env(), QueryMsg::SuggestUpdate {}).unwrap();
        let SuggestUpdateResponse {
            committed_root,
            new_root: _,
        } = from_binary::<SuggestUpdateResponse>(&res).unwrap();

        let improper_root = H256::repeat_byte(1);
        let update = updater
            .sign_update(committed_root, improper_root)
            .await
            .unwrap();

        // Submit improper update
        let info = mock_info("submitter", &coins(100, "earth"));
        let msg = ExecuteMsg::Update {
            committed_root,
            new_root: improper_root,
            signature: update.signature.to_vec(),
        };
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // Check for improper update event
        assert_eq!("ImproperUpdate".to_owned(), res.events[0].ty);

        // Check home failed
        let res = query(deps.as_ref(), mock_env(), QueryMsg::State {}).unwrap();
        let state = from_binary::<StateResponse>(&res).unwrap().state;
        assert_eq!(States::Failed, state);
    }

    #[tokio::test]
    async fn rejects_update_from_non_updater() {
        let not_updater_privkey =
            "2111111111111111111111111111111111111111111111111111111111111111";
        let not_updater: Updater = Updater::from_privkey(not_updater_privkey, LOCAL_DOMAIN);

        let mut deps = mock_dependencies_with_balance(&coins(100, "token"));

        let init_msg = InstantiateMsg {
            local_domain: LOCAL_DOMAIN,
            updater: UPDATER_PUBKEY.to_owned(),
        };
        let info = mock_info("owner", &coins(100, "earth"));

        let res = instantiate(deps.as_mut(), mock_env(), info.clone(), init_msg).unwrap();
        assert_eq!(0, res.messages.len());

        // Dispatch several messages
        let info = mock_info("dispatcher", &coins(100, "earth"));
        for i in 1..3 {
            let msg = ExecuteMsg::Dispatch {
                destination: i * 1000,
                recipient: "recipient".to_owned(),
                message_body: [i as u8].repeat(100),
            };

            execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
        }

        // Sign update with wrong updater
        let res = query(deps.as_ref(), mock_env(), QueryMsg::SuggestUpdate {}).unwrap();
        let SuggestUpdateResponse {
            committed_root,
            new_root,
        } = from_binary::<SuggestUpdateResponse>(&res).unwrap();
        let update = not_updater
            .sign_update(committed_root, new_root)
            .await
            .unwrap();

        // Submit update and ensure error
        let info = mock_info("submitter", &coins(100, "earth"));
        let msg = ExecuteMsg::Update {
            committed_root,
            new_root,
            signature: update.signature.to_vec(),
        };
        let res = execute(deps.as_mut(), mock_env(), info, msg);
        assert!(res.is_err());

        // Ensure no state changes
        let res = query(deps.as_ref(), mock_env(), QueryMsg::SuggestUpdate {}).unwrap();
        let SuggestUpdateResponse {
            committed_root: check_committed_root,
            new_root: check_new_root,
        } = from_binary::<SuggestUpdateResponse>(&res).unwrap();
        assert_eq!(committed_root, check_committed_root);
        assert_eq!(new_root, check_new_root);
    }

    #[tokio::test]
    async fn failed_on_valid_double_update() {
        let updater: Updater = Updater::from_privkey(UPDATER_PRIVKEY, LOCAL_DOMAIN);

        let mut deps = mock_dependencies_with_balance(&coins(100, "token"));

        let init_msg = InstantiateMsg {
            local_domain: LOCAL_DOMAIN,
            updater: UPDATER_PUBKEY.to_owned(),
        };
        let info = mock_info("owner", &coins(100, "earth"));

        let res = instantiate(deps.as_mut(), mock_env(), info.clone(), init_msg).unwrap();
        assert_eq!(0, res.messages.len());

        // Dispatch several messages
        let info = mock_info("dispatcher", &coins(100, "earth"));
        for i in 1..3 {
            let msg = ExecuteMsg::Dispatch {
                destination: i * 1000,
                recipient: "recipient".to_owned(),
                message_body: [i as u8].repeat(100),
            };

            execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
        }

        // Sign double update
        let res = query(deps.as_ref(), mock_env(), QueryMsg::SuggestUpdate {}).unwrap();
        let SuggestUpdateResponse {
            committed_root,
            new_root,
        } = from_binary::<SuggestUpdateResponse>(&res).unwrap();

        let bad_root = H256::repeat_byte(1);
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

        // Check home failed
        let res = query(deps.as_ref(), mock_env(), QueryMsg::State {}).unwrap();
        let state = from_binary::<StateResponse>(&res).unwrap().state;
        assert_eq!(States::Failed, state);
    }

    #[test]
    fn only_owner_sets_updater_manager() {
        let mut deps = mock_dependencies_with_balance(&coins(100, "token"));

        let init_msg = InstantiateMsg {
            local_domain: LOCAL_DOMAIN,
            updater: UPDATER_PUBKEY.to_owned(),
        };
        let info = mock_info("owner", &coins(100, "earth"));

        let res = instantiate(deps.as_mut(), mock_env(), info.clone(), init_msg).unwrap();
        assert_eq!(0, res.messages.len());

        let msg = ExecuteMsg::SetUpdaterManager {
            updater_manager: "new_updater_manager".to_owned(),
        };

        // Try setting updater as non-owner
        let non_owner_info = mock_info("non_owner", &coins(100, "earth"));
        let fail_res = execute(deps.as_mut(), mock_env(), non_owner_info, msg.clone());
        assert!(fail_res.is_err());

        // Set updater as owner
        let _success_res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // Check updater manager address different
        let res = query(deps.as_ref(), mock_env(), QueryMsg::UpdaterManager {}).unwrap();
        let updater_manager = from_binary::<UpdaterManagerResponse>(&res)
            .unwrap()
            .updater_manager;
        assert_eq!("new_updater_manager".to_owned(), updater_manager);
    }
}
