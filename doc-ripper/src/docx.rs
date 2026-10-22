use std::cell::RefCell;
use std::fs;
use std::path::Path;

use docx_rs::{
    read_docx_with_options, DocumentChild, Docx, DrawingData, HyperlinkData, InsertChild, Paragraph,
    ParagraphChild, ReadDocxOptions, Run, RunChild, StructuredDataTagChild, Table, TableCellContent,
    TableChild, TableRowChild,
};

use crate::convert::base_name;
use crate::images::Images;
use crate::output::write_file;
use crate::run::Outcome;

/// Converte un `.docx` in `<nome>_docx.md` mantenendo titoli, paragrafi, elenchi e tabelle (REQ-011)
/// ed estraendo le immagini (REQ-020).
pub fn convert(src: &Path, dst: &Path, rel: &Path) -> Result<Outcome, String> {
    let bytes = fs::read(src.join(rel)).map_err(|e| e.to_string())?;
    let options = ReadDocxOptions::default().with_image_previews(false);
    let docx = read_docx_with_options(&bytes, options).map_err(|e| format!("docx non valido: {e}"))?;

    let md = Markdown { docx: &docx, images: RefCell::new(Images::new(rel)) };
    let markdown = md.document();
    let out_rel = rel.with_file_name(format!("{}.md", base_name(rel)));
    let out_path = dst.join(&out_rel);
    md.images.borrow().write(&out_path)?;
    write_file(&out_path, markdown.as_bytes())?;
    Ok(Outcome { outputs: vec![out_rel], skipped: Vec::new() })
}

/// Blocchi Markdown: le voci consecutive dello stesso elenco (stesso id di numerazione)
/// si separano con un solo a capo.
enum Block {
    Text(String),
    Item(String, usize),
}

struct Markdown<'a> {
    docx: &'a Docx,
    images: RefCell<Images>,
}

impl Markdown<'_> {
    fn document(&self) -> String {
        let mut blocks = Vec::new();
        for child in &self.docx.document.children {
            match child {
                DocumentChild::Paragraph(p) => blocks.extend(self.paragraph(p)),
                DocumentChild::Table(t) => blocks.push(Block::Text(self.table(t))),
                DocumentChild::StructuredDataTag(sdt) => self.sdt(&sdt.children, &mut blocks),
                _ => {}
            }
        }
        let mut out = String::new();
        let mut prev_list = None;
        for block in blocks {
            let (text, list) = match block {
                Block::Text(t) => (t, None),
                Block::Item(t, id) => (t, Some(id)),
            };
            if !out.is_empty() {
                out.push_str(if list.is_some() && list == prev_list { "\n" } else { "\n\n" });
            }
            out.push_str(&text);
            prev_list = list;
        }
        out.push('\n');
        out
    }

    fn sdt(&self, children: &[StructuredDataTagChild], blocks: &mut Vec<Block>) {
        for child in children {
            match child {
                StructuredDataTagChild::Paragraph(p) => blocks.extend(self.paragraph(p)),
                StructuredDataTagChild::Table(t) => blocks.push(Block::Text(self.table(t))),
                StructuredDataTagChild::StructuredDataTag(inner) => self.sdt(&inner.children, blocks),
                _ => {}
            }
        }
    }

    fn paragraph(&self, p: &Paragraph) -> Option<Block> {
        let text = self.inline(&p.children);
        let text = text.trim();
        if text.is_empty() {
            return None;
        }
        if let Some(level) = self.heading_level(p) {
            let one_line = text.replace("  \n", " ");
            return Some(Block::Text(format!("{} {one_line}", "#".repeat(level))));
        }
        if let Some((id, bullet, level)) = self.list_item(p) {
            let marker = if bullet { "-" } else { "1." };
            let text = text.replace("  \n", "  \n   ");
            return Some(Block::Item(format!("{}{marker} {text}", "   ".repeat(level)), id));
        }
        Some(Block::Text(text.to_string()))
    }

    /// Livello di titolo dallo stile (`heading N`, `Title`) o dal livello di struttura.
    fn heading_level(&self, p: &Paragraph) -> Option<usize> {
        let style = p.property.style.as_ref().and_then(|s| {
            self.docx.styles.styles.iter().find(|st| st.style_id == s.val)
        });
        let level = match style {
            Some(st) if st.name.is_heading() => st.name.get_heading_number(),
            Some(st) if st.name.starts_with("Title") => Some(1),
            _ => None,
        };
        let outline = p
            .property
            .outline_lvl
            .as_ref()
            .or_else(|| style.and_then(|st| st.paragraph_property.outline_lvl.as_ref()))
            .map(|o| o.v + 1);
        // outlineLvl 9 significa "corpo del testo".
        level.or(outline.filter(|&l| l <= 9)).map(|l| l.clamp(1, 6))
    }

    /// Voce di elenco: (id della numerazione, puntato?, livello di rientro).
    fn list_item(&self, p: &Paragraph) -> Option<(usize, bool, usize)> {
        let np = p.property.numbering_property.as_ref()?;
        let id = np.id.as_ref()?.id;
        if id == 0 {
            return None;
        }
        let level = np.level.as_ref().map_or(0, |l| l.val);
        let numberings = &self.docx.numberings;
        let bullet = numberings
            .numberings
            .iter()
            .find(|n| n.id == id)
            .and_then(|n| numberings.abstract_nums.iter().find(|a| a.id == n.abstract_num_id))
            .and_then(|a| a.levels.iter().find(|l| l.level == level))
            .is_none_or(|l| l.format.val == "bullet");
        Some((id, bullet, level))
    }

    fn inline(&self, children: &[ParagraphChild]) -> String {
        let mut out = String::new();
        for child in children {
            match child {
                ParagraphChild::Run(run) => out.push_str(&self.run(run)),
                ParagraphChild::Insert(ins) => {
                    for c in &ins.children {
                        if let InsertChild::Run(run) = c {
                            out.push_str(&self.run(run));
                        }
                    }
                }
                ParagraphChild::Hyperlink(link) => {
                    let text = self.inline(&link.children);
                    match &link.link {
                        HyperlinkData::External { rid, .. } => {
                            match self.docx.hyperlinks.iter().find(|(id, _, _)| id == rid) {
                                Some((_, url, _)) if !text.trim().is_empty() => {
                                    out.push_str(&format!("[{text}]({})", url.replace(' ', "%20")))
                                }
                                _ => out.push_str(&text),
                            }
                        }
                        _ => out.push_str(&text),
                    }
                }
                _ => {}
            }
        }
        out
    }

    fn run(&self, run: &Run) -> String {
        let mut out = String::new();
        for child in &run.children {
            match child {
                RunChild::Text(t) => out.push_str(&escape(&t.text)),
                RunChild::Tab(_) | RunChild::PTab(_) => out.push(' '),
                RunChild::Break(_) | RunChild::CarriageReturn(_) => out.push_str("  \n"),
                RunChild::Drawing(drawing) => {
                    if let Some(DrawingData::Pic(pic)) = &drawing.data {
                        out.push_str(&self.image(&pic.id));
                    }
                }
                _ => {}
            }
        }
        out
    }

    /// Riferimento Markdown all'immagine con relazione `rid`, registrata per la scrittura.
    fn image(&self, rid: &str) -> String {
        let Some((_, path, image, _)) = self.docx.images.iter().find(|(id, ..)| id == rid) else {
            return String::new();
        };
        let name = Path::new(path).file_name().map_or(rid.to_string(), |n| n.to_string_lossy().into_owned());
        self.images.borrow_mut().add(&name, &image.0)
    }

    /// Tabella GFM: la prima riga fa da intestazione.
    fn table(&self, table: &Table) -> String {
        let rows: Vec<Vec<String>> = table
            .rows
            .iter()
            .map(|TableChild::TableRow(row)| {
                row.cells.iter().map(|TableRowChild::TableCell(cell)| self.cell(&cell.children)).collect()
            })
            .collect();
        let width = rows.iter().map(Vec::len).max().unwrap_or(0).max(1);
        let line = |cells: &[String]| {
            let mut padded: Vec<&str> = cells.iter().map(String::as_str).collect();
            padded.resize(width, "");
            format!("| {} |", padded.join(" | "))
        };
        let mut out = Vec::new();
        let empty = Vec::new();
        out.push(line(rows.first().unwrap_or(&empty)));
        out.push(format!("|{}|", vec!["---"; width].join("|")));
        for row in rows.iter().skip(1) {
            out.push(line(row));
        }
        out.join("\n")
    }

    fn cell(&self, content: &[TableCellContent]) -> String {
        let mut parts = Vec::new();
        for c in content {
            match c {
                TableCellContent::Paragraph(p) => {
                    let text = self.inline(&p.children);
                    if !text.trim().is_empty() {
                        parts.push(text.trim().to_string());
                    }
                }
                TableCellContent::Table(t) => parts.push(
                    t.rows
                        .iter()
                        .map(|TableChild::TableRow(r)| {
                            r.cells.iter().map(|TableRowChild::TableCell(c)| self.cell(&c.children)).collect::<Vec<_>>().join(" ")
                        })
                        .collect::<Vec<_>>()
                        .join("<br>"),
                ),
                _ => {}
            }
        }
        parts.join("<br>").replace("  \n", "<br>").replace('|', "\\|")
    }
}

/// Protegge i caratteri che Markdown interpreterebbe come formattazione.
pub fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if "\\`*_[]<>".contains(c) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protegge_la_formattazione_markdown() {
        assert_eq!(escape("a*b_c [x] <y>"), "a\\*b\\_c \\[x\\] \\<y\\>");
        assert_eq!(escape("Città, 10%"), "Città, 10%");
    }
}
