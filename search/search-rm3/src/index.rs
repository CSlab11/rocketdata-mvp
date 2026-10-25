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
    /// Termine → (sezione, frequenza nel resto della sezione, frequenza nell'intestazione).
    pub postings: HashMap<String, Vec<(u32, u32, u32)>>,
    /// Numero di token del resto e dell'intestazione di ogni sezione.
    pub lengths: Vec<(u32, u32)>,
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
    let mut index = Index { sections: Vec::new(), postings: HashMap::new(), lengths: Vec::new() };
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
    Ok(index)
}

/// Aggiunge la sezione contando a parte i token dell'intestazione e quelli del resto (REQ-067).
fn add(index: &mut Index, analyzer: &Analyzer, section: IndexedSection) {
    let id = index.sections.len() as u32;
    let (first, rest) = section.text.split_once('\n').unwrap_or((&section.text, ""));
    let (heading, body) = match section::heading(first) {
        Some(_) => (analyzer.tokens(&section.heading), analyzer.tokens(rest)),
        None => (Vec::new(), analyzer.tokens(&section.text)),
    };
    let mut counts: HashMap<String, (u32, u32)> = HashMap::new();
    for token in &body {
        counts.entry(token.clone()).or_default().0 += 1;
    }
    for token in &heading {
        counts.entry(token.clone()).or_default().1 += 1;
    }
    for (term, (tf_body, tf_heading)) in counts {
        index.postings.entry(term).or_default().push((id, tf_body, tf_heading));
    }
    index.lengths.push((body.len() as u32, heading.len() as u32));
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
    serde_json::from_str(&json)
        .map_err(|e| format!("indice non valido {}: {e}; ricrearlo con il comando index", path.display()))
}
