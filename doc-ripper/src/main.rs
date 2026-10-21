mod report;
mod run;

use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;

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
    let report = run::process(Vec::new(), |_| Ok(run::Outcome::default()));
    print!("{report}");
    ExitCode::from(run::exit_code(&report))
}
