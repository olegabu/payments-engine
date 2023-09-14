use serde::Deserialize;

/// Client Account id
pub type AccountId = u16;
/// Transaction id
pub type TransactionId = u32;
/// Amounts with no restriction to serialized precision
pub type Money = f64;

/// Transaction is applied to a client account
#[derive(Debug, Deserialize, PartialEq)]
pub struct Transaction {
    /// Transaction id is `tx` in the input
    #[serde(rename = "tx")]
    pub(crate) id: TransactionId,
    /// Client account id is `client` in the input
    #[serde(rename = "client")]
    pub(crate) account_id: AccountId,
    /// Transaction type is `type` in the input
    #[serde(rename = "type")]
    pub(crate) transaction_type: TransactionType,
    /// Amount is optional in Dispute, Resolve, Chargeback transactions
    pub(crate) amount: Option<Money>,
    #[serde(skip)]
    pub(crate) disputed: bool,
}

impl Transaction {
    #[cfg(test)]
    pub(crate) fn new(
        transaction_type: TransactionType,
        client: AccountId,
        tx: TransactionId,
        amount: Option<Money>,
        disputed: bool,
    ) -> Self {
        Self {
            transaction_type,
            account_id: client,
            id: tx,
            amount,
            disputed,
        }
    }
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum TransactionType {
    /// Credit to the client's asset account, meaning it should increase the available and total funds of the client account
    Deposit,
    /// Debit to the client's asset account, meaning it should decrease the available and total funds of the client account
    Withdrawal,
    /// Client's claim that a transaction was erroneous and should be reversed
    Dispute,
    /// Resolution to a dispute, releasing the associated held funds
    Resolve,
    /// Final state of a dispute and represents the client reversing a transaction
    Chargeback,
}

#[cfg(test)]
mod tests {
    use super::*;
    use csv::{ReaderBuilder, Trim};

    #[test]
    fn deserialize_transactions() {
        let csv = "\
type,       client, tx, amount
deposit,         1,  1,    2.0
withdrawal,      2,  2,    1.10001
dispute,         1,  1,
resolve,         1,  1,
chargeback,      1,  1,
";

        let expected = vec![
            Transaction::new(TransactionType::Deposit, 1, 1, Some(2.0), false),
            Transaction::new(TransactionType::Withdrawal, 2, 2, Some(1.10001), false),
            Transaction::new(TransactionType::Dispute, 1, 1, None, false),
            Transaction::new(TransactionType::Resolve, 1, 1, None, false),
            Transaction::new(TransactionType::Chargeback, 1, 1, None, false),
        ];

        let reader = ReaderBuilder::new()
            .trim(Trim::All)
            .from_reader(csv.as_bytes());

        for (result, e) in reader.into_deserialize().zip(expected.iter()) {
            let r: Transaction = result.expect("cannot deserialize transaction");
            assert_eq!(r, *e);
        }
    }
}
