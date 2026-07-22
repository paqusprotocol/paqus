use crate::consensus::supply::{Amount, XPQ};
use crate::crypto::{
    Address, HashDomain, PublicKey, Signature, TransactionHash, domain_hash, public_key_from_seed,
    sign_from_seed, verify,
};
use crate::genesis::CURRENT_CHAIN_PARAMS;
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt;
use zeroize::Zeroize;

pub const CASH_FILE_MAGIC: [u8; 8] = *b"XPQCASH1";
pub const CASH_FILE_VERSION: u8 = 1;
pub const MAX_CASH_FILE_SIZE: usize = 1024;
pub const MAX_QCASH_WITHDRAW_OUTPUTS: usize = 256;
pub const MAX_QCASH_DEPOSIT_INPUTS: usize = 4;

/// Supported cash denominations, expressed in whole XPQ.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(u16)]
pub enum CashDenomination {
    One = 1,
    Two = 2,
    Five = 5,
    Ten = 10,
    Twenty = 20,
    Fifty = 50,
    OneHundred = 100,
}

impl CashDenomination {
    pub const DESCENDING: [Self; 7] = [
        Self::OneHundred,
        Self::Fifty,
        Self::Twenty,
        Self::Ten,
        Self::Five,
        Self::Two,
        Self::One,
    ];

    pub const fn xpq(self) -> u64 {
        self as u16 as u64
    }

    pub const fn amount(self) -> Amount {
        Amount(self.xpq() * XPQ)
    }
}

impl BorshSerialize for CashDenomination {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        BorshSerialize::serialize(&(self.xpq() as u16), writer)
    }
}

impl BorshDeserialize for CashDenomination {
    fn deserialize_reader<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
        match u16::deserialize_reader(reader)? {
            1 => Ok(Self::One),
            2 => Ok(Self::Two),
            5 => Ok(Self::Five),
            10 => Ok(Self::Ten),
            20 => Ok(Self::Twenty),
            50 => Ok(Self::Fifty),
            100 => Ok(Self::OneHundred),
            value => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("unsupported QCash denomination {value}"),
            )),
        }
    }
}

/// A compact run of identical cash coins.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    BorshSerialize,
    BorshDeserialize,
)]
pub struct CashCoin {
    pub denomination: CashDenomination,
    pub count: u64,
}

/// One consensus-visible output created by a withdraw transaction.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    BorshSerialize,
    BorshDeserialize,
)]
pub struct QCashOutput {
    pub coin_index: u32,
    pub denomination: CashDenomination,
    /// Commitment to wallet-held secret material; the secret is never put on-chain.
    pub commitment: [u8; 32],
}

/// Explicit outputs committed by one withdraw transaction.
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, BorshSerialize, BorshDeserialize,
)]
pub struct WithdrawCashMetadata {
    pub outputs: Vec<QCashOutput>,
}

/// Automatic whole-XPQ cash selection with the unconverted on-chain remainder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutomaticWithdrawalPlan {
    pub requested_amount: Amount,
    pub cash_amount: Amount,
    pub remainder: Amount,
    pub denominations: Vec<CashDenomination>,
}

/// Portable bearer coin data stored by the wallet in a `.XPQ` file.
#[derive(Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct CashCoinFile {
    pub version: u8,
    /// Opaque state lookup key. The originating transaction hash is not stored
    /// in the portable bearer file.
    pub coin_id: [u8; 32],
    pub denomination: CashDenomination,
    pub opening_secret: [u8; 32],
}

impl Drop for CashCoinFile {
    fn drop(&mut self) {
        self.opening_secret.zeroize();
    }
}

impl fmt::Debug for CashCoinFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CashCoinFile")
            .field("version", &self.version)
            .field("coin_id", &self.coin_id)
            .field("denomination", &self.denomination)
            .field("opening_secret", &"[REDACTED]")
            .finish()
    }
}

/// Public proof authorizing exactly one cash coin to be credited to one recipient.
/// The wallet-held opening secret is deliberately absent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, BorshSerialize, BorshDeserialize)]
pub struct DepositCashInput {
    pub version: u8,
    pub coin_id: [u8; 32],
    pub denomination: CashDenomination,
    pub spend_public_key: PublicKey,
    pub authorization: Signature,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, BorshSerialize, BorshDeserialize)]
pub struct DepositCashMetadata {
    pub inputs: Vec<DepositCashInput>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QCashError {
    ZeroAmount,
    FractionalXpQ,
    EmptyCoins,
    ZeroCoinCount,
    NonCanonicalCoins,
    AmountOverflow,
    EmptyOutputs,
    InvalidCoinIndex,
    DuplicateCommitment,
    CommitmentCountMismatch,
    DenominationAmountMismatch,
    NoCashableAmount,
    UnsupportedCashFileVersion,
    EmptyDepositInputs,
    DuplicateDepositInput,
    InvalidCommitment,
    InvalidDepositAuthorization,
    InvalidCashFile,
    CashFileTooLarge,
    TooManyWithdrawOutputs,
    TooManyDepositInputs,
}

impl fmt::Display for QCashError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroAmount => f.write_str("QCash amount must be greater than zero"),
            Self::FractionalXpQ => f.write_str("QCash amount must use whole XPQ units"),
            Self::EmptyCoins => f.write_str("QCash metadata must contain at least one coin"),
            Self::ZeroCoinCount => f.write_str("QCash coin count must be greater than zero"),
            Self::NonCanonicalCoins => {
                f.write_str("QCash coins must be unique and ordered by descending denomination")
            }
            Self::AmountOverflow => f.write_str("QCash amount exceeds the supported amount range"),
            Self::EmptyOutputs => f.write_str("withdraw must contain at least one QCash output"),
            Self::InvalidCoinIndex => {
                f.write_str("QCash output indexes must be contiguous from zero")
            }
            Self::DuplicateCommitment => f.write_str("QCash output commitments must be unique"),
            Self::CommitmentCountMismatch => {
                f.write_str("wallet commitment count does not match cash coin count")
            }
            Self::DenominationAmountMismatch => {
                f.write_str("QCash output denominations do not match withdraw amount")
            }
            Self::NoCashableAmount => {
                f.write_str("requested amount contains less than one whole XPQ for QCash")
            }
            Self::UnsupportedCashFileVersion => {
                f.write_str("cash coin file version is unsupported")
            }
            Self::EmptyDepositInputs => f.write_str("cash deposit must contain at least one input"),
            Self::DuplicateDepositInput => {
                f.write_str("cash deposit contains a duplicate coin reference")
            }
            Self::InvalidCommitment => {
                f.write_str("cash coin spending key does not match commitment")
            }
            Self::InvalidDepositAuthorization => {
                f.write_str("cash deposit authorization is invalid for its recipient")
            }
            Self::InvalidCashFile => f.write_str("cash coin file is malformed or corrupted"),
            Self::CashFileTooLarge => f.write_str("cash coin file exceeds maximum size"),
            Self::TooManyWithdrawOutputs => f.write_str("withdraw creates too many QCash outputs"),
            Self::TooManyDepositInputs => f.write_str("cash deposit contains too many inputs"),
        }
    }
}

pub fn cash_coin_commitment(opening_secret: &[u8; 32]) -> [u8; 32] {
    cash_spend_public_key_commitment(&public_key_from_seed(opening_secret))
}

pub fn cash_spend_public_key_commitment(public_key: &PublicKey) -> [u8; 32] {
    domain_hash(HashDomain::QCashCommitment, &public_key.0).0
}

fn deposit_authorization_bytes(
    coin_id: [u8; 32],
    denomination: CashDenomination,
    recipient: Address,
    transaction_commitment: [u8; 32],
) -> Vec<u8> {
    #[derive(BorshSerialize)]
    struct DepositAuthorizationPayload {
        chain_id: u16,
        protocol_version: u8,
        operation: u8,
        coin_id: [u8; 32],
        denomination: CashDenomination,
        recipient: Address,
        transaction_commitment: [u8; 32],
    }

    let payload = DepositAuthorizationPayload {
        chain_id: CURRENT_CHAIN_PARAMS.chain_id,
        protocol_version: CURRENT_CHAIN_PARAMS.protocol_version,
        operation: 1,
        coin_id,
        denomination,
        recipient,
        transaction_commitment,
    };
    domain_hash(
        HashDomain::QCashDepositAuthorization,
        &crate::codec::canonical_bytes(&payload),
    )
    .0
    .to_vec()
}

/// Derives the opaque identifier shared by consensus state and the bearer file.
pub fn cash_coin_id_bytes(withdraw_tx_hash: TransactionHash, output: &QCashOutput) -> [u8; 32] {
    let payload = crate::codec::canonical_bytes(&(withdraw_tx_hash, output));
    domain_hash(HashDomain::QCashCoin, &payload).0
}

/// Encodes one bearer coin using the only supported `.XPQ` binary format.
pub fn encode_cash_coin_file(file: &CashCoinFile) -> Result<Vec<u8>, QCashError> {
    if file.version != CASH_FILE_VERSION {
        return Err(QCashError::UnsupportedCashFileVersion);
    }
    let payload = crate::codec::canonical_bytes(file);
    let payload_len = u32::try_from(payload.len()).map_err(|_| QCashError::CashFileTooLarge)?;
    let checksum = domain_hash(HashDomain::QCashFile, &payload).0;
    let mut bytes = Vec::with_capacity(8 + 4 + payload.len() + checksum.len());
    bytes.extend_from_slice(&CASH_FILE_MAGIC);
    bytes.extend_from_slice(&payload_len.to_le_bytes());
    bytes.extend_from_slice(&payload);
    bytes.extend_from_slice(&checksum);
    if bytes.len() > MAX_CASH_FILE_SIZE {
        return Err(QCashError::CashFileTooLarge);
    }
    Ok(bytes)
}

/// Strictly decodes and checks a canonical `.XPQ` bearer coin file.
pub fn decode_cash_coin_file(bytes: &[u8]) -> Result<CashCoinFile, QCashError> {
    const PREFIX_LEN: usize = 12;
    const CHECKSUM_LEN: usize = 32;
    if bytes.len() > MAX_CASH_FILE_SIZE || bytes.len() < PREFIX_LEN + CHECKSUM_LEN {
        return Err(if bytes.len() > MAX_CASH_FILE_SIZE {
            QCashError::CashFileTooLarge
        } else {
            QCashError::InvalidCashFile
        });
    }
    if bytes[..8] != CASH_FILE_MAGIC {
        return Err(QCashError::InvalidCashFile);
    }
    let payload_len = u32::from_le_bytes(
        bytes[8..12]
            .try_into()
            .map_err(|_| QCashError::InvalidCashFile)?,
    ) as usize;
    let expected_len = PREFIX_LEN
        .checked_add(payload_len)
        .and_then(|length| length.checked_add(CHECKSUM_LEN))
        .ok_or(QCashError::InvalidCashFile)?;
    if bytes.len() != expected_len {
        return Err(QCashError::InvalidCashFile);
    }

    let payload = &bytes[PREFIX_LEN..PREFIX_LEN + payload_len];
    let checksum = &bytes[PREFIX_LEN + payload_len..];
    if checksum != domain_hash(HashDomain::QCashFile, payload).0 {
        return Err(QCashError::InvalidCashFile);
    }
    let file: CashCoinFile =
        crate::codec::canonical_deserialize(payload).map_err(|_| QCashError::InvalidCashFile)?;
    if file.version != CASH_FILE_VERSION || crate::codec::canonical_bytes(&file) != payload {
        return Err(QCashError::InvalidCashFile);
    }
    Ok(file)
}

impl CashCoinFile {
    pub fn new(
        withdraw_tx_hash: TransactionHash,
        output: &QCashOutput,
        opening_secret: [u8; 32],
    ) -> Result<Self, QCashError> {
        let file = Self {
            version: CASH_FILE_VERSION,
            coin_id: cash_coin_id_bytes(withdraw_tx_hash, output),
            denomination: output.denomination,
            opening_secret,
        };
        if cash_coin_commitment(&file.opening_secret) != output.commitment {
            return Err(QCashError::InvalidCommitment);
        }
        Ok(file)
    }

    pub fn deposit_input_for_transaction(
        &self,
        recipient: Address,
        transaction_commitment: [u8; 32],
    ) -> Result<DepositCashInput, QCashError> {
        if self.version != CASH_FILE_VERSION {
            return Err(QCashError::UnsupportedCashFileVersion);
        }
        Ok(DepositCashInput::authorize(
            self.coin_id,
            self.denomination,
            &self.opening_secret,
            recipient,
            transaction_commitment,
        ))
    }

    pub fn commitment(&self) -> [u8; 32] {
        cash_coin_commitment(&self.opening_secret)
    }
}

impl DepositCashMetadata {
    pub fn from_inputs(inputs: Vec<DepositCashInput>) -> Result<Self, QCashError> {
        let metadata = Self { inputs };
        metadata.validate()?;
        Ok(metadata)
    }

    pub fn new_for_transaction(
        files: &[CashCoinFile],
        recipient: Address,
        transaction_commitment: [u8; 32],
    ) -> Result<Self, QCashError> {
        let inputs = files
            .iter()
            .map(|file| file.deposit_input_for_transaction(recipient, transaction_commitment))
            .collect::<Result<Vec<_>, _>>()?;
        let metadata = Self { inputs };
        metadata.validate()?;
        Ok(metadata)
    }

    pub fn validate_authorizations_for_transaction(
        &self,
        recipient: Address,
        transaction_commitment: [u8; 32],
    ) -> Result<(), QCashError> {
        self.validate()?;
        for input in &self.inputs {
            let message = deposit_authorization_bytes(
                input.coin_id,
                input.denomination,
                recipient,
                transaction_commitment,
            );
            if !verify(&input.spend_public_key, &message, &input.authorization) {
                return Err(QCashError::InvalidDepositAuthorization);
            }
        }
        Ok(())
    }

    pub fn validate(&self) -> Result<(), QCashError> {
        use std::collections::BTreeSet;
        if self.inputs.is_empty() {
            return Err(QCashError::EmptyDepositInputs);
        }
        if self.inputs.len() > MAX_QCASH_DEPOSIT_INPUTS {
            return Err(QCashError::TooManyDepositInputs);
        }
        let mut references = BTreeSet::new();
        for input in &self.inputs {
            if input.version != CASH_FILE_VERSION {
                return Err(QCashError::UnsupportedCashFileVersion);
            }
            if !references.insert(input.coin_id) {
                return Err(QCashError::DuplicateDepositInput);
            }
        }
        self.amount().map(|_| ())
    }

    pub fn amount(&self) -> Result<Amount, QCashError> {
        self.inputs.iter().try_fold(Amount(0), |total, input| {
            total
                .0
                .checked_add(input.denomination.amount().0)
                .map(Amount)
                .ok_or(QCashError::AmountOverflow)
        })
    }
}

impl DepositCashInput {
    /// Builds the public authorization consumed by consensus. Wallet layers
    /// call this after decoding their private bearer-file format.
    pub fn authorize(
        coin_id: [u8; 32],
        denomination: CashDenomination,
        opening_secret: &[u8; 32],
        recipient: Address,
        transaction_commitment: [u8; 32],
    ) -> Self {
        let spend_public_key = public_key_from_seed(opening_secret);
        let message =
            deposit_authorization_bytes(coin_id, denomination, recipient, transaction_commitment);
        Self {
            version: CASH_FILE_VERSION,
            coin_id,
            denomination,
            spend_public_key,
            authorization: sign_from_seed(opening_secret, &message),
        }
    }

    pub fn commitment(&self) -> [u8; 32] {
        cash_spend_public_key_commitment(&self.spend_public_key)
    }
}

impl WithdrawCashMetadata {
    /// Plans automatic denomination selection. Fractions remain on-chain.
    pub fn plan_automatic(amount: Amount) -> Result<AutomaticWithdrawalPlan, QCashError> {
        let cash_amount = Amount(amount.0 - (amount.0 % XPQ));
        let remainder = Amount(amount.0 % XPQ);
        if cash_amount.0 == 0 {
            return Err(QCashError::NoCashableAmount);
        }

        let runs = format_cash_coins(cash_amount)?;
        let mut denominations = Vec::new();
        for run in runs {
            let count = usize::try_from(run.count).map_err(|_| QCashError::AmountOverflow)?;
            if denominations.len().saturating_add(count) > MAX_QCASH_WITHDRAW_OUTPUTS {
                return Err(QCashError::TooManyWithdrawOutputs);
            }
            denominations.extend(std::iter::repeat_n(run.denomination, count));
        }
        Ok(AutomaticWithdrawalPlan {
            requested_amount: amount,
            cash_amount,
            remainder,
            denominations,
        })
    }

    pub fn from_automatic_plan(
        plan: &AutomaticWithdrawalPlan,
        commitments: &[[u8; 32]],
    ) -> Result<Self, QCashError> {
        Self::with_denominations(plan.cash_amount, &plan.denominations, commitments)
    }

    pub fn new(amount: Amount, commitments: &[[u8; 32]]) -> Result<Self, QCashError> {
        let runs = format_cash_coins(amount)?;
        let coin_count = runs.iter().try_fold(0u64, |total, run| {
            total
                .checked_add(run.count)
                .ok_or(QCashError::AmountOverflow)
        })?;
        if coin_count != commitments.len() as u64 {
            return Err(QCashError::CommitmentCountMismatch);
        }
        let coin_count = usize::try_from(coin_count).map_err(|_| QCashError::AmountOverflow)?;
        if coin_count > MAX_QCASH_WITHDRAW_OUTPUTS {
            return Err(QCashError::TooManyWithdrawOutputs);
        }

        let mut outputs = Vec::with_capacity(commitments.len());
        let mut coin_index = 0u32;
        for run in runs {
            for _ in 0..run.count {
                outputs.push(QCashOutput {
                    coin_index,
                    denomination: run.denomination,
                    commitment: commitments[coin_index as usize],
                });
                coin_index = coin_index
                    .checked_add(1)
                    .ok_or(QCashError::AmountOverflow)?;
            }
        }
        let metadata = Self { outputs };
        metadata.validate()?;
        Ok(metadata)
    }

    pub fn with_denominations(
        amount: Amount,
        denominations: &[CashDenomination],
        commitments: &[[u8; 32]],
    ) -> Result<Self, QCashError> {
        if denominations.len() != commitments.len() {
            return Err(QCashError::CommitmentCountMismatch);
        }
        if denominations.len() > MAX_QCASH_WITHDRAW_OUTPUTS {
            return Err(QCashError::TooManyWithdrawOutputs);
        }
        let outputs = denominations
            .iter()
            .copied()
            .zip(commitments.iter().copied())
            .enumerate()
            .map(|(coin_index, (denomination, commitment))| QCashOutput {
                coin_index: coin_index as u32,
                denomination,
                commitment,
            })
            .collect();
        let metadata = Self { outputs };
        metadata.validate_amount(amount)?;
        Ok(metadata)
    }

    pub fn validate(&self) -> Result<(), QCashError> {
        use std::collections::BTreeSet;

        if self.outputs.is_empty() {
            return Err(QCashError::EmptyOutputs);
        }
        if self.outputs.len() > MAX_QCASH_WITHDRAW_OUTPUTS {
            return Err(QCashError::TooManyWithdrawOutputs);
        }
        let mut commitments = BTreeSet::new();
        for (index, output) in self.outputs.iter().enumerate() {
            if output.coin_index as usize != index {
                return Err(QCashError::InvalidCoinIndex);
            }
            if !commitments.insert(output.commitment) {
                return Err(QCashError::DuplicateCommitment);
            }
            if index > 0 && output.denomination > self.outputs[index - 1].denomination {
                return Err(QCashError::NonCanonicalCoins);
            }
        }
        self.amount().map(|_| ())
    }

    pub fn amount(&self) -> Result<Amount, QCashError> {
        self.outputs.iter().try_fold(Amount(0), |total, output| {
            total
                .0
                .checked_add(output.denomination.amount().0)
                .map(Amount)
                .ok_or(QCashError::AmountOverflow)
        })
    }

    pub fn validate_amount(&self, expected: Amount) -> Result<(), QCashError> {
        self.validate()?;
        if self.amount()? != expected {
            return Err(QCashError::DenominationAmountMismatch);
        }
        Ok(())
    }
}

impl Error for QCashError {}

/// Formats a whole-XPQ amount using the fewest supported coins.
pub fn format_cash_coins(amount: Amount) -> Result<Vec<CashCoin>, QCashError> {
    if amount.0 == 0 {
        return Err(QCashError::ZeroAmount);
    }
    if !amount.0.is_multiple_of(XPQ) {
        return Err(QCashError::FractionalXpQ);
    }

    let mut remaining = amount.0 / XPQ;
    let mut coins = Vec::with_capacity(CashDenomination::DESCENDING.len());
    for denomination in CashDenomination::DESCENDING {
        let count = remaining / denomination.xpq();
        if count > 0 {
            coins.push(CashCoin {
                denomination,
                count,
            });
            remaining %= denomination.xpq();
        }
    }
    Ok(coins)
}
