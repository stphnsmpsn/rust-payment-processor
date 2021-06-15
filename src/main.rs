mod account;
mod bank;
mod errors;
mod transaction;
use bank::Bank;
use log::{error, info};
use structopt::StructOpt;
#[macro_use]
extern crate log;
use env_logger::Env;

#[derive(StructOpt, Debug)]
struct Cli {
    #[structopt(parse(from_os_str))]
    input_file: std::path::PathBuf,
}

fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("off")).init();
    info!("Rust Payment Processor Started");
    let args = Cli::from_args();
    let mut bank = Bank::new();
    match csv::ReaderBuilder::new().trim(csv::Trim::All).from_path(args.input_file) {
        Ok(mut reader) => {
            bank.process_record_set(&mut reader);
            bank.print_accounts();
        }
        Err(e) => {
            error!("{}", e);
        }
    }
}
