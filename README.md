# rust-payment-processor
A basic payment processor, written in Rust

This payment processor is a simple rust crate generated using `cargo init`.
It can be built using `cargo build`, and ran using `cargo run`. The application takes a single command-line argument
which is to be the path of the input data. Data should be in `.csv` format, but we do not care about the extension.
If the path can not be found, an error will be logged to stderr, and the application will exit.

This repo has been analyzed for security vulnerabilities using `cargo-audit`, has been linted with *Clippy*, and 
formatted with `cargo fmt`. It has also been run with `cargo-valgrind` against various data sets and no issues have been reported.

## Assumptions

1. Disputes, resolves, and chargebacks are only possible on `TransactionType::Deposit`.
2. Transactions happen chronologically in a file
3. Once an account has been locked due to a chargeback, all subsequent transactions to this account will return an error. 
4. We will not terminate the application in the event of a bad transaction, we will simply discard it and move on.

## Usage

To process a CSV formatted list of transactions, simply run the application as follows:
```shell
cargo run -- sample-input/transactions.csv
``` 

If you wish to pipe your output to a file, you may do so by: 
```shell
cargo run -- sample-input/transactions.csv > accounts.csv
``` 

To run the tests, run:
```shell
cargo test
```
### Input Data Format

This simple payment processor takes in CSV formatted data with the following columns:

|column|description                  |
|------|-----------------------------|
|type  |  A String. ("deposit"  "withdrawal" "dispute" "resolve" or "chargeback")|
|client| a valid u16 client ID       |
|tx    | a valid u32 transaction ID  |
|amount| decimal value with a precision of up to four places past the decimal|

An example data set containing only deposits and withdrawals is shown below. More data sets can be found in the 
repo under 'sample-input'.

```csv
type,       client, tx, amount
deposit,    1,      1,  1.0
deposit,    2,      2,  2.0
deposit,    1,      3,  2.0
withdrawal, 1,      4,  1.5
withdrawal, 2,      5,  3.0
```

A few points on CSV formatting: 
1. Values are case-sensitive.
2. The *Only* supported delimiter is: `,`.
3. Whitespace doesn't matter.
4. Column ordering doesn't matter.
5. Amounts will be rounded to four decimal places.

### Logging

This crate uses an env-logger; by default log messages of type `error` will be written to stderr. You can control the
log level through the use of environment variables as described below:
```shell
RUST_LOG=off cargo run sample-input/transactions.csv
```

Valid levels for `RUST_LOG` are: 
* error
* warn
* info
* debug
* trace

## Core Dependencies

### SERDE

Serde is a framework for serializing and deserializing Rust data structures efficiently and generically. To learn 
more, check it out on crates.io [here](https://crates.io/crates/serde).

Using SERDE allows me to define a `Transaction` struct as shown below and simply derive the functionality needed
to serialize / deserialize it. Since the CSV crate provides support for SERDE, using them in common allows very
readable (maintainable) code and reduces boilerplate. 

Serde also provides support for enumerations (internally tagged, externally tagged, and untagged). This allows me 
to be confident that a Transaction that was deserialized properly definitely has valid data for all of the types 
within the struct. 

Note: Below, I derived the functionality for both Serialize and Deserialize even though I do not serialize transactions.
That's ok; in Rust, we only pay for what we use and since I do not use this functionality, I don't incur any additional
overhead for including it here. 

```rust
#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct Transaction {
    #[serde(rename = "type")]
    kind: TransactionType,
    client: u16,
    tx: u32,
    amount: Option<Decimal>,
    #[serde(default)]
    under_dispute: bool,
}
```

### Decimal

A Decimal implementation written in pure Rust suitable for financial calculations that require significant integral 
and fractional digits with no round-off errors. To learn more, check it out on crates.io
[here](https://crates.io/crates/rust_decimal).

The rust_decimal crate is really great. Not only does it provide data types suitable for financial calculations, but 
it also provides functions to normalize and round (according to various rounding schemes) our data. Additionally,
rust_decimal_macros provides some super useful macros to make creating Decimals very easy. 

The Decimal data type also supports all common arithmetic operations out of the box. 

### CSV

A fast and flexible CSV reader and writer for Rust, with support for Serde. To learn more, check it out on crates.io
[here](https://crates.io/crates/csv).

The CSV crate makes dealing with CSV data a snap. Especially with how nicely it plays with SERDE. 

## Future Improvements

In the future, I will consider implementing making the following improvements: 

1. Implementing a custom deserializer for transactions to eliminate the need to check for
negative amounts.
2. Cleaning up the tests.
3. Profiling the application against large data sets.
4. Running Valgrind, Callgrind and KCacheGrind with much larger data sets to identify possible
   bottlenecks caused by branch prediction / cache misses.
5. Cache optimization (if analysis warrants it).
6. Supporting various input data formats.
7. Non-volatile storage, likely in a relational database.    
8. Creating a CI pipeline that runs an automated suite of tests on every PR and merge to devel/main branches. 