# Changelog

## 2026-10-25
- REQ-068 richiesto — search-rm3: opzione --heading-weight, maggiore di 1, predefinita 3
- REQ-067 implementato — BM25F: frequenze e lunghezze con i token dell'intestazione pesati 3 volte (`src/bm25.rs`, `src/index.rs`, `src/rm3.rs`, `src/section.rs`, `tests/query.rs`)
- REQ-068 implementato — Opzione --heading-weight, maggiore di 1, predefinita 3 (`src/bm25.rs`, `src/main.rs`, `src/query.rs`, `tests/query.rs`)

## 2026-10-24
- REQ-059 richiesto — search-rm3: progetto Cargo autonomo in search/search-rm3/
- REQ-060 richiesto — search-rm3: nessuna dipendenza da altri tool o sotto-tool
- REQ-061 richiesto — search-rm3: funzionamento su Windows 10 e 11
- REQ-062 richiesto — search-rm3: funzionamento offline senza risorse esterne
- REQ-063 richiesto — search-rm3: analisi del testo inglese con stopword e Porter2, identificatori tecnici interi
- REQ-064 richiesto — search-rm3: punteggio BM25 con k1 = 1,2 e b = 0,75
- REQ-065 richiesto — search-rm3: espansione della query con RM3 e termini riportati in terms
- REQ-066 richiesto — search-rm3: opzioni --fb-docs, --fb-terms e --lambda con i valori predefiniti
- REQ-059 implementato — Progetto Cargo search-rm3 con i comandi index e query (`.gitignore`, `Cargo.lock`, `Cargo.toml`, `src/main.rs`, `tests/cli.rs`)
- REQ-060 implementato — Workspace Cargo proprio, nessuna path dependency (`Cargo.toml`)
- REQ-063 implementato — Analisi del testo inglese: minuscole, stopword, Porter2, identificatori interi, destinazioni dei link escluse (`Cargo.lock`, `Cargo.toml`, `src/analyze.rs`, `src/main.rs`)
- REQ-064 implementato — Punteggio BM25 delle sezioni con k1 = 1,2 e b = 0,75 e termini pesati (`src/bm25.rs`, `src/main.rs`)
- REQ-061 implementato — Test su Windows 11 con percorsi con spazi, accenti e sottocartelle (`tests/windows.rs`)
- REQ-065 implementato — Espansione RM3 della domanda, numeri puri esclusi, termini rm3 riportati in terms (`src/main.rs`, `src/query.rs`, `src/rm3.rs`, `tests/rm3.rs`)
- REQ-066 implementato — Opzioni --fb-docs, --fb-terms e --lambda con i valori predefiniti (`src/main.rs`, `tests/rm3.rs`)
- REQ-062 implementato — Test che esclude crate di rete e std::net (`tests/offline.rs`)
- REQ-067 richiesto — search-rm3: più peso ai termini dell'intestazione della sezione nel punteggio
