#[cfg(test)]
mod test {
    use common::{home, updater_manager};
    use cosmwasm_std::Addr;
    use cw_multi_test::Executor;
    use ethers_core::types::H256;
    use test_utils::Updater;

    use crate::utils::helpers::{
        app_event_by_ty, instantiate_home, instantiate_updater_manager, mock_app,
    };

    const LOCAL_DOMAIN: u32 = 1000;
    const UPDATER_PRIVKEY: &str =
        "1111111111111111111111111111111111111111111111111111111111111111";

    #[tokio::test]
    async fn home_calls_updater_manager_slash_updater() {
        let mut app = mock_app();

        let updater: Updater = Updater::from_privkey(UPDATER_PRIVKEY, LOCAL_DOMAIN);

        let owner = Addr::unchecked("owner");

        // Instantiate updater manager
        let updater_manager_addr =
            instantiate_updater_manager(&mut app, owner.clone(), updater.address());

        // Instantiate home
        let home_addr = instantiate_home(&mut app, owner.clone(), LOCAL_DOMAIN, updater.address());

        // Set updater manager on home to be updater_manager
        let set_updater_manager_msg = common::home::ExecuteMsg::SetUpdaterManager {
            updater_manager: updater_manager_addr.to_string(),
        };
        app.execute_contract(
            owner.clone(),
            home_addr.clone(),
            &set_updater_manager_msg,
            &[],
        )
        .unwrap();

        // Set home on updater manager to be home
        let set_home_msg = updater_manager::ExecuteMsg::SetHome {
            home: home_addr.to_string(),
        };
        app.execute_contract(
            owner.clone(),
            updater_manager_addr.clone(),
            &set_home_msg,
            &[],
        )
        .unwrap();

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
            .execute_contract(owner, home_addr.clone(), &update_msg, &[])
            .unwrap();
        println!("Improper Update: {:?}", res);

        assert!(app_event_by_ty(&res, "wasm-SlashUpdater").is_some())
    }
}
