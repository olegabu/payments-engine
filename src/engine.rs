use csv::{Error, ReaderBuilder, Trim, WriterBuilder};
use std::collections::HashMap;
use std::io::{Read, Write};

use crate::account::Account;
use crate::transaction::{AccountId, Transaction};

/// Takes transactions as reader input, processes them and outputs accounts with aggregate values
pub struct Engine {
    /// Store accounts in memory for look up by id
    account_map: HashMap<AccountId, Account>,
}

impl Engine {
    pub(crate) fn new() -> Self {
        Self {
            account_map: HashMap::new(),
        }
    }

    /// Read transactions, apply to accounts, write accounts
    pub(crate) fn process<R, W>(&mut self, read: R, write: W)
    where
        R: Read,
        W: Write,
    {
        self.input(read);

        if let Err(e) = self.output(write) {
            eprintln!("Failed to serialize accounts: {}", e);
        }
    }

    /// Deserialize transactions from reader, ignore record if cannot parse it,
    /// apply transactions to accounts' aggregate values and collect accounts in memory
    pub(crate) fn input<R>(&mut self, rdr: R)
    where
        R: Read,
    {
        let mut reader = ReaderBuilder::new()
            .trim(Trim::All) // trim leading and trailing whitespace
            .flexible(true) // allow for missing columns like amount
            .from_reader(rdr);

        for result in reader.deserialize() {
            // parse transaction from csv and ignore if error
            let transaction: Transaction = match result {
                Ok(transaction) => transaction,
                Err(error) => {
                    eprintln!("cannot parse transaction for {error}");
                    continue;
                }
            };

            // find account in the map or create it if not found
            let account = self
                .account_map
                .entry(transaction.account_id)
                .or_insert_with_key(|id| Account::new(*id));

            // apply transaction to the account from csv and ignore if error
            if let Err(error) = account.apply_transaction(transaction) {
                eprintln!("cannot apply transaction for {error}");
                continue;
            }
        }
    }

    /// Serialize accounts from memory to writer
    pub(crate) fn output<W>(&self, wtr: W) -> Result<(), Error>
    where
        W: Write,
    {
        let mut writer = WriterBuilder::new().from_writer(wtr);

        for account in self.account_map.values() {
            writer.serialize(&account)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;
    use approx::assert_relative_eq;

    #[test]
    /// smoke test to observe accounts on std out
    fn process() {
        let csv = "\
type,       client, tx, amount
deposit,         2,  1,    2.1
deposit,         1,  1,    2.0
withdrawal,      2,  2,    1.10001
dispute,         1,  1,
resolve,         1,  1,
chargeback,      1,  1,
";

        let mut engine = Engine::new();
        engine.process(csv.as_bytes(), io::stdout());
    }

    #[test]
    /// balances with rounding
    fn input() {
        let csv = "\
type,       client, tx, amount
deposit,         2,  1,    2.1
deposit,         1,  1,    2.0
withdrawal,      2,  2,    1.10001
dispute,         1,  1,
resolve,         1,  1,
chargeback,      1,  1,
";

        let mut engine = Engine::new();
        engine.input(csv.as_bytes());

        let a1 = engine.account_map.get(&1).unwrap();
        assert_eq!(a1.available.0, 2.0);
        assert_eq!(a1.held.0, 0.0);
        assert_eq!(a1.total.0, 2.0);
        assert_eq!(a1.locked, false);

        let a2 = engine.account_map.get(&2).unwrap();
        assert_relative_eq!(a2.available.0, 1.0, epsilon = 0.00001);
        assert_eq!(a2.held.0, 0.0);
        assert_relative_eq!(a2.total.0, 1.0, epsilon = 0.00001);
        assert_eq!(a2.locked, false);
    }

    #[test]
    /// one withdrawal empties an account another has insufficient funds and leaves balances intact
    fn insufficient_funds() {
        let csv = "\
type,       client, tx, amount
deposit,    1,      1,  1.0
deposit,    2,      2,  2.0
deposit,    1,      3,  2.0
withdrawal, 1,      4,  3.0
withdrawal, 2,      5,  2.1
";

        let mut engine = Engine::new();
        engine.input(csv.as_bytes());

        let a1 = engine.account_map.get(&1).unwrap();
        assert_eq!(a1.available.0, 0.0);
        assert_eq!(a1.held.0, 0.0);
        assert_eq!(a1.total.0, 0.0);
        assert_eq!(a1.locked, false);

        // withdrawal of 2.1 after deposit of 2.0 gets insufficient funds error leaving the total intact
        let a2 = engine.account_map.get(&2).unwrap();
        assert_eq!(a2.available.0, 2.0);
        assert_eq!(a2.held.0, 0.0);
        assert_eq!(a2.total.0, 2.0);
        assert_eq!(a2.locked, false);
    }

    #[test]
    /// dispute moves funds from available into held, withdrawal above available fails leaving it intact
    fn dispute() {
        let csv = "\
type,       client, tx, amount
deposit,    1,      1,  1.0
deposit,    1,      2,  2.0
dispute,    1,      1,  
withdrawal, 1,      3,  2.1
";

        let mut engine = Engine::new();
        engine.input(csv.as_bytes());

        let a1 = engine.account_map.get(&1).unwrap();
        assert_eq!(a1.available.0, 2.0);
        assert_eq!(a1.held.0, 1.0);
        assert_eq!(a1.total.0, 3.0);
        assert_eq!(a1.locked, false);
    }

    #[test]
    /// resolve after dispute brings available balance back and allows withdrawal
    fn resolve() {
        let csv = "\
type,       client, tx, amount
deposit,    1,      1,  1.0
dispute,    1,      1,  
resolve,    1,      1,
withdrawal, 1,      2,  1.0
";

        let mut engine = Engine::new();
        engine.input(csv.as_bytes());

        let a1 = engine.account_map.get(&1).unwrap();
        assert_eq!(a1.available.0, 0.0);
        assert_eq!(a1.held.0, 0.0);
        assert_eq!(a1.total.0, 0.0);
        assert_eq!(a1.locked, false);
    }

    #[test]
    /// dispute with an explicit amount is ignored as ambiguous
    fn dispute_with_amount() {
        let csv = "\
type,       client, tx, amount
deposit,    1,      1,  1.0
dispute,    1,      1,  1.0
withdrawal, 1,      2,  1.0
";

        let mut engine = Engine::new();
        engine.input(csv.as_bytes());

        let a1 = engine.account_map.get(&1).unwrap();
        assert_eq!(a1.available.0, 0.0);
        assert_eq!(a1.held.0, 0.0);
        assert_eq!(a1.total.0, 0.0);
        assert_eq!(a1.locked, false);
    }

    #[test]
    /// resolve a non disputed transaction is ignored
    fn resolve_not_disputed() {
        let csv = "\
type,       client, tx, amount
deposit,    1,      1,  1.0
resolve,    1,      1,
withdrawal, 1,      2,  1.0
";

        let mut engine = Engine::new();
        engine.input(csv.as_bytes());

        let a1 = engine.account_map.get(&1).unwrap();
        assert_eq!(a1.available.0, 0.0);
        assert_eq!(a1.held.0, 0.0);
        assert_eq!(a1.total.0, 0.0);
        assert_eq!(a1.locked, false);
    }

    #[test]
    /// chargeback a non disputed transaction is ignored
    fn chargeback_not_disputed() {
        let csv = "\
type,       client, tx, amount
deposit,    1,      1,  1.0
chargeback,    1,      1,
withdrawal, 1,      2,  1.0
";

        let mut engine = Engine::new();
        engine.input(csv.as_bytes());

        let a1 = engine.account_map.get(&1).unwrap();
        assert_eq!(a1.available.0, 0.0);
        assert_eq!(a1.held.0, 0.0);
        assert_eq!(a1.total.0, 0.0);
        assert_eq!(a1.locked, false);
    }

    #[test]
    /// disputing a transaction with type other than deposit is ignored
    fn dispute_not_deposit() {
        let csv = "\
type,       client, tx, amount
deposit,    1,      1,  1.0
withdrawal, 1,      2,  1.0
dispute,    1,      2,
";

        let mut engine = Engine::new();
        engine.input(csv.as_bytes());

        let a1 = engine.account_map.get(&1).unwrap();
        assert_eq!(a1.available.0, 0.0);
        assert_eq!(a1.held.0, 0.0);
        assert_eq!(a1.total.0, 0.0);
        assert_eq!(a1.locked, false);
    }

    #[test]
    /// chargeback locks account, moves from held back to available, prevents withdrawal
    fn chargeback() {
        let csv = "\
type,       client, tx, amount
deposit,    1,      1,  1.0
deposit,    1,      2,  2.0
dispute,    1,      1,  
chargeback, 1,      1,
withdrawal, 1,      3,  2.0
";

        let mut engine = Engine::new();
        engine.input(csv.as_bytes());

        let a1 = engine.account_map.get(&1).unwrap();
        assert_eq!(a1.available.0, 2.0); // funds intact despite an attempt to withdraw by tx 3
        assert_eq!(a1.held.0, 0.0);
        assert_eq!(a1.total.0, 2.0);
        assert_eq!(a1.locked, true);
    }
}
