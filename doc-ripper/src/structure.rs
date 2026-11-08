//! Markdown dei PDF ricavato dalla struttura di Docling con regole proprie (REQ-048): il documento
//! diventa una sequenza di blocchi, ciascuno con la pagina in cui inizia, e i blocchi diventano Markdown.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::docling::{self, Document, Item, Ref};
use crate::docx::escape;
use crate::images::Images;

#[derive(Debug, Clone, PartialEq)]
pub enum Kind {
    /// Titolo con il suo livello (1-6).
    Heading(usize),
    Paragraph,
    /// Voce di elenco con la profondità di nidificazione (0 per il primo livello).
    ListItem(usize),
    /// Tabella come griglia di celle; la prima riga è l'intestazione.
    Table(Vec<Vec<String>>),
    /// Riferimento Markdown all'immagine estratta.
    Image(String),
    Code,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub kind: Kind,
    pub text: String,
    pub page: u32,
}

/// Blocchi del corpo del documento nell'ordine di lettura di Docling. Le immagini vengono registrate in
/// `images`; i loro file si leggono a partire da `base`, la cartella del JSON.
pub fn blocks(document: &Document, base: &Path, images: &mut Images) -> Vec<Block> {
    let mut walker = Walker { document, base, images, done: HashSet::new(), per_page: HashMap::new(), out: Vec::new() };
    walker.children(&document.body.children, 0);
    walker.out
}

struct Walker<'a> {
    document: &'a Document,
    base: &'a Path,
    images: &'a mut Images,
    /// Elementi già emessi: una didascalia può comparire sia nel corpo sia tra quelle di una tabella.
    done: HashSet<String>,
    /// Immagini già estratte per pagina, per i nomi dei file.
    per_page: HashMap<u32, usize>,
    out: Vec<Block>,
}

impl Walker<'_> {
    fn children(&mut self, refs: &[Ref], depth: usize) {
        for reference in refs {
            self.reference(reference, depth);
        }
    }

    fn reference(&mut self, reference: &Ref, depth: usize) {
        if !self.done.insert(reference.target.clone()) {
            return;
        }
        match self.document.get(reference) {
            Some(Item::Group(group)) => match group.label.as_str() {
                "list" | "ordered_list" => self.children(&group.children, depth + 1),
                // Testo spezzato da formattazioni diverse: un solo paragrafo.
                "inline" => {
                    let parts: Vec<&str> = group
                        .children
                        .iter()
                        .filter_map(|r| match self.document.get(r) {
                            Some(Item::Text(t)) => Some(t.text.trim()),
                            _ => None,
                        })
                        .filter(|t| !t.is_empty())
                        .collect();
                    let page = group.children.iter().find_map(|r| match self.document.get(r) {
                        Some(Item::Text(t)) => Some(docling::page(&t.prov)),
                        _ => None,
                    });
                    group.children.iter().for_each(|r| {
                        self.done.insert(r.target.clone());
                    });
                    self.push(Kind::Paragraph, parts.join(" "), page.unwrap_or(0));
                }
                _ => self.children(&group.children, depth),
            },
            Some(Item::Text(text)) => {
                let kind = match text.label.as_str() {
                    "title" | "section_header" => Kind::Heading(heading_level(&text.text, &text.label, text.level)),
                    "list_item" => Kind::ListItem(depth.saturating_sub(1)),
                    "code" => Kind::Code,
                    _ => Kind::Paragraph,
                };
                self.push(kind, text.text.trim().to_string(), docling::page(&text.prov));
                self.children(&text.children, depth);
            }
            Some(Item::Table(table)) => {
                let page = docling::page(&table.prov);
                let data = &table.data;
                let mut rows = vec![vec![String::new(); data.num_cols]; data.num_rows];
                // Righe coperte solo da celle d'intestazione (almeno una).
                let mut header = vec![(false, true); data.num_rows];
                // Una cella unita ripete il suo valore in ogni cella che copre (REQ-058).
                for cell in &data.table_cells {
                    let row_end = cell.end_row_offset_idx.unwrap_or(cell.start_row_offset_idx + 1).min(data.num_rows);
                    let col_end = cell.end_col_offset_idx.unwrap_or(cell.start_col_offset_idx + 1).min(data.num_cols);
                    for flags in header.iter_mut().take(row_end).skip(cell.start_row_offset_idx) {
                        *flags = (true, flags.1 && cell.column_header);
                    }
                    for row in rows.iter_mut().take(row_end).skip(cell.start_row_offset_idx) {
                        for slot in row.iter_mut().take(col_end).skip(cell.start_col_offset_idx) {
                            *slot = cell.text.trim().to_string();
                        }
                    }
                }
                let header_rows = header.iter().take_while(|&&(covered, all)| covered && all).count();
                if header_rows > 1 {
                    rows.splice(0..header_rows, [merge_header(&rows[..header_rows])]);
                }
                if !rows.is_empty() && data.num_cols > 0 {
                    self.out.push(Block { kind: Kind::Table(rows), text: String::new(), page });
                }
                self.children(&table.captions, depth);
                self.children(&table.footnotes, depth);
            }
            Some(Item::Picture(picture)) => {
                let page = docling::page(&picture.prov);
                if let Some(bytes) = picture.image.as_ref().and_then(|image| std::fs::read(self.base.join(&image.uri)).ok()) {
                    let count = self.per_page.entry(page).or_default();
                    *count += 1;
                    let link = self.images.add(&format!("pagina{page}_immagine{count}.png"), &bytes);
                    self.out.push(Block { kind: Kind::Image(link), text: String::new(), page });
                }
                // Il testo dentro l'immagine (etichette di un diagramma) non entra nel Markdown: solo le didascalie.
                self.children(&picture.captions, depth);
            }
            None => {}
        }
    }

    fn push(&mut self, kind: Kind, text: String, page: u32) {
        if !text.is_empty() {
            self.out.push(Block { kind, text, page });
        }
    }
}

/// Livello di un titolo: dalla numerazione di clausola (REQ-055) o di annesso (REQ-056) se c'è, altrimenti
/// quello di Docling.
fn heading_level(text: &str, label: &str, docling_level: Option<usize>) -> usize {
    if let Some(level) = clause_level(text).or_else(|| annex_level(text)) {
        return level;
    }
    if label == "title" { 1 } else { docling_level.unwrap_or(1).clamp(1, 6) }
}

/// `4.2.3 Titolo` (o `4.2.3. Titolo`) → 3: tanti livelli quanti componenti del numero, al massimo sei.
fn clause_level(text: &str) -> Option<usize> {
    let (number, rest) = text.trim().split_once(char::is_whitespace)?;
    if rest.trim().is_empty() {
        return None;
    }
    let parts: Vec<&str> = number.strip_suffix('.').unwrap_or(number).split('.').collect();
    parts.iter().all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit())).then(|| parts.len().min(6))
}

/// `Annex A …` (o `Allegato`, `Appendix`, `Appendice`) → 1; `A.1.2 Titolo` → 3, al massimo sei.
fn annex_level(text: &str) -> Option<usize> {
    let is_letter = |s: &str| s.len() == 1 && s.bytes().all(|b| b.is_ascii_uppercase());
    let mut words = text.split_whitespace();
    let first = words.next()?;
    if ["annex", "allegato", "appendix", "appendice"].contains(&first.to_lowercase().as_str()) {
        let letter = words.next()?.trim_end_matches(|c: char| !c.is_ascii_alphanumeric());
        return is_letter(letter).then_some(1);
    }
    words.next()?;
    let parts: Vec<&str> = first.strip_suffix('.').unwrap_or(first).split('.').collect();
    let numbered = parts[1..].iter().all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()));
    (parts.len() > 1 && is_letter(parts[0]) && numbered).then(|| parts.len().min(6))
}

/// Più righe d'intestazione in una sola: per colonna i testi sovrapposti uniti con ` / `, senza ripetere due
/// volte lo stesso testo (es. una cella unita su più righe) (REQ-059).
fn merge_header(rows: &[Vec<String>]) -> Vec<String> {
    (0..rows[0].len())
        .map(|col| {
            let mut parts: Vec<&str> = Vec::new();
            for text in rows.iter().map(|row| row[col].as_str()).filter(|t| !t.is_empty()) {
                if !parts.contains(&text) {
                    parts.push(text);
                }
            }
            parts.join(" / ")
        })
        .collect()
}

/// Markdown dei blocchi: un blocco per paragrafo, voci di elenco consecutive senza riga vuota tra loro.
pub fn to_markdown(blocks: &[Block]) -> String {
    let mut out = String::new();
    for (i, block) in blocks.iter().enumerate() {
        if i > 0 {
            let tight = matches!(block.kind, Kind::ListItem(_)) && matches!(blocks[i - 1].kind, Kind::ListItem(_));
            out.push_str(if tight { "\n" } else { "\n\n" });
        }
        match &block.kind {
            Kind::Heading(level) => out.push_str(&format!("{} {}", "#".repeat(*level), escape(&block.text))),
            Kind::Paragraph => out.push_str(&escape(&block.text)),
            Kind::ListItem(depth) => out.push_str(&format!("{}- {}", "  ".repeat(*depth), escape(&block.text))),
            Kind::Table(rows) => out.push_str(&table(rows)),
            Kind::Image(link) => out.push_str(link),
            Kind::Code => out.push_str(&format!("```\n{}\n```", block.text)),
        }
    }
    out.push('\n');
    out
}

fn table(rows: &[Vec<String>]) -> String {
    let line = |row: &Vec<String>| {
        let cells: Vec<String> = row.iter().map(|c| escape(c).replace('|', "\\|").replace(['\r', '\n'], " ")).collect();
        format!("| {} |", cells.join(" | "))
    };
    let mut lines = vec![line(&rows[0]), format!("|{}", "---|".repeat(rows[0].len()))];
    lines.extend(rows[1..].iter().map(line));
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn document(json: &str) -> Document {
        serde_json::from_str(json).unwrap()
    }

    fn markdown(json: &str) -> String {
        let mut images = Images::new(Path::new("doc.pdf"));
        to_markdown(&blocks(&document(json), Path::new("."), &mut images))
    }

    #[test]
    fn titoli_paragrafi_ed_elenchi_nell_ordine_del_corpo() {
        let md = markdown(
            r##"{
            "body": {"children": [{"$ref": "#/texts/0"}, {"$ref": "#/texts/1"}, {"$ref": "#/groups/0"}, {"$ref": "#/texts/4"}]},
            "groups": [{"self_ref": "#/groups/0", "label": "list", "children": [{"$ref": "#/texts/2"}, {"$ref": "#/texts/3"}]}],
            "texts": [
                {"self_ref": "#/texts/0", "label": "title", "text": "Documento", "prov": [{"page_no": 1}]},
                {"self_ref": "#/texts/1", "label": "section_header", "level": 2, "text": "Ambito", "prov": [{"page_no": 1}]},
                {"self_ref": "#/texts/2", "label": "list_item", "text": "primo", "prov": [{"page_no": 1}]},
                {"self_ref": "#/texts/3", "label": "list_item", "text": "secondo", "prov": [{"page_no": 1}]},
                {"self_ref": "#/texts/4", "label": "text", "text": "Testo *finale*", "prov": [{"page_no": 2}]}
            ]}"##,
        );
        assert_eq!(md, "# Documento\n\n## Ambito\n\n- primo\n- secondo\n\nTesto \\*finale\\*\n");
    }

    #[test]
    fn tabella_con_prima_riga_come_intestazione() {
        let md = markdown(
            r##"{
            "body": {"children": [{"$ref": "#/tables/0"}]},
            "tables": [{"self_ref": "#/tables/0", "data": {"num_rows": 2, "num_cols": 2, "table_cells": [
                {"text": "Voce", "start_row_offset_idx": 0, "start_col_offset_idx": 0},
                {"text": "Valore", "start_row_offset_idx": 0, "start_col_offset_idx": 1},
                {"text": "a|b", "start_row_offset_idx": 1, "start_col_offset_idx": 0},
                {"text": "5", "start_row_offset_idx": 1, "start_col_offset_idx": 1}
            ]}}]}"##,
        );
        assert_eq!(md, "| Voce | Valore |\n|---|---|\n| a\\|b | 5 |\n");
    }

    #[test]
    fn celle_unite_ripetute_in_ogni_cella_coperta() {
        let md = markdown(
            r##"{
            "body": {"children": [{"$ref": "#/tables/0"}]},
            "tables": [{"data": {"num_rows": 3, "num_cols": 3, "table_cells": [
                {"text": "Voce", "start_row_offset_idx": 0, "end_row_offset_idx": 1, "start_col_offset_idx": 0, "end_col_offset_idx": 1},
                {"text": "Livello", "start_row_offset_idx": 0, "end_row_offset_idx": 1, "start_col_offset_idx": 1, "end_col_offset_idx": 3},
                {"text": "Urto", "start_row_offset_idx": 1, "end_row_offset_idx": 3, "start_col_offset_idx": 0, "end_col_offset_idx": 1},
                {"text": "5", "start_row_offset_idx": 1, "end_row_offset_idx": 2, "start_col_offset_idx": 1, "end_col_offset_idx": 2},
                {"text": "20", "start_row_offset_idx": 1, "end_row_offset_idx": 2, "start_col_offset_idx": 2, "end_col_offset_idx": 3},
                {"text": "100", "start_row_offset_idx": 2, "end_row_offset_idx": 3, "start_col_offset_idx": 1, "end_col_offset_idx": 3}
            ]}}]}"##,
        );
        assert_eq!(md, "| Voce | Livello | Livello |\n|---|---|---|\n| Urto | 5 | 20 |\n| Urto | 100 | 100 |\n");
    }

    #[test]
    fn intestazioni_su_piu_righe_fuse_in_una() {
        let cell = |text: &str, r0: usize, r1: usize, c0: usize, c1: usize, header: bool| {
            format!(r#"{{"text": "{text}", "start_row_offset_idx": {r0}, "end_row_offset_idx": {r1}, "start_col_offset_idx": {c0}, "end_col_offset_idx": {c1}, "column_header": {header}}}"#)
        };
        let cells = [
            cell("Voce", 0, 2, 0, 1, true),
            cell("Livello", 0, 1, 1, 3, true),
            cell("Min", 1, 2, 1, 2, true),
            cell("Max", 1, 2, 2, 3, true),
            cell("Urto", 2, 3, 0, 1, false),
            cell("5", 2, 3, 1, 2, false),
            cell("20", 2, 3, 2, 3, false),
        ];
        let json = format!(
            r##"{{"body": {{"children": [{{"$ref": "#/tables/0"}}]}},
               "tables": [{{"data": {{"num_rows": 3, "num_cols": 3, "table_cells": [{}]}}}}]}}"##,
            cells.join(",")
        );
        assert_eq!(markdown(&json), "| Voce | Livello / Min | Livello / Max |\n|---|---|---|\n| Urto | 5 | 20 |\n");
    }

    #[test]
    fn una_sola_riga_di_intestazione_resta_com_e() {
        let md = markdown(
            r##"{"body": {"children": [{"$ref": "#/tables/0"}]},
            "tables": [{"data": {"num_rows": 2, "num_cols": 1, "table_cells": [
                {"text": "Voce", "start_row_offset_idx": 0, "start_col_offset_idx": 0, "column_header": true},
                {"text": "Urto", "start_row_offset_idx": 1, "start_col_offset_idx": 0}
            ]}}]}"##,
        );
        assert_eq!(md, "| Voce |\n|---|\n| Urto |\n");
    }

    #[test]
    fn didascalia_emessa_una_volta_e_testo_dei_gruppi_inline_unito() {
        let md = markdown(
            r##"{
            "body": {"children": [{"$ref": "#/texts/0"}, {"$ref": "#/tables/0"}, {"$ref": "#/groups/0"}]},
            "groups": [{"self_ref": "#/groups/0", "label": "inline", "children": [{"$ref": "#/texts/1"}, {"$ref": "#/texts/2"}]}],
            "texts": [
                {"self_ref": "#/texts/0", "label": "caption", "text": "Table 1: Livelli"},
                {"self_ref": "#/texts/1", "label": "text", "text": "Testo in"},
                {"self_ref": "#/texts/2", "label": "text", "text": "grassetto."}
            ],
            "tables": [{"self_ref": "#/tables/0", "captions": [{"$ref": "#/texts/0"}],
                "data": {"num_rows": 1, "num_cols": 1, "table_cells": [{"text": "x", "start_row_offset_idx": 0, "start_col_offset_idx": 0}]}}]}"##,
        );
        assert_eq!(md, "Table 1: Livelli\n\n| x |\n|---|\n\nTesto in grassetto.\n");
    }

    #[test]
    fn immagine_estratta_con_nome_per_pagina() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("immagini")).unwrap();
        std::fs::write(dir.path().join("immagini").join("a.png"), b"png").unwrap();
        let doc = document(
            r##"{
            "body": {"children": [{"$ref": "#/pictures/0"}]},
            "texts": [{"self_ref": "#/texts/0", "label": "caption", "text": "Figure 1: Schema"},
                      {"self_ref": "#/texts/1", "label": "text", "text": "etichetta interna"}],
            "pictures": [{"self_ref": "#/pictures/0", "image": {"uri": "immagini/a.png"}, "prov": [{"page_no": 3}],
                "captions": [{"$ref": "#/texts/0"}], "children": [{"$ref": "#/texts/0"}, {"$ref": "#/texts/1"}]}]}"##,
        );
        let mut images = Images::new(Path::new("doc.pdf"));
        let md = to_markdown(&blocks(&doc, dir.path(), &mut images));
        assert_eq!(md, "![pagina3_immagine1.png](doc_pdf_img/pagina3_immagine1.png)\n\nFigure 1: Schema\n");
    }

    /// Titolo di Docling con il livello indicato.
    fn header(text: &str, level: usize) -> String {
        format!(
            r##"{{"body": {{"children": [{{"$ref": "#/texts/0"}}]}},
               "texts": [{{"label": "section_header", "level": {level}, "text": "{text}"}}]}}"##
        )
    }

    #[test]
    fn livello_dei_titoli_dalla_numerazione_di_clausola() {
        assert_eq!(markdown(&header("4 Requirements", 1)), "# 4 Requirements\n");
        assert_eq!(markdown(&header("4.2 Materials", 1)), "## 4.2 Materials\n");
        assert_eq!(markdown(&header("4.2.3 Metals", 1)), "### 4.2.3 Metals\n");
        assert_eq!(markdown(&header("4.2.3. Metals", 2)), "### 4.2.3. Metals\n");
        assert_eq!(markdown(&header("1.2.3.4.5.6.7 Profondo", 1)), "###### 1.2.3.4.5.6.7 Profondo\n");
        // Non è una numerazione di clausola: livello di Docling.
        assert_eq!(markdown(&header("4.2", 2)), "## 4.2\n");
        assert_eq!(markdown(&header("4.a Varianti", 2)), "## 4.a Varianti\n");
    }

    #[test]
    fn livello_dei_titoli_degli_annessi() {
        assert_eq!(markdown(&header("Annex A (normative) Test methods", 3)), "# Annex A (normative) Test methods\n");
        assert_eq!(markdown(&header("Allegato B", 2)), "# Allegato B\n");
        assert_eq!(markdown(&header("Annex C: Glossary", 2)), "# Annex C: Glossary\n");
        assert_eq!(markdown(&header("A.1 Scope", 1)), "## A.1 Scope\n");
        assert_eq!(markdown(&header("A.1.2. Detail", 1)), "### A.1.2. Detail\n");
        // Non sono annessi: livello di Docling.
        assert_eq!(markdown(&header("Annex", 2)), "## Annex\n");
        assert_eq!(markdown(&header("AB.1 Sigla", 2)), "## AB.1 Sigla\n");
        assert_eq!(markdown(&header("A. Introduzione", 2)), "## A. Introduzione\n");
    }

    #[test]
    fn titoli_senza_numerazione_con_il_livello_di_docling() {
        assert_eq!(markdown(&header("Introduction", 2)), "## Introduction\n");
        assert_eq!(markdown(&header("Foreword", 1)), "# Foreword\n");
        assert_eq!(markdown(&header("Profondo", 9)), "###### Profondo\n");
        let title = r##"{"body": {"children": [{"$ref": "#/texts/0"}]},
            "texts": [{"label": "title", "level": 3, "text": "Space product assurance"}]}"##;
        assert_eq!(markdown(title), "# Space product assurance\n");
        let no_level = r##"{"body": {"children": [{"$ref": "#/texts/0"}]},
            "texts": [{"label": "section_header", "text": "Scope"}]}"##;
        assert_eq!(markdown(no_level), "# Scope\n");
    }

    #[test]
    fn nessun_testo_nessun_blocco() {
        let doc = document(r##"{"body": {"children": []}}"##);
        assert!(blocks(&doc, Path::new("."), &mut Images::new(Path::new("doc.pdf"))).is_empty());
    }
}
