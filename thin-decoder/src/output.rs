//! Scrittura del file JSON di output: per ogni domanda, la lista dei
//! risultati dei motori attivi (regole, LLM).

use serde::Serialize;

use crate::color::Color;

pub const NOT_FOUND: &str = "Not stated in the text.";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    /// Risposta ricavata da un claim (o da un fatto derivato).
    Answered,
    /// Nessun claim corrisponde: si restituisce la frase più vicina.
    TextualEvidence,
    /// Il testo non contiene l'informazione richiesta.
    NotFound,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct Evidence {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub claim: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub claim_color: Option<Color>,
    pub doc: String,
    pub sentence: usize,
    pub start: usize,
    pub end: usize,
    pub text: String,
}

/// Motore che ha prodotto il risultato.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Engine {
    Rules,
    Llm,
}

/// Specifica di compito: tipo di richiesta e, per il motore a regole, il
/// pattern della domanda con la sua origine.
#[derive(Debug, Clone, Serialize)]
pub struct Task {
    #[serde(rename = "type")]
    pub request: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    /// Tripla con `?` al posto della variabile.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected: Option<String>,
    /// Origine del pattern: `rules` o `llm`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<&'static str>,
}

impl Task {
    pub fn question() -> Task {
        Task { request: "domanda", kind: None, pattern: None, expected: None, source: None }
    }
}

/// Risultato di un motore per una domanda.
#[derive(Debug, Clone, Serialize)]
pub struct QA {
    /// Domanda, già riportata nella voce di output.
    #[serde(skip)]
    #[cfg_attr(not(test), allow(dead_code))]
    pub question: String,
    pub engine: Engine,
    pub task: Task,
    pub answer: String,
    pub status: Status,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub evidence: Vec<Evidence>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub reasoning: Vec<String>,
    pub color: Color,
}

impl QA {
    pub fn not_found(question: &str) -> QA {
        QA {
            question: question.to_string(),
            answer: NOT_FOUND.to_string(),
            status: Status::NotFound,
            evidence: Vec::new(),
            reasoning: Vec::new(),
            color: Color::Blu,
            engine: Engine::Rules,
            task: Task::question(),
        }
    }
}

/// Voce di output: una domanda con i risultati dei motori attivi.
#[derive(Debug, Clone, Serialize)]
pub struct Entry {
    pub question: String,
    pub results: Vec<QA>,
}

pub fn write_output(path: &str, answers: &[Entry]) -> Result<(), String> {
    let json = serde_json::to_string_pretty(answers).map_err(|e| e.to_string())?;
    std::fs::write(path, json + "\n").map_err(|e| format!("impossibile scrivere {path}: {e}"))
}

/// Stampa a terminale la domanda numerata e, per ogni motore, la risposta e
/// le fonti.
pub fn print_entry(index: usize, entry: &Entry) {
    println!("[{index}] {}", entry.question);
    for qa in &entry.results {
        print_result(qa);
    }
}

fn print_result(qa: &QA) {
    let engine = match qa.engine {
        Engine::Rules => "",
        Engine::Llm => " (llm)",
    };
    let tag = match qa.status {
        Status::Answered => "",
        Status::TextualEvidence => "  (evidenza testuale)",
        Status::NotFound => "  (lacuna)",
    };
    println!("    → {}{tag} [{}]{engine}", qa.answer, qa.color.label());
    for ev in &qa.evidence {
        let color = ev.claim_color.map(|c| format!(" [{}]", c.label())).unwrap_or_default();
        println!("      fonte: frase {}{color} «{}»", ev.sentence, ev.text);
    }
    for r in &qa.reasoning {
        println!("      regola: {r}");
    }
}
