use crate::block::{Block, GenesisAllocation};
use crate::error::GenesisError;
use crate::ledger::{Ledger, calculate_state_root};
use crate::params::{GENESIS_PREMINE, HASH_SIZE};
use crate::state::Account;
use crate::types::{Address, Amount, Hash};
use std::collections::BTreeMap;

pub const GENESIS_PREMINE_ADDRESS: Address = Address::ZERO;
pub const GENESIS_MINER_ADDRESS: Address = Address::ZERO;
pub const GENESIS_TIMESTAMP: u64 = 1_700_000_000;
pub const GENESIS_HASH: [u8; HASH_SIZE] = [
    50, 172, 1, 214, 84, 193, 254, 87, 209, 37, 6, 69, 107, 183, 35, 127, 75, 175, 33, 74, 53, 115,
    177, 31, 205, 177, 40, 151, 77, 149, 134, 79, 64, 49, 133, 108, 174, 83, 168, 89, 197, 173,
    197, 210, 136, 6, 112, 115, 149, 113, 5, 123, 113, 178, 87, 86, 66, 229, 204, 230, 209, 110,
    254, 29,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GenesisConfig {
    pub miner_address: Address,
    pub timestamp: u64,
}

pub fn genesis_premine_amount() -> Result<Amount, GenesisError> {
    Ok(Amount(GENESIS_PREMINE))
}

pub fn create_genesis_block(config: GenesisConfig) -> Block {
    let premine = genesis_premine_amount().expect("genesis premine amount should be valid");
    let mut block = Block::genesis(
        config.miner_address,
        config.timestamp,
        vec![GenesisAllocation::new(GENESIS_PREMINE_ADDRESS, premine)],
    );
    let mut accounts = BTreeMap::new();
    accounts.insert(
        GENESIS_PREMINE_ADDRESS,
        Account::new(GENESIS_PREMINE_ADDRESS, premine),
    );
    block.set_state_root(calculate_state_root(&accounts));
    block
}

pub fn create_genesis_ledger(config: GenesisConfig) -> Result<Ledger, GenesisError> {
    let mut ledger = Ledger::new();
    ledger.apply_block(create_genesis_block(config))?;

    Ok(ledger)
}

pub fn genesis_block() -> Block {
    create_genesis_block(GenesisConfig {
        miner_address: GENESIS_MINER_ADDRESS,
        timestamp: GENESIS_TIMESTAMP,
    })
}

pub fn genesis_hash() -> Hash {
    Hash(GENESIS_HASH)
}

pub fn genesis_ledger() -> Result<Ledger, GenesisError> {
    let mut ledger = Ledger::new();
    ledger.apply_block(genesis_block())?;

    Ok(ledger)
}

pub fn create_default_genesis_ledger(
    miner_address: Address,
    timestamp: u64,
) -> Result<Ledger, GenesisError> {
    create_genesis_ledger(GenesisConfig {
        miner_address,
        timestamp,
    })
}
