#[cfg(test)]
mod test {
    use common::{home, nomad_base, updater_manager};
    use cosmwasm_std::Addr;
    use cw_multi_test::Executor;
    use ethers_core::types::H160;
    use test_utils::Updater;

    use crate::utils::helpers::{instantiate_home, instantiate_updater_manager, mock_app};

    const LOCAL_DOMAIN: u32 = 1000;
    const UPDATER_PRIVKEY: &str =
        "1111111111111111111111111111111111111111111111111111111111111111";

    #[test]
    fn updater_manager_calls_home_set_updater() {
        let mut app = mock_app();

        let updater: Updater = Updater::from_privkey(UPDATER_PRIVKEY, LOCAL_DOMAIN);

        let local_domain = 1000;
        let owner = Addr::unchecked("owner");

        // Instantiate updater manager
        let updater_manager_addr =
            instantiate_updater_manager(&mut app, owner.clone(), updater.address());

        // Instantiate home
        let home_addr = instantiate_home(&mut app, owner.clone(), local_domain, updater.address());

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

        // Execute updater_manager::set_updater
        let new_updater = H160::repeat_byte(5);
        let set_updater_msg = updater_manager::ExecuteMsg::SetUpdater {
            updater: new_updater,
        };
        let res = app
            .execute_contract(owner, updater_manager_addr.clone(), &set_updater_msg, &[])
            .unwrap();
        println!("Set Updater: {:?}", res);

        // Check updater manager updater is new_updater
        let updater_manager_updater_res: updater_manager::UpdaterResponse = app
            .wrap()
            .query_wasm_smart(updater_manager_addr, &updater_manager::QueryMsg::Updater {})
            .unwrap();
        let updater_manager_updater = updater_manager_updater_res.updater;
        assert_eq!(new_updater, updater_manager_updater);

        // Check home updater is now new_updater
        let home_updater_res: nomad_base::UpdaterResponse = app
            .wrap()
            .query_wasm_smart(home_addr, &home::QueryMsg::Updater {})
            .unwrap();
        let home_updater = home_updater_res.updater;
        assert_eq!(new_updater, home_updater);
    }
}
