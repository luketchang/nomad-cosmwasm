use std::ops::Add;

use common::updater_manager;
use cosmwasm_std::Addr;
use cw_multi_test::{App, AppBuilder, BankKeeper, ContractWrapper, Executor};

use crate::utils::{
    instantiate_home, instantiate_updater_manager, mock_app, store_updater_manager_code,
};

#[test]
fn updater_manager_calls_home_set_updater() {
    let mut app = mock_app();

    let owner = Addr::unchecked("owner");
    let updater = Addr::unchecked("updater");
    let updater_manager_addr =
        instantiate_updater_manager(&mut app, owner.clone(), updater.clone());

    let local_domain = 1000;
    let home_addr = instantiate_home(&mut app, owner.clone(), local_domain, updater.clone());

    let set_updater_manager_msg = common::home::ExecuteMsg::SetUpdaterManager {
        updater_manager: updater_manager_addr.to_string(),
    };
    let res = app.execute_contract(
        owner.clone(),
        home_addr.clone(),
        &set_updater_manager_msg,
        &[],
    );
    println!("set updater manager: {:?}", res);

    let set_home_msg = updater_manager::ExecuteMsg::SetHome {
        home: home_addr.to_string(),
    };
    let res = app.execute_contract(
        owner.clone(),
        updater_manager_addr.clone(),
        &set_home_msg,
        &[],
    );
    println!("set home: {:?}", res);

    let new_updater = Addr::unchecked("new_updater");
    let set_updater_msg = updater_manager::ExecuteMsg::SetUpdater {
        updater: new_updater.to_string(),
    };

    let res = app.execute_contract(owner, updater_manager_addr, &set_updater_msg, &[]);
    println!("set updater (new): {:?}", res);
}
