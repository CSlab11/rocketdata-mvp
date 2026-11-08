use std::fmt;
use std::path::PathBuf;

/// Perché un file o un foglio non è stato convertito.
#[derive(Debug, Clone, PartialEq)]
pub enum SkipReason {
    Unsupported,
    Scanned,
    EmptySheet(String),
}

impl fmt::Display for SkipReason {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SkipReason::Unsupported => write!(f, "non supportato"),
            SkipReason::Scanned => write!(f, "scansionato"),
            SkipReason::EmptySheet(name) => write!(f, "foglio vuoto \"{name}\""),
        }
    }
}

/// Esito dell'esecuzione (REQ-017). I percorsi sono relativi alle cartelle di origine e destinazione.
#[derive(Debug, Default)]
pub struct Report {
    pub converted: Vec<(PathBuf, Vec<PathBuf>)>,
    pub skipped: Vec<(PathBuf, SkipReason)>,
    pub failed: Vec<(PathBuf, String)>,
    /// Note da mostrare accanto ai file convertiti o saltati (es. ripiego su PDFium, REQ-070).
    pub notes: Vec<(PathBuf, String)>,
}

impl Report {
    ///  (nota) da aggiungere alla riga del file, se ne ha una.
    fn note(&self, src: &PathBuf) -> String {
        self.notes.iter().find(|(p, _)| p == src).map(|(_, n)| format!(" ({n})")).unwrap_or_default()
    }
}

impl fmt::Display for Report {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "Riepilogo")?;
        writeln!(f, "  Convertiti: {}", self.converted.len())?;
        for (src, outputs) in &self.converted {
            let outputs: Vec<String> = outputs.iter().map(|p| p.display().to_string()).collect();
            writeln!(f, "    {} -> {}{}", src.display(), outputs.join(", "), self.note(src))?;
        }
        writeln!(f, "  Saltati: {}", self.skipped.len())?;
        for (src, reason) in &self.skipped {
            writeln!(f, "    {} - {}{}", src.display(), reason, self.note(src))?;
        }
        writeln!(f, "  Falliti: {}", self.failed.len())?;
        for (src, error) in &self.failed {
            writeln!(f, "    {} - {}", src.display(), error)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn elenca_convertiti_saltati_e_falliti() {
        let report = Report {
            converted: vec![(
                PathBuf::from("vendite.xlsx"),
                vec![PathBuf::from("vendite_xlsx_Gennaio.csv"), PathBuf::from("vendite_xlsx_Febbraio.csv")],
            )],
            skipped: vec![
                (PathBuf::from("note.txt"), SkipReason::Unsupported),
                (PathBuf::from("scan.pdf"), SkipReason::Scanned),
                (PathBuf::from("vendite.xlsx"), SkipReason::EmptySheet("Marzo".into())),
            ],
            failed: vec![(PathBuf::from("rotto.docx"), "archivio non valido".into())],
            notes: vec![(PathBuf::from("scan.pdf"), "ripiego PDFium: Docling non disponibile".into())],
        };
        let text = report.to_string();
        assert!(text.contains("scan.pdf - scansionato (ripiego PDFium: Docling non disponibile)"));
        assert!(text.contains("Convertiti: 1"));
        assert!(text.contains("vendite.xlsx -> vendite_xlsx_Gennaio.csv, vendite_xlsx_Febbraio.csv"));
        assert!(text.contains("Saltati: 3"));
        assert!(text.contains("note.txt - non supportato"));
        assert!(text.contains("scan.pdf - scansionato"));
        assert!(text.contains("vendite.xlsx - foglio vuoto \"Marzo\""));
        assert!(text.contains("Falliti: 1"));
        assert!(text.contains("rotto.docx - archivio non valido"));
    }
}
