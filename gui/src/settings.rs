use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Ultime cartelle usate per doc-ripper (REQ-035).
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct Settings {
    pub src: String,
    pub dst: String,
}

/// `%APPDATA%\rocketdata-gui\settings.json`.
pub fn default_path() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|dir| PathBuf::from(dir).join("rocketdata-gui").join("settings.json"))
}

/// Impostazioni salvate, o vuote se il file manca o non è leggibile.
pub fn load(path: &Path) -> Settings {
    fs::read_to_string(path).ok().and_then(|text| serde_json::from_str(&text).ok()).unwrap_or_default()
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
        let settings = Settings { src: r"C:\Dati\Città".into(), dst: r"D:\Uscita".into() };
        save(&path, &settings).unwrap();
        assert_eq!(load(&path), settings);
        fs::write(&path, "non json").unwrap();
        assert_eq!(load(&path), Settings::default());
    }
}
