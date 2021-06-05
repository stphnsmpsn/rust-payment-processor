use rust_decimal::prelude::*;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Account {
    client: u16,
    available: Decimal,
    held: Decimal,
    total: Decimal,
    locked: bool,
}

impl Account {
    pub fn new(client: u16) -> Account {
        Account {
            client,
            available: dec!(0),
            held: dec!(0),
            locked: false,
            total: dec!(0),
        }
    }
}
