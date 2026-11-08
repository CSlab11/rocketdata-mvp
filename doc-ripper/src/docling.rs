//! Conversione dei PDF con Docling (REQ-048): esecuzione dello script Python distribuito in `docling\`
//! accanto all'eseguibile e lettura del `DoclingDocument` JSON che produce.

use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde::Deserialize;

/// Cartella `docling\` accanto a doc-ripper.exe, se contiene il Python e lo script di conversione.
pub fn installation() -> Option<PathBuf> {
    let dir = std::env::current_exe().ok()?.parent()?.join("docling");
    (python(&dir).is_file() && dir.join("convert_pdf.py").is_file()).then_some(dir)
}

fn python(installation: &Path) -> PathBuf {
    installation.join("python").join("python.exe")
}

/// Blocchi del documento `pdf`, dalla cache in `out` se il PDF (sha1) e la versione di Docling sono quelli
/// dell'esecuzione precedente (REQ-069), altrimenti eseguendo Docling.
pub fn convert(installation: &Path, pdf: &Path, out: &Path, progress: &str) -> Result<Vec<Document>, String> {
    let key = format!("sha1 {}\ndocling {}\n", sha1(pdf)?, version(installation));
    let key_path = out.join("chiave.txt");
    if std::fs::read_to_string(&key_path).is_ok_and(|saved| saved == key)
        && let Ok(documents) = read_blocks(out)
    {
        return Ok(documents);
    }
    // Una conversione interrotta non deve lasciare una chiave valida accanto a blocchi incompleti.
    let _ = std::fs::remove_file(&key_path);
    let documents = run(installation, pdf, out, progress)?;
    std::fs::write(&key_path, key).map_err(|e| format!("{}: {e}", key_path.display()))?;
    Ok(documents)
}

fn sha1(path: &Path) -> Result<String, String> {
    let mut file = std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut hasher = sha1_smol::Sha1::new();
    let mut buffer = vec![0u8; 1 << 16];
    loop {
        let n = file.read(&mut buffer).map_err(|e| format!("{}: {e}", path.display()))?;
        if n == 0 {
            return Ok(hasher.digest().to_string());
        }
        hasher.update(&buffer[..n]);
    }
}

/// Versione di Docling installata, dal nome della cartella `docling-<versione>.dist-info` del Python distribuito.
fn version(installation: &Path) -> String {
    let packages = installation.join("python").join("Lib").join("site-packages");
    std::fs::read_dir(packages)
        .into_iter()
        .flatten()
        .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
        .find_map(|name| {
            let version = name.strip_prefix("docling-")?.strip_suffix(".dist-info")?;
            version.starts_with(|c: char| c.is_ascii_digit()).then(|| version.to_string())
        })
        .unwrap_or_else(|| "sconosciuta".to_string())
}

/// Esegue Docling su `pdf`, scrivendo i blocchi JSON e le immagini in `out`, e restituisce i blocchi del
/// documento nell'ordine delle pagine. Lo script converte a blocchi di pagine e dopo ciascuno scrive
/// `pagina X/Y`, che diventa la riga di avanzamento `<progress> pagina X/Y` (REQ-054).
fn run(installation: &Path, pdf: &Path, out: &Path, progress: &str) -> Result<Vec<Document>, String> {
    let mut command = Command::new(python(installation));
    // -I: le variabili PYTHON* e i pacchetti dell'utente non entrano nel Python distribuito.
    command.arg("-I").arg(installation.join("convert_pdf.py")).arg("--pdf").arg(pdf).arg("--out").arg(out);
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    #[cfg(windows)]
    {
        // Senza console propria: lanciato dalla gui, Python non deve aprire finestre.
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    let mut child = command.spawn().map_err(|e| format!("Docling non avviabile: {e}"))?;
    // Lo stderr (log di Docling) si legge a parte: un buffer pieno bloccherebbe lo script.
    let mut stderr = child.stderr.take().unwrap();
    let errors = std::thread::spawn(move || {
        let mut text = String::new();
        let _ = stderr.read_to_string(&mut text);
        text
    });
    for line in BufReader::new(child.stdout.take().unwrap()).lines().map_while(Result::ok) {
        if line.starts_with("pagina ") {
            println!("{progress} {}", line.trim());
        }
    }
    let status = child.wait().map_err(|e| format!("Docling: {e}"))?;
    let errors = errors.join().unwrap_or_default();
    if !status.success() {
        let reason = errors.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("errore sconosciuto");
        return Err(format!("Docling: {}", reason.trim()));
    }
    read_blocks(out)
}

/// Blocchi `blocco-NNNN.json` di `dir`, in ordine di pagina.
pub fn read_blocks(dir: &Path) -> Result<Vec<Document>, String> {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(dir)
        .map_err(|e| format!("{}: {e}", dir.display()))?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|p| {
            let name = p.file_name().unwrap_or_default().to_string_lossy();
            name.starts_with("blocco-") && name.ends_with(".json")
        })
        .collect();
    paths.sort();
    if paths.is_empty() {
        return Err(format!("Docling non ha prodotto alcun blocco in {}", dir.display()));
    }
    paths.iter().map(|p| read(p)).collect()
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
    #[serde(default = "body")]
    pub content_layer: String,
}

#[derive(Debug, Deserialize)]
pub struct Text {
    #[serde(default)]
    pub label: String,
    #[serde(default = "body")]
    pub content_layer: String,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub level: Option<usize>,
    /// Numerazione o simbolo della voce di elenco (es. `a)`, `1.`, `•`).
    #[serde(default)]
    pub marker: Option<String>,
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
    /// Fine (esclusa) dell'estensione della cella; assente vale una sola riga o colonna.
    #[serde(default)]
    pub end_row_offset_idx: Option<usize>,
    #[serde(default)]
    pub end_col_offset_idx: Option<usize>,
    #[serde(default)]
    pub column_header: bool,
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
    #[serde(default)]
    pub bbox: Option<BBox>,
}

#[derive(Debug, Deserialize)]
pub struct BBox {
    /// Margine sinistro in punti.
    pub l: f32,
}

/// Livello predefinito degli elementi: il corpo del documento.
fn body() -> String {
    "body".to_string()
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
