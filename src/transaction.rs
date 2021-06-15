#![forbid(unsafe_code)] // for good measure
use crate::errors::BankingError;
use rust_decimal::prelude::*;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

const DECIMAL_PLACES: u32 = 4;

/// `TransactionType` enumerates the supported transaction types of this crate
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum TransactionType {
    #[serde(rename = "deposit")]
    Deposit,
    #[serde(rename = "withdrawal")]
    Withdrawal,
    /// represents a client’s claim that a transaction was erroneous and should be reversed.
    /// The transaction shouldn’t be reversed yet but the associated funds should be held.
    ///
    /// This means that:
    /// 1. the clients' available funds should decrease by the amount disputed
    /// 2. the clients' held funds should increase by the amount disputed
    /// 3. the clients' total funds should remain the same
    #[serde(rename = "dispute")]
    Dispute,
    ///  represents a resolution to a dispute, releasing the associated held funds.
    ///     
    /// This means that:
    /// 1. the clients' held funds should decrease by the amount no longer disputed
    /// 2. the clients' available funds should increase by the amount no longer disputed
    /// 3. the clients' total funds should remain the same
    #[serde(rename = "resolve")]
    Resolve,
    /// A chargeback is the final state of a dispute and represents the client reversing a
    /// transaction.Funds that were held have now been withdrawn
    ///
    /// This means that:
    /// 1. the clients' held funds and total funds should decrease by the amount previously disputed
    /// 2. the client’s account should be immediately frozen.
    #[serde(rename = "chargeback")]
    Chargeback,
}

/// `Transaction` provides a structured representation of each transaction record. It derives
/// deserialize so that we may create Transaction structs easily by reading serialized data from a
/// CSV file  
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Transaction {
    #[serde(rename = "type")]
    pub kind: TransactionType,
    pub client: u16,
    pub tx: u32,
    pub amount: Option<Decimal>,
    #[serde(default)]
    pub under_dispute: bool,
}

impl Transaction {
    /// round the transaction to the specified number of decimal places
    pub fn round_to(&mut self, decimal_places: u32) {
        if let Some(amount) = self.amount {
            self.amount = Option::from(amount.round_dp(decimal_places));
        }
    }

    /// Determines if a transaction is valid. A valid transaction must be for an amount greater
    /// than 0 for deposits and withdrawals. In time, I would probably favor implementing a custom
    /// deserializer to take responsibility of this functionality, but for now, this is fine.
    pub fn validate(&mut self) -> Result<(), BankingError> {
        match self.kind {
            TransactionType::Deposit | TransactionType::Withdrawal => {
                if let Some(amount) = self.amount {
                    if amount <= dec![0] {
                        return Err(BankingError::InvalidTransaction);
                    }
                } else {
                    return Err(BankingError::InvalidTransaction);
                }
            }
            _ => {}
        }

        self.round_to(DECIMAL_PLACES);
        Ok(())
    }

    /// Disputes, resolves, and chargebacks all reference a previous transaction. This function
    /// validates that the incoming dispute, resolve, or chargeback is valid.
    /// In order to be valid:
    /// 1. the referenced transaction type must be `TransactionType::Deposit`
    /// 2. the referenced transaction client must match that of the current transaction
    /// 3. a resolve or chargeback can only occur if the transaction is under dispute
    /// 4. a dispute should not be processed if that transaction is already under dispute
    pub fn validate_against_stored(&mut self, stored_transaction: &mut Transaction) -> Result<(), BankingError> {
        match self.kind {
            TransactionType::Dispute => {
                if stored_transaction.kind != TransactionType::Deposit {
                    return Err(BankingError::InvalidTransaction);
                }
                if self.client != stored_transaction.client {
                    return Err(BankingError::ClientMismatch);
                }
                if stored_transaction.under_dispute {
                    return Err(BankingError::DuplicateDisputeRequest);
                }
            }
            TransactionType::Resolve | TransactionType::Chargeback => {
                if stored_transaction.kind != TransactionType::Deposit {
                    return Err(BankingError::InvalidTransaction);
                }
                if self.client != stored_transaction.client {
                    return Err(BankingError::ClientMismatch);
                }
                if !stored_transaction.under_dispute {
                    return Err(BankingError::UndisputedTransaction);
                }
            }
            _ => {}
        }
        Ok(())
    }
}
