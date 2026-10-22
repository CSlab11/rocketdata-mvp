use std::path::Path;

use crate::report::SkipReason;
use crate::run::Outcome;

/// Tipi di file che doc-ripper sa convertire.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Kind {
    Pdf,
    Docx,
    Spreadsheet,
}

impl Kind {
    pub fn of(path: &Path) -> Option<Kind> {
        let ext = path.extension()?.to_str()?.to_ascii_lowercase();
        match ext.as_str() {
            "pdf" => Some(Kind::Pdf),
            "docx" => Some(Kind::Docx),
            "xlsx" | "xls" | "ods" => Some(Kind::Spreadsheet),
            _ => None,
        }
    }
}

/// Converte il file `rel` (relativo a `src`) scrivendo i risultati sotto `dst`.
/// I file di tipo non supportato non vengono elaborati né copiati (REQ-019).
pub fn convert(src: &Path, dst: &Path, rel: &Path) -> Result<Outcome, String> {
    match Kind::of(rel) {
        None => Ok(Outcome::skipped(SkipReason::Unsupported)),
        Some(Kind::Spreadsheet) => crate::excel::convert(src, dst, rel),
        Some(Kind::Docx) => crate::docx::convert(src, dst, rel),
        Some(Kind::Pdf) => crate::pdf::convert(src, dst, rel),
    }
}

/// Base dei nomi generati: nome del file seguito dall'estensione in minuscolo
/// (es. `report.PDF` → `report_pdf`).
pub fn base_name(rel: &Path) -> String {
    let stem = rel.file_stem().unwrap_or_default().to_string_lossy();
    let ext = rel.extension().unwrap_or_default().to_string_lossy().to_ascii_lowercase();
    format!("{stem}_{ext}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn riconosce_le_estensioni() {
        assert_eq!(Kind::of(Path::new("a/report.PDF")), Some(Kind::Pdf));
        assert_eq!(Kind::of(Path::new("report.docx")), Some(Kind::Docx));
        assert_eq!(Kind::of(Path::new("vendite.xlsx")), Some(Kind::Spreadsheet));
        assert_eq!(Kind::of(Path::new("vendite.xls")), Some(Kind::Spreadsheet));
        assert_eq!(Kind::of(Path::new("vendite.ods")), Some(Kind::Spreadsheet));
        assert_eq!(Kind::of(Path::new("note.txt")), None);
        assert_eq!(Kind::of(Path::new("vecchio.doc")), None);
        assert_eq!(Kind::of(Path::new("senza_estensione")), None);
    }
}
