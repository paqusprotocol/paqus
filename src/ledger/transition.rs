use crate::block::Block;
use crate::block::BlockHeight;
use crate::consensus::supply::Amount;
use crate::crypto::Address;
use crate::crypto::{BlockHash, StateRoot, TransactionHash};
use crate::event::{ProtocolEvent, ProtocolEventKind};
use crate::ledger::{CONFIRMATION_DEPTH, Ledger, LedgerError};
use crate::state::Account;
use crate::transaction::{EcashTransactionKind, SignedTransaction, Transaction};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TransactionExecution {
    pub transaction_hash: TransactionHash,
    pub from: crate::crypto::Address,
    pub to: crate::crypto::Address,
    pub amount: Amount,
    pub fee: Amount,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockExecution {
    pub block_hash: BlockHash,
    pub height: BlockHeight,
    pub state_root_before: StateRoot,
    pub state_root_after: StateRoot,
    pub transactions: Vec<TransactionExecution>,
}

impl TransactionExecution {
    pub fn from_signed(transaction: &SignedTransaction) -> Self {
        Self::from_payload(transaction.hash(), &transaction.transaction)
    }

    pub fn from_payload(transaction_hash: TransactionHash, transaction: &Transaction) -> Self {
        Self {
            transaction_hash,
            from: transaction.from,
            to: transaction.to,
            amount: transaction.amount,
            fee: transaction.fee,
        }
    }
}

pub(crate) fn apply_transaction_to_state(
    accounts: &mut BTreeMap<Address, Account>,
    transaction: &Transaction,
    height: BlockHeight,
) -> Result<(), LedgerError> {
    transaction.validate_for_height(height)?;
    if !accounts.contains_key(&transaction.from) {
        return Err(LedgerError::AccountNotFound);
    }

    {
        let sender = accounts
            .get_mut(&transaction.from)
            .ok_or(LedgerError::AccountNotFound)?;
        sender.apply_outgoing_transaction(transaction, height)?;
    }

    let spendable_height = crate::block::Height(height.0.saturating_add(CONFIRMATION_DEPTH as u64));
    let receiver = accounts
        .entry(transaction.to)
        .or_insert_with(|| Account::new(transaction.to, Amount(0)));
    receiver.apply_incoming_transaction(transaction, spendable_height)?;

    Ok(())
}

pub fn validate_transaction_against_state(
    accounts: &BTreeMap<Address, Account>,
    transaction: &Transaction,
    height: BlockHeight,
) -> Result<(), LedgerError> {
    let mut staged = accounts.clone();
    apply_transaction_to_state(&mut staged, transaction, height)
}

pub fn validate_signed_transaction_against_state(
    accounts: &BTreeMap<Address, Account>,
    transaction: &SignedTransaction,
    height: BlockHeight,
) -> Result<(), LedgerError> {
    transaction
        .validate_signed_for_height(height)
        .map_err(LedgerError::from)?;
    validate_transaction_against_state(accounts, &transaction.transaction, height)
}

impl Ledger {
    pub fn events_for_block(&self, block_hash: &BlockHash) -> &[ProtocolEvent] {
        self.events_by_block
            .get(block_hash)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn event(&self, id: crate::event::EventId) -> Option<&ProtocolEvent> {
        self.events_by_block
            .values()
            .flatten()
            .find(|event| event.id() == id)
    }

    pub(crate) fn record_protocol_events(&mut self, block: &Block) {
        let height = block.height();
        let block_hash = block.hash();
        let mut events = Vec::with_capacity(
            block.transaction_count()
                + block.genesis_allocations.len()
                + usize::from(!block.is_genesis()),
        );
        let mut emit = |transaction_hash, kind| {
            let event_index = u32::try_from(events.len())
                .expect("a valid block cannot contain more than u32::MAX events");
            events.push(ProtocolEvent::new(
                height,
                block_hash,
                transaction_hash,
                event_index,
                kind,
            ));
        };

        for signed in &block.transactions {
            let tx = &signed.transaction;
            emit(
                Some(tx.hash()),
                ProtocolEventKind::Transfer {
                    from: tx.from,
                    to: tx.to,
                    amount: tx.amount,
                    fee: tx.fee,
                },
            );
        }
        for signed in &block.ecash_transactions {
            let tx = &signed.transaction;
            let kind = match &tx.kind {
                EcashTransactionKind::WithdrawCash { amount, .. } => {
                    ProtocolEventKind::EcashWithdrawn {
                        signer: tx.signer,
                        amount: *amount,
                    }
                }
                EcashTransactionKind::DepositCash {
                    recipient,
                    metadata,
                } => ProtocolEventKind::EcashDeposited {
                    signer: tx.signer,
                    recipient: *recipient,
                    amount: metadata
                        .amount()
                        .expect("an applied eCash deposit has a valid amount"),
                },
            };
            emit(Some(tx.hash()), kind);
        }
        if block.is_genesis() {
            for allocation in &block.genesis_allocations {
                emit(
                    None,
                    ProtocolEventKind::GenesisAllocation {
                        recipient: allocation.to,
                        amount: allocation.amount,
                    },
                );
            }
        } else if let Some(coinbase) = &block.coinbase {
            emit(
                None,
                ProtocolEventKind::CoinbasePaid {
                    miner: coinbase.to,
                    subsidy: coinbase.subsidy,
                    fees: coinbase.fees,
                },
            );
        }

        self.events_by_block.insert(block_hash, events);
    }

    pub(crate) fn apply_transaction_at(
        &mut self,
        transaction: &Transaction,
        height: BlockHeight,
    ) -> Result<(), LedgerError> {
        apply_transaction_to_state(&mut self.accounts, transaction, height)?;
        self.refresh_account_state(&transaction.from);
        self.refresh_account_state(&transaction.to);
        Ok(())
    }

    pub fn validate_transaction_against_state(
        &self,
        transaction: &Transaction,
        height: BlockHeight,
    ) -> Result<(), LedgerError> {
        validate_transaction_against_state(&self.accounts, transaction, height)
    }

    pub fn validate_signed_transaction_against_state(
        &self,
        transaction: &SignedTransaction,
        height: BlockHeight,
    ) -> Result<(), LedgerError> {
        validate_signed_transaction_against_state(&self.accounts, transaction, height)
    }

    pub fn validate_block(&self, block: &Block) -> Result<StateRoot, LedgerError> {
        self.staged_after_validated_block(block)
            .map(|(_, expected_state_root)| expected_state_root)
    }

    pub fn execute_block(&self, block: &Block) -> Result<(Ledger, BlockExecution), LedgerError> {
        let state_root_before = self.state_root();
        let (mut staged, expected_state_root) = self.staged_after_validated_block(block)?;
        let mut committed_block = block.clone();
        if committed_block.state_root() == crate::crypto::Hash([0; crate::crypto::HASH_SIZE]) {
            committed_block.set_state_root(expected_state_root);
        }
        let block_hash = committed_block.hash();
        staged.record_protocol_events(&committed_block);
        staged.chain.insert_block(committed_block)?;

        let execution = BlockExecution {
            block_hash,
            height: block.height(),
            state_root_before,
            state_root_after: expected_state_root,
            transactions: block
                .transactions
                .iter()
                .map(TransactionExecution::from_signed)
                .collect(),
        };

        Ok((staged, execution))
    }

    pub(crate) fn staged_after_validated_block(
        &self,
        block: &Block,
    ) -> Result<(Self, StateRoot), LedgerError> {
        block.validate()?;
        self.chain.validate_next_block(block)?;

        let mut staged = self.clone();

        for transaction in &block.transactions {
            staged.apply_transaction_at(&transaction.transaction, block.height())?;
        }

        let block_hash = block.hash();
        for transaction in &block.ecash_transactions {
            staged.apply_signed_ecash_transaction_in_block(
                transaction,
                block.height(),
                block_hash,
            )?;
        }
        if block.is_genesis() {
            for allocation in &block.genesis_allocations {
                staged.create_account(allocation.to, allocation.amount)?;
            }
        } else {
            staged.apply_coinbase(block)?;
        }

        let expected_state_root = if block.is_genesis() {
            staged.state_root()
        } else {
            staged.protocol_state_root()
        };
        if !block.is_genesis()
            && block.state_root() != StateRoot::ZERO
            && block.state_root() != expected_state_root
        {
            return Err(LedgerError::InvalidStateRoot);
        }

        staged.validate_supply()?;
        Ok((staged, expected_state_root))
    }
}
