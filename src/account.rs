use serde::{Serialize, Serializer};
use std::collections::HashMap;
use crate::transaction::{Transaction, AccountId, TransactionId, TransactionType};
use thiserror::Error;

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

#[derive(Debug, Error)]
pub(crate) enum Error {
    #[error("account {0:?} is locked")]
    AccountLocked(AccountId),

    #[error("transaction {0:?} not found")]
    TransactionNotFound(TransactionId),

    #[error("account {0:?} has insufficient funds ")]
    InsufficientFunds(AccountId),

    #[error("amount is missing for transaction {0:?}")]
    AmountMissingWhenRequired(TransactionId),

    #[error("amount is present for transaction {0:?} where it is ambiguous and must be omitted")]
    AmountPresentWhenAmbiguous(TransactionId),

    #[error("state of transaction {0:?} is invalid")]
    InvalidTransactionState(TransactionId),

    #[error("type of transaction {0:?} is invalid")]
    InvalidTransactionType(TransactionId),
}

/// Client account keeps balances of client funds calculated as aggregates of transactions
#[derive(Serialize)]
pub struct Account {
    /// Account aka client id is `client` in the input
    #[serde(rename = "client")]
    id: AccountId,
    /// Funds available
    pub(crate) available: MoneyAggregate,
    /// Funds held for disputes
    pub(crate) held: MoneyAggregate,
    /// Sum of funds available and held
    pub(crate) total: MoneyAggregate,
    /// Account is locked for a chargeback, no transactions can be accepted
    pub(crate) locked: bool,

    /// Keep all transactions of this account in memory for quick lookups by id
    #[serde(skip)]
    transactions: HashMap<TransactionId, Transaction>,
}

impl Account {
    // Create an empty account when its id is first encountered in transaction input
    pub(crate) fn new(id: AccountId) -> Self {
        Self { 
            id,
            locked: false,
            available: MoneyAggregate(0.0),
            held: MoneyAggregate(0.0),
            total: MoneyAggregate(0.0),
            transactions: HashMap::new()
         }
    }

    fn add_transaction(&mut self, transaction: Transaction) {
        self.transactions.insert(transaction.id, transaction);
    }

    fn get_transaction(&mut self, id: TransactionId) -> Result<&mut Transaction, Error> {
        let transaction = self.transactions.get_mut(&id).ok_or(Error::TransactionNotFound(id))?;
        Ok(transaction)
    }

    fn deposit(&mut self, transaction: Transaction) -> Result<(), Error> {
        match transaction.amount {
            Some(amount) => {
                self.available.0 += amount;
                self.total.0 += amount;
                
                self.add_transaction(transaction);
                
                Ok(())
            }
            None => return Err(Error::AmountMissingWhenRequired(transaction.id))
        }
    }

    fn withdraw(&mut self, transaction: Transaction) -> Result<(), Error> {
        match transaction.amount {
            Some(amount) => {
                let available = self.available.0 - amount;

                if available < 0.0 {
                    return Err(Error::InsufficientFunds(self.id))
                }

                self.available.0 = available;
                self.total.0 -= amount;
                
                self.add_transaction(transaction);

                Ok(())
            }
            None => return Err(Error::AmountMissingWhenRequired(transaction.id))
        }
    }

    fn dispute(&mut self, transaction: Transaction) -> Result<(), Error> {
        match transaction.amount {
            Some(..) => return Err(Error::AmountPresentWhenAmbiguous(transaction.id)),
            None => {
                let transaction = self.get_transaction(transaction.id)?;

                // error out if it's already disputed and not change any balances
                if transaction.disputed {
                    return Err(Error::InvalidTransactionState(transaction.id))
                }

                // if dispute can result in a chargeback then it only makes sense if disputed transaction is a deposit
                if transaction.transaction_type != TransactionType::Deposit {
                    return Err(Error::InvalidTransactionType(transaction.id))
                }

                transaction.disputed = true;

                let amount = transaction.amount.ok_or(Error::AmountMissingWhenRequired(transaction.id))?;
                
                self.available.0 -=  amount;
                self.held.0 +=  amount;

                Ok(())
            }
        }
    }

    fn resolve(&mut self, transaction: Transaction) -> Result<(), Error> {
        match transaction.amount {
            Some(..) => return Err(Error::AmountPresentWhenAmbiguous(transaction.id)),
            None => {
                let transaction = self.get_transaction(transaction.id)?;

                if !transaction.disputed {
                    return Err(Error::InvalidTransactionState(transaction.id));
                }

                transaction.disputed = false;

                let amount = transaction.amount.ok_or(Error::AmountMissingWhenRequired(transaction.id))?;
                
                self.available.0 +=  amount;
                self.held.0 -=  amount;

                Ok(())
            }
        }
    }

    fn chargeback(&mut self, transaction: Transaction) -> Result<(), Error> {
        match transaction.amount {
            Some(..) => return Err(Error::AmountPresentWhenAmbiguous(transaction.id)),
            None => {
                let transaction = self.get_transaction(transaction.id)?;

                if !transaction.disputed {
                    return Err(Error::InvalidTransactionState(transaction.id));
                }

                let amount = transaction.amount.ok_or(Error::AmountMissingWhenRequired(transaction.id))?;
                
                self.held.0 -= amount;
                self.total.0 -= amount;

                self.locked = true;

                Ok(())
            }
        }
    }

    /// Apply a transaction to this account's aggregates
    pub(crate) fn apply_transaction(&mut self, transaction: Transaction) -> Result<(), Error> {
        if self.locked {
            return Err(Error::AccountLocked(self.id));
        }

        match transaction.transaction_type {
            TransactionType::Deposit => self.deposit(transaction),
            TransactionType::Withdrawal => self.withdraw(transaction),
            TransactionType::Dispute => self.dispute(transaction),
            TransactionType::Resolve => self.resolve(transaction),
            TransactionType::Chargeback => self.chargeback(transaction)
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