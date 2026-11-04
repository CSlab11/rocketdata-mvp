# Changelog

## 2026-11-04
- REQ-152 richiesto — search: `index` con `--exclude` ripetibile e senza esclusioni predefinite legate ad altri tool
- REQ-076 annullato — sostituito da REQ-152 (esclusioni passate con `--exclude` invece che predefinite)
- REQ-152 implementato — `search-rm3 index` con `--exclude` ripetibile, tolte le esclusioni fisse di `index.md` e `.index` (`search-rm3/src/index.rs`, `search-rm3/src/main.rs`, `search-rm3/tests/index.rs`)

## 2026-10-25
- REQ-076 richiesto — search: index esclude la cartella .index nella radice di --src
- REQ-054 annullato — sostituito da REQ-076 (esclusione di .index)
- REQ-076 implementato — index salta la cartella .index nella radice di --src (`search-rm3/src/index.rs`, `search-rm3/tests/index.rs`)

## 2026-10-24
- REQ-053 richiesto — search: sotto-tool search-<suffisso>.exe intercambiabili con i comandi index e query
- REQ-054 richiesto — search: comando index che indicizza i .md di --src (escluso index.md) in --index
- REQ-055 richiesto — search: la sezione Markdown come unità di ricerca
- REQ-056 richiesto — search: comando query con risultato JSON (query, terms, results con il testo completo della sezione)
- REQ-057 richiesto — search: marcatore del sotto-tool nell'indice ed errore su indice assente o di altro sotto-tool
- REQ-058 richiesto — search: codici di uscita, errori su stderr e opzioni dei sotto-tool facoltative
- REQ-055 implementato — Suddivisione dei .md in sezioni per intestazione ATX, blocchi di codice esclusi (`search-rm3/src/main.rs`, `search-rm3/src/section.rs`)
- REQ-054 implementato — Comando index: sezioni dei .md di --src, escluso index.md della radice, in index.json (`search-rm3/Cargo.lock`, `search-rm3/Cargo.toml`, `search-rm3/src/index.rs`, `search-rm3/src/main.rs`, `search-rm3/tests/index.rs`)
- REQ-057 implementato — Marcatore search-tool nell'indice, errore su indice assente o di altro sotto-tool (`search-rm3/src/index.rs`, `search-rm3/src/main.rs`, `search-rm3/tests/marker.rs`)
- REQ-056 implementato — Comando query: JSON con query, terms e results con il testo completo della sezione (`search-rm3/src/main.rs`, `search-rm3/src/query.rs`, `search-rm3/tests/query.rs`)
- REQ-058 implementato — Codici di uscita, errori su stderr e opzioni RM3 facoltative verificati dai test (`search-rm3/tests/errors.rs`)
