//! File di uscita dei classificatori (appendice E).

use std::fs;
use std::path::Path;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::classifier::Decision;
use crate::iterate::Job;

/// Etichette di un'unità.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Item {
    pub document: String,
    pub line_start: usize,
    pub line_end: usize,
    /// Un valore per faccetta (REQ-115).
    pub labels: Map<String, Value>,
}

impl Item {
    pub fn new(job: &Job, decision: Decision) -> Item {
        Item {
            document: job.document.clone(),
            line_start: job.unit.line_start,
            line_end: job.unit.line_end,
            labels: decision.labels,
        }
    }
}

/// Unità scartata, con il motivo.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Skipped {
    pub document: String,
    pub line_start: usize,
    pub line_end: usize,
    pub error: String,
}

/// Contenuto del file di uscita di un'esecuzione (REQ-114).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Output {
    pub schema_version: String,
    pub classifier: String,
    pub created_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_id: Option<String>,
    pub items: Vec<Item>,
    pub skipped: Vec<Skipped>,
}

impl Output {
    pub fn new(schema_version: &str, classifier: String) -> Output {
        Output {
            schema_version: schema_version.to_string(),
            classifier,
            created_at: humantime::format_rfc3339_seconds(SystemTime::now()).to_string(),
            prompt_id: None,
            items: Vec::new(),
            skipped: Vec::new(),
        }
    }

    /// Scrive il file, con gli elementi per documento e poi per riga.
    pub fn write(&mut self, path: &Path) -> Result<(), String> {
        self.items.sort_by(|a, b| (&a.document, a.line_start).cmp(&(&b.document, b.line_start)));
        self.skipped.sort_by(|a, b| (&a.document, a.line_start).cmp(&(&b.document, b.line_start)));
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
            fs::create_dir_all(dir).map_err(|e| format!("impossibile creare {}: {e}", dir.display()))?;
        }
        fs::write(path, json).map_err(|e| format!("impossibile scrivere {}: {e}", path.display()))
    }

    pub fn read(path: &Path) -> Result<Output, String> {
        let text = fs::read_to_string(path).map_err(|e| format!("impossibile leggere {}: {e}", path.display()))?;
        serde_json::from_str(&text).map_err(|e| format!("{}: file di uscita non valido: {e}", path.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(document: &str, line_start: usize) -> Item {
        Item { document: document.into(), line_start, line_end: line_start, labels: Map::new() }
    }

    #[test]
    fn formato_dell_appendice_e() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sotto/out.json");
        let mut output = Output::new("0.1.0", "rules-v0.1.0".into());
        output.items.push(item("b.md", 1));
        output.items.push(item("a.md", 9));
        output.items.push(item("a.md", 2));
        output.skipped.push(Skipped { document: "a.md".into(), line_start: 4, line_end: 5, error: "motivo".into() });
        output.write(&path).unwrap();

        let saved: serde_json::Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(saved["schema_version"], "0.1.0");
        assert_eq!(saved["classifier"], "rules-v0.1.0");
        assert_eq!(saved["created_at"].as_str().unwrap().len(), "2026-11-02T10:00:00Z".len());
        assert!(saved.get("prompt_id").is_none(), "solo per l'LLM");
        assert_eq!(saved["skipped"][0]["error"], "motivo");
        let order: Vec<_> = Output::read(&path).unwrap().items.iter().map(|i| (i.document.clone(), i.line_start)).collect();
        assert_eq!(order, [("a.md".to_string(), 2), ("a.md".to_string(), 9), ("b.md".to_string(), 1)]);
    }
}
