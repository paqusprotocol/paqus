use crate::block::Block;
use crate::genesis::error::GenesisError;
use crate::ledger::Ledger;
use crate::params::{GENESIS_PREMINE, HASH_SIZE};
use crate::types::{Address, Amount, Hash, Height, Nonce};

pub const GENESIS_PREMINE_ADDRESS: Address = Address([0; 20]);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GenesisConfig {
    pub premine_address: Address,
    pub miner_address: Address,
    pub timestamp: u64,
}

pub fn genesis_premine_amount() -> Result<Amount, GenesisError> {
    Ok(Amount(GENESIS_PREMINE))
}

pub fn create_genesis_block(config: GenesisConfig) -> Block {
    Block::new(
        Height(0),
        Hash([0; HASH_SIZE]),
        config.miner_address,
        config.timestamp,
        Nonce(0),
        vec![],
    )
}

pub fn create_genesis_ledger(config: GenesisConfig) -> Result<Ledger, GenesisError> {
    let mut ledger = Ledger::new();
    let premine = genesis_premine_amount()?;

    ledger.create_account(config.premine_address, premine)?;
    ledger.chain.insert_block(create_genesis_block(config))?;

    Ok(ledger)
}

pub fn create_default_genesis_ledger(
    miner_address: Address,
    timestamp: u64,
) -> Result<Ledger, GenesisError> {
    create_genesis_ledger(GenesisConfig {
        premine_address: GENESIS_PREMINE_ADDRESS,
        miner_address,
        timestamp,
    })
}
