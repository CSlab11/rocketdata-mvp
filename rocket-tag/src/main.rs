use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Duration;

use clap::{Parser, Subcommand};

use rocket_tag::{iterate, labels};
use rocket_tag::classifier::DocInfo;
use rocket_tag::llm::{self, Format, Llm, Settings};
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
    /// Classifica un file Markdown o una cartella con un LLM locale
    Llm(LlmArgs),
}

#[derive(clap::Args)]
struct LlmArgs {
    /// File Markdown o cartella
    #[arg(value_name = "INGRESSO")]
    input: PathBuf,
    /// File di uscita
    #[arg(short, value_name = "FILE")]
    output: PathBuf,
    /// File dello schema
    #[arg(long, value_name = "FILE")]
    schema: PathBuf,
    /// Regole di decisione per il modello (prompt_guide.md)
    #[arg(long, value_name = "FILE")]
    guide: PathBuf,
    /// Esempi per il modello (guide_examples.json)
    #[arg(long, value_name = "FILE")]
    examples: PathBuf,
    /// URL del server compatibile con l'API OpenAI
    #[arg(long, value_name = "URL")]
    base_url: String,
    /// Nome del modello
    #[arg(long, value_name = "NOME")]
    model: String,
    /// Regole da cui ricavare lato e tipo dei documenti (sezione documents)
    #[arg(long, value_name = "FILE")]
    rules: Option<PathBuf>,
    /// Etichette umane da cui ricavare lato e tipo dei documenti
    #[arg(long, value_name = "FILE")]
    labels: Option<PathBuf>,
    /// Modo in cui si vincola la risposta
    #[arg(long, value_enum, default_value_t = Format::JsonSchema)]
    format: Format,
    /// Temperatura del modello
    #[arg(long, default_value_t = 0.0)]
    temperature: f64,
    /// Secondi di attesa per ogni richiesta
    #[arg(long, value_name = "SECONDI", default_value_t = 180)]
    timeout: u64,
    /// File o cartella da non leggere, relativo all'ingresso (ripetibile)
    #[arg(long, value_name = "PERCORSO")]
    exclude: Vec<String>,
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
        Command::Classify { command: ClassifyCommand::Llm(args) } => run_classify_llm(&args),
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

/// Classifica le unità con il server LLM e scrive il file di uscita (REQ-106).
fn run_classify_llm(args: &LlmArgs) -> Result<(), String> {
    let schema = Schema::load(&args.schema)?;
    let read = |path: &Path| fs::read_to_string(path).map_err(|e| format!("impossibile leggere {}: {e}", path.display()));
    let guide = read(&args.guide)?;
    let examples: Vec<serde_json::Value> =
        serde_json::from_str(&read(&args.examples)?).map_err(|e| format!("{}: {e}", args.examples.display()))?;
    let rules = args.rules.as_deref().map(|path| checked_rules(&schema, path)).transpose()?;
    let settings = Settings {
        base_url: args.base_url.clone(),
        model: args.model.clone(),
        // Mai da riga di comando: sarebbe visibile nell'elenco dei processi (REQ-132).
        api_key: std::env::var("ROCKET_TAG_API_KEY").ok().filter(|key| !key.is_empty()),
        format: args.format,
        temperature: args.temperature,
        timeout: Duration::from_secs(args.timeout),
    };
    let llm = Llm::new(&schema, &guide, &examples, settings);
    let known = known_documents(args.labels.as_deref())?;
    let info = |document: &str| {
        let from_rules = || rules.as_ref().and_then(|r| r.document_info(document));
        known.get(document).cloned().or_else(from_rules).unwrap_or_default()
    };
    let jobs = iterate::jobs(&iterate::documents(&args.input, &args.exclude)?, &info)?;
    let mut result = Output::new(&schema.version, llm.classifier_name());
    result.prompt_id = Some(llm.prompt_id.clone());
    llm::classify_all(&llm, &schema, &jobs, &mut result);
    result.write(&args.output)
}

fn run_rules_check(schema: &Path, rules: &Path) -> Result<(), String> {
    let schema = Schema::load(schema)?;
    let rules = checked_rules(&schema, rules)?;
    println!("rules.yaml v{} coerente con lo schema v{}", rules.version, schema.version);
    Ok(())
}
