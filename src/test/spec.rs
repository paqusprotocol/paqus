use crate::block::Block;
use crate::block::{Height, Nonce};
use crate::codec::{
    HashDomain, block_bytes, block_header_bytes, decode_block, decode_signed_transaction,
    decode_transaction, domain_hash, hash_bytes, signed_transaction_bytes, state_root_bytes,
    transaction_bytes,
};
use crate::consensus::supply::Amount;
use crate::crypto::{Address, PublicKey, Signature};
use crate::crypto::{HASH_SIZE, Hash};
use crate::crypto::{address_from_public_key, generate_keypair, sign, verify};
use crate::genesis::{GENESIS_HASH, genesis_block};
use crate::ledger::Ledger;
use crate::ledger::LedgerError;
use crate::ledger::fork_choice::ForkChoice;
use crate::ledger::{FINALITY_DEPTH, plan_reorg, validate_transaction_against_state};
use crate::transaction::{SignedTransaction, Transaction};

fn hex(bytes: &[u8]) -> String {
    hex::encode(bytes)
}

const TEST_FEE: u64 = 2;

#[test]
fn canonical_spec_vectors_are_stable() {
    let public_key = PublicKey([3; crate::crypto::PUBLIC_KEY_SIZE]);
    let signature = Signature([4; crate::crypto::SIGNATURE_SIZE]);
    let from = address_from_public_key(&public_key);
    let to = Address([2; crate::crypto::ADDRESS_SIZE]);
    let transaction = Transaction::new(from, to, Amount(10), Amount(TEST_FEE), Nonce(0));
    let signed = SignedTransaction::new(transaction.clone(), public_key, signature);
    let mut block = Block::new(
        Height(0),
        Hash([0; HASH_SIZE]),
        Address([9; crate::crypto::ADDRESS_SIZE]),
        1_700_000_000,
        Nonce(0),
        vec![],
    );
    let mut ledger = Ledger::new();
    ledger.create_account(from, Amount(100)).unwrap();
    ledger.create_account(to, Amount(5)).unwrap();
    ledger.apply_block(block.clone()).unwrap();
    let state_root = ledger.state_root();
    block.set_state_root(state_root);

    assert_eq!(
        hex(&transaction_bytes(&transaction)),
        "02e4676c71beacf3b678c3199d2e6298fe89c9574e02020202020202020202020202020202020202020a00000000000000020000000000000000000000000000000000000000000000"
    );
    assert_eq!(
        hex(&transaction.hash().0),
        "bf998fd4ef11753f434029f2c1c1070ffeba4338f36cc653df94107be29c9867"
    );
    assert_eq!(signed_transaction_bytes(&signed).len(), 7292);
    assert_eq!(
        hex(&signed.hash().0),
        "bf998fd4ef11753f434029f2c1c1070ffeba4338f36cc653df94107be29c9867"
    );
    assert_eq!(
        hex(&signed.wtxid().0),
        "4f4adba7d019498e746173d7b560f63d103545cd677bdd600d8fb0ddb1d28171"
    );
    assert_eq!(
        hex(&block_header_bytes(&block.header)),
        "010000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000006c61dc371e9b45bdcb247802665f353c4cae97eddd0cae3f035b0588fe17a5a909090909090909090909090909090909090909090100000000f15365000000000000000000000000"
    );
    assert_eq!(
        hex(&block_bytes(&block)),
        "010000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000006c61dc371e9b45bdcb247802665f353c4cae97eddd0cae3f035b0588fe17a5a909090909090909090909090909090909090909090100000000f15365000000000000000000000000000000000000000000"
    );
    assert_eq!(
        hex(&block.hash().0),
        "59538881a8cbc6f47c690ab48509e31c8238efea206432aff2cbf7a4a53c24af"
    );
    assert_eq!(
        hex(&GENESIS_HASH),
        "ce45a7eed810f61e7b168b719dd9cf2c5c2344c07079cad09a0fa92e2282eb6f"
    );
    assert_eq!(GENESIS_HASH, genesis_block().hash().0);
    assert_eq!(
        hex(&state_root_bytes(&state_root)),
        "6c61dc371e9b45bdcb247802665f353c4cae97eddd0cae3f035b0588fe17a5a9"
    );

    let keypair = generate_keypair();
    let signed_message = b"paqus signature verification vector";
    let valid_signature = sign(&keypair.secret_key, signed_message);
    assert_eq!(
        hex(signed_message),
        "7061717573207369676e617475726520766572696669636174696f6e20766563746f72"
    );
    assert_ne!(hash_bytes(&keypair.public_key.0), Hash([0; HASH_SIZE]));
    assert_ne!(hash_bytes(&valid_signature.0), Hash([0; HASH_SIZE]));
    assert!(verify(
        &keypair.public_key,
        signed_message,
        &valid_signature
    ));
}

#[test]
fn decode_validation_rejects_invalid_or_mismatched_bytes() {
    let transaction = Transaction::new(
        Address([1; crate::crypto::ADDRESS_SIZE]),
        Address([2; crate::crypto::ADDRESS_SIZE]),
        Amount(10),
        Amount(TEST_FEE),
        Nonce(0),
    );
    assert_eq!(
        decode_transaction(&transaction_bytes(&transaction)),
        Ok(transaction)
    );

    let same_sender = Transaction::new(
        Address([1; crate::crypto::ADDRESS_SIZE]),
        Address([1; crate::crypto::ADDRESS_SIZE]),
        Amount(10),
        Amount(TEST_FEE),
        Nonce(0),
    );
    assert!(decode_transaction(&transaction_bytes(&same_sender)).is_err());
    assert!(decode_signed_transaction(&[1, 2, 3]).is_err());

    let keypair = generate_keypair();
    let from = address_from_public_key(&keypair.public_key);
    let signed_payload = Transaction::new(
        from,
        Address([2; crate::crypto::ADDRESS_SIZE]),
        Amount(10),
        Amount(TEST_FEE),
        Nonce(0),
    );
    let valid_signature = sign(&keypair.secret_key, &signed_payload.signing_bytes());
    let valid_signed =
        SignedTransaction::new(signed_payload.clone(), keypair.public_key, valid_signature);
    assert_eq!(
        decode_signed_transaction(&signed_transaction_bytes(&valid_signed)),
        Ok(valid_signed)
    );

    let invalid_signed = SignedTransaction::new(
        signed_payload,
        keypair.public_key,
        Signature([1; crate::crypto::SIGNATURE_SIZE]),
    );
    assert!(decode_signed_transaction(&signed_transaction_bytes(&invalid_signed)).is_err());

    let block = Block::new(
        Height(0),
        Hash([0; HASH_SIZE]),
        Address([9; crate::crypto::ADDRESS_SIZE]),
        1_700_000_000,
        Nonce(0),
        vec![],
    );
    assert_eq!(decode_block(&block_bytes(&block)), Ok(block));
}

#[test]
fn invariants_versioning_reorg_and_hash_domains_are_explicit() {
    let mut ledger = Ledger::new();
    let from = Address([1; crate::crypto::ADDRESS_SIZE]);
    let to = Address([2; crate::crypto::ADDRESS_SIZE]);
    ledger.create_account(from, Amount(100)).unwrap();
    ledger.create_account(to, Amount(5)).unwrap();
    crate::ledger::validate_ledger_invariants(&ledger).unwrap();

    let transaction = Transaction::new(from, to, Amount(10), Amount(TEST_FEE), Nonce(0));
    assert_eq!(
        validate_transaction_against_state(&ledger.accounts, &transaction, Height(1)),
        Ok(())
    );

    let genesis = Block::new(
        Height(0),
        Hash([0; HASH_SIZE]),
        Address([9; crate::crypto::ADDRESS_SIZE]),
        1_700_000_000,
        Nonce(0),
        vec![],
    );
    let child = Block::new(
        Height(1),
        genesis.hash(),
        Address([9; crate::crypto::ADDRESS_SIZE]),
        1_700_000_001,
        Nonce(0),
        vec![],
    );
    let mut active = Ledger::new();
    active.apply_block(genesis.clone()).unwrap();
    let mut fork_choice = ForkChoice::new();
    fork_choice.insert_block(genesis.clone()).unwrap();
    fork_choice.insert_block(child.clone()).unwrap();
    let plan = plan_reorg(&active, &fork_choice, child.hash()).unwrap();
    assert_eq!(plan.ancestor, genesis.hash());
    assert_eq!(plan.apply, vec![child]);

    let bytes = transaction_bytes(&transaction);
    assert_ne!(
        domain_hash(HashDomain::Transaction, &bytes),
        domain_hash(HashDomain::BlockHeader, &bytes)
    );
}

#[test]
fn finalized_blocks_cannot_be_reorged() {
    let miner = Address([9; crate::crypto::ADDRESS_SIZE]);
    let genesis = Block::new(
        Height(0),
        Hash([0; HASH_SIZE]),
        miner,
        1_700_000_000,
        Nonce(0),
        vec![],
    );
    let mut active = Ledger::new();
    let mut fork_choice = ForkChoice::new();
    active.chain.insert_block(genesis.clone()).unwrap();
    fork_choice.insert_block(genesis.clone()).unwrap();

    let mut previous = genesis;
    for height in 1..=FINALITY_DEPTH as u64 + 1 {
        let block = Block::new(
            Height(height),
            previous.hash(),
            miner,
            1_700_000_000 + height,
            Nonce(height),
            vec![],
        );
        active.chain.insert_block(block.clone()).unwrap();
        fork_choice.insert_block(block.clone()).unwrap();
        previous = block;
    }

    let side = Block::new(
        Height(1),
        active.block(&Height(0)).unwrap().hash(),
        miner,
        1_700_000_001,
        Nonce(99),
        vec![],
    );
    fork_choice.insert_block(side.clone()).unwrap();

    assert_eq!(
        plan_reorg(&active, &fork_choice, side.hash()),
        Err(LedgerError::InvalidParent)
    );
}
