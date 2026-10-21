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
