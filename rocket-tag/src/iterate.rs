//! Iteratore sui file Markdown: dalle cartelle alle unità da classificare.

use std::fs;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use crate::markdown::{self, Unit};

/// File Markdown da classificare.
#[derive(Debug, Clone, PartialEq)]
pub struct Document {
    pub name: String,
    pub path: PathBuf,
}

/// Unità con ciò che serve al classificatore.
#[derive(Debug, Clone)]
pub struct Job {
    pub document: String,
    pub unit: Unit,
}

/// Il file indicato, o i file `*.md` della cartella e delle sottocartelle (REQ-085).
pub fn documents(input: &Path) -> Result<Vec<Document>, String> {
    if input.is_file() {
        return Ok(vec![Document { name: input.display().to_string(), path: input.to_path_buf() }]);
    }
    if !input.is_dir() {
        return Err(format!("ingresso non trovato: {}", input.display()));
    }
    let mut documents = Vec::new();
    for entry in WalkDir::new(input).sort_by_file_name() {
        let entry = entry.map_err(|e| format!("impossibile leggere {}: {e}", input.display()))?;
        let is_md = entry.path().extension().is_some_and(|ext| ext.eq_ignore_ascii_case("md"));
        if entry.file_type().is_file() && is_md {
            documents.push(Document { name: entry.path().display().to_string(), path: entry.into_path() });
        }
    }
    Ok(documents)
}

/// Unità di tutti i documenti, nell'ordine dei file, per chiamare il classificatore su ciascuna
/// (REQ-085).
pub fn jobs(documents: &[Document]) -> Result<Vec<Job>, String> {
    let mut jobs = Vec::new();
    for document in documents {
        let text = fs::read_to_string(&document.path)
            .map_err(|e| format!("impossibile leggere {}: {e}", document.path.display()))?;
        jobs.extend(markdown::parse(&text).into_iter().map(|unit| Job { document: document.name.clone(), unit }));
    }
    Ok(jobs)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn folder(files: &[(&str, &str)]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for (name, text) in files {
            let path = dir.path().join(name);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, text).unwrap();
        }
        dir
    }

    #[test]
    fn legge_ricorsivamente_i_file_md() {
        let dir = folder(&[("a.md", "uno\n"), ("sotto/b.MD", "due\n\ntre\n"), ("c.txt", "no\n")]);
        let found = documents(dir.path()).unwrap();
        assert_eq!(found.len(), 2);
        let texts: Vec<_> = jobs(&found).unwrap().into_iter().map(|j| j.unit.text).collect();
        assert_eq!(texts, ["uno", "due", "tre"]);
    }

    #[test]
    fn accetta_un_singolo_file() {
        let dir = folder(&[("a.md", "uno\n")]);
        assert_eq!(documents(&dir.path().join("a.md")).unwrap().len(), 1);
    }

    #[test]
    fn ingresso_inesistente_e_un_errore() {
        let dir = tempfile::tempdir().unwrap();
        assert!(documents(&dir.path().join("manca")).unwrap_err().contains("ingresso non trovato"));
    }
}
