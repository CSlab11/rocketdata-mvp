use std::cell::OnceCell;
use std::path::Path;

use pdfium_render::prelude::*;

use crate::convert::base_name;
use crate::docx::escape;
use crate::output::write_file;
use crate::run::Outcome;

thread_local! {
    static PDFIUM: OnceCell<Result<Pdfium, String>> = const { OnceCell::new() };
}

/// Carica pdfium.dll dalla cartella dell'eseguibile, altrimenti dalle librerie di sistema.
fn load_pdfium() -> Result<Pdfium, String> {
    let exe_dir = std::env::current_exe().ok().and_then(|p| p.parent().map(Path::to_path_buf));
    let bindings = exe_dir
        .map(|dir| Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path(&dir)))
        .filter(Result::is_ok)
        .unwrap_or_else(Pdfium::bind_to_system_library)
        .map_err(|e| format!("pdfium.dll non disponibile accanto a doc-ripper.exe ({e})"))?;
    Ok(Pdfium::new(bindings))
}

/// Converte un `.pdf` in `<nome>_pdf.md` ricostruendo titoli, paragrafi, elenchi e tabelle (REQ-010).
pub fn convert(src: &Path, dst: &Path, rel: &Path) -> Result<Outcome, String> {
    let pages = PDFIUM.with(|cell| match cell.get_or_init(load_pdfium) {
        Ok(pdfium) => extract(pdfium, &src.join(rel)),
        Err(e) => Err(e.clone()),
    })?;
    let markdown = layout::to_markdown(&pages);
    let out_rel = rel.with_file_name(format!("{}.md", base_name(rel)));
    write_file(&dst.join(&out_rel), markdown.as_bytes())?;
    Ok(Outcome { outputs: vec![out_rel], skipped: Vec::new() })
}

/// Frammenti di testo di ogni pagina, con posizione e dimensione del carattere.
fn extract(pdfium: &Pdfium, path: &Path) -> Result<Vec<Vec<layout::Run>>, String> {
    let document = pdfium.load_pdf_from_file(path, None).map_err(|e| format!("pdf non valido: {e}"))?;
    let mut pages = Vec::new();
    for page in document.pages().iter() {
        let mut runs = Vec::new();
        for object in page.objects().iter() {
            let Some(text_object) = object.as_text_object() else { continue };
            let text = text_object.text();
            let Ok(bounds) = object.bounds() else { continue };
            if text.trim().is_empty() {
                continue;
            }
            runs.push(layout::Run {
                left: bounds.left().value,
                right: bounds.right().value,
                top: bounds.top().value,
                bottom: bounds.bottom().value,
                size: text_object.scaled_font_size().value,
                text,
            });
        }
        pages.push(runs);
    }
    Ok(pages)
}

/// Ricostruzione della struttura da testo posizionato. Coordinate PDF: y cresce verso l'alto.
mod layout {
    use super::escape;

    #[derive(Debug, Clone)]
    pub struct Run {
        pub left: f32,
        pub right: f32,
        pub top: f32,
        pub bottom: f32,
        pub size: f32,
        pub text: String,
    }

    /// Riga di testo; `cells` separate da spazi orizzontali ampi (possibili colonne di tabella).
    #[derive(Debug, Clone)]
    struct Line {
        top: f32,
        bottom: f32,
        size: f32,
        cells: Vec<Cell>,
    }

    #[derive(Debug, Clone)]
    struct Cell {
        left: f32,
        text: String,
    }

    impl Line {
        fn left(&self) -> f32 {
            self.cells[0].left
        }
        fn text(&self) -> String {
            self.cells.iter().map(|c| c.text.as_str()).collect::<Vec<_>>().join(" ")
        }
    }

    /// Distanza orizzontale, in multipli della dimensione del carattere, oltre la quale
    /// due frammenti appartengono a colonne diverse.
    const COLUMN_GAP: f32 = 2.0;

    fn lines(runs: &[Run]) -> Vec<Line> {
        let mut runs: Vec<&Run> = runs.iter().collect();
        runs.sort_by(|a, b| b.top.total_cmp(&a.top));
        // Raggruppa i frammenti che si sovrappongono verticalmente.
        let mut groups: Vec<Vec<&Run>> = Vec::new();
        for run in runs {
            let middle = (run.top + run.bottom) / 2.0;
            match groups.last_mut() {
                Some(group) if group.iter().any(|r| middle <= r.top && middle >= r.bottom) => group.push(run),
                _ => groups.push(vec![run]),
            }
        }
        groups
            .into_iter()
            .map(|mut group| {
                group.sort_by(|a, b| a.left.total_cmp(&b.left));
                let size = group.iter().map(|r| r.size).fold(0.0, f32::max);
                let mut cells: Vec<Cell> = Vec::new();
                let mut prev_right = f32::NEG_INFINITY;
                for run in &group {
                    let gap = run.left - prev_right;
                    let text = run.text.trim();
                    match cells.last_mut() {
                        Some(cell) if gap < COLUMN_GAP * size => {
                            if gap > 0.1 * size || run.text.starts_with(' ') || cell.text.ends_with(' ') {
                                cell.text.push(' ');
                            }
                            cell.text.push_str(text);
                        }
                        _ => cells.push(Cell { left: run.left, text: text.to_string() }),
                    }
                    prev_right = run.right;
                }
                for cell in &mut cells {
                    cell.text = cell.text.split_whitespace().collect::<Vec<_>>().join(" ");
                }
                merge_list_marker(&mut cells);
                Line {
                    top: group.iter().map(|r| r.top).fold(f32::MIN, f32::max),
                    bottom: group.iter().map(|r| r.bottom).fold(f32::MAX, f32::min),
                    size,
                    cells,
                }
            })
            .collect()
    }

    /// Un punto elenco staccato dal testo non è una colonna: lo riunisce alla cella successiva.
    fn merge_list_marker(cells: &mut Vec<Cell>) {
        if cells.len() >= 2 && list_marker(&format!("{} x", cells[0].text)).is_some() && cells[0].text.len() <= 4 {
            let next = cells.remove(1);
            cells[0].text = format!("{} {}", cells[0].text, next.text);
        }
    }

    /// Riconosce l'inizio di una voce di elenco: (puntato?, testo senza marcatore).
    fn list_marker(text: &str) -> Option<(bool, &str)> {
        let (marker, rest) = text.split_once(' ')?;
        let rest = rest.trim_start();
        if rest.is_empty() {
            return None;
        }
        if ["•", "◦", "▪", "▫", "‣", "●", "○", "■", "□", "–", "-", "*", "·", "\u{f0b7}"].contains(&marker) {
            return Some((true, rest));
        }
        let body = marker.strip_suffix('.').or_else(|| marker.strip_suffix(')'))?;
        let numeric = !body.is_empty() && body.len() <= 3 && body.chars().all(|c| c.is_ascii_digit());
        let letter = body.len() == 1 && body.chars().all(|c| c.is_ascii_lowercase());
        (numeric || letter).then_some((false, rest))
    }

    enum Block {
        Heading(usize, String),
        Paragraph(String),
        Item { bullet: bool, indent: f32, text: String },
        Table(Vec<Vec<String>>),
    }

    pub fn to_markdown(pages: &[Vec<Run>]) -> String {
        let pages: Vec<Vec<Line>> = pages.iter().map(|runs| lines(runs)).collect();
        let body = body_size(&pages);
        let heading_sizes = heading_sizes(&pages, body);

        let mut blocks: Vec<Block> = Vec::new();
        for page in &pages {
            let mut i = 0;
            // Un paragrafo può proseguire dalla pagina precedente.
            let mut prev: Option<&Line> = None;
            while i < page.len() {
                let line = &page[i];
                let text = line.text();

                // Tabella: almeno due righe consecutive con due o più colonne.
                let rows = page[i..].iter().take_while(|l| l.cells.len() >= 2).count();
                if rows >= 2 {
                    blocks.push(Block::Table(table(&page[i..i + rows])));
                    i += rows;
                    prev = None;
                    continue;
                }

                if line.cells.len() == 1 && is_heading(line, body) {
                    let level = heading_sizes.iter().position(|&s| line.size >= s - 0.5).unwrap_or(2) + 1;
                    blocks.push(Block::Heading(level, escape(&text)));
                    prev = None;
                } else if let Some((bullet, rest)) = list_marker(&text) {
                    blocks.push(Block::Item { bullet, indent: line.left(), text: escape(rest) });
                    prev = Some(line);
                } else {
                    let continues = prev.is_some_and(|p| (p.bottom - line.top) < 0.9 * line.size && (p.size - line.size).abs() < 0.5);
                    match blocks.last_mut() {
                        Some(Block::Paragraph(p)) | Some(Block::Item { text: p, .. }) if continues => join_line(p, &escape(&text)),
                        _ => blocks.push(Block::Paragraph(escape(&text))),
                    }
                    prev = Some(line);
                }
                i += 1;
            }
        }
        render(&blocks)
    }

    /// Unisce una riga al paragrafo, ricomponendo le parole spezzate con il trattino.
    fn join_line(paragraph: &mut String, line: &str) {
        let hyphenated = paragraph.ends_with('-')
            && paragraph.chars().rev().nth(1).is_some_and(char::is_alphabetic)
            && line.chars().next().is_some_and(char::is_lowercase);
        if hyphenated {
            paragraph.pop();
        } else {
            paragraph.push(' ');
        }
        paragraph.push_str(line);
    }

    /// Dimensione del testo normale: la più frequente, pesata per numero di caratteri.
    fn body_size(pages: &[Vec<Line>]) -> f32 {
        let mut counts: Vec<(f32, usize)> = Vec::new();
        for line in pages.iter().flatten() {
            let size = (line.size * 2.0).round() / 2.0;
            let chars = line.text().chars().count();
            match counts.iter_mut().find(|(s, _)| *s == size) {
                Some((_, n)) => *n += chars,
                None => counts.push((size, chars)),
            }
        }
        counts.iter().max_by_key(|(_, n)| *n).map_or(12.0, |(s, _)| *s)
    }

    fn is_heading(line: &Line, body: f32) -> bool {
        line.size >= body * 1.15 && line.text().chars().count() <= 200
    }

    /// Dimensioni dei titoli dalla più grande alla più piccola: indice + 1 = livello (max 3).
    fn heading_sizes(pages: &[Vec<Line>], body: f32) -> Vec<f32> {
        let mut sizes: Vec<f32> = Vec::new();
        for line in pages.iter().flatten().filter(|l| l.cells.len() == 1 && is_heading(l, body)) {
            let size = line.size.round();
            if !sizes.contains(&size) {
                sizes.push(size);
            }
        }
        sizes.sort_by(|a, b| b.total_cmp(a));
        sizes.truncate(3);
        sizes
    }

    /// Distribuisce le celle nelle colonne ricavate dalle posizioni orizzontali.
    fn table(lines: &[Line]) -> Vec<Vec<String>> {
        let widest = lines.iter().max_by_key(|l| l.cells.len()).unwrap();
        let columns: Vec<f32> = widest.cells.iter().map(|c| c.left).collect();
        lines
            .iter()
            .map(|line| {
                let mut row = vec![String::new(); columns.len()];
                for cell in &line.cells {
                    let col = (0..columns.len())
                        .min_by(|&a, &b| (columns[a] - cell.left).abs().total_cmp(&(columns[b] - cell.left).abs()))
                        .unwrap();
                    if !row[col].is_empty() {
                        row[col].push(' ');
                    }
                    row[col].push_str(&escape(&cell.text).replace('|', "\\|"));
                }
                row
            })
            .collect()
    }

    fn render(blocks: &[Block]) -> String {
        let mut out = String::new();
        let mut list_indents: Vec<f32> = Vec::new();
        let mut prev_item = false;
        for block in blocks {
            let is_item = matches!(block, Block::Item { .. });
            if !out.is_empty() {
                out.push_str(if is_item && prev_item { "\n" } else { "\n\n" });
            }
            if !is_item {
                list_indents.clear();
            }
            match block {
                Block::Heading(level, text) => out.push_str(&format!("{} {text}", "#".repeat(*level))),
                Block::Paragraph(text) => out.push_str(text),
                Block::Item { bullet, indent, text } => {
                    // Livello di annidamento dal rientro del marcatore.
                    while list_indents.last().is_some_and(|&l| *indent < l - 2.0) {
                        list_indents.pop();
                    }
                    if list_indents.last().is_none_or(|&l| *indent > l + 2.0) {
                        list_indents.push(*indent);
                    }
                    let level = list_indents.len() - 1;
                    let marker = if *bullet { "-" } else { "1." };
                    out.push_str(&format!("{}{marker} {text}", "   ".repeat(level)));
                }
                Block::Table(rows) => {
                    let width = rows[0].len();
                    let line = |row: &Vec<String>| format!("| {} |", row.join(" | "));
                    out.push_str(&line(&rows[0]));
                    out.push_str(&format!("\n|{}|", vec!["---"; width].join("|")));
                    for row in &rows[1..] {
                        out.push('\n');
                        out.push_str(&line(row));
                    }
                }
            }
            prev_item = is_item;
        }
        out.push('\n');
        out
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        /// Frammento su una riga con base `y` e carattere `size`.
        fn run(x: f32, y: f32, size: f32, text: &str) -> Run {
            let width = text.chars().count() as f32 * size * 0.5;
            Run { left: x, right: x + width, top: y + size * 0.7, bottom: y, size, text: text.into() }
        }

        #[test]
        fn riconosce_i_marcatori_di_elenco() {
            assert_eq!(list_marker("• Primo"), Some((true, "Primo")));
            assert_eq!(list_marker("- Primo"), Some((true, "Primo")));
            assert_eq!(list_marker("12. Dodici"), Some((false, "Dodici")));
            assert_eq!(list_marker("a) Lettera"), Some((false, "Lettera")));
            assert_eq!(list_marker("2026. Anno"), None);
            assert_eq!(list_marker("Testo normale"), None);
            assert_eq!(list_marker("•"), None);
        }

        #[test]
        fn ricompone_le_parole_spezzate() {
            let mut p = "Una parola spez-".to_string();
            join_line(&mut p, "zata qui");
            assert_eq!(p, "Una parola spezzata qui");
            let mut p = "Ruolo: senior-".to_string();
            join_line(&mut p, "Manager");
            assert_eq!(p, "Ruolo: senior- Manager");
        }

        #[test]
        fn struttura_completa() {
            let page = vec![
                run(72.0, 780.0, 24.0, "Relazione annuale"),
                run(72.0, 740.0, 16.0, "Introduzione"),
                run(72.0, 715.0, 11.0, "Questo paragrafo continua"),
                run(72.0, 702.0, 11.0, "sulla riga successiva."),
                run(72.0, 680.0, 11.0, "Secondo paragrafo con *stelle*."),
                run(72.0, 655.0, 11.0, "•"),
                run(90.0, 655.0, 11.0, "Primo punto"),
                run(108.0, 641.0, 11.0, "◦ Sotto punto"),
                run(72.0, 627.0, 11.0, "• Secondo punto"),
                run(72.0, 600.0, 11.0, "Voce"),
                run(250.0, 600.0, 11.0, "Q1"),
                run(350.0, 600.0, 11.0, "Q2"),
                run(72.0, 586.0, 11.0, "Affitto"),
                run(250.0, 586.0, 11.0, "100"),
                run(350.0, 586.0, 11.0, "110"),
                run(72.0, 572.0, 11.0, "Utenze"),
                run(350.0, 572.0, 11.0, "45"),
                run(72.0, 545.0, 14.0, "Conclusioni"),
                run(72.0, 525.0, 11.0, "Fine."),
            ];
            let md = to_markdown(&[page]);
            let expected = concat!(
                "# Relazione annuale\n\n",
                "## Introduzione\n\n",
                "Questo paragrafo continua sulla riga successiva.\n\n",
                "Secondo paragrafo con \\*stelle\\*.\n\n",
                "- Primo punto\n   - Sotto punto\n- Secondo punto\n\n",
                "| Voce | Q1 | Q2 |\n|---|---|---|\n| Affitto | 100 | 110 |\n| Utenze |  | 45 |\n\n",
                "### Conclusioni\n\n",
                "Fine.\n",
            );
            assert_eq!(md, expected);
        }
    }
}
