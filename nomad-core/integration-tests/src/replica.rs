use std::str::FromStr;

use common::{addr_to_h256, h256_to_string, replica, test::test_replica, Encode, NomadMessage};
use cosmwasm_std::{from_binary, Addr};
use cw_multi_test::{App, AppBuilder, BankKeeper, ContractWrapper, Executor};
use ethers_core::types::H256;
use test_utils::Updater;

use crate::utils::{
    app_event_by_ty, instantiate_test_recipient, instantiate_test_replica, mock_app,
};

const CHAIN_ADDR_LENGTH: usize = 11; // e.g. "Contract #0".len()
const REMOTE_DOMAIN: u32 = 1000;
const LOCAL_DOMAIN: u32 = 2000;
const UPDATER_PRIVKEY: &str = "1111111111111111111111111111111111111111111111111111111111111111";
const UPDATER_PUBKEY: &str = "0x19e7e376e7c213b7e7e7e46cc70a5dd086daff2a";

#[test]
fn processes_proved_message() {
    let mut app = mock_app();

    let sender_string = h256_to_string(H256::zero());
    let recipient_string = h256_to_string(H256::repeat_byte(1));

    let owner = Addr::unchecked("owner");
    let sender = Addr::unchecked(&sender_string);
    let recipient = Addr::unchecked(&recipient_string);
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
    let res = app.execute_contract(sender.clone(), replica_addr.clone(), &prove_msg, &[]);
    println!("Prove: {:?}", res);

    let msg = test_replica::ExecuteMsg::ReplicaExecuteMsg(replica::ExecuteMsg::Process {
        message: nomad_message.to_vec(),
    });
    let res = app.execute_contract(sender.clone(), replica_addr.clone(), &msg, &[]).unwrap();
    println!("Process: {:?}", res);

    assert!(app_event_by_ty(&res, "wasm-Handle").is_some())
}
