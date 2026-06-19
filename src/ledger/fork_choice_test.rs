use super::{ForkChoice, ForkChoiceError, block_work};
use crate::block::Block;
use crate::types::{Address, Hash, Height, Nonce};

fn address(byte: u8) -> Address {
    Address([byte; 20])
}

fn block(height: u64, previous_hash: Hash, difficulty: u32, nonce: u64) -> Block {
    Block::with_difficulty(
        Height(height),
        previous_hash,
        address(9),
        difficulty,
        1_700_000_000 + height,
        Nonce(nonce),
        vec![],
    )
}

#[test]
fn rejects_block_when_parent_is_missing() {
    let mut fork_choice = ForkChoice::new();

    assert_eq!(
        fork_choice.insert_block(block(1, Hash([9; 64]), 1, 0)),
        Err(ForkChoiceError::MissingParent)
    );
}

#[test]
fn chooses_tip_with_highest_cumulative_work() {
    let mut fork_choice = ForkChoice::new();
    let genesis = block(0, Hash([0; 64]), 1, 0);
    let genesis_hash = fork_choice.insert_block(genesis).unwrap();
    let low_work = block(1, genesis_hash, 1, 1);
    let high_work = block(1, genesis_hash, 3, 2);
    let high_work_hash = high_work.hash();

    fork_choice.insert_block(low_work).unwrap();
    fork_choice.insert_block(high_work).unwrap();

    let best = fork_choice.best_tip().unwrap();
    assert_eq!(best.hash, high_work_hash);
    assert_eq!(best.cumulative_work, block_work(1) + block_work(3));
}

#[test]
fn chooses_lowest_hash_when_cumulative_work_ties() {
    let mut fork_choice = ForkChoice::new();
    let genesis = block(0, Hash([0; 64]), 1, 0);
    let genesis_hash = fork_choice.insert_block(genesis).unwrap();
    let first = block(1, genesis_hash, 1, 1);
    let second = block(1, genesis_hash, 1, 2);
    let expected = first.hash().min(second.hash());

    fork_choice.insert_block(first).unwrap();
    fork_choice.insert_block(second).unwrap();

    assert_eq!(fork_choice.best_tip().unwrap().hash, expected);
}
