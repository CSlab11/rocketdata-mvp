//! Scomposizione del Markdown in unità (appendice A).

use std::sync::LazyLock;

use regex::Regex;
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};

static HEADING: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^ {0,3}(#{1,6})(?:\s+(.*))?$").unwrap());
static CLOSING_HASHES: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s+#+\s*$").unwrap());
static CLAUSE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(\d+(?:\.\d+)*)\.?\s+\S").unwrap());
static SEPARATOR: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^ {0,3}(?:(?:-\s*){3,}|(?:\*\s*){3,}|(?:_\s*){3,})$").unwrap());
static LIST_ITEM: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(\s*)(?:[-*+]|\d+[.)])(?:\s+(.*))?$").unwrap());
static TABLE_DELIMITER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*\|?\s*:?-+:?\s*(?:\|\s*:?-+:?\s*)*\|?\s*$").unwrap());
static FENCE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^ {0,3}(`{3,}|~{3,})").unwrap());
static IMAGE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"!\[[^\]]*\]\([^)]*\)").unwrap());

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Paragraph,
    ListItem,
    TableRow,
}

impl Kind {
    /// Nome del tipo di unità nei file di uscita e in `rules.yaml`.
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Paragraph => "paragraph",
            Kind::ListItem => "list_item",
            Kind::TableRow => "table_row",
        }
    }
}

/// Unità da classificare.
#[derive(Debug, Clone, PartialEq)]
pub struct Unit {
    pub kind: Kind,
    pub text: String,
    /// Titoli che contengono l'unità, dal più esterno.
    pub heading_path: Vec<String>,
    /// Numero iniziale del titolo più vicino che ne ha uno.
    pub clause: Option<String>,
    /// Prima e ultima riga del file occupate dall'unità, numerate da 1 e incluse (REQ-087).
    pub line_start: usize,
    pub line_end: usize,
}

/// Primi 16 caratteri esadecimali dello SHA-1 del testo con gli spazi normalizzati (REQ-116).
pub fn text_hash(text: &str) -> String {
    let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let digest = Sha1::digest(normalized.as_bytes());
    digest.iter().take(8).map(|byte| format!("{byte:02x}")).collect()
}

/// Unità di un file Markdown, nell'ordine del file (REQ-086).
pub fn parse(markdown: &str) -> Vec<Unit> {
    let lines = visible_lines(markdown);
    let mut parser = Parser { lines: &lines, headings: Vec::new(), units: Vec::new() };
    parser.run();
    parser.units
}

/// Righe del file con blocchi di codice, commenti HTML e immagini svuotati: i numeri di riga non
/// cambiano e il resto del parser vede solo righe vuote.
fn visible_lines(markdown: &str) -> Vec<String> {
    let mut lines = Vec::new();
    let mut fence: Option<String> = None;
    let mut in_comment = false;
    for line in markdown.trim_start_matches('\u{feff}').lines() {
        if let Some(open) = &fence {
            let trimmed = line.trim();
            let closes = trimmed.len() >= open.len() && trimmed.chars().all(|c| Some(c) == open.chars().next());
            if closes {
                fence = None;
            }
            lines.push(String::new());
            continue;
        }
        if !in_comment {
            if let Some(found) = FENCE.captures(line) {
                fence = Some(found[1].to_string());
                lines.push(String::new());
                continue;
            }
        }
        let visible = without_comments(line, &mut in_comment);
        lines.push(IMAGE.replace_all(&visible, "").into_owned());
    }
    lines
}

fn without_comments(line: &str, in_comment: &mut bool) -> String {
    let mut visible = String::new();
    let mut rest = line;
    loop {
        if *in_comment {
            match rest.find("-->") {
                Some(end) => {
                    rest = &rest[end + 3..];
                    *in_comment = false;
                }
                None => return visible,
            }
        } else {
            match rest.find("<!--") {
                Some(start) => {
                    visible.push_str(&rest[..start]);
                    rest = &rest[start + 4..];
                    *in_comment = true;
                }
                None => {
                    visible.push_str(rest);
                    return visible;
                }
            }
        }
    }
}

struct Parser<'a> {
    lines: &'a [String],
    headings: Vec<(usize, String)>,
    units: Vec<Unit>,
}

impl Parser<'_> {
    fn run(&mut self) {
        let mut i = 0;
        while i < self.lines.len() {
            let line = &self.lines[i];
            if line.trim().is_empty() || SEPARATOR.is_match(line) {
                i += 1;
            } else if HEADING.is_match(line) {
                self.heading(line);
                i += 1;
            } else if self.table_at(i) {
                i = self.table(i);
            } else if LIST_ITEM.is_match(line) {
                i = self.list_item(i);
            } else {
                i = self.paragraph(i);
            }
        }
    }

    fn table_at(&self, i: usize) -> bool {
        self.lines[i].contains('|')
            && self.lines.get(i + 1).is_some_and(|next| next.contains('|') && TABLE_DELIMITER.is_match(next))
    }

    /// Il titolo sostituisce quelli di livello uguale o maggiore.
    fn heading(&mut self, line: &str) {
        let found = HEADING.captures(line).unwrap();
        let level = found[1].len();
        let title = found.get(2).map_or("", |m| m.as_str());
        let title = CLOSING_HASHES.replace(title, "").trim().to_string();
        self.headings.retain(|(l, _)| *l < level);
        self.headings.push((level, title));
    }

    /// `first` e `last` sono indici in `lines`, da 0.
    fn push(&mut self, kind: Kind, text: String, first: usize, last: usize) {
        let heading_path = self.headings.iter().map(|(_, title)| title.clone()).collect();
        let clause = self
            .headings
            .iter()
            .rev()
            .find_map(|(_, title)| CLAUSE.captures(title).map(|found| found[1].to_string()));
        self.units.push(Unit { kind, text, heading_path, clause, line_start: first + 1, line_end: last + 1 });
    }

    /// Ogni riga dati è un'unità `intestazione: valore; …`.
    fn table(&mut self, start: usize) -> usize {
        let header = cells(&self.lines[start]);
        let mut i = start + 2;
        while i < self.lines.len() && !self.lines[i].trim().is_empty() && self.lines[i].contains('|') {
            let row = cells(&self.lines[i]);
            let parts: Vec<String> = row
                .iter()
                .enumerate()
                .filter(|(_, value)| !value.is_empty())
                .map(|(column, value)| match header.get(column).filter(|h| !h.is_empty()) {
                    Some(name) => format!("{name}: {value}"),
                    None => value.clone(),
                })
                .collect();
            if !parts.is_empty() {
                self.push(Kind::TableRow, parts.join("; "), i, i);
            }
            i += 1;
        }
        i
    }

    /// Una voce per unità, con le righe di continuazione indentate.
    fn list_item(&mut self, start: usize) -> usize {
        let found = LIST_ITEM.captures(&self.lines[start]).unwrap();
        let indent = found[1].len();
        let mut parts = vec![found.get(2).map_or("", |m| m.as_str()).trim().to_string()];
        let mut i = start + 1;
        while i < self.lines.len() && self.continues_item(i, indent) {
            parts.push(self.lines[i].trim().to_string());
            i += 1;
        }
        let text = join(&parts);
        if !text.is_empty() {
            self.push(Kind::ListItem, text, start, i - 1);
        }
        i
    }

    fn continues_item(&self, i: usize, indent: usize) -> bool {
        let line = &self.lines[i];
        let line_indent = line.len() - line.trim_start().len();
        line_indent > indent && !self.block_starts(i) && !line.trim().is_empty()
    }

    fn paragraph(&mut self, start: usize) -> usize {
        let mut parts = vec![self.lines[start].trim().to_string()];
        let mut i = start + 1;
        while i < self.lines.len() && !self.lines[i].trim().is_empty() && !self.block_starts(i) {
            parts.push(self.lines[i].trim().to_string());
            i += 1;
        }
        self.push(Kind::Paragraph, join(&parts), start, i - 1);
        i
    }

    fn block_starts(&self, i: usize) -> bool {
        let line = &self.lines[i];
        SEPARATOR.is_match(line) || HEADING.is_match(line) || LIST_ITEM.is_match(line) || self.table_at(i)
    }
}

fn join(parts: &[String]) -> String {
    parts.iter().filter(|p| !p.is_empty()).cloned().collect::<Vec<_>>().join(" ")
}

/// Celle di una riga di tabella; `\|` è un carattere del testo.
fn cells(row: &str) -> Vec<String> {
    let row = row.trim();
    let row = row.strip_prefix('|').unwrap_or(row);
    let mut cells = Vec::new();
    let mut cell = String::new();
    let mut chars = row.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' if chars.peek() == Some(&'|') => {
                cell.push('|');
                chars.next();
            }
            '|' => cells.push(std::mem::take(&mut cell).trim().to_string()),
            c => cell.push(c),
        }
    }
    if !cell.trim().is_empty() {
        cells.push(cell.trim().to_string());
    }
    cells
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXAMPLE: &str = "# 4 Verification\n\n## 4.2 Thermal tests\n\nThermal cycling will be performed on each flight unit,\nwith 8 cycles between -30 °C and +60 °C.\n\n- Unit level: 8 cycles\n- Board level: \n  100 cycles\n\n| Test | Level |\n| --- | --- |\n| TVAC | Unit |\n";

    #[test]
    fn esempio_dell_appendice_a() {
        let units = parse(EXAMPLE);
        let found: Vec<_> = units.iter().map(|u| (u.kind, u.text.as_str())).collect();
        assert_eq!(
            found,
            [
                (Kind::Paragraph, "Thermal cycling will be performed on each flight unit, with 8 cycles between -30 °C and +60 °C."),
                (Kind::ListItem, "Unit level: 8 cycles"),
                (Kind::ListItem, "Board level: 100 cycles"),
                (Kind::TableRow, "Test: TVAC; Level: Unit"),
            ]
        );
        for unit in &units {
            assert_eq!(unit.heading_path, ["4 Verification", "4.2 Thermal tests"]);
            assert_eq!(unit.clause.as_deref(), Some("4.2"));
        }
    }

    #[test]
    fn text_hash_con_spazi_normalizzati() {
        assert_eq!(text_hash("abc"), "a9993e364706816a");
        assert_eq!(text_hash("  a   b	c "), text_hash("a b c"));
    }

    #[test]
    fn righe_dell_appendice_a() {
        let lines: Vec<_> = parse(EXAMPLE).iter().map(|u| (u.line_start, u.line_end)).collect();
        assert_eq!(lines, [(5, 6), (8, 8), (9, 10), (14, 14)]);
    }

    #[test]
    fn i_numeri_di_riga_contano_anche_le_righe_escluse() {
        let units = parse("```\nx\n```\n<!-- a\nb -->\nTesto\n");
        assert_eq!((units[0].line_start, units[0].line_end), (6, 6));
    }

    #[test]
    fn esclude_codice_immagini_commenti_e_separatori() {
        let markdown = "```\ncodice\n```\n\n![figura](a.png)\n\n<!-- image -->\n\n<!-- commento\nsu due righe -->\n\n---\n\n~~~rust\nfn main() {}\n~~~\n\nTesto\n";
        let units = parse(markdown);
        assert_eq!(units.len(), 1);
        assert_eq!(units[0].text, "Testo");
    }

    #[test]
    fn i_titoli_non_sono_unita_e_aggiornano_il_percorso() {
        let units = parse("# A\n\n## B\n\n### C\n\nuno\n\n## D ##\n\ndue\n\ntesto senza numero\n");
        assert_eq!(units[0].heading_path, ["A", "B", "C"]);
        assert_eq!(units[1].heading_path, ["A", "D"]);
        assert_eq!(units[0].clause, None);
    }

    #[test]
    fn clausola_dal_titolo_piu_vicino_che_ne_ha_una() {
        let units = parse("# 4.7.4 Interfaces\n\n## Dettagli\n\ntesto\n\n# 5. Altro\n\ntesto\n");
        assert_eq!(units[0].clause.as_deref(), Some("4.7.4"));
        assert_eq!(units[1].clause.as_deref(), Some("5"));
    }

    #[test]
    fn elenchi_di_ogni_tipo() {
        let units = parse("- a\n* b\n+ c\n1. d\n2) e\n  - f\n");
        let texts: Vec<_> = units.iter().map(|u| u.text.as_str()).collect();
        assert_eq!(texts, ["a", "b", "c", "d", "e", "f"]);
        assert!(units.iter().all(|u| u.kind == Kind::ListItem));
    }

    #[test]
    fn tabelle_con_celle_vuote_e_senza_intestazione() {
        let units = parse("| A | B | C |\n|---|:--|--:|\n| 1 |   | 3 |\n\n|  |  |\n|---|---|\n| x | y |\n");
        assert_eq!(units[0].text, "A: 1; C: 3");
        assert_eq!(units[1].text, "x; y");
    }

    #[test]
    fn tabella_con_barra_nel_testo() {
        let units = parse("| A |\n|---|\n| x \\| y |\n");
        assert_eq!(units[0].text, "A: x | y");
    }

    #[test]
    fn immagine_dentro_un_paragrafo() {
        let units = parse("Prima ![figura](a.png) dopo\n");
        assert_eq!(units[0].text, "Prima  dopo");
    }

    #[test]
    fn fine_riga_windows_e_bom() {
        let units = parse("\u{feff}# T\r\n\r\nriga uno\r\nriga due\r\n");
        assert_eq!(units[0].text, "riga uno riga due");
        assert_eq!(units[0].heading_path, ["T"]);
    }
}
