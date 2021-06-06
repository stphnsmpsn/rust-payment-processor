//! # A simple payment processor written in Rust
//! This crate simulates some basic banking operations such as deposits, withdrawals, disputes, resolves, and chargebacks.
//!
//! All transactions are performed using fixed precision data types as floating point types are not suitable
//! for financial calculations. Any amounts containing more than four digits of precision after the decimal will
//! be normalized to four digits of precision after the decimal. The `Decimal` data type has a max value of
//! 4_294_967_295 with 19 digits of precision after the decimal.
//!
//! Accounts are stored in a HashMap providing constant time O(1) lookup.
//!
//! If the account associated with a given transaction does not exist, we do one of two things:
//! 1. If the transaction is a deposit, we create the account and deposit the funds
//! 2. If the transaction is anything other than a deposit, we have an error
//!
//! This crate leverages exiting community crates: SERDE, CSV, and Decimal.
//! These three crates are used in combination to enable quick and easy serialization/deserialization to/from CSV
//! formatted data.
//!
//! ## Getting started
//!
//! ```csv
//! type,       client, tx, amount
//! deposit,    1,      1,  1.0
//! deposit,    2,      2,  2.0
//! deposit,    1,      3,  2.0
//! withdrawal, 1,      4,  1.5
//! withdrawal, 2,      5,  3.0
//! dispute,    2,      2,  2.0
//! ```
//!
//! ## Usage
//! ```
//! let mut bank = Bank::new();
//! let mut reader = make_csv_reader(&args.input_file)?;
//! bank.process_record_set(&mut reader);
//! bank.print_accounts();
//! ```
//!
//!
#![forbid(unsafe_code)] // for good measure
use rust_decimal::prelude::*;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io;

/// `Account` contains a structured representation of an account
#[derive(Serialize, Deserialize, Debug)]
struct Account {
    client: u16,
    available: Decimal,
    held: Decimal,
    total: Decimal,
    locked: bool,
}

///
///
///
impl Account {
    ///
    ///
    ///
    fn new(client: u16) -> Account {
        Account {
            client,
            available: dec!(0),
            held: dec!(0),
            locked: false,
            total: dec!(0),
        }
    }

    ///
    ///
    ///
    fn deposit(&mut self, amount: &Decimal) -> Result<(), BankingError> {
        self.available += amount;
        self.total += amount;
        Ok(())
    }

    ///
    ///
    ///
    fn withdraw(&mut self, amount: &Decimal) -> Result<(), BankingError> {
        if self.available < *amount {
            return Err(BankingError::InsufficientFunds);
        }

        self.available -= amount;
        self.total -= amount;
        Ok(())
    }

    ///
    ///
    ///
    fn dispute(&mut self, amount: &Decimal) -> Result<(), BankingError> {
        self.available -= amount;
        self.held += amount;
        Ok(())
    }

    ///
    ///
    ///
    fn resolve(&mut self, amount: &Decimal) -> Result<(), BankingError> {
        self.held -= amount;
        self.available += amount;
        Ok(())
    }

    ///
    ///
    ///
    fn chargeback(&mut self, amount: &Decimal) -> Result<(), BankingError> {
        self.total -= amount;
        self.held -= amount;
        self.locked = true;
        Ok(())
    }
}

/// `TransactionType` enumerates the supported transaction types of this crate
#[derive(Serialize, Deserialize, Debug)]
enum TransactionType {
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
    /// A chargeback is the final state of a dispute and represents the client reversing a transaction.
    /// Funds that were held have now been withdrawn
    ///
    /// This means that:
    /// 1. the clients' held funds and total funds should decrease by the amount previously disputed
    /// 2. the client’s account should be immediately frozen.
    #[serde(rename = "chargeback")]
    Chargeback,
}

/// `Transaction` provides a structured representation of each transaction record. It derives deserialize so that we may
/// create Transaction structs easily by reading serialized data from a CSV file  
#[derive(Serialize, Deserialize, Debug)]
struct Transaction {
    #[serde(rename = "type")]
    kind: TransactionType,
    client: u16,
    tx: u32,
    amount: Option<Decimal>,
    #[serde(default)]
    under_dispute: bool,
}

///
///
///
impl Transaction {
    ///
    ///
    ///
    fn normalize(&mut self, decimal_places: u32) {
        if let Some(amount) = self.amount {
            self.amount = Option::from(amount.round_dp(decimal_places));
        }
    }

    ///
    ///
    ///
    fn is_valid(&self) -> bool {
        return match self.amount {
            Some(amount) => {
                if amount >= dec![0] {
                    true
                } else {
                    false
                }
            }
            None => {
                return match self.kind {
                    TransactionType::Dispute => true,
                    TransactionType::Resolve => true,
                    TransactionType::Chargeback => true,
                    _ => false,
                }
            }
        };
    }
}

const DECIMAL_PLACES: u32 = 4;

#[derive(Debug)]
enum BankingError {
    InvalidTransaction,
    TransactionStorageError,
    NoSuchAccount,
    NoSuchTransaction,
    InsufficientFunds,
    ClientMismatch,
    UndisputedTransaction,
}

/// `Bank` provides storage for items that would commonly be owned by a bank, such as `Account`s and `Transaction`s.
pub struct Bank {
    accounts: HashMap<u16, Account>,
    transactions: HashMap<u32, Transaction>,
}

impl Bank {
    /// This function ....
    pub fn new() -> Bank {
        Bank {
            accounts: HashMap::<u16, Account>::new(),
            transactions: HashMap::<u32, Transaction>::new(),
        }
    }

    ///
    pub fn process_record_set(&mut self, reader: &mut csv::Reader<File>) {
        for result in reader.deserialize() {
            if let Ok(transaction) = result {
                match self.process_transaction(transaction) {
                    Err(_e) => {
                        // TODO: Implement error handling, logging, etc...
                    }
                    _ => {}
                }
            }
        }
    }

    /// Print accounts in CSV format to stdout
    pub fn print_accounts(&self) {
        let mut wtr = csv::WriterBuilder::new().from_writer(io::stdout());
        for account in &self.accounts {
            match wtr.serialize(account.1) {
                Err(_e) => { /* TODO: handle error */ }
                _ => {}
            }
        }
    }

    ///
    ///
    ///
    fn store_transaction(&mut self, transaction: Transaction) -> Result<(), BankingError> {
        if self.transactions.contains_key(&transaction.tx) || !transaction.is_valid() {
            return Err(BankingError::TransactionStorageError);
        }
        self.transactions.insert(transaction.tx, transaction);
        Ok(())
    }

    ///
    ///
    ///
    fn retrieve_account(
        client: u16,
        accounts: &mut HashMap<u16, Account>,
        create: bool,
    ) -> Result<&mut Account, BankingError> {
        if create {
            if !accounts.contains_key(&client) {
                accounts.insert(client, Account::new(client));
            };
        }
        return match accounts.get_mut(&client) {
            Some(account) => Ok(account),
            None => Err(BankingError::NoSuchAccount),
        };
    }

    ///
    ///
    ///
    fn retrieve_transaction(
        tx_id: u32,
        transactions: &mut HashMap<u32, Transaction>,
    ) -> Result<&mut Transaction, BankingError> {
        return match transactions.get_mut(&tx_id) {
            Some(transaction) => Ok(transaction),
            None => Err(BankingError::NoSuchTransaction),
        };
    }

    ///
    ///
    ///
    fn process_transaction(&mut self, mut transaction: Transaction) -> Result<(), BankingError> {
        // Check if the transaction is valid
        if !transaction.is_valid() {
            return Err(BankingError::InvalidTransaction);
        }

        // Normalize the value to four decimal places
        transaction.normalize(DECIMAL_PLACES);

        match transaction.kind {
            ////////////////////////////////////////////////////////////////////////////////
            TransactionType::Deposit => {
                let account = Bank::retrieve_account(transaction.client, &mut self.accounts, true)?;
                account.deposit(&transaction.amount.unwrap_or_else(|| dec!(0)))?;
                self.store_transaction(transaction)?;
                Ok(())
            }
            ////////////////////////////////////////////////////////////////////////////////
            TransactionType::Withdrawal => {
                let account = Bank::retrieve_account(transaction.client, &mut self.accounts, false)?;
                account.withdraw(&transaction.amount.unwrap_or_else(|| dec!(0)))?;
                self.store_transaction(transaction)?;
                Ok(())
            }
            ////////////////////////////////////////////////////////////////////////////////
            TransactionType::Dispute => {
                let mut stored_transaction = Bank::retrieve_transaction(transaction.tx, &mut self.transactions)?;
                if stored_transaction.client != transaction.client {
                    return Err(BankingError::ClientMismatch);
                }
                let account = Bank::retrieve_account(transaction.client, &mut self.accounts, false)?;
                account.dispute(&stored_transaction.amount.unwrap_or_else(|| dec!(0)))?;
                stored_transaction.under_dispute = true;
                Ok(())
            }
            ////////////////////////////////////////////////////////////////////////////////
            TransactionType::Resolve => {
                let mut stored_transaction = Bank::retrieve_transaction(transaction.tx, &mut self.transactions)?;
                if stored_transaction.client != transaction.client {
                    return Err(BankingError::ClientMismatch);
                }
                if !stored_transaction.under_dispute {
                    return Err(BankingError::UndisputedTransaction);
                }
                let account = Bank::retrieve_account(transaction.client, &mut self.accounts, false)?;
                account.resolve(&stored_transaction.amount.unwrap_or_else(|| dec!(0)))?;
                stored_transaction.under_dispute = false;
                Ok(())
            }
            ////////////////////////////////////////////////////////////////////////////////
            TransactionType::Chargeback => {
                let mut stored_transaction = Bank::retrieve_transaction(transaction.tx, &mut self.transactions)?;
                if stored_transaction.client != transaction.client {
                    return Err(BankingError::ClientMismatch);
                }
                if !stored_transaction.under_dispute {
                    return Err(BankingError::UndisputedTransaction);
                }
                let account = Bank::retrieve_account(transaction.client, &mut self.accounts, false)?;
                account.chargeback(&stored_transaction.amount.unwrap_or_else(|| dec!(0)))?;
                stored_transaction.under_dispute = false;
                Ok(())
            }
        }
    }
}
