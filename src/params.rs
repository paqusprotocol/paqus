pub const CHAIN_NAME: &str = "paqus";
pub const CHAIN_ID: u8 = 1;
pub const COIN_NAME: &str = "XPQ";
pub const UNIT_NAME: &str = "ie";
pub const NETWORK_MAGIC: [u8; 4] = [0x58, 0x50, 0x51, 0x01];
pub const PROTOCOL_STAGE: &str = "devnet";
pub const PROTOCOL_VERSION: u8 = 1;
pub const STORAGE_VERSION: u8 = 1;
pub const BLOCK_VERSION: u8 = 1;
pub const TRANSACTION_VERSION: u8 = 1;
pub const DEFAULT_RPC_PORT: u16 = 9_933;
pub const DEFAULT_P2P_PORT: u16 = 30_333;
pub const DEFAULT_RPC_ADDR: &str = "127.0.0.1:9933";
pub const DEFAULT_P2P_ADDR: &str = "[::]:30333";

pub const UNIT: u32 = 1;
pub const COIN: u32 = 100;
pub const DECIMALS: u8 = 2;

pub const MAX_UNIT_SUPPLY: u32 = u32::MAX;
pub const GENESIS_PREMINE: u32 = 95;
pub const MAX_COIN_SUPPLY: u32 = MAX_UNIT_SUPPLY / COIN;
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

pub const BLOCK_REWARD: u32 = 2_500;
pub const TAIL_EMISSION: u32 = 100;
pub const BLOCK_REWARD_MATURITY: u32 = 20;
pub const FINALITY_DEPTH: u32 = 1;
pub const MIN_FEE: u32 = 1;
pub const BASE_FEE: u32 = 2;
pub const SLOW_FEE: u32 = 1;
pub const NORMAL_FEE: u32 = BASE_FEE;
pub const FAST_FEE: u32 = 3;
pub const AGGRESSIVE_FEE: u32 = 5;

pub const MIN_DIFFICULTY: u32 = 1;
pub const MAX_DIFFICULTY: u32 = (PROOF_OF_WORK_HASH_SIZE * 8) as u32;
pub const DIFFICULTY_START: u32 = 1;
pub const DIFFICULTY_ADJUSTMENT_INTERVAL: u64 = 10;
pub const MIN_DIFFICULTY_TIMESPAN_FACTOR: u64 = 4;
pub const MAX_FUTURE_TIME: u32 = 2 * MINUTE;

pub const ADDRESS_SIZE: usize = 20;
pub const PUBLIC_KEY_SIZE: usize = 2_592;
pub const SECRET_KEY_SIZE: usize = 4_896;
pub const SIGNATURE_SIZE: usize = 4_627;
pub const HASH_SIZE: usize = 64;
pub const PROOF_OF_WORK_HASH_SIZE: usize = 32;

pub const MAX_TX_SIZE: usize = 10 * 1024;
pub const MEMPOOL_EXPIRY_SECS: u64 = (30 * MINUTE) as u64;
pub const MAX_MEMPOOL_TXS: usize = 1_000;
pub const MAX_MEMPOOL_BYTES: usize = 10 * 1024 * 1024;
pub const MAX_BLOCK_SIZE: usize = 2 * 1024 * 1024;
pub const MAX_BLOCK_TXS: usize = 250;
pub const MAX_NETWORK_MESSAGE_SIZE: usize = 4 * 1024 * 1024;

#[cfg(test)]
mod test {
    use super::{DEFAULT_P2P_ADDR, DEFAULT_P2P_PORT, DEFAULT_RPC_ADDR, DEFAULT_RPC_PORT};
    use std::net::{IpAddr, Ipv6Addr, SocketAddr};

    #[test]
    fn default_rpc_address_is_localhost() {
        let addr = DEFAULT_RPC_ADDR
            .parse::<SocketAddr>()
            .expect("default rpc address should parse");

        assert_eq!(addr.port(), DEFAULT_RPC_PORT);
        assert!(addr.ip().is_loopback());
    }

    #[test]
    fn default_p2p_address_binds_ipv6_wildcard() {
        let addr = DEFAULT_P2P_ADDR
            .parse::<SocketAddr>()
            .expect("default p2p address should parse");

        assert_eq!(addr.port(), DEFAULT_P2P_PORT);
        assert_eq!(addr.ip(), IpAddr::V6(Ipv6Addr::UNSPECIFIED));
    }
}
