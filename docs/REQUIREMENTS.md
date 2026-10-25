# Requisiti e tracciabilità

## Requisiti

Requisiti di progetto. I requisiti dei singoli tool sono in [`doc-ripper/REQUIREMENTS.md`](doc-ripper/REQUIREMENTS.md), [`gui/REQUIREMENTS.md`](gui/REQUIREMENTS.md) e [`search/REQUIREMENTS.md`](search/REQUIREMENTS.md), con i sotto-tool in [`search/search-rm3/REQUIREMENTS.md`](search/search-rm3/REQUIREMENTS.md).

| ID | Requisito | Stato | File impattati |
|----|-----------|-------|----------------|
| REQ-001 | L'applicazione deve essere scritta in Rust. | aperto | — |
| REQ-002 | L'applicazione deve essere composta da più tool, ciascuno una sotto-applicazione indipendente. | aperto | — |
| REQ-003 | Ogni tool deve risiedere in una propria cartella, con lo stesso nome del tool. | aperto | — |
| REQ-004 | L'applicazione deve includere il tool `doc-ripper`, nella cartella `doc-ripper/`. | aperto | — |
| REQ-005 | I tool non devono condividere nulla tra loro: nessun codice, libreria interna, configurazione o file di build in comune. | aperto | — |
| REQ-006 | L'applicazione deve includere il tool `gui`, nella cartella `gui/`. | aperto | — |
| REQ-040 | Il repository deve fornire nella radice uno script `package.ps1` che compila in release `doc-ripper` e `gui` e crea la cartella `dist\` con `gui.exe`, `doc-ripper.exe`, `pdfium.dll`, `start.cmd` e `stop.cmd`, così che `dist\start.cmd` avvii la gui con `doc-ripper` disponibile. Lo script non appartiene a nessun tool (REQ-005). | implementato | `.gitignore`, `package.ps1` |
| REQ-050 | Il repository deve fornire nella radice un `README.md` che spieghi a chi lo ha appena clonato da git come usare RocketData: prerequisiti (Windows 10 o 11, Rust, accesso a Internet per scaricare PDFium), preparazione della cartella `dist\` con `package.ps1`, avvio e arresto con `dist\start.cmd` e `dist\stop.cmd`, uso di `doc-ripper` dall'interfaccia di `gui` e da riga di comando (`--src`, `--dst`) e risultati prodotti (`.md`, `.csv`, `index.md`). | implementato | `README.md` |
| REQ-051 | Il repository deve versionare la cartella `dist\` prodotta da `package.ps1` (REQ-040), così che chi clona il repository o scarica un tag possa avviare RocketData con `dist\start.cmd` senza compilare; `dist\` si aggiorna solo nei commit di release, rigenerata con `package.ps1` dal codice di quel commit, e ogni commit di release ha un tag `vX.Y.Z`. | implementato | `.gitignore`, `dist/doc-ripper.exe`, `dist/gui.exe`, `dist/pdfium-LICENSE.txt`, `dist/pdfium.dll`, `dist/start.cmd`, `dist/stop.cmd` |
| REQ-052 | L'applicazione deve includere il tool `search`, nella cartella `search/`, composto da sotto-tool (vedi Note) che cercano nei file Markdown con l'interfaccia comune definita in `search/REQUIREMENTS.md`. | aperto | — |
### Note

- **Sotto-tool.** Un tool può dichiarare più sotto-tool: implementazioni diverse della stessa funzione, con funzioni e interfaccia equivalenti almeno nell'utilizzo (stessi ingressi, stesso tipo di risultato), così che chi li usa possa scambiarli senza cambiare il modo in cui li invoca. Ogni sotto-tool sta in una sottocartella del tool il cui nome inizia con il nome della cartella del tool, seguito da un suffisso diverso per ciascun sotto-tool. Esempio: la ricerca può avere più implementazioni, ciascuna un sotto-tool di `search/`, come `search/search-fulltext/` e `search/search-semantica/`.

## Bug

| ID | Descrizione | Req | Stato | File impattati | Modifiche necessarie |
|----|-------------|-----|-------|----------------|----------------------|
| BUG-001 | `search-rm3` non tiene conto degli escape Markdown nell'analisi del testo: `doc-ripper` scrive `RCC\_APB2ENR`, la barra rovesciata separa i token e l'identificatore viene spezzato in `rcc` e `apb2enr`, così la domanda `RCC_APB2ENR` (token `rcc_apb2enr`) restituisce zero risultati sul reference manual STM32; atteso l'identificatore intero e la sezione 8.3.7 tra i risultati. Con la stessa causa, una `\]` letterale seguita da `(` viene presa per l'inizio della destinazione di un link e il testo successivo non viene indicizzato. | REQ-063 | risolto | `search/search-rm3/src/analyze.rs` | In `analyze.rs`: in `strip_link_targets` ignorare le `](` la cui `]` è preceduta da `\`; poi, prima della divisione in token, togliere la `\` che precede un carattere di punteggiatura ASCII (escape CommonMark, es. `\_` → `_`). Aggiungere unit test con `RCC\_APB2ENR` → `rcc_apb2enr` e con `a\](b)` che conserva `b`. |
| BUG-002 | Nei `.md` generati dai PDF, le voci di sommario e di indice analitico con i puntini guida e il numero di pagina (es. `### 1 Overview of the manual . . . . . 40`, `### ETH\_MMCTIR . . . . .1050`) diventano intestazioni Markdown: nel reference manual STM32 sono 37 delle 960 intestazioni; atteso che restino testo (paragrafo), perché non sono titoli del documento. Effetto su `search-rm3`: creano sezioni fittizie e, con il peso delle intestazioni di REQ-067, vengono premiate. | REQ-010 | aperto | `doc-ripper/src/pdf.rs` | In `pdf.rs`, `is_heading` decide solo in base alla dimensione del carattere (≥ 1,15 × corpo) e alla lunghezza: aggiungere l'esclusione delle righe che contengono puntini guida, cioè almeno tre `.` consecutivi separati al più da uno spazio (regex `(\. ?){3,}`), seguiti da un numero di pagina; la riga passa allora al ramo paragrafo. Aggiungere in `doc-ripper/tests/pdf.rs` un test con una riga in carattere grande con puntini guida che deve uscire come paragrafo e non come `#`. |
