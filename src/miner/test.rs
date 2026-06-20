use super::{MiningConfig, mine_candidate_block};
use crate::consensus::{Consensus, ConsensusConfig};
use crate::ledger::Ledger;
use crate::mempool::Mempool;
use crate::types::{Address, Amount};

fn address(byte: u8) -> Address {
    Address([byte; 20])
}

#[test]
fn mines_candidate_block_until_pow_is_valid() {
    let consensus = Consensus {
        config: ConsensusConfig { difficulty: 0 },
    };
    let mut ledger = Ledger::new();
    let miner = address(9);
    ledger.create_account(miner, Amount(0)).unwrap();
    let mempool = Mempool::new();

    let result = mine_candidate_block(
        &mempool,
        &ledger,
        &consensus,
        miner,
        1_700_000_000,
        MiningConfig {
            difficulty: 0,
            max_attempts: 1,
            transaction_limit: 10,
        },
    )
    .unwrap()
    .expect("difficulty 0 should produce a test block immediately");

    assert_eq!(result.attempts, 1);
    assert_eq!(result.block.difficulty(), 0);
    assert_eq!(consensus.validate_proof_of_work(&result.block), Ok(()));
}
