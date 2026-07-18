//! Deterministic metadata primitives for representing XPQ as eCash coins.

use crate::consensus::supply::{Amount, XPQ};
use crate::crypto::{
    Address, HashDomain, PublicKey, Signature, TransactionHash, domain_hash, public_key_from_seed,
    sign_from_seed, verify,
};
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt;

pub const CASH_FILE_MAGIC: [u8; 8] = *b"XPQCASH1";
pub const CASH_FILE_VERSION: u8 = 1;
pub const MAX_CASH_FILE_SIZE: usize = 1024;

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
                format!("unsupported eCash denomination {value}"),
            )),
        }
    }
}

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
pub enum EcashOperation {
    Deposit,
    Withdraw,
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

/// Canonical eCash metadata. Coin runs must be unique and sorted largest first.
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, BorshSerialize, BorshDeserialize,
)]
pub struct EcashMetadata {
    pub operation: EcashOperation,
    pub coins: Vec<CashCoin>,
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
pub struct EcashOutput {
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
    pub outputs: Vec<EcashOutput>,
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
pub struct CashCoinFile {
    pub version: u8,
    /// Opaque state lookup key. The originating transaction hash is not stored
    /// in the portable bearer file.
    pub coin_id: [u8; 32],
    pub denomination: CashDenomination,
    pub opening_secret: [u8; 32],
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
pub enum EcashError {
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
}

impl fmt::Display for EcashError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroAmount => f.write_str("eCash amount must be greater than zero"),
            Self::FractionalXpQ => f.write_str("eCash amount must use whole XPQ units"),
            Self::EmptyCoins => f.write_str("eCash metadata must contain at least one coin"),
            Self::ZeroCoinCount => f.write_str("eCash coin count must be greater than zero"),
            Self::NonCanonicalCoins => {
                f.write_str("eCash coins must be unique and ordered by descending denomination")
            }
            Self::AmountOverflow => f.write_str("eCash amount exceeds the supported amount range"),
            Self::EmptyOutputs => f.write_str("withdraw must contain at least one eCash output"),
            Self::InvalidCoinIndex => {
                f.write_str("eCash output indexes must be contiguous from zero")
            }
            Self::DuplicateCommitment => f.write_str("eCash output commitments must be unique"),
            Self::CommitmentCountMismatch => {
                f.write_str("wallet commitment count does not match cash coin count")
            }
            Self::DenominationAmountMismatch => {
                f.write_str("eCash output denominations do not match withdraw amount")
            }
            Self::NoCashableAmount => {
                f.write_str("requested amount contains less than one whole XPQ for eCash")
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
        }
    }
}

pub fn cash_coin_commitment(opening_secret: &[u8; 32]) -> [u8; 32] {
    cash_spend_public_key_commitment(&public_key_from_seed(opening_secret))
}

pub fn cash_spend_public_key_commitment(public_key: &PublicKey) -> [u8; 32] {
    domain_hash(HashDomain::EcashCommitment, &public_key.0).0
}

fn deposit_authorization_bytes(
    coin_id: [u8; 32],
    denomination: CashDenomination,
    recipient: Address,
) -> Vec<u8> {
    let payload = crate::codec::canonical_bytes(&(coin_id, denomination, recipient));
    let mut bytes = Vec::with_capacity(32 + payload.len());
    bytes.extend_from_slice(b"PAQUS_ECASH_DEPOSIT_AUTH_V1");
    bytes.extend_from_slice(&payload);
    bytes
}

/// Derives the opaque identifier shared by consensus state and the bearer file.
pub fn cash_coin_id_bytes(withdraw_tx_hash: TransactionHash, output: &EcashOutput) -> [u8; 32] {
    let payload = crate::codec::canonical_bytes(&(withdraw_tx_hash, output));
    domain_hash(HashDomain::EcashCoin, &payload).0
}

/// Encodes one bearer coin using the only supported `.XPQ` binary format.
pub fn encode_cash_coin_file(file: &CashCoinFile) -> Result<Vec<u8>, EcashError> {
    if file.version != CASH_FILE_VERSION {
        return Err(EcashError::UnsupportedCashFileVersion);
    }
    let payload = crate::codec::canonical_bytes(file);
    let payload_len = u32::try_from(payload.len()).map_err(|_| EcashError::CashFileTooLarge)?;
    let checksum = domain_hash(HashDomain::EcashFile, &payload).0;
    let mut bytes = Vec::with_capacity(8 + 4 + payload.len() + checksum.len());
    bytes.extend_from_slice(&CASH_FILE_MAGIC);
    bytes.extend_from_slice(&payload_len.to_le_bytes());
    bytes.extend_from_slice(&payload);
    bytes.extend_from_slice(&checksum);
    if bytes.len() > MAX_CASH_FILE_SIZE {
        return Err(EcashError::CashFileTooLarge);
    }
    Ok(bytes)
}

/// Strictly decodes and checks a canonical `.XPQ` bearer coin file.
pub fn decode_cash_coin_file(bytes: &[u8]) -> Result<CashCoinFile, EcashError> {
    const PREFIX_LEN: usize = 12;
    const CHECKSUM_LEN: usize = 32;
    if bytes.len() > MAX_CASH_FILE_SIZE || bytes.len() < PREFIX_LEN + CHECKSUM_LEN {
        return Err(if bytes.len() > MAX_CASH_FILE_SIZE {
            EcashError::CashFileTooLarge
        } else {
            EcashError::InvalidCashFile
        });
    }
    if bytes[..8] != CASH_FILE_MAGIC {
        return Err(EcashError::InvalidCashFile);
    }
    let payload_len = u32::from_le_bytes(
        bytes[8..12]
            .try_into()
            .map_err(|_| EcashError::InvalidCashFile)?,
    ) as usize;
    let expected_len = PREFIX_LEN
        .checked_add(payload_len)
        .and_then(|length| length.checked_add(CHECKSUM_LEN))
        .ok_or(EcashError::InvalidCashFile)?;
    if bytes.len() != expected_len {
        return Err(EcashError::InvalidCashFile);
    }

    let payload = &bytes[PREFIX_LEN..PREFIX_LEN + payload_len];
    let checksum = &bytes[PREFIX_LEN + payload_len..];
    if checksum != domain_hash(HashDomain::EcashFile, payload).0 {
        return Err(EcashError::InvalidCashFile);
    }
    let file: CashCoinFile =
        crate::codec::canonical_deserialize(payload).map_err(|_| EcashError::InvalidCashFile)?;
    if file.version != CASH_FILE_VERSION || crate::codec::canonical_bytes(&file) != payload {
        return Err(EcashError::InvalidCashFile);
    }
    Ok(file)
}

impl CashCoinFile {
    pub fn new(
        withdraw_tx_hash: TransactionHash,
        output: &EcashOutput,
        opening_secret: [u8; 32],
    ) -> Result<Self, EcashError> {
        let file = Self {
            version: CASH_FILE_VERSION,
            coin_id: cash_coin_id_bytes(withdraw_tx_hash, output),
            denomination: output.denomination,
            opening_secret,
        };
        if cash_coin_commitment(&file.opening_secret) != output.commitment {
            return Err(EcashError::InvalidCommitment);
        }
        Ok(file)
    }

    pub fn deposit_input(&self, recipient: Address) -> Result<DepositCashInput, EcashError> {
        if self.version != CASH_FILE_VERSION {
            return Err(EcashError::UnsupportedCashFileVersion);
        }
        let spend_public_key = public_key_from_seed(&self.opening_secret);
        let message = deposit_authorization_bytes(self.coin_id, self.denomination, recipient);
        Ok(DepositCashInput {
            version: self.version,
            coin_id: self.coin_id,
            denomination: self.denomination,
            spend_public_key,
            authorization: sign_from_seed(&self.opening_secret, &message),
        })
    }

    pub fn commitment(&self) -> [u8; 32] {
        cash_coin_commitment(&self.opening_secret)
    }
}

impl DepositCashMetadata {
    pub fn from_inputs(inputs: Vec<DepositCashInput>) -> Result<Self, EcashError> {
        let metadata = Self { inputs };
        metadata.validate()?;
        Ok(metadata)
    }

    pub fn new(files: &[CashCoinFile], recipient: Address) -> Result<Self, EcashError> {
        let inputs = files
            .iter()
            .map(|file| file.deposit_input(recipient))
            .collect::<Result<Vec<_>, _>>()?;
        let metadata = Self { inputs };
        metadata.validate()?;
        Ok(metadata)
    }

    pub fn validate_authorizations(&self, recipient: Address) -> Result<(), EcashError> {
        self.validate()?;
        for input in &self.inputs {
            let message = deposit_authorization_bytes(input.coin_id, input.denomination, recipient);
            if !verify(&input.spend_public_key, &message, &input.authorization) {
                return Err(EcashError::InvalidDepositAuthorization);
            }
        }
        Ok(())
    }

    pub fn validate(&self) -> Result<(), EcashError> {
        use std::collections::BTreeSet;
        if self.inputs.is_empty() {
            return Err(EcashError::EmptyDepositInputs);
        }
        let mut references = BTreeSet::new();
        for input in &self.inputs {
            if input.version != CASH_FILE_VERSION {
                return Err(EcashError::UnsupportedCashFileVersion);
            }
            if !references.insert(input.coin_id) {
                return Err(EcashError::DuplicateDepositInput);
            }
        }
        self.amount().map(|_| ())
    }

    pub fn amount(&self) -> Result<Amount, EcashError> {
        self.inputs.iter().try_fold(Amount(0), |total, input| {
            total
                .0
                .checked_add(input.denomination.amount().0)
                .map(Amount)
                .ok_or(EcashError::AmountOverflow)
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
    ) -> Self {
        let spend_public_key = public_key_from_seed(opening_secret);
        let message = deposit_authorization_bytes(coin_id, denomination, recipient);
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
    pub fn plan_automatic(amount: Amount) -> Result<AutomaticWithdrawalPlan, EcashError> {
        let cash_amount = Amount(amount.0 - (amount.0 % XPQ));
        let remainder = Amount(amount.0 % XPQ);
        if cash_amount.0 == 0 {
            return Err(EcashError::NoCashableAmount);
        }

        let runs = format_cash_coins(cash_amount)?;
        let mut denominations = Vec::new();
        for run in runs {
            let count = usize::try_from(run.count).map_err(|_| EcashError::AmountOverflow)?;
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
    ) -> Result<Self, EcashError> {
        Self::with_denominations(plan.cash_amount, &plan.denominations, commitments)
    }

    pub fn new(amount: Amount, commitments: &[[u8; 32]]) -> Result<Self, EcashError> {
        let runs = format_cash_coins(amount)?;
        let coin_count = runs.iter().try_fold(0u64, |total, run| {
            total
                .checked_add(run.count)
                .ok_or(EcashError::AmountOverflow)
        })?;
        if coin_count != commitments.len() as u64 {
            return Err(EcashError::CommitmentCountMismatch);
        }

        let mut outputs = Vec::with_capacity(commitments.len());
        let mut coin_index = 0u32;
        for run in runs {
            for _ in 0..run.count {
                outputs.push(EcashOutput {
                    coin_index,
                    denomination: run.denomination,
                    commitment: commitments[coin_index as usize],
                });
                coin_index = coin_index
                    .checked_add(1)
                    .ok_or(EcashError::AmountOverflow)?;
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
    ) -> Result<Self, EcashError> {
        if denominations.len() != commitments.len() {
            return Err(EcashError::CommitmentCountMismatch);
        }
        let outputs = denominations
            .iter()
            .copied()
            .zip(commitments.iter().copied())
            .enumerate()
            .map(|(coin_index, (denomination, commitment))| EcashOutput {
                coin_index: coin_index as u32,
                denomination,
                commitment,
            })
            .collect();
        let metadata = Self { outputs };
        metadata.validate_amount(amount)?;
        Ok(metadata)
    }

    pub fn validate(&self) -> Result<(), EcashError> {
        use std::collections::BTreeSet;

        if self.outputs.is_empty() {
            return Err(EcashError::EmptyOutputs);
        }
        let mut commitments = BTreeSet::new();
        for (index, output) in self.outputs.iter().enumerate() {
            if output.coin_index as usize != index {
                return Err(EcashError::InvalidCoinIndex);
            }
            if !commitments.insert(output.commitment) {
                return Err(EcashError::DuplicateCommitment);
            }
            if index > 0 && output.denomination > self.outputs[index - 1].denomination {
                return Err(EcashError::NonCanonicalCoins);
            }
        }
        self.amount().map(|_| ())
    }

    pub fn amount(&self) -> Result<Amount, EcashError> {
        self.outputs.iter().try_fold(Amount(0), |total, output| {
            total
                .0
                .checked_add(output.denomination.amount().0)
                .map(Amount)
                .ok_or(EcashError::AmountOverflow)
        })
    }

    pub fn validate_amount(&self, expected: Amount) -> Result<(), EcashError> {
        self.validate()?;
        if self.amount()? != expected {
            return Err(EcashError::DenominationAmountMismatch);
        }
        Ok(())
    }
}

impl Error for EcashError {}

impl EcashMetadata {
    pub fn new(operation: EcashOperation, amount: Amount) -> Result<Self, EcashError> {
        Ok(Self {
            operation,
            coins: format_cash_coins(amount)?,
        })
    }

    pub fn deposit(amount: Amount) -> Result<Self, EcashError> {
        Self::new(EcashOperation::Deposit, amount)
    }

    pub fn withdraw(amount: Amount) -> Result<Self, EcashError> {
        Self::new(EcashOperation::Withdraw, amount)
    }

    pub fn validate(&self) -> Result<(), EcashError> {
        if self.coins.is_empty() {
            return Err(EcashError::EmptyCoins);
        }

        let mut previous = None;
        for coin in &self.coins {
            if coin.count == 0 {
                return Err(EcashError::ZeroCoinCount);
            }
            if previous.is_some_and(|value| coin.denomination >= value) {
                return Err(EcashError::NonCanonicalCoins);
            }
            previous = Some(coin.denomination);
        }

        self.amount().map(|_| ())
    }

    pub fn amount(&self) -> Result<Amount, EcashError> {
        self.coins.iter().try_fold(Amount(0), |total, coin| {
            let value = coin
                .denomination
                .amount()
                .0
                .checked_mul(coin.count)
                .ok_or(EcashError::AmountOverflow)?;
            total
                .0
                .checked_add(value)
                .map(Amount)
                .ok_or(EcashError::AmountOverflow)
        })
    }
}

/// Formats a whole-XPQ amount using the fewest supported coins.
pub fn format_cash_coins(amount: Amount) -> Result<Vec<CashCoin>, EcashError> {
    if amount.0 == 0 {
        return Err(EcashError::ZeroAmount);
    }
    if !amount.0.is_multiple_of(XPQ) {
        return Err(EcashError::FractionalXpQ);
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
