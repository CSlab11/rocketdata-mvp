# Requisiti software — RocketData MVP

Fonte unica dei requisiti del progetto. Ogni modifica al codice parte da un requisito (`/req`) e corrisponde a un commit.

## Legenda

- **Tipo**: `F` funzionale · `NF` non funzionale · `BUG` correzione
- **Priorità**: `Alta` · `Media` · `Bassa`
- **Stato**: `Proposto` → `Analizzato` (`/arc`) → `Implementato` → `Verificato` (`/checkreq`)

## Tabella requisiti

| ID | Titolo | Descrizione | Tipo | Priorità | Stato | Implementazione (file / funzione) | Commit |
|----|--------|-------------|------|----------|-------|----------------|--------|
| REQ-001 | Linguaggio Rust | L'applicazione deve essere scritta in Rust. | NF | Alta | Proposto | | |
| REQ-002 | Architettura a tool | L'applicazione deve essere composta da più tool, ciascuno una sotto-applicazione indipendente. | NF | Alta | Proposto | | |
| REQ-003 | Cartella per tool | Ogni tool deve risiedere in una propria cartella, con lo stesso nome del tool. | NF | Alta | Proposto | | |
| REQ-004 | Tool doc-ripper | L'applicazione deve includere il tool `doc-ripper`, nella cartella `doc-ripper/`. | F | Alta | Proposto | | |
| REQ-005 | Isolamento dei tool | I tool non devono condividere nulla tra loro: nessun codice, libreria interna, configurazione o file di build in comune. | NF | Alta | Proposto | | |
| REQ-006 | Tool gui | L'applicazione deve includere il tool `gui`, nella cartella `gui/`. | F | Alta | Proposto | | |

## Note

- Gli ID sono progressivi e non vengono riutilizzati; un requisito rimosso resta in tabella con stato `Annullato`.
- Un requisito passa a `Implementato` solo se la colonna **Implementazione** contiene almeno un link al file e alla funzione che lo realizzano.
- Formato del link (percorso relativo alla root del repo): ``[`src/modulo.rs::funzione`](src/modulo.rs#L42)``. Più link separati da `<br>`.
- `/checkreq` verifica che ogni link punti a un file esistente e che la funzione indicata sia ancora presente.
