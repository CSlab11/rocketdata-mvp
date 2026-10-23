use std::fs;
use std::path::Path;
use std::process::Command;

use rust_xlsxwriter::Workbook;

fn xlsx(path: &Path, sheets: &[&str]) {
    let mut wb = Workbook::new();
    for name in sheets {
        let ws = wb.add_worksheet();
        ws.set_name(*name).unwrap();
        ws.write_string(0, 0, "dato").unwrap();
    }
    wb.save(path).unwrap();
}

/// Verifica che il link, un percorso assoluto senza `file://`, porti al file indicato.
fn assert_points_to(link: &str, file: &Path) {
    assert!(!link.starts_with("file:"), "{link}");
    let path = url::Url::parse(&format!("file:///{link}")).unwrap().to_file_path().unwrap();
    assert_eq!(fs::canonicalize(path).unwrap(), fs::canonicalize(file).unwrap());
}

#[test]
fn indice_con_link_relativi_e_originali() {
    let src = tempfile::tempdir().unwrap();
    let dst = tempfile::tempdir().unwrap();
    fs::create_dir(src.path().join("conti 2026")).unwrap();
    xlsx(&src.path().join("conti 2026/vendite.xlsx"), &["Gennaio", "Febbraio"]);
    fs::write(src.path().join("note.txt"), "non supportato").unwrap();

    let out = Command::new(env!("CARGO_BIN_EXE_doc-ripper"))
        .args(["--src", src.path().to_str().unwrap(), "--dst", dst.path().to_str().unwrap()])
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stdout));

    let index = fs::read_to_string(dst.path().join("index.md")).unwrap();
    let rows: Vec<&str> = index.lines().filter(|l| l.starts_with("| [")).collect();
    assert_eq!(rows.len(), 2, "{index}");
    assert!(!index.contains("note.txt"), "{index}");

    for (row, sheet) in rows.iter().zip(["Gennaio", "Febbraio"]) {
        let links: Vec<&str> = row.split("](").skip(1).map(|s| s.split(')').next().unwrap()).collect();
        let generated = format!("conti 2026/vendite_xlsx_{sheet}.csv");
        assert_eq!(links[0], generated.replace(' ', "%20"));
        assert!(dst.path().join(&generated).is_file());
        assert_points_to(links[1], &src.path().join("conti 2026/vendite.xlsx"));
    }
}
