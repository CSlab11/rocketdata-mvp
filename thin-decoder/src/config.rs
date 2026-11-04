//! Configurazione LLM (file JSON passato con `--config`). Senza file, o con
//! i valori predefiniti, il sistema usa solo le regole e nessun LLM.

use serde::Deserialize;

use crate::llm::Llm;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Server e modello LLM; assente = nessun LLM disponibile.
    #[serde(default)]
    pub llm: Option<LlmConfig>,
    /// Primo passaggio: estrazione dei claim.
    #[serde(default)]
    pub extract: Strategy,
    /// Frasi inviate all'LLM per l'estrazione.
    #[serde(default)]
    pub extract_scope: Scope,
    /// Secondo passaggio: elaborazione (risposte).
    #[serde(default)]
    pub reason: Strategy,
    /// Interpretazione con LLM delle domande che le regole non riconoscono.
    #[serde(default)]
    pub question_llm: bool,
    /// Proposta di ontologia con LLM (solo comando `propose-ontology`).
    #[serde(default)]
    pub ontology_llm: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Scope {
    /// Solo le frasi in cui le regole non trovano claim.
    #[default]
    Uncovered,
    /// Tutte le frasi.
    All,
}

/// Motori usati in un passaggio.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Strategy {
    #[default]
    Rules,
    Llm,
    Both,
}

impl Strategy {
    pub fn rules(self) -> bool {
        self != Strategy::Llm
    }

    pub fn llm(self) -> bool {
        self != Strategy::Rules
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LlmConfig {
    /// Indirizzo del server, es. `http://localhost:11434`.
    pub url: String,
    pub model: String,
    /// Versione del modello (es. digest di Ollama), parte della chiave di cache.
    #[serde(default)]
    pub version: String,
    /// Cartella della cache delle risposte.
    #[serde(default = "default_cache_dir")]
    pub cache_dir: String,
    /// Temperatura di campionamento; 0 per la riproducibilità.
    #[serde(default)]
    pub temperature: f64,
    #[serde(default = "default_seed")]
    pub seed: u64,
}

fn default_seed() -> u64 {
    42
}

fn default_cache_dir() -> String {
    ".td-cache".into()
}

impl Config {
    /// Vero se almeno una funzione LLM è attiva durante le domande; la proposta
    /// di ontologia non ne fa parte.
    pub fn uses_llm(&self) -> bool {
        self.extract.llm() || self.reason.llm() || self.question_llm
    }

    /// Client LLM se almeno una funzione LLM è attiva; errore se manca la
    /// sezione `llm`.
    pub fn llm_client(&self) -> Result<Option<Llm>, String> {
        match (&self.llm, self.uses_llm()) {
            (_, false) => Ok(None),
            (Some(cfg), true) => Ok(Some(Llm::new(cfg.clone()))),
            (None, true) => Err("configurazione: le funzioni LLM attive richiedono la sezione llm".into()),
        }
    }
}

pub fn read_config(path: &str) -> Result<Config, String> {
    let raw = std::fs::read_to_string(path)
        .map_err(|e| format!("impossibile leggere {path}: {e}"))?;
    serde_json::from_str(&raw).map_err(|e| format!("configurazione non valida in {path}: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_and_server_only_configs_use_no_llm() {
        assert!(!Config::default().uses_llm());
        let c: Config =
            serde_json::from_str(r#"{"llm": {"url": "http://localhost:1", "model": "m"}}"#).unwrap();
        assert!(!c.uses_llm());
    }

    #[test]
    fn extract_strategy_and_missing_server() {
        let c: Config = serde_json::from_str(r#"{"extract": "both"}"#).unwrap();
        assert!(c.extract.rules() && c.extract.llm() && c.uses_llm());
        assert!(c.llm_client().is_err());
        assert_eq!(Config::default().extract, Strategy::Rules);
        assert_eq!(Config::default().reason, Strategy::Rules);
        let c: Config = serde_json::from_str(r#"{"reason": "llm"}"#).unwrap();
        assert!(c.uses_llm() && !c.reason.rules());
    }

    #[test]
    fn question_and_ontology_flags_are_separate() {
        let c: Config = serde_json::from_str(r#"{"question_llm": true}"#).unwrap();
        assert!(c.question_llm && !c.ontology_llm && c.uses_llm());
        let c: Config = serde_json::from_str(r#"{"ontology_llm": true}"#).unwrap();
        assert!(c.ontology_llm && !c.question_llm && !c.uses_llm());
        let d = Config::default();
        assert!(!d.question_llm && !d.ontology_llm);
    }
}
