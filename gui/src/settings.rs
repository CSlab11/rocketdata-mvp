use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Ultime cartelle usate per doc-ripper (REQ-035) e ultimo numero di risultati della ricerca
/// (REQ-072).
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct Settings {
    pub src: String,
    pub dst: String,
    /// Assente nei file salvati prima di REQ-072.
    #[serde(default)]
    pub top: Option<u32>,
}

/// `%APPDATA%\rocketdata-gui\settings.json`.
pub fn default_path() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|dir| PathBuf::from(dir).join("rocketdata-gui").join("settings.json"))
}

/// Impostazioni salvate, o vuote se il file manca o non è leggibile.
pub fn load(path: &Path) -> Settings {
    fs::read_to_string(path).ok().and_then(|text| serde_json::from_str(&text).ok()).unwrap_or_default()
}

/// Modifica le impostazioni salvate lasciando invariati gli altri campi.
pub fn update(path: &Path, change: impl FnOnce(&mut Settings)) -> Result<(), String> {
    let mut settings = load(path);
    change(&mut settings);
    save(path, &settings)
}

/// Salva subito: `stop.cmd` termina il server senza dargli modo di salvare alla chiusura.
pub fn save(path: &Path, settings: &Settings) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn salva_e_rilegge() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("rocketdata-gui/settings.json");
        assert_eq!(load(&path), Settings::default());
        let settings = Settings { src: r"C:\Dati\Città".into(), dst: r"D:\Uscita".into(), top: Some(25) };
        save(&path, &settings).unwrap();
        assert_eq!(load(&path), settings);
        fs::write(&path, "non json").unwrap();
        assert_eq!(load(&path), Settings::default());
    }

    #[test]
    fn aggiorna_senza_perdere_gli_altri_campi() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        // File scritto prima di REQ-072: senza `top`.
        fs::write(&path, r#"{"src":"C:\\A","dst":"C:\\B"}"#).unwrap();
        update(&path, |s| s.top = Some(3)).unwrap();
        assert_eq!(load(&path), Settings { src: r"C:\A".into(), dst: r"C:\B".into(), top: Some(3) });
        update(&path, |s| s.dst = r"C:\C".into()).unwrap();
        assert_eq!(load(&path).top, Some(3));
    }
}
