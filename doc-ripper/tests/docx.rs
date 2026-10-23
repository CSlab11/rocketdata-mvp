use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use docx_rs::{
    AbstractNumbering, BreakType, Docx, Hyperlink, HyperlinkType, IndentLevel, Level, LevelJc, LevelText,
    NumberFormat, Numbering, NumberingId, Paragraph, Run, Start, Style, StyleType, Table, TableCell, TableRow,
};

fn run(src: &Path, dst: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_doc-ripper"))
        .args(["--src", src.to_str().unwrap(), "--dst", dst.to_str().unwrap()])
        .output()
        .unwrap()
}

fn p(text: &str) -> Paragraph {
    Paragraph::new().add_run(Run::new().add_text(text))
}

fn cell(text: &str) -> TableCell {
    TableCell::new().add_paragraph(p(text))
}

fn level(n: usize, format: &str) -> Level {
    Level::new(n, Start::new(1), NumberFormat::new(format), LevelText::new("%1."), LevelJc::new("left"))
}

fn sample(path: &Path) {
    let docx = Docx::new()
        // Word in italiano: id "Titolo1", nome predefinito "heading 1".
        .add_style(Style::new("Titolo1", StyleType::Paragraph).name("heading 1"))
        .add_style(Style::new("Heading2", StyleType::Paragraph).name("heading 2"))
        // docx-rs aggiunge da sé una numerazione con id 1: si usano id diversi.
        .add_abstract_numbering(AbstractNumbering::new(10).add_level(level(0, "bullet")).add_level(level(1, "bullet")))
        .add_numbering(Numbering::new(10, 10))
        .add_abstract_numbering(AbstractNumbering::new(20).add_level(level(0, "decimal")))
        .add_numbering(Numbering::new(20, 20))
        .add_paragraph(p("Relazione trimestrale").style("Titolo1"))
        .add_paragraph(p("Testo con *asterischi* e una riga").add_run(Run::new().add_break(BreakType::TextWrapping).add_text("a capo.")))
        .add_paragraph(Paragraph::new())
        .add_paragraph(p("Punti chiave").style("Heading2"))
        .add_paragraph(p("Primo").numbering(NumberingId::new(10), IndentLevel::new(0)))
        .add_paragraph(p("Dettaglio").numbering(NumberingId::new(10), IndentLevel::new(1)))
        .add_paragraph(p("Secondo").numbering(NumberingId::new(10), IndentLevel::new(0)))
        .add_paragraph(p("Passo uno").numbering(NumberingId::new(20), IndentLevel::new(0)))
        .add_paragraph(
            Paragraph::new()
                .add_run(Run::new().add_text("Vedi "))
                .add_hyperlink(
                    Hyperlink::new("https://example.com/a b", HyperlinkType::External)
                        .add_run(Run::new().add_text("il sito")),
                ),
        )
        .add_table(Table::new(vec![
            TableRow::new(vec![cell("Voce"), cell("Importo")]),
            TableRow::new(vec![cell("Affitto | sede"), cell("1.200")]),
        ]));
    docx.build().pack(fs::File::create(path).unwrap()).unwrap();
}


/// Separa la riga "Origine" dal resto del Markdown e verifica che il link porti al file originale.
fn split_origin(md: &str, original: &Path) -> String {
    let (first, rest) = md.split_once("

").unwrap();
    assert!(first.starts_with("Origine: ["), "{first}");
    let url = first.split_once("](").unwrap().1.strip_suffix(')').unwrap();
    assert!(!url.starts_with("file:"), "{url}");
    let path = url::Url::parse(&format!("file:///{url}")).unwrap().to_file_path().unwrap();
    assert_eq!(fs::canonicalize(path).unwrap(), fs::canonicalize(original).unwrap());
    rest.to_string()
}

#[test]
fn converte_docx_in_markdown_strutturato() {
    let src = tempfile::tempdir().unwrap();
    let dst = tempfile::tempdir().unwrap();
    fs::create_dir(src.path().join("doc")).unwrap();
    sample(&src.path().join("doc/report.docx"));

    let out = run(src.path(), dst.path());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("Convertiti: 1"), "{stdout}");

    let md = fs::read_to_string(dst.path().join("doc/report_docx.md")).unwrap();
    let md = split_origin(&md, &src.path().join("doc/report.docx"));
    let expected = concat!(
        "# Relazione trimestrale\n\n",
        "Testo con \\*asterischi\\* e una riga  \na capo.\n\n",
        "## Punti chiave\n\n",
        "- Primo\n   - Dettaglio\n- Secondo\n\n",
        "1. Passo uno\n\n",
        "Vedi [il sito](https://example.com/a%20b)\n\n",
        "| Voce | Importo |\n|---|---|\n| Affitto \\| sede | 1.200 |\n",
    );
    assert_eq!(md, expected);
}

/// PNG 1x1 valido.
const PNG: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52, 0x00, 0x00,
    0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00,
    0x0D, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0xF8, 0xCF, 0xC0, 0xF0, 0x1F, 0x00, 0x05, 0x00, 0x01, 0xFF,
    0x89, 0x99, 0x3D, 0x1D, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
];

#[test]
fn estrae_le_immagini_accanto_al_markdown() {
    let src = tempfile::tempdir().unwrap();
    let dst = tempfile::tempdir().unwrap();
    let pic = docx_rs::Pic::new_with_dimensions(PNG.to_vec(), 1, 1);
    Docx::new()
        .add_paragraph(p("Prima"))
        .add_paragraph(Paragraph::new().add_run(Run::new().add_image(pic)))
        .add_paragraph(p("Dopo"))
        .build()
        .pack(fs::File::create(src.path().join("Il report.docx")).unwrap())
        .unwrap();

    let out = run(src.path(), dst.path());
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stdout));

    let img_dir = dst.path().join("Il report_docx_img");
    let images: Vec<_> = fs::read_dir(&img_dir).unwrap().map(|e| e.unwrap().file_name()).collect();
    assert_eq!(images.len(), 1);
    let name = images[0].to_string_lossy().to_string();
    assert_eq!(fs::read(img_dir.join(&name)).unwrap(), PNG);

    let md = fs::read_to_string(dst.path().join("Il report_docx.md")).unwrap();
    let md = split_origin(&md, &src.path().join("Il report.docx"));
    assert_eq!(md, format!("Prima\n\n![{name}](Il%20report_docx_img/{name})\n\nDopo\n"));
}

#[test]
fn salta_i_docx_scansionati() {
    let src = tempfile::tempdir().unwrap();
    let dst = tempfile::tempdir().unwrap();
    let pic = docx_rs::Pic::new_with_dimensions(PNG.to_vec(), 1, 1);
    Docx::new()
        .add_paragraph(Paragraph::new().add_run(Run::new().add_image(pic)))
        .build()
        .pack(fs::File::create(src.path().join("scansione.docx")).unwrap())
        .unwrap();

    let out = run(src.path(), dst.path());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("scansione.docx - scansionato"), "{stdout}");
    assert!(!dst.path().join("scansione_docx.md").exists());
    assert!(!dst.path().join("scansione_docx_img").exists());
}

#[test]
fn docx_corrotto_fallisce() {
    let src = tempfile::tempdir().unwrap();
    let dst = tempfile::tempdir().unwrap();
    fs::write(src.path().join("rotto.docx"), "non sono un docx").unwrap();

    let out = run(src.path(), dst.path());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(1), "{stdout}");
    assert!(stdout.contains("rotto.docx - docx non valido"), "{stdout}");
}
