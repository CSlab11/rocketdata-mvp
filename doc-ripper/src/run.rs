use std::path::{Path, PathBuf};

use crate::report::{Report, SkipReason};

/// Risultato della conversione di un file: i file generati e ciò che è stato saltato.
#[derive(Debug, Default)]
pub struct Outcome {
    pub outputs: Vec<PathBuf>,
    pub skipped: Vec<SkipReason>,
}

impl Outcome {
    pub fn skipped(reason: SkipReason) -> Self {
        Outcome { outputs: Vec::new(), skipped: vec![reason] }
    }
}

/// Converte ogni file con `convert`; un errore non interrompe l'elaborazione degli altri (REQ-018).
pub fn process<I, F>(files: I, mut convert: F) -> Report
where
    I: IntoIterator<Item = PathBuf>,
    F: FnMut(&Path) -> Result<Outcome, String>,
{
    let mut report = Report::default();
    for file in files {
        match convert(&file) {
            Ok(outcome) => {
                for reason in outcome.skipped {
                    report.skipped.push((file.clone(), reason));
                }
                if !outcome.outputs.is_empty() {
                    report.converted.push((file, outcome.outputs));
                }
            }
            Err(error) => report.failed.push((file, error)),
        }
    }
    report
}

/// Codice di uscita: 0 se nessuna conversione è fallita, altrimenti 1 (REQ-018).
pub fn exit_code(report: &Report) -> u8 {
    if report.failed.is_empty() { 0 } else { 1 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prosegue_dopo_un_errore() {
        let files = vec![PathBuf::from("a.docx"), PathBuf::from("rotto.docx"), PathBuf::from("c.docx")];
        let mut seen = Vec::new();
        let report = process(files, |f| {
            seen.push(f.to_path_buf());
            if f == Path::new("rotto.docx") {
                Err("archivio non valido".into())
            } else {
                Ok(Outcome { outputs: vec![f.with_extension("md")], skipped: Vec::new() })
            }
        });
        assert_eq!(seen.len(), 3);
        assert_eq!(report.converted.len(), 2);
        assert_eq!(report.failed, vec![(PathBuf::from("rotto.docx"), "archivio non valido".to_string())]);
        assert_eq!(exit_code(&report), 1);
    }

    #[test]
    fn codice_zero_senza_errori() {
        let report = process(vec![PathBuf::from("a.txt")], |_| Ok(Outcome::skipped(SkipReason::Unsupported)));
        assert_eq!(report.skipped.len(), 1);
        assert!(report.converted.is_empty());
        assert_eq!(exit_code(&report), 0);
    }
}
