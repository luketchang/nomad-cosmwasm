// TODO: local domain query, permissionless unenroll

#[cfg(test)]
mod test {
    use common::connection_manager;
    use cosmwasm_std::Addr;
    use cw_multi_test::Executor;
    use ethers_core::types::{H160, H256};
    use test_utils::Watcher;

    use crate::utils::helpers::{
        app_event_by_ty, instantiate_connection_manager, instantiate_home,
        instantiate_test_replica, mock_app,
    };

    const CHAIN_ADDR_LENGTH_BYTES: usize = 11; // e.g. "Contract #0".len()
    const LOCAL_DOMAIN: u32 = 1000;
    const REMOTE_DOMAIN: u32 = 2000;
    const WATCHER_PRIVKEY: &str =
        "2111111111111111111111111111111111111111111111111111111111111111";

    #[test]
    fn retrieves_local_domain_from_home() {
        let mut app = mock_app();

        let updater = H160::repeat_byte(1);
        let owner = Addr::unchecked("owner");

        // Instantiate connection manager
        let connection_manager_addr =
            instantiate_connection_manager(&mut app, owner.clone(), CHAIN_ADDR_LENGTH_BYTES);

        // Instantiate home
        let home_addr = instantiate_home(&mut app, owner.clone(), LOCAL_DOMAIN, updater);

        // Set connection manager on home to be home
        let set_home_msg = common::connection_manager::ExecuteMsg::SetHome {
            home: home_addr.to_string(),
        };
        app.execute_contract(
            owner.clone(),
            connection_manager_addr.clone(),
            &set_home_msg,
            &[],
        )
        .unwrap();

        // Check home updater is now new_updater
        let local_domain_res: connection_manager::LocalDomainResponse = app
            .wrap()
            .query_wasm_smart(home_addr, &connection_manager::QueryMsg::LocalDomain {})
            .unwrap();
        let local_domain = local_domain_res.local_domain;
        assert_eq!(LOCAL_DOMAIN, local_domain);
    }

    #[tokio::test]
    async fn unenrolls_replica_on_valid_signed_failure() {
        let mut app = mock_app();

        let watcher = Watcher::from_privkey(WATCHER_PRIVKEY, REMOTE_DOMAIN);

        let updater = H160::repeat_byte(1);
        let owner = Addr::unchecked("owner");

        // Instantiate connection manager
        let connection_manager_addr =
            instantiate_connection_manager(&mut app, owner.clone(), CHAIN_ADDR_LENGTH_BYTES);

        // Instantiate replica to enroll
        let replica_addr = instantiate_test_replica(
            &mut app,
            owner.clone(),
            CHAIN_ADDR_LENGTH_BYTES,
            LOCAL_DOMAIN,
            REMOTE_DOMAIN,
            updater,
            H256::zero(),
            100,
        );

        // Owner enroll replica
        let enroll_replica_msg = common::connection_manager::ExecuteMsg::OwnerEnrollReplica {
            domain: REMOTE_DOMAIN,
            replica: replica_addr.to_string(),
        };
        app.execute_contract(
            owner.clone(),
            connection_manager_addr.clone(),
            &enroll_replica_msg,
            &[],
        )
        .unwrap();

        // Check replica enrolled
        let is_replica_res: connection_manager::IsReplicaResponse = app
            .wrap()
            .query_wasm_smart(
                connection_manager_addr.clone(),
                &connection_manager::QueryMsg::IsReplica {
                    replica: replica_addr.to_string(),
                },
            )
            .unwrap();
        let is_replica = is_replica_res.is_replica;
        assert!(is_replica);

        // Set watcher permissions for replica
        let set_permission_msg = common::connection_manager::ExecuteMsg::SetWatcherPermission {
            watcher: watcher.address(),
            domain: REMOTE_DOMAIN,
            access: true,
        };
        app.execute_contract(
            owner.clone(),
            connection_manager_addr.clone(),
            &set_permission_msg,
            &[],
        )
        .unwrap();

        // Sign failure notification
        let signed_failure = watcher
            .sign_failure_notification(H256::from(updater))
            .await
            .unwrap();

        // Permissionlessly unenroll replica
        let unenroll_replica_msg = common::connection_manager::ExecuteMsg::UnenrollReplica {
            domain: REMOTE_DOMAIN,
            updater: H256::from(updater),
            signature: signed_failure.signature.to_vec(),
        };
        let res = app
            .execute_contract(
                owner.clone(),
                connection_manager_addr.clone(),
                &unenroll_replica_msg,
                &[],
            )
            .unwrap();
        println!("\nUnenroll replica success: {:?}", res);

        // Check event
        assert!(app_event_by_ty(&res, "wasm-ReplicaUnenrolled").is_some());

        // Check replica unenrolled
        let is_replica_res: connection_manager::IsReplicaResponse = app
            .wrap()
            .query_wasm_smart(
                connection_manager_addr,
                &connection_manager::QueryMsg::IsReplica {
                    replica: replica_addr.to_string(),
                },
            )
            .unwrap();
        let is_replica = is_replica_res.is_replica;
        assert!(!is_replica);
    }

    #[tokio::test]
    async fn unenroll_replica_reverts_if_no_such_replica() {
        let mut app = mock_app();

        let mut watcher = Watcher::from_privkey(WATCHER_PRIVKEY, REMOTE_DOMAIN);

        let updater = H160::repeat_byte(1);
        let owner = Addr::unchecked("owner");

        // Instantiate connection manager
        let connection_manager_addr =
            instantiate_connection_manager(&mut app, owner.clone(), CHAIN_ADDR_LENGTH_BYTES);

        // Instantiate replica to enroll
        let replica_addr = instantiate_test_replica(
            &mut app,
            owner.clone(),
            CHAIN_ADDR_LENGTH_BYTES,
            LOCAL_DOMAIN,
            REMOTE_DOMAIN,
            updater,
            H256::zero(),
            100,
        );

        // Owner enroll replica
        let enroll_replica_msg = common::connection_manager::ExecuteMsg::OwnerEnrollReplica {
            domain: REMOTE_DOMAIN,
            replica: replica_addr.to_string(),
        };
        app.execute_contract(
            owner.clone(),
            connection_manager_addr.clone(),
            &enroll_replica_msg,
            &[],
        )
        .unwrap();

        // Check replica enrolled
        let is_replica_res: connection_manager::IsReplicaResponse = app
            .wrap()
            .query_wasm_smart(
                connection_manager_addr.clone(),
                &connection_manager::QueryMsg::IsReplica {
                    replica: replica_addr.to_string(),
                },
            )
            .unwrap();
        let is_replica = is_replica_res.is_replica;
        assert!(is_replica);

        // Set watcher permissions for replica
        let set_permission_msg = common::connection_manager::ExecuteMsg::SetWatcherPermission {
            watcher: watcher.address(),
            domain: REMOTE_DOMAIN,
            access: true,
        };
        app.execute_contract(
            owner.clone(),
            connection_manager_addr.clone(),
            &set_permission_msg,
            &[],
        )
        .unwrap();

        // Change watcher home domain so that it signs notification
        // saying unenroll replica for chain 9999
        watcher.home_domain = 9999;

        // Sign invalid notification
        let signed_failure = watcher
            .sign_failure_notification(H256::from(updater))
            .await
            .unwrap();

        // Expect unenroll call to fail
        let unenroll_replica_msg = common::connection_manager::ExecuteMsg::UnenrollReplica {
            domain: 9999,
            updater: H256::from(updater),
            signature: signed_failure.signature.to_vec(),
        };
        let res = app.execute_contract(
            owner.clone(),
            connection_manager_addr.clone(),
            &unenroll_replica_msg,
            &[],
        );
        println!("\nUnenroll replica failure (no replica): {:?}", res);

        // Assert call returned error and no handle event emitted
        assert!(res.is_err());
        assert!(res.err().unwrap().to_string().contains("does not exist"));
    }

    #[tokio::test]
    async fn unenroll_replica_reverts_if_not_watcher_sig() {
        let mut app = mock_app();

        let watcher = Watcher::from_privkey(WATCHER_PRIVKEY, REMOTE_DOMAIN);

        let fake_watcher_privkey: &str =
            "3111111111111111111111111111111111111111111111111111111111111111";
        let not_watcher = Watcher::from_privkey(fake_watcher_privkey, LOCAL_DOMAIN);

        let updater = H160::repeat_byte(1);
        let owner = Addr::unchecked("owner");

        // Instantiate connection manager
        let connection_manager_addr =
            instantiate_connection_manager(&mut app, owner.clone(), CHAIN_ADDR_LENGTH_BYTES);

        // Instantiate replica to enroll
        let replica_addr = instantiate_test_replica(
            &mut app,
            owner.clone(),
            CHAIN_ADDR_LENGTH_BYTES,
            LOCAL_DOMAIN,
            REMOTE_DOMAIN,
            updater,
            H256::zero(),
            100,
        );

        // Owner enroll replica
        let enroll_replica_msg = common::connection_manager::ExecuteMsg::OwnerEnrollReplica {
            domain: REMOTE_DOMAIN,
            replica: replica_addr.to_string(),
        };
        app.execute_contract(
            owner.clone(),
            connection_manager_addr.clone(),
            &enroll_replica_msg,
            &[],
        )
        .unwrap();

        // Check replica enrolled
        let is_replica_res: connection_manager::IsReplicaResponse = app
            .wrap()
            .query_wasm_smart(
                connection_manager_addr.clone(),
                &connection_manager::QueryMsg::IsReplica {
                    replica: replica_addr.to_string(),
                },
            )
            .unwrap();
        let is_replica = is_replica_res.is_replica;
        assert!(is_replica);

        // Set watcher permissions for replica
        let set_permission_msg = common::connection_manager::ExecuteMsg::SetWatcherPermission {
            watcher: watcher.address(),
            domain: REMOTE_DOMAIN,
            access: true,
        };
        app.execute_contract(
            owner.clone(),
            connection_manager_addr.clone(),
            &set_permission_msg,
            &[],
        )
        .unwrap();

        // Sign invalid notification
        let signed_failure = not_watcher
            .sign_failure_notification(H256::from(updater))
            .await
            .unwrap();

        // Expect unenroll call to fail
        let unenroll_replica_msg = common::connection_manager::ExecuteMsg::UnenrollReplica {
            domain: REMOTE_DOMAIN,
            updater: H256::from(updater),
            signature: signed_failure.signature.to_vec(),
        };
        let res = app.execute_contract(
            owner.clone(),
            connection_manager_addr.clone(),
            &unenroll_replica_msg,
            &[],
        );
        println!("\nUnenroll replica failure (not watcher): {:?}", res);

        // Assert call returned error
        assert!(res.is_err());
        assert!(res
            .err()
            .unwrap()
            .to_string()
            .contains("does not have permissions for replica"));
    }

    #[tokio::test]
    async fn unenroll_replica_reverts_if_updater_not_match_replica() {
        let mut app = mock_app();

        let watcher = Watcher::from_privkey(WATCHER_PRIVKEY, REMOTE_DOMAIN);

        let fake_updater = H160::repeat_byte(2);
        let updater = H160::repeat_byte(1);
        let owner = Addr::unchecked("owner");

        // Instantiate connection manager
        let connection_manager_addr =
            instantiate_connection_manager(&mut app, owner.clone(), CHAIN_ADDR_LENGTH_BYTES);

        // Instantiate replica to enroll
        let replica_addr = instantiate_test_replica(
            &mut app,
            owner.clone(),
            CHAIN_ADDR_LENGTH_BYTES,
            LOCAL_DOMAIN,
            REMOTE_DOMAIN,
            updater,
            H256::zero(),
            100,
        );

        // Owner enroll replica
        let enroll_replica_msg = common::connection_manager::ExecuteMsg::OwnerEnrollReplica {
            domain: REMOTE_DOMAIN,
            replica: replica_addr.to_string(),
        };
        app.execute_contract(
            owner.clone(),
            connection_manager_addr.clone(),
            &enroll_replica_msg,
            &[],
        )
        .unwrap();

        // Check replica enrolled
        let is_replica_res: connection_manager::IsReplicaResponse = app
            .wrap()
            .query_wasm_smart(
                connection_manager_addr.clone(),
                &connection_manager::QueryMsg::IsReplica {
                    replica: replica_addr.to_string(),
                },
            )
            .unwrap();
        let is_replica = is_replica_res.is_replica;
        assert!(is_replica);

        // Set watcher permissions for replica
        let set_permission_msg = common::connection_manager::ExecuteMsg::SetWatcherPermission {
            watcher: watcher.address(),
            domain: REMOTE_DOMAIN,
            access: true,
        };
        app.execute_contract(
            owner.clone(),
            connection_manager_addr.clone(),
            &set_permission_msg,
            &[],
        )
        .unwrap();

        // Sign invalid notification
        let signed_failure = watcher
            .sign_failure_notification(H256::from(updater))
            .await
            .unwrap();

        // Expect unenroll call to fail
        let unenroll_replica_msg = common::connection_manager::ExecuteMsg::UnenrollReplica {
            domain: REMOTE_DOMAIN,
            updater: H256::from(fake_updater),
            signature: signed_failure.signature.to_vec(),
        };
        let res = app.execute_contract(
            owner.clone(),
            connection_manager_addr.clone(),
            &unenroll_replica_msg,
            &[],
        );
        println!("\nUnenroll replica failure (not updater): {:?}", res);

        // Assert call returned error
        assert!(res.is_err());
        assert!(res
            .err()
            .unwrap()
            .to_string()
            .contains("Not current updater"));
    }
}
