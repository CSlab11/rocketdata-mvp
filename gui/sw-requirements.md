# Requisiti software — gui

Requisiti specifici del tool `gui` (vedi [REQ-006](../sw-requirements.md)). I requisiti di progetto restano in [`../sw-requirements.md`](../sw-requirements.md) e valgono anche qui; questo file contiene solo ciò che riguarda `gui`.

Ogni modifica al codice del tool parte da un requisito (`/req`) e corrisponde a un commit.

## Legenda

- **Tipo**: `F` funzionale · `NF` non funzionale · `BUG` correzione
- **Priorità**: `Alta` · `Media` · `Bassa`
- **Stato**: `Proposto` → `Analizzato` (`/arc`) → `Implementato` → `Verificato` (`/checkreq`)
- **Origine**: requisito di progetto da cui deriva (se presente)

## Tabella requisiti

| ID | Titolo | Descrizione | Tipo | Priorità | Origine | Stato | Implementazione (file / funzione) | Commit |
|----|--------|-------------|------|----------|---------|-------|----------------|--------|
| GUI-001 | Progetto Rust autonomo | `gui` deve essere un progetto Cargo autonomo con il proprio `Cargo.toml` in `gui/`. | NF | Alta | REQ-001, REQ-003 | Proposto | | |
| GUI-002 | Nessuna dipendenza da altri tool | `gui` non deve dipendere da codice, crate, configurazioni o file di build di altri tool (niente workspace Cargo condiviso, niente path dependency verso cartelle esterne a `gui/`). | NF | Alta | REQ-005 | Proposto | | |
| GUI-003 | Piattaforma Windows | `gui` deve funzionare su Windows 10 e Windows 11. | NF | Alta | | Proposto | | |
| GUI-004 | Server web locale aperto nel browser | `gui` deve essere un server web locale scritto in Rust che serve un'interfaccia web (HTML/CSS/JS). All'avvio deve aprire automaticamente l'interfaccia nel browser predefinito. | NF | Alta | REQ-001 | Proposto | | |
| GUI-005 | Interfaccia multifunzione | `gui` deve offrire più funzioni all'interno della stessa interfaccia; una di queste è l'avvio di `doc-ripper`. | F | Alta | | Proposto | | |
| GUI-006 | Avvio di doc-ripper come processo esterno | `gui` deve avviare `doc-ripper` eseguendone il binario come processo separato, passando i parametri da riga di comando (`--src`, `--dst`), e leggerne output e codice di uscita. Non deve importare né includere codice di `doc-ripper`. | F | Alta | REQ-005 | Proposto | | |
| GUI-007 | Posizione dell'eseguibile di doc-ripper | `gui` deve cercare `doc-ripper.exe` nella stessa cartella del proprio eseguibile. Se non lo trova, deve mostrare un messaggio di errore e disabilitare l'avvio di `doc-ripper`. | F | Alta | | Proposto | | |
| GUI-008 | Selezione delle cartelle | La schermata di `doc-ripper` deve permettere di scegliere la cartella di origine e quella di destinazione con un pulsante "Sfoglia" che apre il selettore di cartelle di Windows. Il selettore è aperto dal server, perché il browser non può fornire il percorso completo di una cartella. | F | Alta | | Proposto | | |
| GUI-009 | Avanzamento in tempo reale | Durante l'esecuzione, la schermata deve mostrare in tempo reale l'output prodotto da `doc-ripper`. | F | Alta | | Proposto | | |
| GUI-010 | Riepilogo finale | Al termine dell'esecuzione, la schermata deve mostrare l'esito: file convertiti, saltati e falliti, come riportati nel riepilogo di `doc-ripper` (DR-011), e se l'esecuzione è riuscita in base al codice di uscita. | F | Alta | | Proposto | | |
| GUI-011 | Apertura dei risultati | Al termine dell'esecuzione, la schermata deve offrire due pulsanti: uno apre la cartella di destinazione in Esplora file, l'altro apre `index.md` (DR-016). | F | Media | | Proposto | | |
| GUI-012 | Esecuzione annullabile | Durante l'esecuzione di `doc-ripper` l'interfaccia deve restare utilizzabile e deve offrire un pulsante "Annulla" che interrompe il processo. | F | Alta | | Proposto | | |
| GUI-013 | Memoria delle ultime cartelle | `gui` deve ricordare le ultime cartelle di origine e destinazione usate e proporle precompilate al successivo avvio, anche dopo il riavvio del server. | F | Media | | Proposto | | |
| GUI-014 | Accesso solo locale | Il server deve accettare connessioni solo da `localhost` (`127.0.0.1`), così che l'interfaccia non sia raggiungibile da altri computer in rete. | NF | Alta | | Proposto | | |
| GUI-015 | Script di avvio e arresto | `gui` deve fornire uno script di avvio, che avvia il server se non è già in esecuzione, e uno script di arresto, che lo termina insieme a un eventuale `doc-ripper` in corso. | F | Media | | Implementato | [`start.cmd`](start.cmd)<br>[`stop.cmd`](stop.cmd) | |

## Note

- Gli ID usano il prefisso `GUI-`, sono progressivi e non vengono riutilizzati; un requisito rimosso resta in tabella con stato `Annullato`.
- Un requisito passa a `Implementato` solo se la colonna **Implementazione** contiene almeno un link al file e alla funzione che lo realizzano.
- Formato del link (percorso relativo a `gui/`): ``[`src/modulo.rs::funzione`](src/modulo.rs#L42)``. Più link separati da `<br>`.
- `/checkreq` verifica che ogni link punti a un file esistente e che la funzione indicata sia ancora presente.
