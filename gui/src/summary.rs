//! Lettura del riepilogo stampato da doc-ripper (REQ-032). Formato:
//!
//! ```text
//! Riepilogo
//!   Convertiti: N
//!     <origine> -> <generato>, <generato>
//!   Saltati: N
//!     <file> - <motivo>
//!   Falliti: N
//!     <file> - <errore>
//! ```

use serde::Serialize;

#[derive(Debug, Default, Clone, PartialEq, Serialize)]
pub struct Summary {
    pub converted: Vec<Converted>,
    pub skipped: Vec<Entry>,
    pub failed: Vec<Entry>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Converted {
    pub file: String,
    pub outputs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Entry {
    pub file: String,
    pub reason: String,
}

enum Section {
    Converted,
    Skipped,
    Failed,
}

/// Riepilogo contenuto nelle righe di output, se presente.
pub fn parse(lines: &[String]) -> Option<Summary> {
    let start = lines.iter().rposition(|l| l == "Riepilogo")?;
    let mut summary = Summary::default();
    let mut section = None;
    for line in &lines[start + 1..] {
        if line.starts_with("  Convertiti:") {
            section = Some(Section::Converted);
        } else if line.starts_with("  Saltati:") {
            section = Some(Section::Skipped);
        } else if line.starts_with("  Falliti:") {
            section = Some(Section::Failed);
        } else if let Some(item) = line.strip_prefix("    ") {
            match section {
                Some(Section::Converted) => {
                    if let Some((file, outputs)) = item.split_once(" -> ") {
                        let outputs = outputs.split(", ").map(str::to_string).collect();
                        summary.converted.push(Converted { file: file.to_string(), outputs });
                    }
                }
                Some(Section::Skipped) => summary.skipped.push(split_skipped(item)),
                Some(Section::Failed) => summary.failed.push(split_failed(item)),
                None => {}
            }
        }
    }
    Some(summary)
}

fn entry(file: &str, reason: &str) -> Entry {
    Entry { file: file.to_string(), reason: reason.to_string() }
}

/// Il motivo è uno di quelli noti: si cerca dalla fine, così un " - " nel nome del file non conta.
fn split_skipped(item: &str) -> Entry {
    for reason in ["non supportato", "scansionato"] {
        if let Some(file) = item.strip_suffix(&format!(" - {reason}")) {
            return entry(file, reason);
        }
    }
    match item.rfind(" - foglio vuoto \"") {
        Some(i) => entry(&item[..i], &item[i + 3..]),
        None => split_at_first(item),
    }
}

/// Il file termina con un'estensione supportata: si taglia al primo " - " dopo di essa.
fn split_failed(item: &str) -> Entry {
    let lower = item.to_ascii_lowercase();
    for (i, _) in item.match_indices(" - ") {
        if [".pdf", ".docx", ".xlsx", ".xls", ".ods"].iter().any(|ext| lower[..i].ends_with(ext)) {
            return entry(&item[..i], &item[i + 3..]);
        }
    }
    split_at_first(item)
}

fn split_at_first(item: &str) -> Entry {
    match item.split_once(" - ") {
        Some((file, reason)) => entry(file, reason),
        None => entry(item, ""),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(text: &str) -> Vec<String> {
        text.lines().map(str::to_string).collect()
    }

    #[test]
    fn legge_il_riepilogo_di_doc_ripper() {
        let output = lines(concat!(
            "Riepilogo\n",
            "  Convertiti: 2\n",
            "    doc\\Report - Q1.pdf -> doc\\Report - Q1_pdf.md\n",
            "    vendite.xlsx -> vendite_xlsx_Gennaio.csv, vendite_xlsx_Febbraio.csv\n",
            "  Saltati: 3\n",
            "    note - bozza.txt - non supportato\n",
            "    scansione.pdf - scansionato\n",
            "    vendite.xlsx - foglio vuoto \"Marzo - Aprile\"\n",
            "  Falliti: 1\n",
            "    Contratto - v2.docx - docx non valido: archivio - corrotto\n",
        ));
        let summary = parse(&output).unwrap();
        assert_eq!(
            summary.converted,
            vec![
                Converted { file: "doc\\Report - Q1.pdf".into(), outputs: vec!["doc\\Report - Q1_pdf.md".into()] },
                Converted {
                    file: "vendite.xlsx".into(),
                    outputs: vec!["vendite_xlsx_Gennaio.csv".into(), "vendite_xlsx_Febbraio.csv".into()],
                },
            ]
        );
        assert_eq!(
            summary.skipped,
            vec![
                entry("note - bozza.txt", "non supportato"),
                entry("scansione.pdf", "scansionato"),
                entry("vendite.xlsx", "foglio vuoto \"Marzo - Aprile\""),
            ]
        );
        assert_eq!(summary.failed, vec![entry("Contratto - v2.docx", "docx non valido: archivio - corrotto")]);
    }

    #[test]
    fn senza_riepilogo() {
        assert_eq!(parse(&lines("Errore: la cartella di origine non esiste")), None);
        assert_eq!(parse(&lines("Riepilogo\n  Convertiti: 0\n  Saltati: 0\n  Falliti: 0")), Some(Summary::default()));
    }
}
