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

## Bug

| ID | Descrizione | Req | Stato | File impattati | Modifiche necessarie |
|----|-------------|-----|-------|----------------|----------------------|
