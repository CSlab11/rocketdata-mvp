//! Lettura del file JSON di input: `{"questions": [...], "text": "..."}`.

use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Input {
    pub questions: Vec<String>,
    pub text: String,
}

pub fn read_input(path: &str) -> Result<Input, String> {
    let raw = std::fs::read_to_string(path)
        .map_err(|e| format!("impossibile leggere {path}: {e}"))?;
    serde_json::from_str(&raw).map_err(|e| format!("JSON di input non valido in {path}: {e}"))
}
