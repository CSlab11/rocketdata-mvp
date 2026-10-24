mod analyze;
mod section;

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

/// Cerca nei file Markdown con BM25 ed espansione della query RM3.
#[derive(Parser)]
#[command(version)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Indicizza i file .md di una cartella
    Index {
        /// Cartella dei file .md
        #[arg(long, value_name = "ORIGINE")]
        src: PathBuf,
        /// Cartella dell'indice
        #[arg(long, value_name = "INDICE")]
        index: PathBuf,
    },
    /// Cerca una domanda nell'indice
    Query {
        /// Cartella dell'indice
        #[arg(long, value_name = "INDICE")]
        index: PathBuf,
        /// Domanda
        #[arg(long, value_name = "DOMANDA")]
        q: String,
        /// Numero massimo di risultati
        #[arg(long, value_name = "N", default_value_t = 10)]
        top: usize,
    },
}

fn main() -> ExitCode {
    match Args::parse().command {
        Command::Index { .. } => ExitCode::SUCCESS,
        Command::Query { .. } => ExitCode::SUCCESS,
    }
}
