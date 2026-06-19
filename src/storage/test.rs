use super::{StateSnapshot, Storage, StorageError};
use crate::block::Block;
use crate::ledger::Ledger;
use crate::params::STORAGE_VERSION;
use crate::state::Account;
use crate::types::{Address, Amount, Hash, Height, Nonce};

fn address(byte: u8) -> Address {
    Address([byte; 20])
}

fn block(height: u64, previous_hash: Hash) -> Block {
    Block::new(
        Height(height),
        previous_hash,
        address(9),
        1_700_000_000 + height,
        Nonce(0),
        vec![],
    )
}

#[test]
fn stores_and_loads_blocks_by_height_and_hash() {
    let storage = Storage::temporary().unwrap();
    let block = block(0, Hash([0; 64]));
    let hash = block.hash();

    storage.save_block(&block).unwrap();

    assert_eq!(
        storage.load_block_by_height(Height(0)).unwrap(),
        Some(block.clone())
    );
    assert_eq!(storage.load_block_by_hash(&hash).unwrap(), Some(block));
}

#[test]
fn initializes_storage_version_for_empty_database() {
    let storage = Storage::temporary().unwrap();

    assert_eq!(
        storage.load_storage_version().unwrap(),
        Some(STORAGE_VERSION)
    );
}

#[test]
fn rejects_unsupported_storage_version() {
    let storage = Storage::temporary().unwrap();
    storage
        .test_meta()
        .unwrap()
        .insert(
            b"storage_version",
            borsh::to_vec(&STORAGE_VERSION.saturating_add(1))
                .unwrap()
                .as_slice(),
        )
        .unwrap();

    assert!(matches!(
        storage.load_ledger(),
        Err(StorageError::UnsupportedStorageVersion {
            expected: STORAGE_VERSION,
            found
        }) if found == STORAGE_VERSION.saturating_add(1)
    ));
}

#[test]
fn rejects_existing_database_without_storage_version() {
    let storage = Storage::temporary().unwrap();
    storage
        .test_meta()
        .unwrap()
        .remove(b"storage_version")
        .unwrap();
    storage.save_block(&block(0, Hash([0; 64]))).unwrap();

    assert!(matches!(
        storage.load_ledger(),
        Err(StorageError::MissingStorageVersion)
    ));
}

#[test]
fn rejects_block_loaded_from_wrong_height_key() {
    let storage = Storage::temporary().unwrap();
    let block = block(1, Hash([0; 64]));
    let bytes = borsh::to_vec(&block).unwrap();

    storage
        .test_blocks_by_height()
        .unwrap()
        .insert(Height(0).0.to_be_bytes(), bytes)
        .unwrap();

    assert!(matches!(
        storage.load_block_by_height(Height(0)),
        Err(StorageError::Integrity(
            "stored block height does not match height key"
        ))
    ));
}

#[test]
fn rejects_block_loaded_from_wrong_hash_key() {
    let storage = Storage::temporary().unwrap();
    let block = block(0, Hash([0; 64]));
    let bytes = borsh::to_vec(&block).unwrap();
    let wrong_hash = Hash([7; 64]);

    storage
        .test_blocks_by_hash()
        .unwrap()
        .insert(wrong_hash.0.as_slice(), bytes)
        .unwrap();

    assert!(matches!(
        storage.load_block_by_hash(&wrong_hash),
        Err(StorageError::Integrity(
            "stored block hash does not match hash key"
        ))
    ));
}

#[test]
fn stores_and_loads_accounts() {
    let storage = Storage::temporary().unwrap();
    let account = Account::with_nonce(address(1), Amount(25), Nonce(7));

    storage.save_account(&account).unwrap();

    assert_eq!(storage.load_account(&address(1)).unwrap(), Some(account));
    assert_eq!(storage.load_account(&address(2)).unwrap(), None);
}

#[test]
fn stores_and_loads_chain_tip() {
    let storage = Storage::temporary().unwrap();
    let hash = Hash([7; 64]);

    assert_eq!(storage.load_tip().unwrap(), None);

    storage.save_tip(Height(3), &hash).unwrap();

    assert_eq!(storage.load_tip().unwrap(), Some((Height(3), hash)));
}

#[test]
fn validates_stored_chain_integrity() {
    let storage = Storage::temporary().unwrap();
    let genesis = block(0, Hash([0; 64]));
    let next = block(1, genesis.hash());

    storage.save_block(&genesis).unwrap();
    storage.save_block(&next).unwrap();
    storage.save_tip(next.height(), &next.hash()).unwrap();

    assert!(storage.validate_chain_integrity().is_ok());
}

#[test]
fn rejects_chain_integrity_when_tip_block_is_missing() {
    let storage = Storage::temporary().unwrap();

    storage.save_tip(Height(3), &Hash([7; 64])).unwrap();

    assert!(matches!(
        storage.validate_chain_integrity(),
        Err(StorageError::Integrity(
            "stored tip height block is missing"
        ))
    ));
}

#[test]
fn rejects_chain_integrity_when_previous_link_is_broken() {
    let storage = Storage::temporary().unwrap();
    let genesis = block(0, Hash([0; 64]));
    let next = block(1, Hash([9; 64]));

    storage.save_block(&genesis).unwrap();
    storage.save_block(&next).unwrap();
    storage.save_tip(next.height(), &next.hash()).unwrap();

    assert!(matches!(
        storage.validate_chain_integrity(),
        Err(StorageError::Integrity(
            "stored chain block previous hash is broken"
        ))
    ));
}

#[test]
fn stores_ledger_snapshot() {
    let storage = Storage::temporary().unwrap();
    let mut ledger = Ledger::new();
    let mut genesis = block(0, Hash([0; 64]));

    ledger.create_account(address(1), Amount(100)).unwrap();
    genesis.set_state_root(ledger.state_root());
    let hash = genesis.hash();
    ledger.chain.insert_block(genesis.clone()).unwrap();

    storage.save_ledger(&ledger).unwrap();

    assert_eq!(
        storage.load_account(&address(1)).unwrap().unwrap().balance,
        Amount(100)
    );
    assert_eq!(
        storage.load_block_by_height(Height(0)).unwrap(),
        Some(genesis)
    );
    assert_eq!(storage.load_tip().unwrap(), Some((Height(0), hash)));
}

#[test]
fn loads_ledger_snapshot() {
    let storage = Storage::temporary().unwrap();
    let mut ledger = Ledger::new();
    let genesis = block(0, Hash([0; 64]));
    let hash = genesis.hash();

    ledger.create_account(address(1), Amount(100)).unwrap();
    ledger.chain.insert_block(genesis).unwrap();
    storage.save_ledger(&ledger).unwrap();

    let restored = storage.load_ledger().unwrap();

    assert_eq!(restored.balance(&address(1)), Some(Amount(100)));
    assert_eq!(restored.tip_height(), Some(Height(0)));
    assert_eq!(restored.tip_hash(), Some(hash));
}

#[test]
fn stores_and_loads_genesis_accounts() {
    let storage = Storage::temporary().unwrap();
    let mut accounts = std::collections::BTreeMap::new();
    accounts.insert(address(1), Account::new(address(1), Amount(100)));

    storage.save_genesis_accounts(&accounts).unwrap();

    assert_eq!(storage.load_genesis_accounts().unwrap(), Some(accounts));
}

#[test]
fn stores_and_loads_state_snapshot() {
    let storage = Storage::temporary().unwrap();
    let mut ledger = Ledger::new();
    let mut genesis = block(0, Hash([0; 64]));

    ledger.create_account(address(1), Amount(100)).unwrap();
    genesis.set_state_root(ledger.state_root());
    let hash = genesis.hash();
    ledger.chain.insert_block(genesis.clone()).unwrap();
    storage.save_block(&genesis).unwrap();
    storage.save_state_snapshot(&ledger).unwrap();

    let snapshot = storage.load_state_snapshot(Height(0)).unwrap().unwrap();

    assert_eq!(snapshot.height, Height(0));
    assert_eq!(snapshot.block_hash, hash);
    assert_eq!(snapshot.state_root, ledger.state_root());
    assert_eq!(
        snapshot
            .accounts
            .get(&address(1))
            .map(|account| account.balance),
        Some(Amount(100))
    );
    assert!(snapshot.verify_state_root());
    assert!(
        snapshot.verify_against_block(&storage.load_block_by_height(Height(0)).unwrap().unwrap())
    );
}

#[test]
fn rejects_tampered_state_snapshot_root() {
    let storage = Storage::temporary().unwrap();
    let mut accounts = std::collections::BTreeMap::new();
    accounts.insert(address(1), Account::new(address(1), Amount(100)));
    let snapshot = StateSnapshot {
        height: Height(0),
        block_hash: Hash([1; 64]),
        state_root: Hash([9; 64]),
        accounts,
    };

    storage
        .test_state_snapshots()
        .unwrap()
        .insert(Height(0).0.to_be_bytes(), borsh::to_vec(&snapshot).unwrap())
        .unwrap();

    assert!(matches!(
        storage.load_state_snapshot(Height(0)),
        Err(StorageError::Integrity(
            "stored state snapshot root does not match accounts"
        ))
    ));
}
