use serde::{Serialize, Serializer};
use std::collections::HashMap;
use crate::transaction::{Transaction, AccountId, TransactionId};

/// Amounts with serialized precision of four places past the decimal
pub struct MoneyAggregate(pub(crate) f64);

impl Serialize for MoneyAggregate {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_f64((self.0 * 1_0000.0).round() / 1_0000.0)
    }
}

/// A client account
#[derive(Serialize)]
pub struct Account {
    /// Account aka client id is `client` in the input
    #[serde(rename = "client")]
    id: AccountId,
    /// Funds available
    available: MoneyAggregate,
    /// Funds held for disputes
    held: MoneyAggregate,
    /// Total funds available and held
    total: MoneyAggregate,
    /// Account is locked for a chargeback, no transactions can be accepted
    locked: bool,

    /// Keep all transactions of this account in memory for a quick lookup
    #[serde(skip)]
    transactions: HashMap<TransactionId, Transaction>,
}

impl Account {
    pub(crate) fn new(id: u16) -> Self {
        Self { 
            id,
            available: MoneyAggregate(0.0),
            held: MoneyAggregate(0.0),
            total: MoneyAggregate(0.0),
            locked: false,
            transactions: HashMap::new()
         }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use csv::WriterBuilder;

    #[test]
    fn serialize_accounts() {
        let accounts = vec![
            Account {
                id: 1,
                available: MoneyAggregate(1.0),
                held: MoneyAggregate(0.1),
                total: MoneyAggregate(1.10001), // should round to 1.1
                locked: false,
                transactions: HashMap::new(),
            },
            Account {
                id: 2,
                available: MoneyAggregate(2.0),
                held: MoneyAggregate(0.0001),
                total: MoneyAggregate(2.0001),
                locked: true,
                transactions: HashMap::new(),
            },
        ];

        let mut writer = WriterBuilder::new().from_writer(vec![]);
        for account in accounts.iter() {
            writer.serialize(account).expect("cannot serialize account");
        }

        let csv = String::from_utf8(writer.into_inner().unwrap()).unwrap();
        assert_eq!(
            csv,
            "\
client,available,held,total,locked
1,1.0,0.1,1.1,false
2,2.0,0.0001,2.0001,true
"
        )
    }
}