use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum TransactionType {
    #[serde(rename = "deposit")]
    Deposit,
    #[serde(rename = "withdrawal")]
    Withdrawal,
    #[serde(rename = "dispute")]
    Dispute,
    #[serde(rename = "resolve")]
    Resolve,
    #[serde(rename = "chargeback")]
    Chargeback,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Transaction {
    #[serde(rename = "type")]
    pub kind: TransactionType,
    pub client: u16,
    pub tx: u32,
    pub amount: Decimal,
}

impl Transaction {
    pub fn normalize(&mut self, decimal_places: u32) {
        self.amount = self.amount.round_dp(decimal_places);
    }

    pub fn is_valid(&self) -> bool {
        self.amount > dec![0]
    }
}
