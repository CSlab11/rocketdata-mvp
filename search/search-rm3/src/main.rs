mod analyze;
mod bm25;
mod index;
mod query;
mod rm3;
mod section;

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};

use crate::analyze::Analyzer;

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
        /// Sezioni della prima ricerca usate per l'espansione RM3
        #[arg(long, value_name = "N", default_value_t = 10)]
        fb_docs: usize,
        /// Termini di espansione RM3
        #[arg(long, value_name = "N", default_value_t = 10)]
        fb_terms: usize,
        /// Peso della query originale, tra 0 e 1
        #[arg(long, value_name = "X", default_value_t = 0.5, value_parser = parse_lambda)]
        lambda: f64,
    },
}

fn main() -> ExitCode {
    let result = match Args::parse().command {
        Command::Index { src, index } => run_index(&src, &index),
        Command::Query { index, q, top, fb_docs, fb_terms, lambda } => {
            run_query(&index, &q, top, &rm3::Params { fb_docs, fb_terms, lambda })
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Errore: {error}");
            ExitCode::from(2)
        }
    }
}

fn parse_lambda(value: &str) -> Result<f64, String> {
    match value.parse::<f64>() {
        Ok(x) if (0.0..=1.0).contains(&x) => Ok(x),
        _ => Err("deve essere un numero tra 0 e 1".to_string()),
    }
}

fn run_query(dir: &Path, q: &str, top: usize, params: &rm3::Params) -> Result<(), String> {
    let index = index::load(dir)?;
    let response = query::run(&index, &Analyzer::new(), q, top, params);
    println!("{}", serde_json::to_string_pretty(&response).map_err(|e| e.to_string())?);
    Ok(())
}

fn run_index(src: &Path, dir: &Path) -> Result<(), String> {
    if !src.is_dir() {
        return Err(format!("la cartella di origine non esiste o non è una cartella: {}", src.display()));
    }
    let index = index::build(src, &Analyzer::new())?;
    index::save(&index, dir)?;
    let files = index.sections.iter().map(|s| s.file.as_str()).collect::<HashSet<_>>().len();
    println!("Indicizzati {files} file, {} sezioni", index.sections.len());
    Ok(())
}
