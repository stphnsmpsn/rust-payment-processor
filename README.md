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
```
cargo run -- sample-input/transactions.csv
``` 

If you wish to pipe your output to a file, you may do so by: 
```
cargo run -- sample-input/transactions.csv > accounts.csv
``` 

To run the tests, run:
```
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

```
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
```
RUST_LOG=off cargo run sample-input/transactions.csv
```

Valid levels for `RUST_LOG` are: 
* error
* warn
* info
* debug
* trace

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