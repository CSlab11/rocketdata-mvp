mod convert;
mod csv_out;
mod docx;
mod excel;
mod output;
mod report;
mod run;
mod walk;

use std::fs;
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
    let (src, dst) = match prepare(&args) {
        Ok(paths) => paths,
        Err(error) => {
            eprintln!("Errore: impossibile creare la cartella di destinazione {}: {error}", args.dst.display());
            return ExitCode::from(2);
        }
    };

    let tree = walk::scan(&src, &dst);
    let mut failed_dirs = Vec::new();
    for dir in &tree.dirs {
        if let Err(error) = fs::create_dir_all(dst.join(dir)) {
            failed_dirs.push((dir.clone(), error.to_string()));
        }
    }
    let mut report = run::process(tree.files, |rel| convert::convert(&src, &dst, rel));
    report.failed.extend(failed_dirs);
    report.failed.extend(tree.errors);

    print!("{report}");
    ExitCode::from(run::exit_code(&report))
}

/// Crea la cartella di destinazione se manca e restituisce i percorsi canonici di origine e destinazione.
fn prepare(args: &Args) -> std::io::Result<(PathBuf, PathBuf)> {
    fs::create_dir_all(&args.dst)?;
    Ok((args.src.canonicalize()?, args.dst.canonicalize()?))
}
