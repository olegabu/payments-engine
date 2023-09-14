# Payments Engine

Payments engine prototype processes transactions in a csv file and outputs account balances into stdout as csv.

## Build and test

Unit tests cover [de-](./src/transaction.rs#L68) and [serialization](./src/account.rs#L212), 
[happy paths](./src/engine.rs#L92), [exceptions](./src/engine.rs#L208).

```bash
cargo test
```

## Usage

The CLI takes only one argument the csv file with transactions and outputs into stdout. 
Errors go into stderr.

```bash
cargo run -- transactions.csv > accounts.csv
```

## Design

Domain entities: 
- [Account](./src/account.rs) can serialize into csv with care taken to [round](./src/account.rs#L7) amounts, 
keeps a list of its transactions in memory and has logic to process transactions; 
- [Transaction](./src/transaction.rs) can deserialize from csv.

[Engine](./src/engine.rs) takes its input from a `Read`, processes transactions and outputs accounts into a `Write`.
This is done to accomodate different streams, for example, the same method [input](./src/engine.rs#L36) 
takes csv from a file as well as hardcoded in unit tests.
