use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};

use rocket_tag::iterate;
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
        Command::Classify { command: ClassifyCommand::Rules { input, output, schema, rules } } => {
            run_classify_rules(&input, &output, &schema, &rules)
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

/// Classifica le unità con le regole e scrive il file di uscita (REQ-114).
fn run_classify_rules(input: &Path, output: &Path, schema: &Path, rules: &Path) -> Result<(), String> {
    let schema = Schema::load(schema)?;
    let rules = checked_rules(&schema, rules)?;
    let jobs = iterate::jobs(&iterate::documents(input)?)?;
    let mut result = Output::new(&schema.version, format!("rules-v{}", rules.version));
    for job in jobs {
        result.items.push(Item { document: job.document, line_start: job.unit.line_start, line_end: job.unit.line_end });
    }
    result.write(output)
}

fn run_rules_check(schema: &Path, rules: &Path) -> Result<(), String> {
    let schema = Schema::load(schema)?;
    let rules = checked_rules(&schema, rules)?;
    println!("rules.yaml v{} coerente con lo schema v{}", rules.version, schema.version);
    Ok(())
}
