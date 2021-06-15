//! # A simple payment processor written in Rust
//! This crate simulates some basic banking operations such as deposits, withdrawals, disputes,
//! resolves, and chargebacks.
//!
//! All transactions are performed using fixed precision data types as floating point types are not
//! suitable for financial calculations. Any amounts containing more than four digits of precision
//! after the decimal will be rounded to four digits of precision after the decimal using
//! "Bankers Rounding" rules. e.g. 6.5 -> 6, 7.5 -> 8.
//!
//! The `Decimal` data type has a max value of 4_294_967_295 with 19 digits of precision after the
//! decimal.
//!
//! Accounts are stored in a HashMap providing constant time O(1) lookup.
//!
//! If the account associated with a given transaction does not exist, we do one of two things:
//! 1. If the transaction is a deposit, we create the account and deposit the funds
//! 2. If the transaction is anything other than a deposit, we have an error
//!
//! This crate leverages exiting community crates: SERDE, CSV, and Decimal.
//! These three crates are used in combination to enable quick and easy serialization and
//! deserialization to and from CSV formatted data.
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

#![forbid(unsafe_code)] // for good measure
use crate::account::Account;
use crate::errors::BankingError;
use crate::transaction::*;
use rust_decimal::prelude::*;
use rust_decimal_macros::dec;
use std::collections::HashMap;
use std::fs::File;
use std::io;

//region Bank
/// `Bank` provides storage for items that would commonly be owned by a bank, such as `Account`s
/// and `Transaction`s.
pub struct Bank {
    accounts: HashMap<u16, Account>,
    transactions: HashMap<u32, Transaction>,
}

impl Bank {
    /// Creates a new bank, capable of processing transactions and displaying account information
    pub fn new() -> Bank {
        Bank {
            accounts: HashMap::<u16, Account>::new(),
            transactions: HashMap::<u32, Transaction>::new(),
        }
    }

    /// Given a `csv::Reader<File>`, parse and process each record.
    /// Usage:
    /// ```
    /// let mut bank = Bank::new();
    /// let mut reader = make_csv_reader(&args.input_file)?;
    /// bank.process_record_set(&mut reader);
    /// ```
    pub fn process_record_set(&mut self, reader: &mut csv::Reader<File>) {
        for result in reader.deserialize() {
            if let Ok(transaction) = result {
                match self.process_transaction(transaction) {
                    Err(e) => {
                        error!("Failed to process transaction. Aborted with error: {:?}", e);
                    }
                    _ => {}
                }
            }
        }
    }

    /// Print accounts in CSV format to stdout
    /// Usage:
    /// ```
    /// let mut bank = Bank::new();
    /// let mut reader = make_csv_reader(&args.input_file)?;
    /// bank.process_record_set(&mut reader);
    /// bank.print_accounts();
    /// ```
    pub fn print_accounts(&self) {
        let mut wtr = csv::WriterBuilder::new().from_writer(io::stdout());
        for account in &self.accounts {
            match wtr.serialize(account.1) {
                Err(e) => {
                    error!("Failed to print account. Aborted with error: {:?}", e);
                }
                _ => {}
            }
        }
    }

    /// Returns the account for the specified client id, creating it if it does not exist.
    /// In the event the account is locked due to a chargeback, or the creation of a new
    /// account fails, this function returns an appropriate error.
    fn retrieve_account(client: u16, accounts: &mut HashMap<u16, Account>, create: bool) -> Result<&mut Account, BankingError> {
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

    /// Returns the transaction associated with the specified ID. If no transaction
    /// can be found by this ID, this function returns an appropriate error.
    fn retrieve_transaction(tx_id: u32, transactions: &mut HashMap<u32, Transaction>) -> Result<&mut Transaction, BankingError> {
        return match transactions.get_mut(&tx_id) {
            Some(transaction) => Ok(transaction),
            None => Err(BankingError::NoSuchTransaction),
        };
    }

    /// This function processes the given transaction, taking ownership of the `Transaction` so
    /// that it can be stored for later lookup.
    ///
    /// This function can return several errors but all are BankingError variants.
    fn process_transaction(&mut self, mut transaction: Transaction) -> Result<(), BankingError> {
        debug!("Processing Transaction: {:?}", transaction);
        match transaction.kind {
            ////////////////////////////////////////////////////////////////////////////////
            TransactionType::Deposit => {
                transaction.validate()?;
                if self.transactions.contains_key(&transaction.tx) {
                    return Err(BankingError::DuplicateTransactionId);
                }
                let account = Bank::retrieve_account(transaction.client, &mut self.accounts, true)?;
                account.deposit(&transaction.amount.unwrap_or_else(|| dec!(0)))?;
                self.transactions.insert(transaction.tx, transaction);
                Ok(())
            }
            ////////////////////////////////////////////////////////////////////////////////
            TransactionType::Withdrawal => {
                transaction.validate()?;
                if self.transactions.contains_key(&transaction.tx) {
                    return Err(BankingError::DuplicateTransactionId);
                }
                let account = Bank::retrieve_account(transaction.client, &mut self.accounts, false)?;
                account.withdraw(&transaction.amount.unwrap_or_else(|| dec!(0)))?;
                self.transactions.insert(transaction.tx, transaction);
                Ok(())
            }
            ////////////////////////////////////////////////////////////////////////////////
            TransactionType::Dispute => {
                let mut stored_transaction = Bank::retrieve_transaction(transaction.tx, &mut self.transactions)?;
                transaction.validate_against_stored(stored_transaction)?;
                let account = Bank::retrieve_account(transaction.client, &mut self.accounts, false)?;
                account.dispute(&stored_transaction.amount.unwrap_or_else(|| dec!(0)))?;
                stored_transaction.under_dispute = true;
                Ok(())
            }
            ////////////////////////////////////////////////////////////////////////////////
            TransactionType::Resolve => {
                let mut stored_transaction = Bank::retrieve_transaction(transaction.tx, &mut self.transactions)?;
                transaction.validate_against_stored(stored_transaction)?;
                let account = Bank::retrieve_account(transaction.client, &mut self.accounts, false)?;
                account.resolve(&stored_transaction.amount.unwrap_or_else(|| dec!(0)))?;
                stored_transaction.under_dispute = false;
                Ok(())
            }
            ////////////////////////////////////////////////////////////////////////////////
            TransactionType::Chargeback => {
                let mut stored_transaction = Bank::retrieve_transaction(transaction.tx, &mut self.transactions)?;
                transaction.validate_against_stored(stored_transaction)?;
                let account = Bank::retrieve_account(transaction.client, &mut self.accounts, false)?;
                account.chargeback(&stored_transaction.amount.unwrap_or_else(|| dec!(0)))?;
                stored_transaction.under_dispute = false;
                Ok(())
            }
        }
    }
}
//endregion

//region Tests
#[cfg(test)]
mod tests {
    use super::*;

    const NEGATIVE_FIVE: i32 = -5;
    const ZERO: u32 = 0;
    const ONE: u32 = 1;
    const TWO: u32 = 2;
    const THREE: u32 = 3;
    const _FOUR: u32 = 4;
    const FIVE: u32 = 5;

    //region Transaction Test Implementation
    // some utility functions to easily make create Transaction objects without cluttering test bodies
    impl Transaction {
        fn make(kind: TransactionType, client: u16, tx: u32, amount: u32, under_dispute: bool) -> Transaction {
            Transaction {
                kind,
                client,
                tx,
                amount: Some(Decimal::from(amount)),
                under_dispute,
            }
        }

        fn make_negative(kind: TransactionType, client: u16, tx: u32, amount: i32) -> Transaction {
            Transaction {
                kind,
                client,
                tx,
                amount: Some(Decimal::from(amount)),
                under_dispute: false,
            }
        }

        fn make_dispute(client: u16, tx: u32) -> Transaction {
            Transaction {
                kind: TransactionType::Dispute,
                client,
                tx,
                amount: None,
                under_dispute: false,
            }
        }

        fn make_resolve(client: u16, tx: u32) -> Transaction {
            Transaction {
                kind: TransactionType::Resolve,
                client,
                tx,
                amount: None,
                under_dispute: false,
            }
        }

        fn make_chargeback(client: u16, tx: u32) -> Transaction {
            Transaction {
                kind: TransactionType::Chargeback,
                client,
                tx,
                amount: None,
                under_dispute: false,
            }
        }
    }
    //endregion

    #[test]
    fn deposit_valid_transaction_returns_ok_and_adds_to_account() -> Result<(), BankingError> {
        // SETUP
        let expected = Decimal::from(FIVE);
        let mut bank = Bank::new();
        let tx1 = Transaction::make(TransactionType::Deposit, ONE as u16, ONE, FIVE, false);

        // TEST
        bank.process_transaction(tx1)?;
        let actual = bank.accounts.get(&(ONE as u16)).unwrap().available;
        assert_eq!(expected, actual);

        // TEARDOWN
        Ok(())
    }

    #[test]
    fn deposit_negative_number_returns_invalid_transaction() -> Result<(), BankingError> {
        // SETUP
        let expected = BankingError::InvalidTransaction;
        let mut bank = Bank::new();
        let tx1 = Transaction::make_negative(TransactionType::Deposit, ONE as u16, ONE, NEGATIVE_FIVE);

        // TEST
        let actual = bank.process_transaction(tx1);
        assert!(actual.is_err());
        assert_eq!(expected, actual.unwrap_err());

        // TEARDOWN
        Ok(())
    }

    #[test]
    fn withdrawal_with_insufficient_funds_returns_insufficient_funds() -> Result<(), BankingError> {
        // SETUP
        let expected = BankingError::InsufficientFunds;
        let mut bank = Bank::new();
        let tx1 = Transaction::make(TransactionType::Deposit, ONE as u16, ONE, ONE, false);
        let tx2 = Transaction::make(TransactionType::Withdrawal, ONE as u16, TWO, TWO, false);

        // TEST
        bank.process_transaction(tx1)?;
        let actual = bank.process_transaction(tx2);
        assert!(actual.is_err());
        assert_eq!(expected, actual.unwrap_err());

        // TEARDOWN
        Ok(())
    }

    #[test]
    fn withdrawal_from_inexistent_account_returns_no_such_account() -> Result<(), BankingError> {
        // SETUP
        let expected = BankingError::NoSuchAccount;
        let mut bank = Bank::new();
        let tx1 = Transaction::make(TransactionType::Withdrawal, ONE as u16, TWO, TWO, false);

        // TEST
        let actual = bank.process_transaction(tx1);
        assert!(actual.is_err());
        assert_eq!(expected, actual.unwrap_err());

        // TEARDOWN
        Ok(())
    }

    #[test]
    fn withdrawal_negative_number_returns_invalid_transaction() -> Result<(), BankingError> {
        // SETUP
        let expected = BankingError::InvalidTransaction;
        let mut bank = Bank::new();
        let tx1 = Transaction::make_negative(TransactionType::Withdrawal, ONE as u16, ONE, NEGATIVE_FIVE);

        // TEST
        let actual = bank.process_transaction(tx1);
        assert!(actual.is_err());
        assert_eq!(expected, actual.unwrap_err());

        // TEARDOWN
        Ok(())
    }

    #[test]
    fn withdrawal_works_with_sufficient_funds() -> Result<(), BankingError> {
        // SETUP
        let expected = Decimal::from(THREE);
        let mut bank = Bank::new();
        let tx1 = Transaction::make(TransactionType::Deposit, ONE as u16, ONE, FIVE, false);
        let tx2 = Transaction::make(TransactionType::Withdrawal, ONE as u16, TWO, TWO, false);

        // TEST
        bank.process_transaction(tx1)?;
        bank.process_transaction(tx2)?;
        let actual = bank.accounts.get(&(ONE as u16)).unwrap().available;
        assert_eq!(expected, actual);

        // TEARDOWN
        Ok(())
    }

    #[test]
    fn transact_with_duplicate_transaction_id_returns_duplicate_transaction_id() -> Result<(), BankingError> {
        // SETUP
        let expected = BankingError::DuplicateTransactionId;
        let mut bank = Bank::new();
        let tx1 = Transaction::make(TransactionType::Deposit, ONE as u16, ONE, ONE, false);
        let tx2 = Transaction::make(TransactionType::Deposit, ONE as u16, ONE, ONE, false);
        let tx3 = Transaction::make(TransactionType::Withdrawal, ONE as u16, ONE, ONE, false);

        // TEST
        bank.process_transaction(tx1)?;
        let first_actual = bank.process_transaction(tx2);
        let second_actual = bank.process_transaction(tx3);
        assert!(first_actual.is_err());
        assert_eq!(expected, first_actual.unwrap_err());
        assert!(second_actual.is_err());
        assert_eq!(expected, second_actual.unwrap_err());

        // TEARDOWN
        Ok(())
    }

    #[test]
    fn dispute_transaction_with_invalid_id_returns_no_such_transaction() -> Result<(), BankingError> {
        // SETUP
        let expected = BankingError::NoSuchTransaction;
        let mut bank = Bank::new();
        let tx1 = Transaction::make_dispute(ONE as u16, ONE);

        // TEST
        let actual = bank.process_transaction(tx1);
        assert!(actual.is_err());
        assert_eq!(expected, actual.unwrap_err());

        // TEARDOWN
        Ok(())
    }

    #[test]
    fn dispute_valid_transaction() -> Result<(), BankingError> {
        // SETUP
        let expected_transaction = Transaction::make(TransactionType::Deposit, ONE as u16, ONE, FIVE, true);
        let expected_account = Account {
            client: ONE as u16,
            available: Decimal::from(ZERO),
            total: Decimal::from(FIVE),
            held: Decimal::from(FIVE),
            locked: false,
        };
        let mut bank = Bank::new();
        let tx1 = Transaction::make(TransactionType::Deposit, ONE as u16, ONE, FIVE, false);
        let tx2 = Transaction::make_dispute(ONE as u16, ONE);

        // TEST
        bank.process_transaction(tx1)?;
        bank.process_transaction(tx2)?;

        assert_eq!(expected_transaction, *bank.transactions.get(&ONE).unwrap());
        assert_eq!(expected_account, *bank.accounts.get(&(ONE as u16)).unwrap());
        // TEARDOWN
        Ok(())
    }

    #[test]
    fn dispute_disputed_transaction_returns_already_in_dispute() -> Result<(), BankingError> {
        // SETUP
        let expected_result = BankingError::DuplicateDisputeRequest;
        let expected_transaction = Transaction {
            kind: TransactionType::Deposit,
            client: ONE as u16,
            tx: ONE,
            amount: Some(Decimal::from(FIVE)),
            under_dispute: true,
        };
        let expected_account = Account {
            client: ONE as u16,
            available: Decimal::from(ZERO),
            total: Decimal::from(FIVE),
            held: Decimal::from(FIVE),
            locked: false,
        };
        let mut bank = Bank::new();
        let tx1 = Transaction::make(TransactionType::Deposit, ONE as u16, ONE, FIVE, false);
        let tx2 = Transaction::make_dispute(ONE as u16, ONE);
        let tx3 = Transaction::make_dispute(ONE as u16, ONE);

        // TEST
        bank.process_transaction(tx1)?;
        bank.process_transaction(tx2)?;
        let result = bank.process_transaction(tx3);

        assert_eq!(expected_transaction, *bank.transactions.get(&ONE).unwrap());
        assert_eq!(expected_account, *bank.accounts.get(&(ONE as u16)).unwrap());
        assert_eq!(expected_result, result.unwrap_err());
        // TEARDOWN
        Ok(())
    }

    #[test]
    fn resolve_disputed_transaction_releases_held_funds() -> Result<(), BankingError> {
        // SETUP
        let expected_transaction = Transaction::make(TransactionType::Deposit, ONE as u16, ONE, FIVE, false);
        let expected_account = Account {
            client: ONE as u16,
            available: Decimal::from(FIVE),
            total: Decimal::from(FIVE),
            held: Decimal::from(ZERO),
            locked: false,
        };
        let mut bank = Bank::new();
        let tx1 = Transaction::make(TransactionType::Deposit, ONE as u16, ONE, FIVE, false);
        let tx2 = Transaction::make_dispute(ONE as u16, ONE);
        let tx3 = Transaction::make_resolve(ONE as u16, ONE);

        // TEST
        bank.process_transaction(tx1)?;
        bank.process_transaction(tx2)?;
        bank.process_transaction(tx3)?;

        assert_eq!(expected_account, *bank.accounts.get(&(ONE as u16)).unwrap());
        assert_eq!(expected_transaction, *bank.transactions.get(&ONE).unwrap());

        // TEARDOWN
        Ok(())
    }

    #[test]
    fn chargeback_disputed_transaction_withdraws_available_funds_and_locks_account() -> Result<(), BankingError> {
        // SETUP
        let expected_transaction = Transaction::make(TransactionType::Deposit, ONE as u16, ONE, FIVE, false);
        let expected_account = Account {
            client: ONE as u16,
            available: Decimal::from(ZERO),
            total: Decimal::from(ZERO),
            held: Decimal::from(ZERO),
            locked: true,
        };
        let mut bank = Bank::new();
        let tx1 = Transaction::make(TransactionType::Deposit, ONE as u16, ONE, FIVE, false);
        let tx2 = Transaction::make_dispute(ONE as u16, ONE);
        let tx3 = Transaction::make_chargeback(ONE as u16, ONE);

        // TEST
        bank.process_transaction(tx1)?;
        bank.process_transaction(tx2)?;
        bank.process_transaction(tx3)?;

        assert_eq!(expected_account, *bank.accounts.get(&(ONE as u16)).unwrap());
        assert_eq!(expected_transaction, *bank.transactions.get(&ONE).unwrap());

        // TEARDOWN
        Ok(())
    }

    #[test]
    fn dispute_transaction_after_withdrawal_allows_negative_total() -> Result<(), BankingError> {
        // SETUP
        let expected_transaction = Transaction {
            kind: TransactionType::Deposit,
            client: ONE as u16,
            tx: ONE,
            amount: Some(Decimal::from(FIVE)),
            under_dispute: true,
        };
        let expected_account = Account {
            client: ONE as u16,
            available: Decimal::from(NEGATIVE_FIVE),
            total: Decimal::from(ZERO),
            held: Decimal::from(FIVE),
            locked: false,
        };
        let mut bank = Bank::new();
        let tx1 = Transaction::make(TransactionType::Deposit, ONE as u16, ONE, FIVE, false);
        let tx2 = Transaction::make(TransactionType::Withdrawal, ONE as u16, TWO, FIVE, false);
        let tx3 = Transaction::make_dispute(ONE as u16, ONE);

        // TEST
        bank.process_transaction(tx1)?;
        bank.process_transaction(tx2)?;
        bank.process_transaction(tx3)?;

        assert_eq!(expected_account, *bank.accounts.get(&(ONE as u16)).unwrap());
        assert_eq!(expected_transaction, *bank.transactions.get(&ONE).unwrap());

        // TEARDOWN
        Ok(())
    }

    #[test]
    fn chargeback_transaction_after_withdrawal_allows_negative_total() -> Result<(), BankingError> {
        // SETUP
        let expected_transaction = Transaction::make(TransactionType::Deposit, ONE as u16, ONE, FIVE, false);
        let expected_account = Account {
            client: ONE as u16,
            available: Decimal::from(NEGATIVE_FIVE),
            total: Decimal::from(NEGATIVE_FIVE),
            held: Decimal::from(ZERO),
            locked: true,
        };
        let mut bank = Bank::new();
        let tx1 = Transaction::make(TransactionType::Deposit, ONE as u16, ONE, FIVE, false);
        let tx2 = Transaction::make(TransactionType::Withdrawal, ONE as u16, TWO, FIVE, false);
        let tx3 = Transaction::make_dispute(ONE as u16, ONE);
        let tx4 = Transaction::make_chargeback(ONE as u16, ONE);

        // TEST
        bank.process_transaction(tx1)?;
        bank.process_transaction(tx2)?;
        bank.process_transaction(tx3)?;
        bank.process_transaction(tx4)?;

        assert_eq!(expected_account, *bank.accounts.get(&(ONE as u16)).unwrap());
        assert_eq!(expected_transaction, *bank.transactions.get(&ONE).unwrap());

        // TEARDOWN
        Ok(())
    }

    #[test]
    fn transaction_on_locked_account_returns_account_locked() -> Result<(), BankingError> {
        // SETUP
        let expected_result = BankingError::AccountLocked;
        let expected_transaction = Transaction::make(TransactionType::Deposit, ONE as u16, ONE, FIVE, false);
        let expected_account = Account {
            client: ONE as u16,
            available: Decimal::from(NEGATIVE_FIVE),
            total: Decimal::from(NEGATIVE_FIVE),
            held: Decimal::from(ZERO),
            locked: true,
        };
        let mut bank = Bank::new();
        let tx1 = Transaction::make(TransactionType::Deposit, ONE as u16, ONE, FIVE, false);
        let tx2 = Transaction::make(TransactionType::Withdrawal, ONE as u16, TWO, FIVE, false);
        let tx3 = Transaction::make_dispute(ONE as u16, ONE);
        let tx4 = Transaction::make_chargeback(ONE as u16, ONE);
        let tx5 = Transaction::make(TransactionType::Deposit, ONE as u16, THREE, FIVE, false);

        // TEST
        bank.process_transaction(tx1)?;
        bank.process_transaction(tx2)?;
        bank.process_transaction(tx3)?;
        bank.process_transaction(tx4)?;
        let result = bank.process_transaction(tx5);

        assert_eq!(expected_result, result.unwrap_err());
        assert_eq!(expected_account, *bank.accounts.get(&(ONE as u16)).unwrap());
        assert_eq!(expected_transaction, *bank.transactions.get(&ONE).unwrap());

        // TEARDOWN
        Ok(())
    }

    #[test]
    fn dispute_client_with_wrong_client_returns_client_mismatch() -> Result<(), BankingError> {
        // SETUP
        let expected_result = BankingError::ClientMismatch;
        let expected_transaction = Transaction::make(TransactionType::Deposit, ONE as u16, ONE, FIVE, false);
        let expected_account = Account {
            client: ONE as u16,
            available: Decimal::from(FIVE),
            total: Decimal::from(FIVE),
            held: Decimal::from(ZERO),
            locked: false,
        };
        let mut bank = Bank::new();
        let tx1 = Transaction::make(TransactionType::Deposit, ONE as u16, ONE, FIVE, false);
        let tx2 = Transaction::make_dispute(TWO as u16, ONE);

        // TEST
        bank.process_transaction(tx1)?;
        let result = bank.process_transaction(tx2);

        assert_eq!(expected_transaction, *bank.transactions.get(&ONE).unwrap());
        assert_eq!(expected_account, *bank.accounts.get(&(ONE as u16)).unwrap());
        assert_eq!(expected_result, result.unwrap_err());
        // TEARDOWN
        Ok(())
    }

    #[test]
    fn resolve_transaction_not_under_dispute_returns_undisputed_transaction() -> Result<(), BankingError> {
        // SETUP
        let expected_result = BankingError::UndisputedTransaction;
        let expected_transaction = Transaction::make(TransactionType::Deposit, ONE as u16, ONE, FIVE, false);
        let expected_account = Account {
            client: ONE as u16,
            available: Decimal::from(FIVE),
            total: Decimal::from(FIVE),
            held: Decimal::from(ZERO),
            locked: false,
        };
        let mut bank = Bank::new();
        let tx1 = Transaction::make(TransactionType::Deposit, ONE as u16, ONE, FIVE, false);
        let tx2 = Transaction::make_resolve(ONE as u16, ONE);

        // TEST
        bank.process_transaction(tx1)?;
        let result = bank.process_transaction(tx2);

        assert_eq!(expected_transaction, *bank.transactions.get(&ONE).unwrap());
        assert_eq!(expected_account, *bank.accounts.get(&(ONE as u16)).unwrap());
        assert_eq!(expected_result, result.unwrap_err());
        // TEARDOWN
        Ok(())
    }

    #[test]
    fn dispute_withdrawal_returns_invalid_transaction() -> Result<(), BankingError> {
        // SETUP
        let expected_result = BankingError::InvalidTransaction;
        let expected_account = Account {
            client: ONE as u16,
            available: Decimal::from(ZERO),
            total: Decimal::from(ZERO),
            held: Decimal::from(ZERO),
            locked: false,
        };
        let mut bank = Bank::new();
        let tx1 = Transaction::make(TransactionType::Deposit, ONE as u16, ONE, FIVE, false);
        let tx2 = Transaction::make(TransactionType::Withdrawal, ONE as u16, TWO, FIVE, false);
        let tx3 = Transaction::make_dispute(ONE as u16, TWO);

        // TEST
        bank.process_transaction(tx1)?;
        bank.process_transaction(tx2)?;
        let result = bank.process_transaction(tx3);

        assert_eq!(expected_account, *bank.accounts.get(&(ONE as u16)).unwrap());
        assert_eq!(expected_result, result.unwrap_err());
        // TEARDOWN
        Ok(())
    }
}
//endregion
