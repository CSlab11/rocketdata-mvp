use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Duration;

use clap::{Parser, Subcommand};
use serde_json::{Value, json};

use rocket_tag::{iterate, labels};
use rocket_tag::classifier::{Context, DocInfo};
use rocket_tag::llm::{self, Format, Llm, Run, Settings};
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
    /// Classifica un testo passato da riga di comando
    One(OneArgs),
}

#[derive(clap::Args)]
struct OneArgs {
    /// Testo da classificare
    #[arg(long, value_name = "TESTO")]
    text: String,
    /// File dello schema
    #[arg(long, value_name = "FILE")]
    schema: PathBuf,
    /// File delle regole: classifica con le regole
    #[arg(long, value_name = "FILE", conflicts_with = "base_url", required_unless_present = "base_url")]
    rules: Option<PathBuf>,
    /// URL del server: classifica con l'LLM
    #[arg(long, value_name = "URL", requires_all = ["model", "guide", "examples"])]
    base_url: Option<String>,
    /// Nome del modello
    #[arg(long, value_name = "NOME")]
    model: Option<String>,
    /// Regole di decisione per il modello (prompt_guide.md)
    #[arg(long, value_name = "FILE")]
    guide: Option<PathBuf>,
    /// Esempi per il modello (guide_examples.json)
    #[arg(long, value_name = "FILE")]
    examples: Option<PathBuf>,
    /// Lato del documento
    #[arg(long, value_name = "LATO")]
    side: Option<String>,
    /// Testo dell'unità precedente
    #[arg(long, value_name = "TESTO")]
    prev: Option<String>,
    /// Testo dell'unità successiva
    #[arg(long, value_name = "TESTO")]
    next: Option<String>,
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
    /// File di uscita del classificatore a regole, per le confidenze per accordo
    #[arg(long, value_name = "FILE")]
    rules_out: Option<PathBuf>,
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
    /// Richieste parallele
    #[arg(long, value_name = "N", default_value_t = 2)]
    workers: usize,
    /// Numero massimo di unità da classificare
    #[arg(long, value_name = "N")]
    limit: Option<usize>,
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
        Command::Classify { command: ClassifyCommand::One(args) } => run_classify_one(&args),
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

/// Testo di `prompt_guide.md` ed esempi di `guide_examples.json`.
fn read_guide(guide: &Path, examples: &Path) -> Result<(String, Vec<Value>), String> {
    let read = |path: &Path| fs::read_to_string(path).map_err(|e| format!("impossibile leggere {}: {e}", path.display()));
    let parsed = serde_json::from_str(&read(examples)?).map_err(|e| format!("{}: {e}", examples.display()))?;
    Ok((read(guide)?, parsed))
}

/// Classifica un solo testo e scrive l'esito su stdout (REQ-084).
fn run_classify_one(args: &OneArgs) -> Result<(), String> {
    let schema = Schema::load(&args.schema)?;
    let context =
        Context { side: args.side.clone(), prev: args.prev.clone(), next: args.next.clone(), ..Context::default() };
    let decision = match &args.rules {
        Some(path) => checked_rules(&schema, path)?.classify(&schema, &args.text, &context),
        None => {
            let (Some(base_url), Some(model), Some(guide), Some(examples)) = (&args.base_url, &args.model, &args.guide, &args.examples)
            else {
                return Err("indicare --rules oppure --base-url, --model, --guide ed --examples".to_string());
            };
            let (guide, examples) = read_guide(guide, examples)?;
            let settings = Settings {
                base_url: base_url.clone(),
                model: model.clone(),
                api_key: api_key(),
                format: Format::JsonSchema,
                temperature: 0.0,
                timeout: Duration::from_secs(180),
            };
            let llm = Llm::new(&schema, &guide, &examples, settings);
            llm.classify(&schema, &args.text, &context, None).map_err(|failure| failure.message().to_string())?
        }
    };
    println!("{}", json!({"labels": decision.labels, "confidence": decision.confidence, "rule_ids": decision.rule_ids}));
    Ok(())
}

/// Mai da riga di comando: sarebbe visibile nell'elenco dei processi (REQ-132).
fn api_key() -> Option<String> {
    std::env::var("ROCKET_TAG_API_KEY").ok().filter(|key| !key.is_empty())
}

/// Classifica le unità con il server LLM e scrive il file di uscita (REQ-106).
fn run_classify_llm(args: &LlmArgs) -> Result<(), String> {
    let schema = Schema::load(&args.schema)?;
    let (guide, examples) = read_guide(&args.guide, &args.examples)?;
    let rules = args.rules.as_deref().map(|path| checked_rules(&schema, path)).transpose()?;
    let settings = Settings {
        base_url: args.base_url.clone(),
        model: args.model.clone(),
        api_key: api_key(),
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
    let from_rules = args.rules_out.as_deref().map(Output::read).transpose()?.as_ref().map(llm::rule_labels);
    let (items, mut jobs) = llm::resume(&args.output, &schema, &llm.prompt_id, from_rules.as_ref(), jobs);
    if let Some(limit) = args.limit {
        jobs.truncate(limit);
    }
    let mut result = Output::new(&schema.version, llm.classifier_name());
    result.prompt_id = Some(llm.prompt_id.clone());
    result.items = items;
    let run = Run { llm: &llm, schema: &schema, rules: from_rules.as_ref(), workers: args.workers, path: &args.output };
    run.classify_all(&jobs, &mut result)?;
    result.write(&args.output)
}

fn run_rules_check(schema: &Path, rules: &Path) -> Result<(), String> {
    let schema = Schema::load(schema)?;
    let rules = checked_rules(&schema, rules)?;
    println!("rules.yaml v{} coerente con lo schema v{}", rules.version, schema.version);
    Ok(())
}
