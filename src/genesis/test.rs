use super::{
    GENESIS_PREMINE_ADDRESS, GenesisConfig, create_default_genesis_ledger, create_genesis_block,
    create_genesis_ledger, genesis_premine_amount,
};
use crate::params::GENESIS_PREMINE;
use crate::types::{Address, Amount, Hash, Height};

fn address(byte: u8) -> Address {
    Address([byte; 20])
}

fn config() -> GenesisConfig {
    GenesisConfig {
        premine_address: address(1),
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
fn creates_genesis_ledger_with_premine_and_genesis_block() {
    let ledger = create_genesis_ledger(config()).unwrap();

    assert_eq!(ledger.balance(&address(1)), Some(Amount(GENESIS_PREMINE)));
    assert_eq!(ledger.tip_height(), Some(Height(0)));
    assert_eq!(ledger.block(&Height(0)).unwrap().transaction_count(), 0);
}

#[test]
fn can_allocate_genesis_premine_to_zero_address() {
    let ledger = create_genesis_ledger(GenesisConfig {
        premine_address: GENESIS_PREMINE_ADDRESS,
        miner_address: address(9),
        timestamp: 1_700_000_000,
    })
    .unwrap();

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
