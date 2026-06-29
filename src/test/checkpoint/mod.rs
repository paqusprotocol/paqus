use crate::checkpoint::{checkpoint_for_height, validate_checkpoint};
use crate::params::{CHECKPOINT_INTERVAL, SNAPSHOT_INTERVAL};
use crate::types::{BlockHash, Height};

#[test]
fn accepts_blocks_when_no_checkpoint_exists_for_height() {
    assert_eq!(checkpoint_for_height(Height(123)), None);
    assert!(validate_checkpoint(Height(123), BlockHash([7; 64])));
}

#[test]
fn checkpoint_interval_matches_snapshot_interval() {
    assert_eq!(CHECKPOINT_INTERVAL, SNAPSHOT_INTERVAL);
}
