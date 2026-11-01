//! File di etichette umane (appendice F).

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde_json::Value;

use crate::classifier::DocInfo;

/// Righe del file JSONL, nell'ordine del file.
pub fn read(path: &Path) -> Result<Vec<Value>, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("impossibile leggere {}: {e}", path.display()))?;
    let mut records = Vec::new();
    for (index, line) in text.lines().enumerate().filter(|(_, line)| !line.trim().is_empty()) {
        let record = serde_json::from_str(line).map_err(|e| format!("{}, riga {}: {e}", path.display(), index + 1))?;
        records.push(record);
    }
    Ok(records)
}

/// Lato e tipo per nome del documento: se un documento compare più volte vale l'ultima riga.
pub fn documents(records: &[Value]) -> HashMap<String, DocInfo> {
    records
        .iter()
        .filter(|record| record["level"] == "document")
        .filter_map(|record| {
            let info = DocInfo { side: text(&record["side"]), doc_type: text(&record["doc_type"]) };
            Some((record["document"].as_str()?.to_string(), info))
        })
        .collect()
}

fn text(value: &Value) -> Option<String> {
    value.as_str().map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn etichette_documento_con_l_ultima_riga_che_vale() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("etichette.jsonl");
        fs::write(
            &path,
            concat!(
                r#"{"level":"document","document":"a.md","side":"prime","doc_type":"other"}"#, "\n\n",
                r#"{"level":"paragraph","document":"a.md","line_start":1,"line_end":1}"#, "\n",
                r#"{"level":"document","document":"a.md","side":"supplier","doc_type":"pa_plan"}"#, "\n",
            ),
        )
        .unwrap();
        let records = read(&path).unwrap();
        assert_eq!(records.len(), 3);
        let info = documents(&records);
        assert_eq!(info.len(), 1);
        assert_eq!(info["a.md"], DocInfo { side: Some("supplier".into()), doc_type: Some("pa_plan".into()) });
        assert_eq!(records[1]["level"], json!("paragraph"));
    }

    #[test]
    fn riga_non_valida() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("e.jsonl");
        fs::write(&path, "{}\n{ rotta\n").unwrap();
        assert!(read(&path).unwrap_err().contains("riga 2"));
    }
}
