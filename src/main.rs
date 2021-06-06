mod bank;
use bank::Bank;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
struct Cli {
    #[structopt(parse(from_os_str))]
    input_file: std::path::PathBuf,
}

fn main() {
    let args = Cli::from_args();
    let mut bank = Bank::new();
    if let Ok(mut reader) = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_path(args.input_file)
    {
        bank.process_record_set(&mut reader);
        bank.print_accounts();
    }
}
