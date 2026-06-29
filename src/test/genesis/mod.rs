use crate::genesis::{
    GENESIS_HASH, GENESIS_PREMINE_ADDRESS, GenesisConfig, create_default_genesis_ledger,
    create_genesis_block, create_genesis_ledger, genesis_block, genesis_block_for_chain,
    genesis_ledger, genesis_premine_amount,
};
use crate::params::{CURRENT_CHAIN_PARAMS, GENESIS_PREMINE, PAQUS_CHAIN};
use crate::types::{Address, Amount, Hash, Height};

fn address(byte: u8) -> Address {
    Address([byte; 20])
}

fn config() -> GenesisConfig {
    GenesisConfig {
        miner_address: address(9),
        timestamp: 1_700_000_000,
    }
}

#[test]
fn calculates_genesis_premine_in_smallest_unit() {
    assert_eq!(genesis_premine_amount(), Ok(Amount(GENESIS_PREMINE)));
}

#[test]
fn creates_empty_genesis_block() {
    let block = create_genesis_block(config());

    assert_eq!(block.height(), Height(0));
    assert_eq!(block.previous_hash(), Hash([0; 64]));
    assert_eq!(block.transaction_count(), 0);
    assert_eq!(block.validate(), Ok(()));
}

#[test]
fn creates_genesis_block_with_canonical_hash() {
    let block = create_genesis_block(config());
    let block_hash = block.hash();
    let ledger = create_genesis_ledger(config()).unwrap();

    assert_eq!(ledger.tip_hash(), Some(block_hash));
}

#[test]
fn canonical_genesis_block_matches_genesis_hash() {
    let block = genesis_block();
    let ledger = genesis_ledger().unwrap();

    assert_eq!(block.hash().0, GENESIS_HASH);
    assert_eq!(ledger.tip_hash(), Some(block.hash()));
}

#[test]
fn creates_genesis_ledger_with_premine_and_genesis_block() {
    let ledger = create_genesis_ledger(config()).unwrap();

    assert_eq!(
        ledger.balance(&GENESIS_PREMINE_ADDRESS),
        Some(Amount(GENESIS_PREMINE))
    );
    assert_eq!(ledger.tip_height(), Some(Height(0)));
    assert_eq!(ledger.block(&Height(0)).unwrap().transaction_count(), 0);
}

#[test]
fn can_allocate_genesis_premine_to_zero_address() {
    let ledger = create_genesis_ledger(config()).unwrap();

    assert_eq!(
        ledger.balance(&GENESIS_PREMINE_ADDRESS),
        Some(Amount(GENESIS_PREMINE))
    );
}

#[test]
fn creates_default_genesis_ledger_with_genesis_premine_address() {
    let ledger = create_default_genesis_ledger(address(9), 1_700_000_000).unwrap();

    assert_eq!(
        ledger.balance(&GENESIS_PREMINE_ADDRESS),
        Some(Amount(GENESIS_PREMINE))
    );
}

#[test]
fn chain_params_use_one_protocol_identity() {
    assert_eq!(CURRENT_CHAIN_PARAMS, PAQUS_CHAIN);
    assert_eq!(PAQUS_CHAIN.chain_name, "Paqus");
    assert_eq!(PAQUS_CHAIN.coin_name, "XPQ");
    assert_eq!(PAQUS_CHAIN.protocol_stage, "Mainnet");
}

#[test]
fn genesis_is_selected_from_chain_params() {
    assert_eq!(
        genesis_block_for_chain(PAQUS_CHAIN).hash().0,
        PAQUS_CHAIN.genesis.hash
    );
    assert_eq!(
        genesis_block_for_chain(CURRENT_CHAIN_PARAMS).hash().0,
        GENESIS_HASH
    );
}

#[test]
fn mainnet_zero_premine_address_is_intentional_supply_offset() {
    assert_eq!(
        PAQUS_CHAIN.genesis.premine_address,
        [0; crate::params::ADDRESS_SIZE]
    );
    assert_eq!(GENESIS_PREMINE_ADDRESS, Address::ZERO);
    assert_eq!(PAQUS_CHAIN.genesis.timestamp, 1_700_000_000);
}
