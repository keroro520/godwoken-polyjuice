use crate::helper::{
    self, build_eth_l2_script, new_block_info, setup, CKB_SUDT_ACCOUNT_ID,
    ETH_ADDRESS_REGISTRY_ACCOUNT_ID, L2TX_MAX_CYCLES,
};
use gw_common::state::State;
use gw_generator::{error::TransactionError, traits::StateExt};
use gw_store::{chain_view::ChainView, traits::chain_store::ChainStore};
use gw_types::{packed::RawL2Transaction, prelude::*};

#[derive(Debug, Default)]
pub struct EthToGwArgsBuilder {
    pub(crate) method: u32,
    pub(crate) eth_address: [u8; 20],
}
impl EthToGwArgsBuilder {
    pub fn method(mut self, v: u32) -> Self {
        self.method = v;
        self
    }
    pub fn eth_address(mut self, v: [u8; 20]) -> Self {
        self.eth_address = v;
        self
    }
    pub fn build(self) -> Vec<u8> {
        let mut output: Vec<u8> = vec![0u8; 4];
        output[0..4].copy_from_slice(&self.method.to_le_bytes()[..]);
        output.extend(self.eth_address);
        output
    }
}

#[derive(Debug, Default)]
pub struct GwToEthArgsBuilder {
    pub(crate) method: u32,
    pub(crate) gw_script_hash: [u8; 32],
}
impl GwToEthArgsBuilder {
    pub fn method(mut self, v: u32) -> Self {
        self.method = v;
        self
    }
    pub fn gw_script_hash(mut self, v: [u8; 32]) -> Self {
        self.gw_script_hash = v;
        self
    }
    pub fn build(self) -> Vec<u8> {
        let mut output: Vec<u8> = vec![0u8; 4];
        output[0..4].copy_from_slice(&self.method.to_le_bytes()[..]);
        output.extend(self.gw_script_hash);
        output
    }
}

#[test]
fn test_update_eth_addr_reg_by_contract() {
    let (store, mut state, generator) = setup();
    let block_producer_id = helper::create_block_producer(&mut state);

    // init accounts
    let from_eth_address = [1u8; 20];
    let (from_id, _from_script_hash) =
        helper::create_eth_eoa_account(&mut state, &from_eth_address, 400000);

    // create a new EOA which is not registered
    let eth_eoa_address = [0xeeu8; 20];
    let eth_eoa_account_script = build_eth_l2_script(&eth_eoa_address);
    let eth_eoa_account_script_hash = eth_eoa_account_script.hash();
    let eth_eoa_account_id = state
        .create_account_from_script(eth_eoa_account_script)
        .unwrap();
    assert_eq!(
        state
            .get_sudt_balance(CKB_SUDT_ACCOUNT_ID, &eth_eoa_account_script_hash[..20])
            .unwrap(),
        0u128
    );
    state /* mint CKB to pay fee */
        .mint_sudt(
            CKB_SUDT_ACCOUNT_ID,
            &eth_eoa_account_script_hash[..20],
            2000,
        )
        .unwrap();
    assert_eq!(
        state
            .get_sudt_balance(CKB_SUDT_ACCOUNT_ID, &eth_eoa_account_script_hash[..20])
            .unwrap(),
        2000u128
    );

    // update_eth_address_registry by `ETH Address Registry` layer2 contract
    let run_result = crate::helper::eth_address_regiser(
        &store,
        &mut state,
        &generator,
        eth_eoa_account_id,
        new_block_info(block_producer_id, 1, 1),
        crate::helper::SetMappingArgs::One(eth_eoa_account_script_hash.into()),
    )
    .expect("execute the MSG_SET_MAPPING method of `ETH Address Registry` layer2 contract");
    assert_eq!(run_result.exit_code, 0);
    state.apply_run_result(&run_result).expect("update state");
    // cycles(1176198)] < 1200k cycles
    helper::check_cycles("eth_address_regiser", run_result.used_cycles, 1_200_000);
    assert_eq!(
        // make sure the fee was paid
        state
            .get_sudt_balance(CKB_SUDT_ACCOUNT_ID, &eth_eoa_account_script_hash[..20])
            .unwrap(),
        1000u128
    );

    // try to register the same account again
    let run_err = crate::helper::eth_address_regiser(
        &store,
        &mut state,
        &generator,
        eth_eoa_account_id,
        new_block_info(block_producer_id, 2, 2),
        crate::helper::SetMappingArgs::One(eth_eoa_account_script_hash.into()),
    )
    .expect_err("try to register the same account again");
    assert_eq!(
        run_err,
        TransactionError::InvalidExitCode(-92),
        "ERROR_ETH_ADDRESS_REGISTRY_DUPLICATE"
    );

    // check result: eth_address -> gw_script_hash
    let args = EthToGwArgsBuilder::default()
        .method(0u32)
        .eth_address(eth_eoa_address)
        .build();
    let raw_l2tx = RawL2Transaction::new_builder()
        .from_id(from_id.pack())
        .to_id(ETH_ADDRESS_REGISTRY_ACCOUNT_ID.pack())
        .args(args.pack())
        .build();
    let tip_block_hash = store.get_tip_block_hash().unwrap();
    let db = store.begin_transaction();
    let run_result = generator
        .execute_transaction(
            &ChainView::new(&db, tip_block_hash),
            &state,
            &new_block_info(block_producer_id, 3, 3),
            &raw_l2tx,
            L2TX_MAX_CYCLES,
            None,
        )
        .expect("execute Godwoken contract");
    assert_eq!(run_result.return_data, eth_eoa_account_script_hash);

    // check result: gw_script_hash -> eth_address
    let args = GwToEthArgsBuilder::default()
        .method(1u32)
        .gw_script_hash(eth_eoa_account_script_hash)
        .build();
    let raw_l2tx = RawL2Transaction::new_builder()
        .from_id(from_id.pack())
        .to_id(ETH_ADDRESS_REGISTRY_ACCOUNT_ID.pack())
        .args(args.pack())
        .build();
    let db = store.begin_transaction();
    let tip_block_hash = db.get_tip_block_hash().unwrap();
    let block_info = new_block_info(block_producer_id, 3, 0);
    let run_result = generator
        .execute_transaction(
            &ChainView::new(&db, tip_block_hash),
            &state,
            &block_info,
            &raw_l2tx,
            L2TX_MAX_CYCLES,
            None,
        )
        .expect("execute Godwoken contract");
    assert_eq!(run_result.return_data, eth_eoa_address);

    // New Polyjuice conatract account will be registered in `create_new_account` of polyjuice.h
}

#[test]
fn test_batch_set_mapping_by_contract() {
    let (store, mut state, generator) = setup();
    let block_producer_id = helper::create_block_producer(&mut state);

    // init accounts
    let from_eth_address = [1u8; 20];
    let (from_id, _from_script_hash) =
        helper::create_eth_eoa_account(&mut state, &from_eth_address, 400000);

    // create new EOAs which is not registered
    let eth_eoa_addresses = vec![[0xeeu8; 20], [0xefu8; 20]];
    let mut eth_eoa_script_hashes = vec![];
    for address in eth_eoa_addresses.iter() {
        let account_script = build_eth_l2_script(address);
        let account_script_hash = account_script.hash();
        eth_eoa_script_hashes.push(account_script_hash.into());
        state.create_account_from_script(account_script).unwrap();

        assert_eq!(
            state
                .get_sudt_balance(CKB_SUDT_ACCOUNT_ID, &account_script_hash[..20])
                .unwrap(),
            0u128
        );
        state /* mint CKB to pay fee */
            .mint_sudt(CKB_SUDT_ACCOUNT_ID, &account_script_hash[..20], 200000)
            .unwrap();
        assert_eq!(
            state
                .get_sudt_balance(CKB_SUDT_ACCOUNT_ID, &account_script_hash[..20])
                .unwrap(),
            200000u128
        );
    }

    // update_eth_address_registry by `ETH Address Registry` layer2 contract
    let run_result = crate::helper::eth_address_regiser(
        &store,
        &mut state,
        &generator,
        from_id,
        new_block_info(block_producer_id, 1, 1),
        crate::helper::SetMappingArgs::Batch(eth_eoa_script_hashes.clone()),
    )
    .expect("eth address registered");
    assert_eq!(run_result.exit_code, 0);
    state.apply_run_result(&run_result).expect("update state");

    for (eth_eoa_address, eth_eoa_account_script_hash) in
        eth_eoa_addresses.into_iter().zip(eth_eoa_script_hashes)
    {
        let eth_eoa_account_script_hash: [u8; 32] = eth_eoa_account_script_hash.into();

        // check result: eth_address -> gw_script_hash
        let args = EthToGwArgsBuilder::default()
            .method(0u32)
            .eth_address(eth_eoa_address)
            .build();
        let raw_l2tx = RawL2Transaction::new_builder()
            .from_id(from_id.pack())
            .to_id(ETH_ADDRESS_REGISTRY_ACCOUNT_ID.pack())
            .args(args.pack())
            .build();
        let tip_block_hash = store.get_tip_block_hash().unwrap();
        let db = store.begin_transaction();
        let run_result = generator
            .execute_transaction(
                &ChainView::new(&db, tip_block_hash),
                &state,
                &new_block_info(block_producer_id, 3, 3),
                &raw_l2tx,
                L2TX_MAX_CYCLES,
                None,
            )
            .expect("execute Godwoken contract");
        assert_eq!(run_result.return_data, eth_eoa_account_script_hash);

        // check result: gw_script_hash -> eth_address
        let args = GwToEthArgsBuilder::default()
            .method(1u32)
            .gw_script_hash(eth_eoa_account_script_hash)
            .build();
        let raw_l2tx = RawL2Transaction::new_builder()
            .from_id(from_id.pack())
            .to_id(ETH_ADDRESS_REGISTRY_ACCOUNT_ID.pack())
            .args(args.pack())
            .build();
        let db = store.begin_transaction();
        let tip_block_hash = db.get_tip_block_hash().unwrap();
        let run_result = generator
            .execute_transaction(
                &ChainView::new(&db, tip_block_hash),
                &state,
                &new_block_info(block_producer_id, 3, 3),
                &raw_l2tx,
                L2TX_MAX_CYCLES,
                None,
            )
            .expect("execute Godwoken contract");
        assert_eq!(run_result.return_data, eth_eoa_address);
    }

    // New Polyjuice conatract account will be registered in `create_new_account` of polyjuice.h
}