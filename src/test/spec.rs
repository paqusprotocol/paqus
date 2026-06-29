use crate::block::Block;
use crate::codec::{
    HashDomain, block_bytes, block_header_bytes, decode_block, decode_signed_transaction,
    decode_transaction, domain_hash, hash_bytes, signed_transaction_bytes, state_root_bytes,
    transaction_bytes,
};
use crate::crypto::{address_from_public_key, generate_keypair, sign, verify};
use crate::genesis::{GENESIS_HASH, genesis_block};
use crate::ledger::Ledger;
use crate::ledger::LedgerError;
use crate::ledger::fork_choice::ForkChoice;
use crate::ledger::{plan_reorg, validate_transaction_against_state};
use crate::params::FINALITY_DEPTH;
use crate::transaction::{SignedTransaction, Transaction};
use crate::types::{Address, Amount, Hash, Height, Nonce, PublicKey, Signature};
use crate::version::{active_versions, supported_block_version, supported_transaction_version};

fn hex(bytes: &[u8]) -> String {
    hex::encode(bytes)
}

const TEST_FEE: u32 = 2;

#[test]
fn canonical_spec_vectors_are_stable() {
    let public_key = PublicKey([3; crate::params::PUBLIC_KEY_SIZE]);
    let signature = Signature([4; crate::params::SIGNATURE_SIZE]);
    let from = address_from_public_key(&public_key);
    let to = Address([2; crate::params::ADDRESS_SIZE]);
    let transaction = Transaction::new(from, to, Amount(10), Amount(TEST_FEE), Nonce(0));
    let signed = SignedTransaction::new(transaction.clone(), public_key, signature);
    let mut block = Block::new(
        Height(0),
        Hash([0; crate::params::HASH_SIZE]),
        Address([9; crate::params::ADDRESS_SIZE]),
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
        "0206a9437610970c33def57a35aa4a8045c9e5819702020202020202020202020202020202020202020a0000000200000000000000000000000000000000000000"
    );
    assert_eq!(
        hex(&transaction.hash().0),
        "cfe1fc8c982764fe8860ab0e6cccbe2fdc1f8d3ee37e1ca7bdde85cdde27f925b21d5c716018ae0f81fb8de562d11df19c5c06e78c68f3f46d158765b39ae05f"
    );
    assert_eq!(signed_transaction_bytes(&signed).len(), 7284);
    assert_eq!(
        hex(&signed.hash().0),
        "cfe1fc8c982764fe8860ab0e6cccbe2fdc1f8d3ee37e1ca7bdde85cdde27f925b21d5c716018ae0f81fb8de562d11df19c5c06e78c68f3f46d158765b39ae05f"
    );
    assert_eq!(
        hex(&signed.wtxid().0),
        "0a950c3e12369277bf70c2a198b0f3f9973faf2730d875c6a415c96fa1fb61305421fbb9802ea2b2cf0449179f6067dd37b8724d7cd7b66b00d1aca5982d4dc3"
    );
    assert_eq!(
        hex(&block_header_bytes(&block.header)),
        "0100000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000d42d379119b98a5809238c8be53c250bfe2b64ecac5fd762f2f5786d55a272a48610c96a9f78464ed36ec27d564bc009ca1738b5b834a77dd88ede7e6fa3e54f09090909090909090909090909090909090909090100000000f15365000000000000000000000000"
    );
    assert_eq!(
        hex(&block_bytes(&block)),
        "0100000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000d42d379119b98a5809238c8be53c250bfe2b64ecac5fd762f2f5786d55a272a48610c96a9f78464ed36ec27d564bc009ca1738b5b834a77dd88ede7e6fa3e54f09090909090909090909090909090909090909090100000000f15365000000000000000000000000000000000000000000"
    );
    assert_eq!(
        hex(&block.hash().0),
        "8eecacd13fb3259bd4d3d90e9a7f68ed63f1f3a829fdba47a93ed4522fc5219863962dced281df459cd52d099f3f01040bd0ab8e38a84669d564438aa3270d77"
    );
    assert_eq!(
        hex(&GENESIS_HASH),
        "47fd7ab96672a233d5ea12b6d273ae757c252715fbbcdf70a3ed80ce9fa893abaf16ad35c99118257e4708e36737113296fe01cc603c946e0e9822ef16e0803f"
    );
    assert_eq!(GENESIS_HASH, genesis_block().hash().0);
    assert_eq!(
        hex(&state_root_bytes(&state_root)),
        "d42d379119b98a5809238c8be53c250bfe2b64ecac5fd762f2f5786d55a272a48610c96a9f78464ed36ec27d564bc009ca1738b5b834a77dd88ede7e6fa3e54f"
    );

    let keypair = generate_keypair();
    let signed_message = b"paqus signature verification vector";
    let valid_signature = sign(&keypair.secret_key, signed_message);
    assert_eq!(
        hex(signed_message),
        "7061717573207369676e617475726520766572696669636174696f6e20766563746f72"
    );
    assert_ne!(
        hash_bytes(&keypair.public_key.0),
        Hash([0; crate::params::HASH_SIZE])
    );
    assert_ne!(
        hash_bytes(&valid_signature.0),
        Hash([0; crate::params::HASH_SIZE])
    );
    assert!(verify(
        &keypair.public_key,
        signed_message,
        &valid_signature
    ));
}

#[test]
fn decode_validation_rejects_invalid_or_mismatched_bytes() {
    let transaction = Transaction::new(
        Address([1; crate::params::ADDRESS_SIZE]),
        Address([2; crate::params::ADDRESS_SIZE]),
        Amount(10),
        Amount(TEST_FEE),
        Nonce(0),
    );
    assert_eq!(
        decode_transaction(&transaction_bytes(&transaction)),
        Ok(transaction)
    );

    let same_sender = Transaction::new(
        Address([1; crate::params::ADDRESS_SIZE]),
        Address([1; crate::params::ADDRESS_SIZE]),
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
        Address([2; crate::params::ADDRESS_SIZE]),
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
        Signature([1; crate::params::SIGNATURE_SIZE]),
    );
    assert!(decode_signed_transaction(&signed_transaction_bytes(&invalid_signed)).is_err());

    let block = Block::new(
        Height(0),
        Hash([0; crate::params::HASH_SIZE]),
        Address([9; crate::params::ADDRESS_SIZE]),
        1_700_000_000,
        Nonce(0),
        vec![],
    );
    assert_eq!(decode_block(&block_bytes(&block)), Ok(block));
}

#[test]
fn invariants_versioning_reorg_and_hash_domains_are_explicit() {
    let mut ledger = Ledger::new();
    let from = Address([1; crate::params::ADDRESS_SIZE]);
    let to = Address([2; crate::params::ADDRESS_SIZE]);
    ledger.create_account(from, Amount(100)).unwrap();
    ledger.create_account(to, Amount(5)).unwrap();
    crate::ledger::validate_ledger_invariants(&ledger).unwrap();

    let transaction = Transaction::new(from, to, Amount(10), Amount(TEST_FEE), Nonce(0));
    assert_eq!(
        validate_transaction_against_state(&ledger.accounts, &transaction, Height(1)),
        Ok(())
    );

    let versions = active_versions(Height(0));
    assert!(supported_block_version(Height(0), versions.block));
    assert!(supported_transaction_version(
        Height(0),
        versions.transaction
    ));
    assert!(!supported_block_version(
        Height(0),
        versions.block.saturating_add(1)
    ));

    let genesis = Block::new(
        Height(0),
        Hash([0; crate::params::HASH_SIZE]),
        Address([9; crate::params::ADDRESS_SIZE]),
        1_700_000_000,
        Nonce(0),
        vec![],
    );
    let child = Block::new(
        Height(1),
        genesis.hash(),
        Address([9; crate::params::ADDRESS_SIZE]),
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
    let miner = Address([9; crate::params::ADDRESS_SIZE]);
    let genesis = Block::new(
        Height(0),
        Hash([0; crate::params::HASH_SIZE]),
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
