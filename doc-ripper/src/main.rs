mod report;

use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;

use report::Report;

/// Converte i documenti di una cartella in Markdown e CSV.
#[derive(Parser)]
#[command(version)]
struct Args {
    /// Cartella di origine
    #[arg(long, value_name = "ORIGINE")]
    src: PathBuf,
    /// Cartella di destinazione
    #[arg(long, value_name = "DESTINAZIONE")]
    dst: PathBuf,
}

fn main() -> ExitCode {
    let args = Args::parse();
    if !args.src.is_dir() {
        eprintln!("Errore: la cartella di origine non esiste o non è una cartella: {}", args.src.display());
        return ExitCode::from(2);
    }
    let _ = args.dst;
    let report = Report::default();
    print!("{report}");
    ExitCode::SUCCESS
}
