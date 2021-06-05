use crate::account;
use crate::transaction::*;
use account::Account;
use std::collections::HashMap;

pub enum TransactionError {
    InvalidTransaction,
}

pub struct Bank {
    accounts: HashMap<u16, account::Account>,
}

impl Bank {
    pub fn new() -> Bank {
        Bank {
            accounts: HashMap::<u16, Account>::new(),
        }
    }

    pub fn process_transaction(
        &mut self,
        transaction: Transaction,
    ) -> Result<&Account, TransactionError> {
        if !transaction.is_valid() {
            return Err(TransactionError::InvalidTransaction);
        }

        if !self.accounts.contains_key(&transaction.client) {
            self.accounts
                .insert(transaction.client, Account::new(transaction.client));
        }

        // normally would not use ? but we just inserted the account if it didn't exist.
        let account = self.accounts.get_mut(&transaction.client).unwrap();

        match transaction.kind {
            TransactionType::Deposit => {
                println!("Processing Deposit");
            }
            TransactionType::Withdrawal => {
                println!("Processing Withdrawal");
            }
            TransactionType::Dispute => {
                println!("Processing Dispute");
            }
            TransactionType::Resolve => {
                println!("Processing Resolve");
            }
            TransactionType::Chargeback => {
                println!("Processing Chargeback");
            }
        }

        Ok(account)
    }
}
