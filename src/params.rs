pub const CHAIN_NAME: &str = "paqus";
pub const CHAIN_ID: u8 = 1;
pub const COIN_NAME: &str = "XPQ";
pub const UNIT_NAME: &str = "QBit";
pub const PROTOCOL_STAGE: &str = "Testnet";
pub const PROTOCOL_VERSION: u8 = 1;
pub const BLOCK_VERSION: u8 = 1;
pub const TRANSACTION_VERSION: u8 = 1;

pub const UNIT: u32 = 1;
pub const XPQ: u32 = 100;
pub const DECIMALS: u8 = 2;

pub const MAX_UNIT_SUPPLY: u32 = u32::MAX;
pub const GENESIS_PREMINE: u32 = 95;  // 0.95 XPQ
pub const MAX_XPQ_SUPPLY: u32 = MAX_UNIT_SUPPLY / XPQ;
pub const MAX_MINED_SUPPLY: u32 = MAX_UNIT_SUPPLY - GENESIS_PREMINE;

pub const SECOND: u32 = 1;
pub const MINUTE: u32 = 60 * SECOND;
pub const HOUR: u32 = 60 * MINUTE;
pub const DAY: u32 = 24 * HOUR;
pub const BLOCK_TIME: u32 = 5 * MINUTE;
pub const BLOCKS_PER_DAY: u64 = DAY as u64 / BLOCK_TIME as u64;
pub const BLOCKS_PER_YEAR: u64 = 365 * BLOCKS_PER_DAY;
pub const TAIL_EMISSION_START_YEARS: u64 = 4;
pub const TAIL_EMISSION_START_HEIGHT: u64 = TAIL_EMISSION_START_YEARS * BLOCKS_PER_YEAR;

pub const BLOCK_REWARD: u32 = 5_000; // 50 XPQ
pub const TAIL_EMISSION: u32 = 100; // 1 XPQ
pub const BLOCK_REWARD_MATURITY: u32 = 10;
pub const FINALITY_DEPTH: u32 = 1;
pub const MIN_FEE: u32 = 2;

pub const MIN_DIFFICULTY: u32 = 1;
pub const MAX_DIFFICULTY: u32 = (PROOF_OF_WORK_HASH_SIZE * 8) as u32;
pub const DIFFICULTY_START: u32 = 1;
pub const DIFFICULTY_ADJUSTMENT_INTERVAL: u64 = 2016;
pub const MIN_DIFFICULTY_TIMESPAN_FACTOR: u64 = 2;
pub const MAX_FUTURE_TIME: u32 = 150 * SECOND;

pub const ADDRESS_SIZE: usize = 20;
pub const PUBLIC_KEY_SIZE: usize = 2_592;
pub const SECRET_KEY_SIZE: usize = 4_896;
pub const SIGNATURE_SIZE: usize = 4_627;
pub const HASH_SIZE: usize = 64;
pub const PROOF_OF_WORK_HASH_SIZE: usize = 32;

pub const MAX_TX_SIZE: usize = 10 * 1024;
pub const MAX_BLOCK_SIZE: usize = 2 * 1024 * 1024;
pub const MAX_BLOCK_TXS: usize = 250;
