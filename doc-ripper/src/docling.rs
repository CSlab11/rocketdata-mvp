//! Conversione dei PDF con Docling (REQ-048): esecuzione dello script Python distribuito in `docling\`
//! accanto all'eseguibile e lettura del `DoclingDocument` JSON che produce.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Deserialize;

/// Cartella `docling\` accanto a doc-ripper.exe, se contiene il Python e lo script di conversione.
pub fn installation() -> Option<PathBuf> {
    let dir = std::env::current_exe().ok()?.parent()?.join("docling");
    (python(&dir).is_file() && dir.join("convert_pdf.py").is_file()).then_some(dir)
}

fn python(installation: &Path) -> PathBuf {
    installation.join("python").join("python.exe")
}

/// Esegue Docling su `pdf`, scrivendo il JSON e le immagini in `out`, e restituisce il documento.
pub fn run(installation: &Path, pdf: &Path, out: &Path) -> Result<Document, String> {
    let mut command = Command::new(python(installation));
    // -I: le variabili PYTHON* e i pacchetti dell'utente non entrano nel Python distribuito.
    command.arg("-I").arg(installation.join("convert_pdf.py")).arg("--pdf").arg(pdf).arg("--out").arg(out);
    #[cfg(windows)]
    {
        // Senza console propria: lanciato dalla gui, Python non deve aprire finestre.
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    let output = command.output().map_err(|e| format!("Docling non avviabile: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let reason = stderr.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("errore sconosciuto");
        return Err(format!("Docling: {}", reason.trim()));
    }
    read(&out.join("documento.json"))
}

/// Legge un `DoclingDocument` JSON.
pub fn read(path: &Path) -> Result<Document, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    serde_json::from_str(&text).map_err(|e| format!("JSON di Docling non valido ({}): {e}", path.display()))
}

/// Parte del `DoclingDocument` usata da doc-ripper. I campi non elencati vengono ignorati.
#[derive(Debug, Deserialize)]
pub struct Document {
    pub body: Node,
    #[serde(default)]
    pub groups: Vec<Group>,
    #[serde(default)]
    pub texts: Vec<Text>,
    #[serde(default)]
    pub tables: Vec<Table>,
    #[serde(default)]
    pub pictures: Vec<Picture>,
}

#[derive(Debug, Deserialize)]
pub struct Node {
    #[serde(default)]
    pub children: Vec<Ref>,
}

/// Riferimento JSON Pointer a un elemento (es. `#/texts/3`).
#[derive(Debug, Clone, Deserialize)]
pub struct Ref {
    #[serde(rename = "$ref")]
    pub target: String,
}

#[derive(Debug, Deserialize)]
pub struct Group {
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub children: Vec<Ref>,
}

#[derive(Debug, Deserialize)]
pub struct Text {
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub level: Option<usize>,
    #[serde(default)]
    pub children: Vec<Ref>,
    #[serde(default)]
    pub prov: Vec<Prov>,
}

#[derive(Debug, Deserialize)]
pub struct Table {
    pub data: TableData,
    #[serde(default)]
    pub captions: Vec<Ref>,
    #[serde(default)]
    pub footnotes: Vec<Ref>,
    #[serde(default)]
    pub prov: Vec<Prov>,
}

#[derive(Debug, Deserialize)]
pub struct TableData {
    #[serde(default)]
    pub num_rows: usize,
    #[serde(default)]
    pub num_cols: usize,
    #[serde(default)]
    pub table_cells: Vec<Cell>,
}

#[derive(Debug, Deserialize)]
pub struct Cell {
    #[serde(default)]
    pub text: String,
    pub start_row_offset_idx: usize,
    pub start_col_offset_idx: usize,
}

#[derive(Debug, Deserialize)]
pub struct Picture {
    #[serde(default)]
    pub image: Option<ImageRef>,
    #[serde(default)]
    pub captions: Vec<Ref>,
    #[serde(default)]
    pub prov: Vec<Prov>,
}

#[derive(Debug, Deserialize)]
pub struct ImageRef {
    pub uri: String,
}

#[derive(Debug, Deserialize)]
pub struct Prov {
    pub page_no: u32,
}

/// Elemento a cui punta un riferimento.
pub enum Item<'a> {
    Group(&'a Group),
    Text(&'a Text),
    Table(&'a Table),
    Picture(&'a Picture),
}

impl Document {
    pub fn get(&self, reference: &Ref) -> Option<Item<'_>> {
        let (kind, index) = reference.target.strip_prefix("#/")?.split_once('/')?;
        let index: usize = index.parse().ok()?;
        match kind {
            "groups" => self.groups.get(index).map(Item::Group),
            "texts" => self.texts.get(index).map(Item::Text),
            "tables" => self.tables.get(index).map(Item::Table),
            "pictures" => self.pictures.get(index).map(Item::Picture),
            _ => None,
        }
    }
}

/// Prima pagina in cui compare un elemento (0 se Docling non la indica).
pub fn page(prov: &[Prov]) -> u32 {
    prov.first().map_or(0, |p| p.page_no)
}
