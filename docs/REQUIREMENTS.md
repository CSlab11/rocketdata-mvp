# Requisiti e tracciabilità

## Requisiti

Requisiti di progetto. I requisiti dei singoli tool sono in [`doc-ripper/REQUIREMENTS.md`](doc-ripper/REQUIREMENTS.md) e [`gui/REQUIREMENTS.md`](gui/REQUIREMENTS.md).

| ID | Requisito | Stato | File impattati |
|----|-----------|-------|----------------|
| REQ-001 | L'applicazione deve essere scritta in Rust. | aperto | — |
| REQ-002 | L'applicazione deve essere composta da più tool, ciascuno una sotto-applicazione indipendente. | aperto | — |
| REQ-003 | Ogni tool deve risiedere in una propria cartella, con lo stesso nome del tool. | aperto | — |
| REQ-004 | L'applicazione deve includere il tool `doc-ripper`, nella cartella `doc-ripper/`. | aperto | — |
| REQ-005 | I tool non devono condividere nulla tra loro: nessun codice, libreria interna, configurazione o file di build in comune. | aperto | — |
| REQ-006 | L'applicazione deve includere il tool `gui`, nella cartella `gui/`. | aperto | — |
| REQ-040 | Il repository deve fornire nella radice uno script `package.ps1` che compila in release `doc-ripper` e `gui` e crea la cartella `dist\` con `gui.exe`, `doc-ripper.exe`, `pdfium.dll`, `start.cmd` e `stop.cmd`, così che `dist\start.cmd` avvii la gui con `doc-ripper` disponibile. Lo script non appartiene a nessun tool (REQ-005). | implementato | `.gitignore`, `package.ps1` |
| REQ-050 | Il repository deve fornire nella radice un `README.md` che spieghi a chi lo ha appena clonato da git come usare RocketData: prerequisiti (Windows 10 o 11, Rust, accesso a Internet per scaricare PDFium), preparazione della cartella `dist\` con `package.ps1`, avvio e arresto con `dist\start.cmd` e `dist\stop.cmd`, uso di `doc-ripper` dall'interfaccia di `gui` e da riga di comando (`--src`, `--dst`) e risultati prodotti (`.md`, `.csv`, `index.md`). | aperto | — |
| REQ-051 | Il repository deve versionare la cartella `dist\` prodotta da `package.ps1` (REQ-040), così che chi clona il repository o scarica un tag possa avviare RocketData con `dist\start.cmd` senza compilare; `dist\` si aggiorna solo nei commit di release, rigenerata con `package.ps1` dal codice di quel commit, e ogni commit di release ha un tag `vX.Y.Z`. | aperto | — |
### Note

- **Sotto-tool.** Un tool può dichiarare più sotto-tool: implementazioni diverse della stessa funzione, con funzioni e interfaccia equivalenti almeno nell'utilizzo (stessi ingressi, stesso tipo di risultato), così che chi li usa possa scambiarli senza cambiare il modo in cui li invoca. Ogni sotto-tool sta in una sottocartella del tool il cui nome inizia con il nome della cartella del tool, seguito da un suffisso diverso per ciascun sotto-tool. Esempio: la ricerca può avere più implementazioni, ciascuna un sotto-tool di `search/`, come `search/search-fulltext/` e `search/search-semantica/`.

## Bug

| ID | Descrizione | Req | Stato | File impattati | Modifiche necessarie |
|----|-------------|-----|-------|----------------|----------------------|
