use common::{home, nomad_base, updater_manager};
use cosmwasm_std::{from_binary, Addr};
use cw_multi_test::{App, AppBuilder, BankKeeper, ContractWrapper, Executor};
use ethers_core::types::H256;
use test_utils::Updater;

use crate::utils::{
    app_event_by_ty, instantiate_home, instantiate_updater_manager, mock_app,
    store_updater_manager_code,
};

const LOCAL_DOMAIN: u32 = 1000;
const UPDATER_PRIVKEY: &str = "1111111111111111111111111111111111111111111111111111111111111111";
const UPDATER_PUBKEY: &str = "0x19e7e376e7c213b7e7e7e46cc70a5dd086daff2a";

#[tokio::test]
async fn home_calls_updater_manager_slash_updater() {
    let mut app = mock_app();

    let updater: Updater = Updater::from_privkey(UPDATER_PRIVKEY, LOCAL_DOMAIN);

    let owner = Addr::unchecked("owner");
    let updater_addr = Addr::unchecked(UPDATER_PUBKEY);

    // Instantiate updater manager
    let updater_manager_addr =
        instantiate_updater_manager(&mut app, owner.clone(), updater_addr.clone());

    // Instantiate home
    let home_addr = instantiate_home(&mut app, owner.clone(), LOCAL_DOMAIN, updater_addr.clone());

    // Set updater manager on home to be updater_manager
    let set_updater_manager_msg = common::home::ExecuteMsg::SetUpdaterManager {
        updater_manager: updater_manager_addr.to_string(),
    };
    let res = app.execute_contract(
        owner.clone(),
        home_addr.clone(),
        &set_updater_manager_msg,
        &[],
    );

    // Set home on updater manager to be home
    let set_home_msg = updater_manager::ExecuteMsg::SetHome {
        home: home_addr.to_string(),
    };
    let res = app.execute_contract(
        owner.clone(),
        updater_manager_addr.clone(),
        &set_home_msg,
        &[],
    );

    // Sign improper update (queue empty so anything is improper)
    let suggested: home::SuggestUpdateResponse = app
        .wrap()
        .query_wasm_smart(home_addr.clone(), &home::QueryMsg::SuggestUpdate {})
        .unwrap();

    let improper_root = H256::repeat_byte(1);
    let update = updater
        .sign_update(suggested.committed_root, improper_root)
        .await
        .unwrap();

    // Execute improper update
    let update_msg = home::ExecuteMsg::ImproperUpdate {
        old_root: suggested.committed_root,
        new_root: improper_root,
        signature: update.signature.to_vec(),
    };
    let res = app
        .execute_contract(updater_addr.clone(), home_addr.clone(), &update_msg, &[])
        .unwrap();
    println!("Improper Update: {:?}", res);

    assert!(app_event_by_ty(&res, "wasm-SlashUpdater").is_some())
}
