use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

use crate::analyze::Analyzer;
use crate::section;

const INDEX_FILE: &str = "index.json";
/// Marcatore con il nome del sotto-tool che ha creato l'indice (REQ-057).
const MARKER_FILE: &str = "search-tool";
const TOOL: &str = "search-rm3";

/// Indice BM25 delle sezioni dei `.md` di una cartella.
#[derive(Serialize, Deserialize)]
pub struct Index {
    pub sections: Vec<IndexedSection>,
    /// Termine → (sezione, frequenza del termine nella sezione).
    pub postings: HashMap<String, Vec<(u32, u32)>>,
    /// Numero di token di ogni sezione.
    pub lengths: Vec<u32>,
    pub avg_length: f64,
}

#[derive(Serialize, Deserialize)]
pub struct IndexedSection {
    /// Percorso del `.md` relativo alla cartella indicizzata, con separatore `/`.
    pub file: String,
    pub heading: String,
    pub line: usize,
    pub text: String,
}

/// Indicizza ricorsivamente i `.md` di `src`, escluso `index.md` nella radice (REQ-054).
pub fn build(src: &Path, analyzer: &Analyzer) -> Result<Index, String> {
    let mut index = Index { sections: Vec::new(), postings: HashMap::new(), lengths: Vec::new(), avg_length: 0.0 };
    let entries = WalkDir::new(src).sort_by_file_name().into_iter();
    for entry in entries {
        let entry = entry.map_err(|e| format!("impossibile leggere {}: {e}", src.display()))?;
        let path = entry.path();
        let is_md = path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("md"));
        if !entry.file_type().is_file() || !is_md || path == src.join("index.md") {
            continue;
        }
        let content = fs::read_to_string(path).map_err(|e| format!("impossibile leggere {}: {e}", path.display()))?;
        let rel = path.strip_prefix(src).unwrap();
        let file = rel.components().map(|c| c.as_os_str().to_string_lossy()).collect::<Vec<_>>().join("/");
        for section in section::split(&content) {
            add(&mut index, analyzer, IndexedSection { file: file.clone(), heading: section.heading, line: section.line, text: section.text });
        }
    }
    let total: u64 = index.lengths.iter().map(|&l| l as u64).sum();
    index.avg_length = if index.lengths.is_empty() { 0.0 } else { total as f64 / index.lengths.len() as f64 };
    Ok(index)
}

fn add(index: &mut Index, analyzer: &Analyzer, section: IndexedSection) {
    let id = index.sections.len() as u32;
    let tokens = analyzer.tokens(&section.text);
    let mut counts: HashMap<String, u32> = HashMap::new();
    for token in &tokens {
        *counts.entry(token.clone()).or_default() += 1;
    }
    for (term, tf) in counts {
        index.postings.entry(term).or_default().push((id, tf));
    }
    index.lengths.push(tokens.len() as u32);
    index.sections.push(section);
}

/// Scrive l'indice nella cartella, sostituendo quello esistente.
pub fn save(index: &Index, dir: &Path) -> Result<(), String> {
    fs::create_dir_all(dir).map_err(|e| format!("impossibile creare {}: {e}", dir.display()))?;
    let json = serde_json::to_string(index).map_err(|e| e.to_string())?;
    for (name, content) in [(MARKER_FILE, TOOL), (INDEX_FILE, json.as_str())] {
        let path = dir.join(name);
        fs::write(&path, content).map_err(|e| format!("impossibile scrivere {}: {e}", path.display()))?;
    }
    Ok(())
}

/// Legge l'indice dalla cartella.
pub fn load(dir: &Path) -> Result<Index, String> {
    let marker = fs::read_to_string(dir.join(MARKER_FILE))
        .map_err(|_| format!("la cartella non contiene un indice di {TOOL}: {}", dir.display()))?;
    if marker.trim() != TOOL {
        return Err(format!("l'indice in {} è stato creato da {}, non da {TOOL}", dir.display(), marker.trim()));
    }
    let path = dir.join(INDEX_FILE);
    let json = fs::read_to_string(&path).map_err(|e| format!("impossibile leggere l'indice {}: {e}", path.display()))?;
    serde_json::from_str(&json).map_err(|e| format!("indice non valido {}: {e}", path.display()))
}
