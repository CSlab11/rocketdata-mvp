use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};

use rocket_tag::{iterate, labels};
use rocket_tag::classifier::DocInfo;
use rocket_tag::output::{Item, Output};
use rocket_tag::rules::RuleSet;
use rocket_tag::schema::Schema;

/// Classifica i paragrafi dei file Markdown con regole o con un LLM locale.
#[derive(Parser)]
#[command(version)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Classifica file Markdown
    Classify {
        #[command(subcommand)]
        command: ClassifyCommand,
    },
    /// Controlli sul file delle regole
    Rules {
        #[command(subcommand)]
        command: RulesCommand,
    },
}

#[derive(Subcommand)]
enum ClassifyCommand {
    /// Classifica un file Markdown o una cartella con le regole
    Rules {
        /// File Markdown o cartella
        #[arg(value_name = "INGRESSO")]
        input: PathBuf,
        /// File di uscita
        #[arg(short, value_name = "FILE")]
        output: PathBuf,
        /// File dello schema
        #[arg(long, value_name = "FILE")]
        schema: PathBuf,
        /// File delle regole
        #[arg(long, value_name = "FILE")]
        rules: PathBuf,
        /// Etichette umane da cui ricavare lato e tipo dei documenti
        #[arg(long, value_name = "FILE")]
        labels: Option<PathBuf>,
        /// File o cartella da non leggere, relativo all'ingresso (ripetibile)
        #[arg(long, value_name = "PERCORSO")]
        exclude: Vec<String>,
    },
}

#[derive(Subcommand)]
enum RulesCommand {
    /// Controlla rules.yaml contro lo schema
    Check {
        /// File dello schema
        #[arg(long, value_name = "FILE")]
        schema: PathBuf,
        /// File delle regole
        #[arg(long, value_name = "FILE")]
        rules: PathBuf,
    },
}

fn main() -> ExitCode {
    let result = match Args::parse().command {
        Command::Classify { command: ClassifyCommand::Rules { input, output, schema, rules, labels, exclude } } => {
            run_classify_rules(&input, &output, &schema, &rules, labels.as_deref(), &exclude)
        }
        Command::Rules { command: RulesCommand::Check { schema, rules } } => run_rules_check(&schema, &rules),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Errore: {error}");
            ExitCode::FAILURE
        }
    }
}

/// Regole controllate contro lo schema: un errore blocca l'esecuzione (REQ-103).
fn checked_rules(schema: &Schema, path: &Path) -> Result<RuleSet, String> {
    let rules = RuleSet::load(path)?;
    let check = rules.check(schema);
    for warning in &check.warnings {
        eprintln!("avviso: {warning}");
    }
    if !check.errors.is_empty() {
        return Err(check.errors.join("\n"));
    }
    Ok(rules)
}

/// Lato e tipo dalle etichette documento del file di etichette umane, se fornito (REQ-089).
fn known_documents(labels: Option<&Path>) -> Result<HashMap<String, DocInfo>, String> {
    match labels {
        Some(path) => Ok(labels::documents(&labels::read(path)?)),
        None => Ok(HashMap::new()),
    }
}

/// Classifica le unità con le regole e scrive il file di uscita (REQ-114).
fn run_classify_rules(
    input: &Path,
    output: &Path,
    schema: &Path,
    rules: &Path,
    labels: Option<&Path>,
    exclude: &[String],
) -> Result<(), String> {
    let schema = Schema::load(schema)?;
    let rules = checked_rules(&schema, rules)?;
    let known = known_documents(labels)?;
    let info = |document: &str| {
        known.get(document).cloned().or_else(|| rules.document_info(document)).unwrap_or_default()
    };
    let jobs = iterate::jobs(&iterate::documents(input, exclude)?, &info)?;
    let mut result = Output::new(&schema.version, format!("rules-v{}", rules.version));
    for (index, job) in jobs.iter().enumerate() {
        let decision = rules.classify(&schema, &job.unit.text, &job.context());
        result.items.push(Item::new(job, decision));
        println!("[{}/{}] {}", index + 1, jobs.len(), job.document);
    }
    result.write(output)
}

fn run_rules_check(schema: &Path, rules: &Path) -> Result<(), String> {
    let schema = Schema::load(schema)?;
    let rules = checked_rules(&schema, rules)?;
    println!("rules.yaml v{} coerente con lo schema v{}", rules.version, schema.version);
    Ok(())
}
