use crate::params::{SNAPSHOT_INTERVAL, SNAPSHOT_MIN_CONFIRMATIONS};
use crate::snapshot::{is_snapshot_finalized, is_snapshot_height, snapshot_root};
use crate::types::{BlockHash, Hash, Height, StateRoot};

#[test]
fn snapshot_root_is_deterministic_and_domain_separated() {
    let root = snapshot_root(
        Height(100),
        BlockHash([1; 64]),
        StateRoot([2; 64]),
        Hash([3; 64]),
    );

    assert_eq!(
        root,
        snapshot_root(
            Height(100),
            BlockHash([1; 64]),
            StateRoot([2; 64]),
            Hash([3; 64])
        )
    );
    assert_ne!(
        root,
        snapshot_root(
            Height(101),
            BlockHash([1; 64]),
            StateRoot([2; 64]),
            Hash([3; 64])
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
