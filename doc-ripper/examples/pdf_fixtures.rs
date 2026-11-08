//! Genera con PDFium i PDF di prova di `tests/fixtures/` (BUG-001). I test li leggono e basta: PDFium non si
//! può collegare due volte nello stesso processo e non regge l'uso da più thread, quindi non va usata nei test.
//! I PDF sono versionati; si rigenerano solo se cambiano, con `cargo run --example pdf_fixtures`.

use std::path::{Path, PathBuf};

use pdfium_render::prelude::*;

fn main() {
    // target\debug\examples\pdf_fixtures.exe: pdfium.dll è in target\debug, dove la copia build.rs.
    let exe = std::env::current_exe().unwrap();
    let dir = exe.parent().unwrap().parent().unwrap();
    let pdfium = Pdfium::new(Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path(&dir)).unwrap());
    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests").join("fixtures");
    std::fs::create_dir_all(&out).unwrap();

    texts(
        &pdfium,
        &out.join("report.pdf"),
        &[
            (72.0, 780.0, 24.0, "Relazione annuale"),
            (72.0, 740.0, 16.0, "Introduzione"),
            (72.0, 715.0, 11.0, "Questo paragrafo continua"),
            (72.0, 702.0, 11.0, "sulla riga successiva."),
            (72.0, 675.0, 11.0, "- Primo punto"),
            (72.0, 661.0, 11.0, "- Secondo punto"),
            (72.0, 630.0, 11.0, "Voce"),
            (250.0, 630.0, 11.0, "Q1"),
            (350.0, 630.0, 11.0, "Q2"),
            (72.0, 616.0, 11.0, "Affitto"),
            (250.0, 616.0, 11.0, "100"),
            (350.0, 616.0, 11.0, "110"),
            (72.0, 590.0, 11.0, "Fine del documento."),
        ],
    );
    texts(
        &pdfium,
        &out.join("manuale.pdf"),
        &[
            (72.0, 780.0, 16.0, "Contents"),
            (72.0, 750.0, 16.0, "1 Overview of the manual . . . . . 40"),
            (72.0, 720.0, 16.0, "ETH_MMCTIR . . . . .1050"),
            (72.0, 690.0, 11.0, "Testo del corpo del documento."),
            (72.0, 676.0, 11.0, "Altra riga del corpo."),
            (72.0, 662.0, 11.0, "Terza riga del corpo."),
            (72.0, 648.0, 11.0, "Quarta riga del corpo."),
            (72.0, 620.0, 11.0, "Fine... 2"),
        ],
    );

    // Testo, immagine rossa 4×3, testo.
    let mut document = pdfium.create_new_pdf().unwrap();
    let font = document.fonts_mut().helvetica();
    let mut page = document.pages_mut().create_page_at_end(PdfPagePaperSize::a4()).unwrap();
    let objects = page.objects_mut();
    objects.create_text_object(PdfPoints::new(72.0), PdfPoints::new(780.0), "Sopra.", font, PdfPoints::new(11.0)).unwrap();
    let picture = image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(4, 3, image::Rgb([200, 30, 30])));
    objects
        .create_image_object(PdfPoints::new(72.0), PdfPoints::new(600.0), &picture, Some(PdfPoints::new(40.0)), None)
        .unwrap();
    objects.create_text_object(PdfPoints::new(72.0), PdfPoints::new(500.0), "Sotto.", font, PdfPoints::new(11.0)).unwrap();
    document.save_to_file(&out.join("foto.pdf")).unwrap();

    // Solo un'immagine a tutta pagina: documento scansionato.
    let mut document = pdfium.create_new_pdf().unwrap();
    let mut page = document.pages_mut().create_page_at_end(PdfPagePaperSize::a4()).unwrap();
    let scan = image::DynamicImage::ImageLuma8(image::GrayImage::from_pixel(20, 30, image::Luma([240])));
    page.objects_mut()
        .create_image_object(PdfPoints::new(0.0), PdfPoints::new(0.0), &scan, Some(PdfPoints::new(595.0)), None)
        .unwrap();
    document.save_to_file(&out.join("scansione.pdf")).unwrap();

    // Due pagine di testo.
    let mut document = pdfium.create_new_pdf().unwrap();
    let font = document.fonts_mut().helvetica();
    for text in ["Prima pagina.", "Seconda pagina."] {
        let mut page = document.pages_mut().create_page_at_end(PdfPagePaperSize::a4()).unwrap();
        page.objects_mut()
            .create_text_object(PdfPoints::new(72.0), PdfPoints::new(780.0), text, font, PdfPoints::new(11.0))
            .unwrap();
    }
    document.save_to_file(&out.join("doc.pdf")).unwrap();

    // 5000 pagine vuote: abbastanza righe di avanzamento da riempire la pipe (tests/index.rs).
    let mut document = pdfium.create_new_pdf().unwrap();
    for _ in 0..5000 {
        document.pages_mut().create_page_at_end(PdfPagePaperSize::a4()).unwrap();
    }
    document.save_to_file(&out.join("lungo.pdf")).unwrap();

    println!("PDF di prova scritti in {}", out.display());
}

/// PDF di una pagina con i testi indicati: (x, y, dimensione, testo).
fn texts(pdfium: &Pdfium, path: &Path, texts: &[(f32, f32, f32, &str)]) {
    let mut document = pdfium.create_new_pdf().unwrap();
    let font = document.fonts_mut().helvetica();
    let mut page = document.pages_mut().create_page_at_end(PdfPagePaperSize::a4()).unwrap();
    for &(x, y, size, text) in texts {
        page.objects_mut()
            .create_text_object(PdfPoints::new(x), PdfPoints::new(y), text, font, PdfPoints::new(size))
            .unwrap();
    }
    document.save_to_file(path).unwrap();
}
