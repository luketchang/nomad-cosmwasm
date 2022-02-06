use cosmwasm_std::testing::{
    mock_dependencies_with_balance, mock_env, mock_info, MockApi, MockStorage,
};
use cosmwasm_std::{coins, from_binary, Addr, Event};
use cosmwasm_std::{Api, Storage};
use cw_multi_test::{App, AppBuilder, AppResponse, BankKeeper, ContractWrapper, Executor};
use ethers_core::types::H256;

pub(crate) fn mock_app() -> App {
    let env = mock_env();
    let api = MockApi::default();
    let bank = BankKeeper::new();
    let storage = MockStorage::new();

    AppBuilder::new()
        .with_api(api)
        .with_block(env.block)
        .with_bank(bank)
        .with_storage(storage)
        .build(|_, _, _| {})
}

pub(crate) fn instantiate_home(
    app: &mut App,
    owner: Addr,
    local_domain: u32,
    updater: Addr,
) -> Addr {
    let code_id = store_home_code(app);

    let init_msg = common::home::InstantiateMsg {
        local_domain,
        updater: updater.to_string(),
    };

    app.instantiate_contract(code_id, owner, &init_msg, &[], String::from("HOME"), None)
        .unwrap()
}

pub(crate) fn instantiate_test_replica(
    app: &mut App,
    owner: Addr,
    chain_addr_length: usize,
    local_domain: u32,
    remote_domain: u32,
    updater: Addr,
    committed_root: H256,
    optimistic_seconds: u64,
) -> Addr {
    let code_id = store_test_replica_code(app);
    let init_msg = common::replica::InstantiateMsg {
        chain_addr_length,
        local_domain,
        remote_domain,
        updater: updater.to_string(),
        committed_root,
        optimistic_seconds,
    };

    app.instantiate_contract(
        code_id,
        owner,
        &init_msg,
        &[],
        String::from("test_replica"),
        None,
    )
    .unwrap()
}

pub(crate) fn instantiate_updater_manager(app: &mut App, owner: Addr, updater: Addr) -> Addr {
    let code_id = store_updater_manager_code(app);
    let init_msg = common::updater_manager::InstantiateMsg {
        updater: updater.to_string(),
    };

    app.instantiate_contract(
        code_id,
        owner,
        &init_msg,
        &[],
        String::from("UPDATER_MANAGER"),
        None,
    )
    .unwrap()
}

pub(crate) fn instantiate_test_recipient(app: &mut App, deployer: Addr) -> Addr {
    let code_id = store_test_recipient_code(app);
    let init_msg = common::test::test_recipient::InstantiateMsg {};

    app.instantiate_contract(
        code_id,
        deployer,
        &init_msg,
        &[],
        String::from("RECIPIENT"),
        None,
    )
    .unwrap()
}

pub(crate) fn instantiate_bad_recipient(app: &mut App, deployer: Addr) -> Addr {
    let code_id = store_bad_recipient_code(app);
    let init_msg = common::test::test_recipient::InstantiateMsg {};

    app.instantiate_contract(
        code_id,
        deployer,
        &init_msg,
        &[],
        String::from("BAD_RECIPIENT"),
        None,
    )
    .unwrap()
}

pub(crate) fn store_home_code(app: &mut App) -> u64 {
    let home_contract = Box::new(
        ContractWrapper::new_with_empty(
            home::contract::execute,
            home::contract::instantiate,
            home::contract::query,
        )
        .with_reply(home::contract::reply),
    );

    app.store_code(home_contract)
}

pub(crate) fn store_test_replica_code(app: &mut App) -> u64 {
    let test_replica_contract = Box::new(
        ContractWrapper::new_with_empty(
            test_replica::contract::execute,
            test_replica::contract::instantiate,
            test_replica::contract::query,
        )
        .with_reply(test_replica::contract::reply),
    );

    app.store_code(test_replica_contract)
}

pub(crate) fn store_updater_manager_code(app: &mut App) -> u64 {
    let updater_manager_contract = Box::new(
        ContractWrapper::new_with_empty(
            updater_manager::contract::execute,
            updater_manager::contract::instantiate,
            updater_manager::contract::query,
        )
        .with_reply(updater_manager::contract::reply),
    );

    app.store_code(updater_manager_contract)
}

pub(crate) fn store_test_recipient_code(app: &mut App) -> u64 {
    let test_recipient_contract = Box::new(ContractWrapper::new_with_empty(
        test_recipient::contract::execute,
        test_recipient::contract::instantiate,
        test_recipient::contract::query,
    ));

    app.store_code(test_recipient_contract)
}

pub(crate) fn store_bad_recipient_code(app: &mut App) -> u64 {
    let bad_recipient_contract = Box::new(ContractWrapper::new_with_empty(
        bad_recipient::contract::execute,
        bad_recipient::contract::instantiate,
        bad_recipient::contract::query,
    ));

    app.store_code(bad_recipient_contract)
}

pub fn app_event_by_ty(res: &AppResponse, ty: &str) -> Option<Event> {
    res.events
        .iter()
        .find(|event| event.ty == ty)
        .map(|event| event.to_owned())
}
