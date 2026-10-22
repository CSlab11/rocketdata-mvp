# Requisiti e tracciabilità — gui

Requisiti del tool `gui`. I requisiti di progetto e la tabella dei bug sono in [`../REQUIREMENTS.md`](../REQUIREMENTS.md); gli ID `REQ-NNN` sono unici in tutto il repository.

## Requisiti

| ID | Requisito | Stato | File impattati |
|----|-----------|-------|----------------|
| REQ-023 | `gui` deve essere un progetto Cargo autonomo con il proprio `Cargo.toml` in `gui/` (da REQ-001, REQ-003). | implementato | `gui/.gitignore`, `gui/Cargo.lock`, `gui/Cargo.toml`, `gui/src/main.rs` |
| REQ-024 | `gui` non deve dipendere da codice, crate, configurazioni o file di build di altri tool: niente workspace Cargo condiviso, niente path dependency verso cartelle esterne a `gui/` (da REQ-005). | implementato | `gui/Cargo.toml` |
| REQ-025 | `gui` deve funzionare su Windows 10 e Windows 11. | implementato | `gui/src/main.rs` |
| REQ-026 | `gui` deve essere un server web locale scritto in Rust che serve un'interfaccia web (HTML/CSS/JS) e, all'avvio, la apre automaticamente nel browser predefinito (da REQ-001). | implementato | `gui/Cargo.lock`, `gui/Cargo.toml`, `gui/src/main.rs`, `gui/src/server.rs`, `gui/web/app.js`, `gui/web/index.html`, `gui/web/style.css` |
| REQ-027 | `gui` deve offrire più funzioni all'interno della stessa interfaccia, tra cui l'avvio di `doc-ripper`. | aperto | — |
| REQ-028 | `gui` deve avviare `doc-ripper` eseguendone il binario come processo separato con i parametri `--src` e `--dst`, leggendone output e codice di uscita, senza importare né includere codice di `doc-ripper` (da REQ-005). | aperto | — |
| REQ-029 | `gui` deve cercare `doc-ripper.exe` nella cartella del proprio eseguibile e, se non lo trova, mostrare un messaggio di errore e disabilitare l'avvio di `doc-ripper`. | aperto | — |
| REQ-030 | La schermata di `doc-ripper` in `gui` deve permettere di scegliere le cartelle di origine e destinazione con un pulsante "Sfoglia" che apre il selettore di cartelle di Windows, aperto dal server perché il browser non fornisce il percorso completo di una cartella. | aperto | — |
| REQ-031 | Durante l'esecuzione di `doc-ripper`, `gui` deve mostrare in tempo reale l'output prodotto. | aperto | — |
| REQ-032 | Al termine dell'esecuzione di `doc-ripper`, `gui` deve mostrare file convertiti, saltati e falliti come riportati nel riepilogo (REQ-017) e se l'esecuzione è riuscita in base al codice di uscita. | aperto | — |
| REQ-033 | Al termine dell'esecuzione di `doc-ripper`, `gui` deve offrire un pulsante che apre la cartella di destinazione in Esplora file e uno che apre `index.md` (REQ-022). | aperto | — |
| REQ-034 | Durante l'esecuzione di `doc-ripper` l'interfaccia di `gui` deve restare utilizzabile e offrire un pulsante "Annulla" che interrompe il processo. | aperto | — |
| REQ-035 | `gui` deve ricordare le ultime cartelle di origine e destinazione usate e proporle precompilate anche dopo il riavvio del server. | aperto | — |
| REQ-036 | Il server di `gui` deve accettare connessioni solo da `localhost` (`127.0.0.1`), così che l'interfaccia non sia raggiungibile da altri computer in rete. | implementato | `gui/Cargo.lock`, `gui/Cargo.toml`, `gui/src/main.rs`, `gui/src/server.rs`, `gui/tests/common/mod.rs`, `gui/tests/local_only.rs` |
| REQ-037 | `gui` deve fornire uno script di avvio, che avvia il server se non è già in esecuzione, e uno script di arresto, che lo termina insieme a un eventuale `doc-ripper` in corso. | aperto | — |
| REQ-039 | L'interfaccia di `gui` deve offrire un pulsante "Esci" che arresta il server, terminando anche un eventuale `doc-ripper` in corso. | aperto | — |
