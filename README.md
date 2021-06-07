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

### Performance Improvements

Intuition tells me that the current bottleneck of the applications is likely be the reading of records from CSV and 
possibly the parsing of these records into the `Transaction` struct. Once this is addressed, we can look at 
executing tasks in parallel and possibly even prioritizing certain types of transactions.

Having said that, profiling the application against large data sets prior to optimization and having a clear requirement
in mind is key to avoiding the trap of premature optimization. This approach also allows us to focus our efforts and 
measure our improvement. 

#### CSV Parsing

In order to speed up the CSV parsing, I would first follow the advice [here](https://docs.rs/csv/1.0.0/csv/tutorial/index.html#performance).
An effort should be made to amortize allocations and avoid UTF-8 checks by reading and writing ByteRecords instead of
StringRecords. Any `str`s will now be `&[u8]`s, so we lose the API around Strings, but in the interest or performance
that could be a worthwhile tradeoff. At this stage we should also profile the performance of deserializing a CSV byte
record into a `Transaction` struct and determine if it is worth implementing a custom deserializer. 

#### Parallelization 

With CSV parsing sped up, the next thing we can look at is threading. It may make sense to have a reader thread and one 
(or more) processing threads. The reader thread will simply read ByteRecords records from CSV and place the deserialized
`Transaction` at the tail of a queue. The processing thread will take items from the head of the queue and process them. 

Depending on requirements, we could also choose to prioritize certain types of transactions. Deposits and withdrawals 
likely happen significantly more often than disputes, resolves, and chargebacks which are not likely to be issued for 
quite some time after a deposit. For this reason, we could split the processing into two queues; one for deposits and 
withdrawals, and the other for disputes. The dispute processing thread could sleep, waking to process `Transactions` in
its queue on some interval. 

#### Branch Prediction and Cache Optimization 

Lastly, we can leverage tools such as Callgrind and KCacheGrind to identify possible bottlenecks caused by 
branch prediction / cache misses. Optimization here should focus on the 'hot path' as optimization handling of edge
cases will not generally yield any significant improvement. 

### General Improvements 

In addition to the above mentioned performance improvements, I will consider implementing making the following 
improvements: 

1. Clean up the tests.
2. Support various input data formats.
3. Add non-volatile storage, likely in a relational database.
4. Create a CI pipeline that runs an automated suite of tests on every PR and merge to devel/main branches. 