use csv::{Error, ReaderBuilder, Trim, WriterBuilder};
use std::collections::HashMap;
use std::io::{Read, Write};

use crate::account::Account;
use crate::transaction::{AccountId, Transaction};

/// A Client account database.
pub struct Engine {
    map: HashMap<AccountId, Account>,
}

impl Engine {
    pub(crate) fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    pub(crate) fn input<R>(&mut self, rdr: R)
    where
        R: Read,
    {
        let mut reader = ReaderBuilder::new()
            .trim(Trim::All)
            .flexible(true)
            .from_reader(rdr);

        for result in reader.deserialize() {
            let transaction: Transaction = match result {
                Ok(transaction) => transaction,
                Err(error) => {
                    eprintln!("cannot parse transaction for {error}");
                    continue;
                }
            };

            let account = self
                .map
                .entry(transaction.account_id)
                .or_insert_with_key(|id| Account::new(*id));

            if let Err(error) = account.apply_transaction(transaction) {
                eprintln!("cannot apply for {error}");
                continue;
            }
        }
    }

    pub(crate) fn output<W>(self, wtr: W) -> Result<(), Error>
    where
        W: Write,
    {
        let mut writer = WriterBuilder::new().from_writer(wtr);

        for account in self.map.values() {
            writer.serialize(&account)?;
        }

        Ok(())
    }

    // pub(crate) fn process<P: AsRef<Path>>(file: P) {
    //     let mut reader = ReaderBuilder::new()
    //         .trim(Trim::All)
    //         .flexible(true)
    //         .from_reader(file);

    //     for result in reader.deserialize() {
    //         let tx: Transaction = match result {
    //             Ok(tx) => tx,
    //             Err(e) => {
    //                 eprintln!("Failed to parse a transaction record: {}", e);

    //                 // Just ignore the transaction then.
    //                 continue;
    //             }
    //         };

    //         // If account doesn't already exist, create one.
    //         let account = self
    //             .0
    //             .entry(tx.client)
    //             .or_insert_with_key(|id| Account::new(*id));

    //         if let Err(e) = account.execute_transaction(tx) {
    //             eprintln!("Failed to process a transaction record: {}", e);
    //         }
    //     }
    // }

}

// /// Keep all accounts in memory for a quick lookup by id
// pub struct AccountStore {
//     map: HashMap<AccountId, Account>
// }

// impl AccountStore {
//     pub(crate) fn new() -> Self {
//         Self{map: HashMap::new() }
//     }

//     pub(crate) fn get(&mut self, id: AccountId) -> Result<&mut Account, Error> {
//         Ok(self.map.entry(id).or_insert_with_key(|id| Account::new(*id)))
//     }

//     pub(crate) fn list(&self) -> IntoValues<AccountId, Account> {
//         self.map.into_values()
//     }
// }
