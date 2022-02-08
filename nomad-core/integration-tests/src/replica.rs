#[cfg(test)]
mod test {
    use common::{
        addr_to_h256, h256_to_string, replica, test::test_replica, Encode, MessageStatus,
        NomadMessage,
    };
    use cosmwasm_std::{from_binary, Addr};
    use cw_multi_test::Executor;
    use ethers_core::types::H256;
    use merkle::merkle_tree::{merkle_root_from_branch, Proof};

    use crate::utils::helpers::{
        app_event_by_ty, instantiate_bad_recipient, instantiate_test_recipient,
        instantiate_test_replica, mock_app,
    };

    const CHAIN_ADDR_LENGTH: usize = 11; // e.g. "Contract #0".len()
    const REMOTE_DOMAIN: u32 = 1000;
    const LOCAL_DOMAIN: u32 = 2000;
    const UPDATER_PUBKEY: &str = "0x19e7e376e7c213b7e7e7e46cc70a5dd086daff2a";

    #[test]
    fn proves_message() {
        let mut app = mock_app();

        let sender_string = h256_to_string(H256::zero());

        let owner = Addr::unchecked("owner");
        let sender = Addr::unchecked(&sender_string);
        let updater_addr = Addr::unchecked(UPDATER_PUBKEY);
        let committed_root = H256::zero();
        let optimistic_seconds = 100;

        // Instantiate replica
        let replica_addr = instantiate_test_replica(
            &mut app,
            owner.clone(),
            CHAIN_ADDR_LENGTH,
            LOCAL_DOMAIN,
            REMOTE_DOMAIN,
            updater_addr,
            committed_root,
            optimistic_seconds,
        );

        // Load merkle proof
        let merkle_test_cases = test_utils::load_merkle_test_json();
        let test_case = &merkle_test_cases[0];
        let Proof { leaf, index, path } = test_case.proofs[0];

        // Set committed root to match test case
        let set_committed_msg = test_replica::ExecuteMsg::SetCommittedRoot {
            root: test_case.expected_root,
        };
        app.execute_contract(
            sender.clone(),
            replica_addr.clone(),
            &set_committed_msg,
            &[],
        )
        .unwrap();

        // Prove leaf under committed root
        let msg = test_replica::ExecuteMsg::ReplicaExecuteMsg(replica::ExecuteMsg::Prove {
            leaf,
            proof: path,
            index: index.try_into().unwrap(),
        });

        let res = app
            .execute_contract(sender.clone(), replica_addr, &msg, &[])
            .unwrap();
        println!("\nProve: {:?}", res);

        let success = from_binary::<bool>(&res.data.unwrap()).unwrap();
        assert!(success);
    }

    #[test]
    fn rejects_invalid_message_proof() {
        let mut app = mock_app();

        let sender_string = h256_to_string(H256::zero());

        let owner = Addr::unchecked("owner");
        let sender = Addr::unchecked(&sender_string);
        let updater_addr = Addr::unchecked(UPDATER_PUBKEY);
        let committed_root = H256::zero();
        let optimistic_seconds = 100;

        // Instantiate replica
        let replica_addr = instantiate_test_replica(
            &mut app,
            owner.clone(),
            CHAIN_ADDR_LENGTH,
            LOCAL_DOMAIN,
            REMOTE_DOMAIN,
            updater_addr,
            committed_root,
            optimistic_seconds,
        );

        // Load merkle proof but DO NOT set committed root to match expected
        let merkle_test_cases = test_utils::load_merkle_test_json();
        let test_case = &merkle_test_cases[0];
        let Proof { leaf, index, path } = test_case.proofs[0];

        // Try to prove leaf under committed root
        let msg = test_replica::ExecuteMsg::ReplicaExecuteMsg(replica::ExecuteMsg::Prove {
            leaf,
            proof: path,
            index: index.try_into().unwrap(),
        });

        let res = app
            .execute_contract(sender.clone(), replica_addr.clone(), &msg, &[])
            .unwrap();
        println!("\nProve error (invalid proof): {:?}", res);

        // Expect call to complete but return false
        let success = from_binary::<bool>(&res.data.as_ref().unwrap()).unwrap();
        assert!(!success);

        // Query message status to check status == MessageStatus::None
        let query_msg = replica::QueryMsg::MessageStatus { leaf };

        let message_status_res: replica::MessageStatusResponse = app
            .wrap()
            .query_wasm_smart(replica_addr, &query_msg)
            .unwrap();
        let message_status = message_status_res.status;
        assert_eq!(MessageStatus::None, message_status);
    }

    #[test]
    fn processes_proved_message() {
        let mut app = mock_app();

        let sender_string = h256_to_string(H256::zero());

        let owner = Addr::unchecked("owner");
        let sender = Addr::unchecked(&sender_string);
        let updater_addr = Addr::unchecked(UPDATER_PUBKEY);
        let committed_root = H256::zero();
        let optimistic_seconds = 100;

        // Instantiate replica
        let replica_addr = instantiate_test_replica(
            &mut app,
            owner.clone(),
            CHAIN_ADDR_LENGTH,
            LOCAL_DOMAIN,
            REMOTE_DOMAIN,
            updater_addr,
            committed_root,
            optimistic_seconds,
        );

        // Instantiate recipient
        let recipient_addr = instantiate_test_recipient(&mut app, owner.clone());
        let recipient_addr_h256 = addr_to_h256(recipient_addr.clone());

        let nomad_message = NomadMessage {
            origin: REMOTE_DOMAIN,
            sender: H256::zero(),
            nonce: 0,
            destination: LOCAL_DOMAIN,
            recipient: recipient_addr_h256,
            body: "0x".as_bytes().to_vec(),
        };

        // Prove message on test replica
        let prove_msg = test_replica::ExecuteMsg::SetProven {
            leaf: nomad_message.to_leaf(),
        };
        app.execute_contract(sender.clone(), replica_addr.clone(), &prove_msg, &[])
            .unwrap();

        let msg = test_replica::ExecuteMsg::ReplicaExecuteMsg(replica::ExecuteMsg::Process {
            message: nomad_message.to_vec(),
        });
        let res = app
            .execute_contract(sender.clone(), replica_addr.clone(), &msg, &[])
            .unwrap();
        println!("\nProcess: {:?}", res);

        let success = from_binary::<bool>(&res.data.as_ref().unwrap()).unwrap();
        assert!(success);

        assert!(app_event_by_ty(&res, "wasm-Handle").is_some())
    }

    #[test]
    fn fails_to_process_unproved_message() {
        let mut app = mock_app();

        let sender_string = h256_to_string(H256::zero());

        let owner = Addr::unchecked("owner");
        let sender = Addr::unchecked(&sender_string);
        let updater_addr = Addr::unchecked(UPDATER_PUBKEY);
        let committed_root = H256::zero();
        let optimistic_seconds = 100;

        // Instantiate replica
        let replica_addr = instantiate_test_replica(
            &mut app,
            owner.clone(),
            CHAIN_ADDR_LENGTH,
            LOCAL_DOMAIN,
            REMOTE_DOMAIN,
            updater_addr,
            committed_root,
            optimistic_seconds,
        );

        // Instantiate recipient
        let recipient_addr = instantiate_test_recipient(&mut app, owner.clone());
        let recipient_addr_h256 = addr_to_h256(recipient_addr.clone());

        let nomad_message = NomadMessage {
            origin: REMOTE_DOMAIN,
            sender: H256::zero(),
            nonce: 0,
            destination: LOCAL_DOMAIN,
            recipient: recipient_addr_h256,
            body: "0x".as_bytes().to_vec(),
        };

        // Execute process message without prove
        let msg = test_replica::ExecuteMsg::ReplicaExecuteMsg(replica::ExecuteMsg::Process {
            message: nomad_message.to_vec(),
        });
        let res = app.execute_contract(sender.clone(), replica_addr.clone(), &msg, &[]);
        println!("\nProcess error (not proven): {:?}", res);

        // Assert call returned error and no handle event emitted
        assert!(res.is_err());
        assert!(res.err().unwrap().to_string().contains("not yet proven"));
    }

    #[test]
    fn fails_to_process_message_to_wrong_destination() {
        let mut app = mock_app();

        let sender_string = h256_to_string(H256::zero());

        let owner = Addr::unchecked("owner");
        let sender = Addr::unchecked(&sender_string);
        let updater_addr = Addr::unchecked(UPDATER_PUBKEY);
        let committed_root = H256::zero();
        let optimistic_seconds = 100;

        // Instantiate replica
        let replica_addr = instantiate_test_replica(
            &mut app,
            owner.clone(),
            CHAIN_ADDR_LENGTH,
            LOCAL_DOMAIN,
            REMOTE_DOMAIN,
            updater_addr,
            committed_root,
            optimistic_seconds,
        );

        // Instantiate recipient
        let recipient_addr = instantiate_test_recipient(&mut app, owner.clone());
        let recipient_addr_h256 = addr_to_h256(recipient_addr.clone());

        let nomad_message = NomadMessage {
            origin: REMOTE_DOMAIN,
            sender: H256::zero(),
            nonce: 0,
            destination: 3000, // wrong destination
            recipient: recipient_addr_h256,
            body: "0x".as_bytes().to_vec(),
        };

        // Execute process message without prove
        let msg = test_replica::ExecuteMsg::ReplicaExecuteMsg(replica::ExecuteMsg::Process {
            message: nomad_message.to_vec(),
        });
        let res = app.execute_contract(sender.clone(), replica_addr.clone(), &msg, &[]);
        println!("\nProcess error (wrong destination): {:?}", res);

        // Assert call returned error and no handle event emitted
        assert!(res.is_err());
        assert!(res.err().unwrap().to_string().contains("wrong destination"));
    }

    #[test]
    fn processes_message_to_non_existent_recipient_addr() {
        let mut app = mock_app();

        let sender_string = h256_to_string(H256::zero());

        let owner = Addr::unchecked("owner");
        let sender = Addr::unchecked(&sender_string);
        let updater_addr = Addr::unchecked(UPDATER_PUBKEY);
        let committed_root = H256::zero();
        let optimistic_seconds = 100;

        // Instantiate replica
        let replica_addr = instantiate_test_replica(
            &mut app,
            owner.clone(),
            CHAIN_ADDR_LENGTH,
            LOCAL_DOMAIN,
            REMOTE_DOMAIN,
            updater_addr,
            committed_root,
            optimistic_seconds,
        );

        let nomad_message = NomadMessage {
            origin: REMOTE_DOMAIN,
            sender: H256::zero(),
            nonce: 0,
            destination: LOCAL_DOMAIN,
            recipient: H256::repeat_byte(1), // not a real recipient
            body: "0x".as_bytes().to_vec(),
        };

        // Prove message
        let prove_msg = test_replica::ExecuteMsg::SetProven {
            leaf: nomad_message.to_leaf(),
        };
        app.execute_contract(sender.clone(), replica_addr.clone(), &prove_msg, &[])
            .unwrap();

        // Execute process message
        let msg = test_replica::ExecuteMsg::ReplicaExecuteMsg(replica::ExecuteMsg::Process {
            message: nomad_message.to_vec(),
        });
        let res = app
            .execute_contract(sender.clone(), replica_addr.clone(), &msg, &[])
            .unwrap();
        println!("\nProcess (non-existent recipient): {:?}", res);

        // Assert call completed but returned false for success field
        let success = from_binary::<bool>(&res.data.clone().unwrap()).unwrap();
        assert!(!success);
    }

    #[test]
    fn processes_message_for_bad_recipient() {
        let mut app = mock_app();

        let sender_string = h256_to_string(H256::zero());

        let owner = Addr::unchecked("owner");
        let sender = Addr::unchecked(&sender_string);
        let updater_addr = Addr::unchecked(UPDATER_PUBKEY);
        let committed_root = H256::zero();
        let optimistic_seconds = 100;

        // Instantiate replica
        let replica_addr = instantiate_test_replica(
            &mut app,
            owner.clone(),
            CHAIN_ADDR_LENGTH,
            LOCAL_DOMAIN,
            REMOTE_DOMAIN,
            updater_addr,
            committed_root,
            optimistic_seconds,
        );

        // Instantiate recipient with erroneous handle function
        let bad_recipient_addr = instantiate_bad_recipient(&mut app, owner.clone());
        let bad_recipient_addr_h256 = addr_to_h256(bad_recipient_addr.clone());

        let nomad_message = NomadMessage {
            origin: REMOTE_DOMAIN,
            sender: H256::zero(),
            nonce: 0,
            destination: LOCAL_DOMAIN,
            recipient: bad_recipient_addr_h256,
            body: "0x".as_bytes().to_vec(),
        };

        // Prove message
        let prove_msg = test_replica::ExecuteMsg::SetProven {
            leaf: nomad_message.to_leaf(),
        };
        app.execute_contract(sender.clone(), replica_addr.clone(), &prove_msg, &[])
            .unwrap();

        // Execute process message
        let msg = test_replica::ExecuteMsg::ReplicaExecuteMsg(replica::ExecuteMsg::Process {
            message: nomad_message.to_vec(),
        });
        let res = app
            .execute_contract(sender.clone(), replica_addr.clone(), &msg, &[])
            .unwrap();
        println!("\nProcess (bad recipient): {:?}", res);

        // Assert call completed but returned false for success field
        let success = from_binary::<bool>(&res.data.clone().unwrap()).unwrap();
        assert!(!success);
    }

    #[test]
    fn proves_and_processes_message() {
        let mut app = mock_app();

        let sender_string = h256_to_string(H256::zero());

        let owner = Addr::unchecked("owner");
        let sender = Addr::unchecked(&sender_string);
        let updater_addr = Addr::unchecked(UPDATER_PUBKEY);
        let committed_root = H256::zero();
        let optimistic_seconds = 100;

        // Instantiate replica
        let replica_addr = instantiate_test_replica(
            &mut app,
            owner.clone(),
            CHAIN_ADDR_LENGTH,
            LOCAL_DOMAIN,
            REMOTE_DOMAIN,
            updater_addr,
            committed_root,
            optimistic_seconds,
        );

        // Instantiate recipient
        let recipient_addr = instantiate_test_recipient(&mut app, owner.clone());
        let recipient_addr_h256 = addr_to_h256(recipient_addr.clone());

        // Create message
        let nomad_message = NomadMessage {
            origin: REMOTE_DOMAIN,
            sender: H256::zero(),
            nonce: 0,
            destination: LOCAL_DOMAIN,
            recipient: recipient_addr_h256,
            body: "0x".as_bytes().to_vec(),
        };
        let leaf = nomad_message.to_leaf();

        // Load merkle proof
        let merkle_test_cases = test_utils::load_merkle_test_json();
        let test_case = &merkle_test_cases[0];
        let Proof {
            leaf: _,
            index,
            path,
        } = test_case.proofs[0];

        let proof_root = merkle_root_from_branch(leaf, &path, 32, index);

        // Set committed root to calculated proof root
        let set_committed_msg = test_replica::ExecuteMsg::SetCommittedRoot { root: proof_root };
        app.execute_contract(
            sender.clone(),
            replica_addr.clone(),
            &set_committed_msg,
            &[],
        )
        .unwrap();

        // Prove leaf under committed root
        let msg =
            test_replica::ExecuteMsg::ReplicaExecuteMsg(replica::ExecuteMsg::ProveAndProcess {
                message: nomad_message.to_vec(),
                proof: path,
                index: index.try_into().unwrap(),
            });

        let res = app
            .execute_contract(sender.clone(), replica_addr, &msg, &[])
            .unwrap();
        println!("\nProve and process: {:?}", res);

        let success = from_binary::<bool>(&res.data.as_ref().unwrap()).unwrap();
        assert!(success);

        assert!(app_event_by_ty(&res, "wasm-Handle").is_some())
    }

    #[test]
    fn proves_and_processes_fails_if_prove_fails() {
        let mut app = mock_app();

        let sender_string = h256_to_string(H256::zero());

        let owner = Addr::unchecked("owner");
        let sender = Addr::unchecked(&sender_string);
        let updater_addr = Addr::unchecked(UPDATER_PUBKEY);
        let committed_root = H256::zero();
        let optimistic_seconds = 100;

        // Instantiate replica
        let replica_addr = instantiate_test_replica(
            &mut app,
            owner.clone(),
            CHAIN_ADDR_LENGTH,
            LOCAL_DOMAIN,
            REMOTE_DOMAIN,
            updater_addr,
            committed_root,
            optimistic_seconds,
        );

        // Instantiate recipient
        let recipient_addr = instantiate_test_recipient(&mut app, owner.clone());
        let recipient_addr_h256 = addr_to_h256(recipient_addr.clone());

        // Create message
        let nomad_message = NomadMessage {
            origin: REMOTE_DOMAIN,
            sender: H256::zero(),
            nonce: 0,
            destination: LOCAL_DOMAIN,
            recipient: recipient_addr_h256,
            body: "0x".as_bytes().to_vec(),
        };

        // Load random merkle proof from test cases that doesn't match message
        let merkle_test_cases = test_utils::load_merkle_test_json();
        let test_case = &merkle_test_cases[0];
        let Proof {
            leaf: _,
            index,
            path,
        } = test_case.proofs[0];

        // Try to prove leaf under committed root (expect fail)
        let msg =
            test_replica::ExecuteMsg::ReplicaExecuteMsg(replica::ExecuteMsg::ProveAndProcess {
                message: nomad_message.to_vec(),
                proof: path,
                index: index.try_into().unwrap(),
            });

        let res = app.execute_contract(sender.clone(), replica_addr, &msg, &[]);
        println!("\nProve and process error: (prove fails) {:?}", res);

        // Assert call returned error and no handle event emitted
        assert!(res.is_err());
        assert!(res
            .err()
            .unwrap()
            .to_string()
            .contains("Failed to prove message"));
    }

    // TODO: undergased process test case
}
