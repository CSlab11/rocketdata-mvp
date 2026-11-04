//! Thin Decoder: risponde a domande su un testo in inglese con regole
//! deterministiche, grafi di claim e ontologie. Senza configurazione LLM
//! non usa alcun LLM.

mod answer;
mod color;
mod config;
mod extract;
mod input;
mod lexicon;
mod llm;
mod llm_answer;
mod llm_extract;
mod llm_ontology;
mod llm_question;
mod ontology;
mod output;
mod quantity;
mod question;
mod text;

use std::process::ExitCode;

const USAGE: &str = "uso: thin-decoder <input.json> <output.json> [--ontology <ontologia.json>] [--config <config.json>]
     thin-decoder propose-ontology <input.json> <ontologia.json> --config <config.json>";

struct Args {
    input: String,
    output: String,
    ontology: Option<String>,
    config: Option<String>,
}

fn parse_args(raw: &[String]) -> Result<Args, String> {
    let mut positional = Vec::new();
    let mut ontology = None;
    let mut config = None;
    let mut it = raw.iter();
    while let Some(a) = it.next() {
        if a == "--ontology" {
            ontology = Some(it.next().ok_or(USAGE)?.clone());
        } else if a == "--config" {
            config = Some(it.next().ok_or(USAGE)?.clone());
        } else {
            positional.push(a.clone());
        }
    }
    match positional.as_slice() {
        [input, output] => Ok(Args {
            input: input.clone(),
            output: output.clone(),
            ontology,
            config,
        }),
        _ => Err(USAGE.to_string()),
    }
}

fn run(args: &Args) -> Result<(), String> {
    let config = match &args.config {
        Some(path) => config::read_config(path)?,
        None => config::Config::default(),
    };
    let input = input::read_input(&args.input)?;
    let sentences = text::split_sentences(&input.text, text::tokenize(&input.text));
    let llm = config.llm_client()?;
    let rule_claims = match config.extract.rules() {
        true => extract::extract(&input.text, &sentences, &args.input),
        false => Vec::new(),
    };
    // L'ontologia ricavata dal testo usa solo i claim delle regole.
    let mut onto = ontology::from_text(&input.text, &sentences, &rule_claims);
    let claims = match &llm {
        Some(llm) if config.extract.llm() => {
            let chosen = llm_extract::sentences_for(config.extract_scope, &sentences, &rule_claims);
            let llm_claims =
                llm_extract::extract(llm, &input.text, &chosen, &args.input, rule_claims.len() + 1)?;
            llm_extract::merge(rule_claims, llm_claims)
        }
        _ => rule_claims,
    };
    onto.add_defaults();
    if let Some(path) = &args.ontology {
        onto.load_file(path)?;
    }
    let kb = answer::Kb {
        src: &input.text,
        sentences: &sentences,
        claims: &claims,
        onto: &onto,
    };
    // Ogni motore attivo riceve tutti i claim del primo passaggio.
    let mut answers = Vec::new();
    for q in &input.questions {
        let mut results = Vec::new();
        if config.reason.rules() {
            let pattern = match (question::parse(q), &llm) {
                (None, Some(llm)) if config.question_llm => llm_question::pattern(llm, &onto, q)?,
                (p, _) => p,
            };
            results.push(answer::answer_with(&kb, q, pattern));
        }
        if let (true, Some(llm)) = (config.reason.llm(), &llm) {
            results.push(llm_answer::answer(llm, &kb, q)?);
        }
        answers.push(output::Entry { question: q.clone(), results });
    }
    for (i, entry) in answers.iter().enumerate() {
        output::print_entry(i + 1, entry);
    }
    output::write_output(&args.output, &answers)
}

/// Comando separato: propone con l'LLM un'ontologia dal testo di input e la
/// scrive nel formato dell'ontologia a priori, da rivedere a mano.
fn propose_ontology(args: &Args) -> Result<(), String> {
    let path = args.config.as_ref().ok_or(USAGE)?;
    let config = config::read_config(path)?;
    let llm = match (&config.llm, config.ontology_llm) {
        (Some(cfg), true) => llm::Llm::new(cfg.clone()),
        _ => return Err("configurazione: propose-ontology richiede ontology_llm e la sezione llm".into()),
    };
    let input = input::read_input(&args.input)?;
    let onto = llm_ontology::propose(&llm, &input.text)?;
    let json = serde_json::to_string_pretty(&onto).map_err(|e| e.to_string())?;
    std::fs::write(&args.output, json + "
").map_err(|e| format!("impossibile scrivere {}: {e}", args.output))?;
    println!("ontologia proposta in {}: rivederla a mano prima di usarla con --ontology", args.output);
    Ok(())
}

fn main() -> ExitCode {
    let mut raw: Vec<String> = std::env::args().skip(1).collect();
    let propose = raw.first().map_or(false, |a| a == "propose-ontology");
    if propose {
        raw.remove(0);
    }
    let args = match parse_args(&raw) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(2);
        }
    };
    let result = if propose { propose_ontology(&args) } else { run(&args) };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("errore: {e}");
            ExitCode::FAILURE
        }
    }
}
