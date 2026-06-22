use crate::block::Block;
use crate::error::LedgerError;
use crate::ledger::Ledger;
use crate::ledger::fork_choice::ForkChoice;
use crate::types::BlockHash;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReorgPlan {
    pub ancestor: BlockHash,
    pub old_tip: Option<BlockHash>,
    pub new_tip: BlockHash,
    pub apply: Vec<Block>,
}

pub fn plan_reorg(
    active: &Ledger,
    fork_choice: &ForkChoice,
    new_tip: BlockHash,
) -> Result<ReorgPlan, LedgerError> {
    let old_tip = active.tip_hash();
    let ancestor =
        common_ancestor(old_tip, new_tip, fork_choice).ok_or(LedgerError::InvalidParent)?;
    let apply = fork_choice
        .branch_from_ancestor(ancestor, new_tip)
        .ok_or(LedgerError::InvalidParent)?;

    Ok(ReorgPlan {
        ancestor,
        old_tip,
        new_tip,
        apply,
    })
}

pub fn common_ancestor(
    old_tip: Option<BlockHash>,
    new_tip: BlockHash,
    fork_choice: &ForkChoice,
) -> Option<BlockHash> {
    let old_tip = old_tip?;
    let old_ancestors: std::collections::BTreeSet<_> =
        fork_choice.ancestor_hashes(old_tip).into_iter().collect();

    fork_choice
        .ancestor_hashes(new_tip)
        .into_iter()
        .find(|hash| old_ancestors.contains(hash))
}
