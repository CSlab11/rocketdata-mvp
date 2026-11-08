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
                    "title" => Kind::Heading(1),
                    "section_header" => Kind::Heading(text.level.unwrap_or(1).clamp(1, 6)),
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
                for cell in &data.table_cells {
                    if let Some(slot) = rows.get_mut(cell.start_row_offset_idx).and_then(|r| r.get_mut(cell.start_col_offset_idx)) {
                        *slot = cell.text.trim().to_string();
                    }
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

    #[test]
    fn nessun_testo_nessun_blocco() {
        let doc = document(r##"{"body": {"children": []}}"##);
        assert!(blocks(&doc, Path::new("."), &mut Images::new(Path::new("doc.pdf"))).is_empty());
    }
}
