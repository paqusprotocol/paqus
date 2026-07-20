use crate::block::Block;
use crate::crypto::Address;
use crate::crypto::{HASH_SIZE, Hash};
use crate::error::GenesisError;
use crate::ledger::Ledger;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChainParams {
    pub chain_name: &'static str,
    pub chain_id: u16,
    pub coin_name: &'static str,
    pub unit_name: &'static str,
    pub protocol_stage: &'static str,
    pub protocol_version: u8,
    pub pow_algorithm: &'static str,
    pub difficulty_algorithm: &'static str,
    pub network_magic: [u8; 4],
    pub genesis: GenesisParams,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenesisParams {
    pub miner_address: [u8; crate::crypto::ADDRESS_SIZE],
    pub timestamp: u64,
    pub hash: [u8; HASH_SIZE],
}

pub const PAQUS_CHAIN: ChainParams = ChainParams {
    chain_name: "Paqus",
    chain_id: 747,
    coin_name: "XPQ",
    unit_name: "paqus",
    protocol_stage: "Mainnet",
    protocol_version: 2,
    pow_algorithm: "sha3-512",
    difficulty_algorithm: "asert-bits-v2",
    network_magic: [0x58, 0x50, 0x51, 0x02],
    genesis: GenesisParams {
        miner_address: [0; crate::crypto::ADDRESS_SIZE],
        // Fixed timestamp of the first canonical genesis build. This must stay static so all nodes
        // derive the same genesis hash.
        timestamp: 1_700_000_000,
        hash: FROZEN_GENESIS_HASH,
    },
};

/// Frozen mainnet identity for the canonical encoding and block format.
/// Never update this value without defining a new protocol version and chain identity.
pub const FROZEN_GENESIS_HASH: [u8; HASH_SIZE] = [
    64, 129, 62, 134, 193, 213, 37, 97, 244, 253, 76, 217, 157, 26, 57, 219, 40, 57, 244, 197, 149,
    126, 64, 244, 82, 129, 210, 131, 227, 254, 31, 33,
];

pub const CURRENT_CHAIN_PARAMS: ChainParams = PAQUS_CHAIN;

pub const GENESIS_MINER_ADDRESS: Address = Address(CURRENT_CHAIN_PARAMS.genesis.miner_address);
pub const GENESIS_TIMESTAMP: u64 = CURRENT_CHAIN_PARAMS.genesis.timestamp;
pub const GENESIS_HASH: [u8; HASH_SIZE] = CURRENT_CHAIN_PARAMS.genesis.hash;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GenesisConfig {
    pub miner_address: Address,
    pub timestamp: u64,
}

pub fn create_genesis_block(config: GenesisConfig) -> Block {
    create_genesis_block_for_chain(CURRENT_CHAIN_PARAMS, config)
}

pub fn create_genesis_block_for_chain(_params: ChainParams, config: GenesisConfig) -> Block {
    Block::genesis(config.miner_address, config.timestamp, vec![])
}

pub fn create_genesis_ledger(config: GenesisConfig) -> Result<Ledger, GenesisError> {
    create_genesis_ledger_for_chain(CURRENT_CHAIN_PARAMS, config)
}

pub fn create_genesis_ledger_for_chain(
    params: ChainParams,
    config: GenesisConfig,
) -> Result<Ledger, GenesisError> {
    let mut ledger = Ledger::new();
    ledger.apply_block(create_genesis_block_for_chain(params, config))?;

    Ok(ledger)
}

pub fn genesis_block() -> Block {
    genesis_block_for_chain(CURRENT_CHAIN_PARAMS)
}

pub fn genesis_block_for_chain(params: ChainParams) -> Block {
    create_genesis_block_for_chain(
        params,
        GenesisConfig {
            miner_address: Address(params.genesis.miner_address),
            timestamp: params.genesis.timestamp,
        },
    )
}

pub fn validate_genesis_identity(params: ChainParams) -> Result<(), GenesisError> {
    let found = genesis_block_for_chain(params).hash().0;
    if found != params.genesis.hash {
        return Err(GenesisError::HashMismatch {
            expected: params.genesis.hash,
            found,
        });
    }
    Ok(())
}

pub fn genesis_hash() -> Hash {
    Hash(GENESIS_HASH)
}

pub fn genesis_ledger() -> Result<Ledger, GenesisError> {
    genesis_ledger_for_chain(CURRENT_CHAIN_PARAMS)
}

pub fn genesis_ledger_for_chain(params: ChainParams) -> Result<Ledger, GenesisError> {
    validate_genesis_identity(params)?;
    let mut ledger = Ledger::new();
    ledger.apply_block(genesis_block_for_chain(params))?;

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
