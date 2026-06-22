use crate::block::Block;
use crate::codec::{
    HashDomain, block_bytes, block_header_bytes, decode_block, decode_signed_transaction,
    decode_transaction, domain_hash, hash_bytes, signed_transaction_bytes, state_root_bytes,
    transaction_bytes,
};
use crate::crypto::{address_from_public_key, generate_keypair, sign, verify};
use crate::genesis::{GENESIS_HASH, genesis_block};
use crate::ledger::Ledger;
use crate::ledger::fork_choice::ForkChoice;
use crate::ledger::{plan_reorg, validate_transaction_against_state};
use crate::params::BASE_FEE;
use crate::transaction::{SignedTransaction, Transaction};
use crate::types::{Address, Amount, Hash, Height, Nonce, PublicKey, Signature};
use crate::version::{active_versions, supported_block_version, supported_transaction_version};

fn hex(bytes: &[u8]) -> String {
    hex::encode(bytes)
}

#[test]
fn canonical_spec_vectors_are_stable() {
    let public_key = PublicKey([3; crate::params::PUBLIC_KEY_SIZE]);
    let signature = Signature([4; crate::params::SIGNATURE_SIZE]);
    let from = address_from_public_key(&public_key);
    let to = Address([2; crate::params::ADDRESS_SIZE]);
    let transaction = Transaction::new(from, to, Amount(10), Amount(BASE_FEE), Nonce(0));
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
        "0106a9437610970c33def57a35aa4a8045c9e5819702020202020202020202020202020202020202020a000000020000000000000000000000"
    );
    assert_eq!(
        hex(&transaction.hash().0),
        "50f2f095019b93bd397c05133a767e39a145e9e48bef7112af629bc2d4029cd9df261ce9c3755cf21c85746a4c64230b23006696c6918890983f1d6c158612a3"
    );
    assert_eq!(signed_transaction_bytes(&signed).len(), 7276);
    assert_eq!(
        hex(&signed.hash().0),
        "871a225d9f9c305987a42b424749cc1805d02285b26a80fddf3f08806da3d734f97e19d7b9625a5a6f81254769caadf51ab0d51ff91611d929eb424201e1df1b"
    );
    assert_eq!(
        hex(&block_header_bytes(&block.header)),
        "0100000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000f4bf1e8910f238f1b3a5d93cd845c04b43b3db35edc25683be4112ca2d4fcbf3b222e58c3e736e956002a493917b88ee0c27a91d87b3a4eb5d2b053c9e2cb79409090909090909090909090909090909090909090100000000f15365000000000000000000000000"
    );
    assert_eq!(
        hex(&block_bytes(&block)),
        "0100000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000f4bf1e8910f238f1b3a5d93cd845c04b43b3db35edc25683be4112ca2d4fcbf3b222e58c3e736e956002a493917b88ee0c27a91d87b3a4eb5d2b053c9e2cb79409090909090909090909090909090909090909090100000000f15365000000000000000000000000000000000000000000"
    );
    assert_eq!(
        hex(&block.hash().0),
        "9c927974f1f6d79695e224f6822b0e3cb4c3526c53a3117b15219ebb0eb01f4bbc53202fea575a31ca5fcac0d1f8af52de6452cd59feb532e8a9e0a71e285614"
    );
    assert_eq!(
        hex(&GENESIS_HASH),
        "32ac01d654c1fe57d12506456bb7237f4baf214a3573b11fcdb128974d95864f4031856cae53a859c5adc5d2880670739571057b71b2575642e5cce6d16efe1d"
    );
    assert_eq!(GENESIS_HASH, genesis_block().hash().0);
    assert_eq!(
        hex(&state_root_bytes(&state_root)),
        "f4bf1e8910f238f1b3a5d93cd845c04b43b3db35edc25683be4112ca2d4fcbf3b222e58c3e736e956002a493917b88ee0c27a91d87b3a4eb5d2b053c9e2cb794"
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
        Amount(BASE_FEE),
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
        Amount(BASE_FEE),
        Nonce(0),
    );
    assert!(decode_transaction(&transaction_bytes(&same_sender)).is_err());
    assert!(decode_signed_transaction(&[1, 2, 3]).is_err());

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
    crate::invariants::validate_ledger_invariants(&ledger).unwrap();

    let transaction = Transaction::new(from, to, Amount(10), Amount(BASE_FEE), Nonce(0));
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
