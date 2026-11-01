//! Esecuzione di rocket-tag come processo separato: gui ne conosce i comandi e il file di uscita.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::search;
use crate::settings::ConfigPaths;

/// Classificatore di rocket-tag.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Classifier {
    Rules,
    Llm,
}

/// File di configurazione di rocket-tag.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Config {
    pub schema: PathBuf,
    pub rules: PathBuf,
    pub guide: PathBuf,
    pub examples: PathBuf,
}

impl Config {
    /// I file di `rocket-tag-config\` nella cartella di `exe`.
    pub fn beside(exe: &Path) -> Config {
        let dir = exe.with_file_name("rocket-tag-config");
        Config {
            schema: dir.join("label_schema.json"),
            rules: dir.join("rules.yaml"),
            guide: dir.join("prompt_guide.md"),
            examples: dir.join("guide_examples.json"),
        }
    }

    /// I percorsi indicati al posto di quelli predefiniti (REQ-140).
    pub fn with(self, paths: &ConfigPaths) -> Config {
        let chosen = |path: &str, default: PathBuf| if path.trim().is_empty() { default } else { PathBuf::from(path.trim()) };
        Config {
            schema: chosen(&paths.schema, self.schema),
            rules: chosen(&paths.rules, self.rules),
            guide: chosen(&paths.guide, self.guide),
            examples: chosen(&paths.examples, self.examples),
        }
    }
}

/// Testo del file Markdown `document`, solo se il percorso risolto resta dentro `src`, anche
/// seguendo collegamenti: `document` arriva da un file, non dall'utente (REQ-146).
pub fn read_document(src: &Path, document: &str) -> Result<String, String> {
    let outside = || format!("percorso fuori dalla cartella: {document}");
    if !search::is_relative_markdown(Path::new(document)) {
        return Err(outside());
    }
    let path = src.join(document);
    let (root, resolved) = (src.canonicalize().map_err(|e| e.to_string())?, path.canonicalize());
    let resolved = resolved.map_err(|_| format!("il file non esiste più: {document}"))?;
    if !resolved.starts_with(&root) {
        return Err(outside());
    }
    std::fs::read_to_string(resolved).map_err(|e| format!("impossibile leggere {document}: {e}"))
}

/// Server LLM da usare.
#[derive(Debug, Clone, PartialEq)]
pub struct Server {
    pub base_url: String,
    pub model: String,
}

/// File di uscita del classificatore per la cartella `src` (REQ-136).
pub fn output_file(src: &Path, classifier: Classifier) -> PathBuf {
    src.join(".classify").join(match classifier {
        Classifier::Rules => "rules.json",
        Classifier::Llm => "llm.json",
    })
}

/// Argomenti di `rocket-tag classify rules|llm` (REQ-133).
pub fn arguments(classifier: Classifier, src: &Path, config: &Config, server: Option<&Server>) -> Vec<OsString> {
    let name = match classifier {
        Classifier::Rules => "rules",
        Classifier::Llm => "llm",
    };
    let mut args: Vec<OsString> = vec!["classify".into(), name.into(), src.into(), "-o".into()];
    args.push(output_file(src, classifier).into());
    args.extend(["--schema".into(), config.schema.clone().into(), "--rules".into(), config.rules.clone().into()]);
    // L'indice di doc-ripper e l'indice di search-rm3 non sono documenti da classificare.
    args.extend(["--exclude".into(), "index.md".into(), "--exclude".into(), ".index".into()]);
    if classifier == Classifier::Llm {
        // Con il file delle regole valgono le confidenze per accordo (REQ-139).
        let rules = output_file(src, Classifier::Rules);
        if rules.is_file() {
            args.extend(["--rules-out".into(), rules.into()]);
        }
    }
    if let Some(server) = server {
        args.extend(["--guide".into(), config.guide.clone().into(), "--examples".into(), config.examples.clone().into()]);
        args.extend(["--base-url".into(), server.base_url.clone().into(), "--model".into(), server.model.clone().into()]);
    }
    args
}
