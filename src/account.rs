#![forbid(unsafe_code)] // for good measure
use crate::errors::BankingError;
use rust_decimal::prelude::*;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

/// `Account` contains a structured representation of an account
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Account {
    pub client: u16,
    pub available: Decimal,
    pub held: Decimal,
    pub total: Decimal,
    pub locked: bool,
}

impl Account {
    /// Utility function to create a new account with a given client ID
    pub fn new(client: u16) -> Account {
        Account {
            client,
            available: dec!(0),
            held: dec!(0),
            locked: false,
            total: dec!(0),
        }
    }

    /// Deposit the specified value into the account, increasing both the total and available
    /// balances.
    pub fn deposit(&mut self, amount: &Decimal) -> Result<(), BankingError> {
        if self.locked {
            return Err(BankingError::AccountLocked);
        }

        debug!("Pre-deposit: {:?}", self);
        self.available += amount;
        self.total += amount;
        debug!("Post-deposit: {:?}", self);

        Ok(())
    }

    /// Withdraw the specified value from the account, decreasing both the total and available
    /// balances. In the event that insufficient funds are present, this function returns an
    /// appropriate `BankingError`
    pub fn withdraw(&mut self, amount: &Decimal) -> Result<(), BankingError> {
        if self.locked {
            return Err(BankingError::AccountLocked);
        }

        if self.available < *amount {
            return Err(BankingError::InsufficientFunds);
        }

        debug!("Pre-withdrawal: {:?}", self);
        self.available -= amount;
        self.total -= amount;
        debug!("Post-withdrawal: {:?}", self);

        Ok(())
    }

    /// Called in response to a dispute for a previous transaction, this function decreases the
    /// available balance and increases the balance held by the specified amount.
    pub fn dispute(&mut self, amount: &Decimal) -> Result<(), BankingError> {
        if self.locked {
            return Err(BankingError::AccountLocked);
        }

        debug!("Pre-dispute: {:?}", self);
        self.available -= amount;
        self.held += amount;
        debug!("Post-dispute: {:?}", self);

        Ok(())
    }

    /// Resolve a dispute, returning the held funds to the account and reducing the held amount.
    pub fn resolve(&mut self, amount: &Decimal) -> Result<(), BankingError> {
        if self.locked {
            return Err(BankingError::AccountLocked);
        }

        debug!("Pre-resolve: {:?}", self);
        self.held -= amount;
        self.available += amount;
        debug!("Post-resolve: {:?}", self);

        Ok(())
    }

    /// Follow through with a dispute, reversing the transaction by removing the funds from the
    /// account. The total and held amounts are both decreased and the account is locked,
    /// restricting any further transactions from taking place.
    pub fn chargeback(&mut self, amount: &Decimal) -> Result<(), BankingError> {
        if self.locked {
            return Err(BankingError::AccountLocked);
        }

        debug!("Pre-chargeback: {:?}", self);
        self.total -= amount;
        self.held -= amount;
        self.locked = true;
        debug!("Post-chargeback: {:?}", self);

        Ok(())
    }
}
