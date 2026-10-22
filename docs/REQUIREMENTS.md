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
| REQ-040 | Il repository deve fornire nella radice uno script `package.ps1` che compila in release `doc-ripper` e `gui` e crea la cartella `dist\` con `gui.exe`, `doc-ripper.exe`, `pdfium.dll`, `start.cmd` e `stop.cmd`, così che `dist\start.cmd` avvii la gui con `doc-ripper` disponibile. Lo script non appartiene a nessun tool (REQ-005). | aperto | — |

### Note

- **Sotto-tool.** Un tool può dichiarare più sotto-tool: implementazioni diverse della stessa funzione, con funzioni e interfaccia equivalenti almeno nell'utilizzo (stessi ingressi, stesso tipo di risultato), così che chi li usa possa scambiarli senza cambiare il modo in cui li invoca. Ogni sotto-tool sta in una sottocartella del tool il cui nome inizia con il nome della cartella del tool, seguito da un suffisso diverso per ciascun sotto-tool. Esempio: la ricerca può avere più implementazioni, ciascuna un sotto-tool di `search/`, come `search/search-fulltext/` e `search/search-semantica/`.

## Bug

| ID | Descrizione | Req | Stato | File impattati | Modifiche necessarie |
|----|-------------|-----|-------|----------------|----------------------|
