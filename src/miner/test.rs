use super::{MiningConfig, mine_candidate_block};
use crate::consensus::Consensus;
use crate::ledger::Ledger;
use crate::mempool::Mempool;
use crate::types::{Address, Amount};

fn address(byte: u8) -> Address {
    Address([byte; 20])
}

#[test]
fn mines_candidate_block_until_pow_is_valid() {
    let consensus = Consensus::with_default_config();
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
            difficulty: 1,
            max_attempts: 2_000,
            transaction_limit: 10,
        },
    )
    .unwrap()
    .expect("difficulty 1 should be mined within attempt budget");

    assert!(result.attempts <= 2_000);
    assert_eq!(result.block.difficulty(), 1);
    assert_eq!(consensus.validate_proof_of_work(&result.block), Ok(()));
}
