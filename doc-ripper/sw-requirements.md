# Requisiti software — doc-ripper

Requisiti specifici del tool `doc-ripper` (vedi [REQ-004](../sw-requirements.md)). I requisiti di progetto restano in [`../sw-requirements.md`](../sw-requirements.md) e valgono anche qui; questo file contiene solo ciò che riguarda `doc-ripper`.

Ogni modifica al codice del tool parte da un requisito (`/req`) e corrisponde a un commit.

## Legenda

- **Tipo**: `F` funzionale · `NF` non funzionale · `BUG` correzione
- **Priorità**: `Alta` · `Media` · `Bassa`
- **Stato**: `Proposto` → `Analizzato` (`/arc`) → `Implementato` → `Verificato` (`/checkreq`)
- **Origine**: requisito di progetto da cui deriva (se presente)

## Tabella requisiti

| ID | Titolo | Descrizione | Tipo | Priorità | Origine | Stato | Implementazione (file / funzione) | Commit |
|----|--------|-------------|------|----------|---------|-------|----------------|--------|
| DR-001 | Progetto Rust autonomo | `doc-ripper` deve essere un progetto Cargo autonomo con il proprio `Cargo.toml` in `doc-ripper/`. | NF | Alta | REQ-001, REQ-003 | Proposto | | |
| DR-002 | Nessuna dipendenza da altri tool | `doc-ripper` non deve dipendere da codice, crate, configurazioni o file di build di altri tool (niente workspace Cargo condiviso, niente path dependency verso cartelle esterne a `doc-ripper/`). | NF | Alta | REQ-005 | Proposto | | |
| DR-003 | Cartelle di origine e destinazione | `doc-ripper` deve ricevere da riga di comando due percorsi del file system tramite flag con nome: `--src <origine>` (cartella di origine) e `--dst <destinazione>` (cartella di destinazione). | F | Alta | | Proposto | | |
| DR-004 | Conversione PDF → Markdown | `doc-ripper` deve convertire ogni file `.pdf` della cartella di origine in un file Markdown nella cartella di destinazione, chiamato `<nome>_pdf.md` (es. `report.pdf` → `report_pdf.md`). Il Markdown deve riprodurre la struttura del documento: titoli, paragrafi, elenchi e tabelle; le immagini sono gestite secondo DR-014. | F | Alta | | Proposto | | |
| DR-005 | Conversione DOCX → Markdown | `doc-ripper` deve convertire ogni file `.docx` della cartella di origine in un file Markdown nella cartella di destinazione, chiamato `<nome>_docx.md` (es. `report.docx` → `report_docx.md`). Il Markdown deve riprodurre la struttura del documento: titoli, paragrafi, elenchi e tabelle; le immagini sono gestite secondo DR-014. | F | Alta | | Proposto | | |
| DR-006 | Conversione Excel → CSV | `doc-ripper` deve convertire ogni foglio di calcolo `.xlsx`, `.xls` e `.ods` della cartella di origine in file `.csv` nella cartella di destinazione, uno per ciascun foglio (tab), anche quando il foglio è uno solo. Nome: `<nome>_<estensione>_<foglio>.csv` (es. `vendite.xlsx`, foglio `Gennaio` → `vendite_xlsx_Gennaio.csv`); i caratteri del nome del foglio non validi in un nome di file sono sostituiti con `_`. I fogli nascosti vengono esportati; i fogli vuoti vengono saltati e segnalati nel riepilogo (DR-011). | F | Alta | | Proposto | | |
| DR-007 | Documenti scansionati non supportati | `doc-ripper` non supporta, per il momento, i documenti scansionati e non esegue OCR. Un documento da cui non si estrae alcun testo è considerato scansionato: non viene generato il `.md` e il file viene segnalato nel riepilogo (DR-011). | F | Media | | Proposto | | |
| DR-008 | Link al file originale | Ogni file `.md` generato deve contenere in testa il link al file originale, come URL assoluto `file://` (es. `file:///C:/dati/report.pdf`), con i caratteri speciali codificati (es. spazio → `%20`) in modo che il link sia navigabile. Per i `.csv` il link si trova nell'indice (DR-016). | F | Alta | | Proposto | | |
| DR-009 | Elaborazione delle sottocartelle | `doc-ripper` deve elaborare ricorsivamente anche le sottocartelle della cartella di origine, riproducendo nella cartella di destinazione la stessa struttura di cartelle. | F | Alta | | Proposto | | |
| DR-010 | Sovrascrittura dei file esistenti | Se nella cartella di destinazione esiste già un file con il nome di un file da generare, `doc-ripper` deve sovrascriverlo. | F | Media | | Proposto | | |
| DR-011 | Riepilogo di fine esecuzione | Al termine `doc-ripper` deve mostrare un riepilogo con: file convertiti; file saltati con il motivo (non supportato, scansionato, foglio vuoto); file la cui conversione è fallita con il relativo errore. | F | Alta | | Proposto | | |
| DR-012 | Gestione degli errori di conversione | Se la conversione di un file fallisce, `doc-ripper` deve proseguire con i file successivi. Se almeno una conversione è fallita, il processo deve terminare con codice di uscita diverso da 0. | F | Alta | | Proposto | | |
| DR-013 | File non supportati | I file diversi da `.pdf`, `.docx`, `.xlsx`, `.xls` e `.ods` non vengono elaborati né copiati, e vengono segnalati nel riepilogo (DR-011) come non supportati. | F | Media | | Proposto | | |
| DR-014 | Estrazione delle immagini | Le immagini contenute in `.pdf` e `.docx` devono essere estratte in una cartella accanto al `.md`, chiamata `<nome>_<estensione>_img/` (es. `report_pdf_img/`), e richiamate dal `.md` con link relativi nella posizione in cui compaiono nel documento. | F | Media | | Proposto | | |
| DR-015 | Formato dei CSV | I `.csv` devono rispettare RFC 4180: separatore virgola, codifica UTF-8 senza BOM. Le celle contengono valori grezzi: per le formule il risultato calcolato, le date in formato ISO 8601 (es. `2026-10-21`), i numeri con il punto decimale e senza separatore delle migliaia. | F | Alta | | Proposto | | |
| DR-016 | Indice dei file generati | `doc-ripper` deve generare un file `index.md` nella radice della cartella di destinazione che elenca ogni file generato (`.md` e `.csv`) con un link relativo al file generato e il link `file://` al file originale, codificato come in DR-008. | F | Alta | | Proposto | | |

## Note

- Gli ID usano il prefisso `DR-`, sono progressivi e non vengono riutilizzati; un requisito rimosso resta in tabella con stato `Annullato`.
- Un requisito passa a `Implementato` solo se la colonna **Implementazione** contiene almeno un link al file e alla funzione che lo realizzano.
- Formato del link (percorso relativo a `doc-ripper/`): ``[`src/modulo.rs::funzione`](src/modulo.rs#L42)``. Più link separati da `<br>`.
- `/checkreq` verifica che ogni link punti a un file esistente e che la funzione indicata sia ancora presente.
