//! Iteratore sui file Markdown: dalle cartelle alle unità da classificare.

use std::fs;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use crate::classifier::{Context, DocInfo};
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
    /// Testo dell'unità precedente e successiva dello stesso file (REQ-088).
    pub prev: Option<String>,
    pub next: Option<String>,
    pub info: DocInfo,
}

impl Job {
    pub fn context(&self) -> Context {
        Context {
            document: Some(self.document.clone()),
            kind: Some(self.unit.kind),
            heading_path: self.unit.heading_path.clone(),
            clause: self.unit.clause.clone(),
            prev: self.prev.clone(),
            next: self.next.clone(),
            side: self.info.side.clone(),
            doc_type: self.info.doc_type.clone(),
        }
    }
}

/// Il file indicato, o i file `*.md` della cartella e delle sottocartelle (REQ-085), tolti i file
/// e le cartelle di `exclude`, percorsi relativi alla cartella (REQ-128).
pub fn documents(input: &Path, exclude: &[String]) -> Result<Vec<Document>, String> {
    if input.is_file() {
        let name = input.file_name().map_or_else(|| input.display().to_string(), |n| n.to_string_lossy().into_owned());
        return Ok(vec![Document { name, path: input.to_path_buf() }]);
    }
    if !input.is_dir() {
        return Err(format!("ingresso non trovato: {}", input.display()));
    }
    let mut documents = Vec::new();
    for entry in WalkDir::new(input).sort_by_file_name() {
        let entry = entry.map_err(|e| format!("impossibile leggere {}: {e}", input.display()))?;
        let is_md = entry.path().extension().is_some_and(|ext| ext.eq_ignore_ascii_case("md"));
        if entry.file_type().is_file() && is_md {
            let name = relative_name(input, entry.path());
            if !excluded(&name, exclude) {
                documents.push(Document { name, path: entry.into_path() });
            }
        }
    }
    Ok(documents)
}

/// Il file è uno di quelli esclusi o sta in una cartella esclusa, senza distinguere maiuscole e
/// minuscole come il file system di Windows.
fn excluded(name: &str, exclude: &[String]) -> bool {
    let name = name.to_lowercase();
    exclude.iter().any(|path| {
        let path = path.replace('\\', "/").trim_matches('/').to_lowercase();
        name == path || name.starts_with(&format!("{path}/"))
    })
}

/// Percorso relativo alla cartella di ingresso con separatore `/` (REQ-090).
fn relative_name(root: &Path, path: &Path) -> String {
    let relative = path.strip_prefix(root).unwrap_or(path);
    relative.components().map(|c| c.as_os_str().to_string_lossy()).collect::<Vec<_>>().join("/")
}

/// Unità di tutti i documenti, nell'ordine dei file, per chiamare il classificatore su ciascuna
/// (REQ-085). `info` dà lato e tipo di un documento dal suo nome.
pub fn jobs(documents: &[Document], info: &dyn Fn(&str) -> DocInfo) -> Result<Vec<Job>, String> {
    let mut jobs = Vec::new();
    for document in documents {
        let text = fs::read_to_string(&document.path)
            .map_err(|e| format!("impossibile leggere {}: {e}", document.path.display()))?;
        let units = markdown::parse(&text);
        let doc_info = info(&document.name);
        for (i, unit) in units.iter().enumerate() {
            jobs.push(Job {
                document: document.name.clone(),
                unit: unit.clone(),
                prev: i.checked_sub(1).map(|p| units[p].text.clone()),
                next: units.get(i + 1).map(|n| n.text.clone()),
                info: doc_info.clone(),
            });
        }
    }
    Ok(jobs)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unknown(_: &str) -> DocInfo {
        DocInfo::default()
    }

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
        let found = documents(dir.path(), &[]).unwrap();
        assert_eq!(found.len(), 2);
        let texts: Vec<_> = jobs(&found, &unknown).unwrap().into_iter().map(|j| j.unit.text).collect();
        assert_eq!(texts, ["uno", "due", "tre"]);
    }

    #[test]
    fn accetta_un_singolo_file() {
        let dir = folder(&[("a.md", "uno\n")]);
        assert_eq!(documents(&dir.path().join("a.md"), &[]).unwrap().len(), 1);
    }

    #[test]
    fn unita_vicine_dello_stesso_file() {
        let dir = folder(&[("a.md", "uno\n\ntitolo\n\n- due\n"), ("b.md", "tre\n")]);
        let found = jobs(&documents(dir.path(), &[]).unwrap(), &unknown).unwrap();
        let near: Vec<_> = found.iter().map(|j| (j.prev.as_deref(), j.next.as_deref())).collect();
        assert_eq!(near[0], (None, Some("titolo")));
        assert_eq!(near[1], (Some("uno"), Some("due")));
        assert_eq!(near[2], (Some("titolo"), None), "le vicine non passano da un file all'altro");
        assert_eq!(near[3], (None, None));
    }

    #[test]
    fn lato_e_tipo_passati_al_classificatore() {
        let dir = folder(&[("a.md", "uno\n")]);
        let info = |name: &str| DocInfo { side: Some(format!("lato di {name}")), doc_type: None };
        let context = jobs(&documents(dir.path(), &[]).unwrap(), &info).unwrap()[0].context();
        assert_eq!(context.side.as_deref(), Some("lato di a.md"));
        assert_eq!(context.doc_type, None);
    }

    #[test]
    fn nome_del_documento_relativo_con_barra() {
        let dir = folder(&[("a.md", "uno\n"), ("sotto/piano/b.md", "due\n")]);
        let names: Vec<_> = documents(dir.path(), &[]).unwrap().into_iter().map(|d| d.name).collect();
        assert_eq!(names, ["a.md", "sotto/piano/b.md"]);
        assert_eq!(documents(&dir.path().join("sotto/piano/b.md"), &[]).unwrap()[0].name, "b.md");
    }

    #[test]
    fn ingresso_inesistente_e_un_errore() {
        let dir = tempfile::tempdir().unwrap();
        assert!(documents(&dir.path().join("manca"), &[]).unwrap_err().contains("ingresso non trovato"));
    }

    #[test]
    fn esclude_file_e_cartelle() {
        let dir = folder(&[
            ("index.md", "x\n"),
            ("a.md", "x\n"),
            (".index/b.md", "x\n"),
            (".index/sotto/c.md", "x\n"),
            ("sotto/index.md", "x\n"),
            (".indexato/d.md", "x\n"),
        ]);
        let exclude = ["INDEX.md".to_string(), ".index/".to_string()];
        let names: Vec<_> = documents(dir.path(), &exclude).unwrap().into_iter().map(|d| d.name).collect();
        assert_eq!(names, [".indexato/d.md", "a.md", "sotto/index.md"], "solo i percorsi indicati, dalla radice");
    }

    #[test]
    fn esclusione_con_barra_rovesciata() {
        let dir = folder(&[("sotto/a.md", "x\n"), ("b.md", "x\n")]);
        let found = documents(dir.path(), &["sotto\\a.md".to_string()]).unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "b.md");
    }
}
