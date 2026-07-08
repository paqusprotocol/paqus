use crate::block::Height;
use crate::crypto::{BlockHash, Hash, StateRoot};
use crate::snapshot::{SNAPSHOT_INTERVAL, SNAPSHOT_MIN_CONFIRMATIONS};
use crate::snapshot::{is_snapshot_finalized, is_snapshot_height, snapshot_root};

#[test]
fn snapshot_root_is_deterministic_and_domain_separated() {
    let root = snapshot_root(
        Height(100),
        BlockHash([1; crate::crypto::HASH_SIZE]),
        StateRoot([2; crate::crypto::HASH_SIZE]),
        Hash([3; crate::crypto::HASH_SIZE]),
    );

    assert_eq!(
        root,
        snapshot_root(
            Height(100),
            BlockHash([1; crate::crypto::HASH_SIZE]),
            StateRoot([2; crate::crypto::HASH_SIZE]),
            Hash([3; crate::crypto::HASH_SIZE])
        )
    );
    assert_ne!(
        root,
        snapshot_root(
            Height(101),
            BlockHash([1; crate::crypto::HASH_SIZE]),
            StateRoot([2; crate::crypto::HASH_SIZE]),
            Hash([3; crate::crypto::HASH_SIZE])
        )
    );
}

#[test]
fn snapshot_height_and_finality_rules_are_explicit() {
    assert!(!is_snapshot_height(Height(0)));
    assert!(!is_snapshot_height(Height(SNAPSHOT_INTERVAL - 1)));
    assert!(is_snapshot_height(Height(SNAPSHOT_INTERVAL)));

    let snapshot_height = Height(SNAPSHOT_INTERVAL);
    assert!(!is_snapshot_finalized(
        snapshot_height,
        Height(snapshot_height.0 + SNAPSHOT_MIN_CONFIRMATIONS as u64 - 1)
    ));
    assert!(is_snapshot_finalized(
        snapshot_height,
        Height(snapshot_height.0 + SNAPSHOT_MIN_CONFIRMATIONS as u64)
    ));
}
