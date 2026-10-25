//! search-rm3 finto per i test di gui: viene copiato come `search-rm3.exe` accanto a `gui.exe`.
//! Scrive gli argomenti ricevuti in `argomenti-index.txt` e `argomenti-query.txt` nella cartella
//! dell'indice. Se la cartella di origine contiene il file `fallisci`, `index` termina con
//! codice 1 e un messaggio su stderr; `query` fallisce se l'indice non è stato creato.

use std::path::PathBuf;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let value = |flag: &str| args.iter().position(|a| a == flag).and_then(|i| args.get(i + 1)).cloned();
    let fail = |message: &str| -> ! {
        eprintln!("{message}");
        std::process::exit(1);
    };
    let Some(dir) = value("--index").map(PathBuf::from) else { fail("manca --index") };
    match args.first().map(String::as_str) {
        Some("index") => {
            let Some(src) = value("--src").map(PathBuf::from) else { fail("manca --src") };
            if src.join("fallisci").exists() {
                fail("errore finto: indice non creato");
            }
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join("argomenti-index.txt"), args.join("\n")).unwrap();
        }
        Some("query") => {
            if !dir.join("argomenti-index.txt").exists() {
                fail(&format!("la cartella non contiene un indice di search-rm3: {}", dir.display()));
            }
            std::fs::write(dir.join("argomenti-query.txt"), args.join("\n")).unwrap();
            let response = serde_json::json!({
                "query": value("--q").unwrap_or_default(),
                "terms": [{ "term": "clock", "weight": 1.0, "source": "query" }],
                "results": [{
                    "file": "manuali/rm0008.md",
                    "heading": "7.3.7 Clock control",
                    "line": 12,
                    "score": 4.25,
                    "text": "## 7.3.7 Clock control\n\nIl registro **RCC_APB2ENR** abilita i clock.\n",
                }],
            });
            println!("{response}");
        }
        _ => fail("comando sconosciuto"),
    }
}
