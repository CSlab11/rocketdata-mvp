# RocketData

RocketData converte i documenti di una cartella (PDF, DOCX e fogli di calcolo) in Markdown e CSV, pronti da leggere, cercare o elaborare. È composto da due tool indipendenti:

- **`doc-ripper`**: il convertitore, usabile anche da riga di comando;
- **`gui`**: un'interfaccia web locale che avvia `doc-ripper` e ne mostra avanzamento e risultati;
- **`rocket-tag`**: il classificatore dei paragrafi dei file Markdown, a regole o con un LLM locale, usabile dalla `gui` e da riga di comando.

Funziona su **Windows 10 e Windows 11**.

## Uso rapido

La cartella `dist\` del repository contiene già tutto il necessario: non serve compilare nulla.

1. Scarica il repository, preferibilmente all'ultima versione con tag (es. `v0.1.0`):

   ```
   git clone https://github.com/CSlab11/rocketdata-mvp.git
   cd rocketdata-mvp
   git checkout v0.1.0
   ```

   In alternativa scarica lo zip del tag dalla pagina *Releases* / *Tags* di GitHub ed estrailo.

2. Avvia l'interfaccia:

   ```
   dist\start.cmd
   ```

   Il server parte in una finestra ridotta a icona e la pagina si apre nel browser predefinito.

3. Per chiudere usa il pulsante **Esci** in alto a destra nella pagina, oppure:

   ```
   dist\stop.cmd
   ```

   Entrambi interrompono anche una conversione in corso.

Usa un tag: solo nei commit con tag `dist\` è stata rigenerata dal codice di quel commit. Su `main` tra una versione e l'altra può non corrispondere al codice.

## Convertire dall'interfaccia

1. Scegli la **cartella di origine** e la **cartella di destinazione**: puoi scriverle o usare **Sfoglia…**. Le ultime usate vengono riproposte al riavvio.
2. Premi **Avvia conversione**.
3. Durante la conversione vedi:
   - la **barra di avanzamento**, per file e, nei PDF, per pagina;
   - l'output di `doc-ripper` in tempo reale.

   Con **Annulla** interrompi la conversione: i file già generati restano nella destinazione.
4. Al termine trovi:
   - l'esito della conversione;
   - il riepilogo di file convertiti, saltati e falliti;
   - i pulsanti **Apri cartella di destinazione** e **Apri index.md**;
   - l'**anteprima di `index.md`**: cliccando un link si apre il file generato o l'originale con il programma predefinito.

   Se l'indice non è stato scritto, ad esempio dopo un annullamento, l'anteprima mostra "Nessun indice disponibile".

## Convertire da riga di comando

```
dist\doc-ripper.exe --src <cartella di origine> --dst <cartella di destinazione>
```

Esempio:

```
dist\doc-ripper.exe --src C:\dati\documenti --dst C:\dati\convertiti
```

`pdfium.dll` deve trovarsi nella stessa cartella di `doc-ripper.exe`.

Durante l'esecuzione `doc-ripper` scrive:
- prima di ogni file, una riga di avanzamento, es. `[3/12] report.pdf`;
- nei PDF, una riga per ogni pagina, es. `[3/12] pagina 120/1100`.

Alla fine scrive il riepilogo.

| Codice di uscita | Significato |
|---|---|
| `0` | Nessuna conversione fallita |
| `1` | Almeno un file non è stato convertito (vedi "Falliti" nel riepilogo); gli altri sono stati elaborati |
| `2` | Parametri mancanti, cartella di origine inesistente o destinazione non creabile |

## Classificare i paragrafi

Nella schermata **Classificazione** della gui:

1. Scegli la cartella dei file Markdown: viene proposta l'ultima destinazione di `doc-ripper`.
2. Scegli **Regole**, oppure **LLM** indicando URL e modello di un server locale compatibile con l'API OpenAI. L'eventuale chiave non viene salvata.
3. Premi **Classifica**: la pagina resta utilizzabile, con barra di avanzamento e **Annulla**.
4. I risultati compaiono per documento e unità, con i filtri per etichetta e confidenza, le unità scartate e l'avviso se i Markdown sono stati modificati dopo.

I risultati sono in `.classify\rules.json` e `.classify\llm.json` dentro la cartella. Un'esecuzione LLM annullata riprende dalle unità già classificate.

Da riga di comando (i file di configurazione sono in `dist\rocket-tag-config\`):

```
dist\rocket-tag.exe classify rules <cartella> -o rules.json --schema <schema> --rules <regole>
dist\rocket-tag.exe evaluate <etichette.jsonl> rules.json --schema <schema> --report report.md
```

`dist\rocket-tag.exe --help` elenca tutti i comandi. La chiave del server LLM si passa con la variabile d'ambiente `ROCKET_TAG_API_KEY`.

## Cosa viene prodotto

`doc-ripper` elabora ricorsivamente le sottocartelle e riproduce nella destinazione la stessa struttura. I file già presenti con lo stesso nome vengono sovrascritti.

| Origine | Risultato |
|---|---|
| `report.pdf` | `report_pdf.md`, immagini in `report_pdf_img/` |
| `report.docx` | `report_docx.md`, immagini in `report_docx_img/` |
| `vendite.xlsx` / `.xls` / `.ods` | un CSV per foglio, es. `vendite_xlsx_Gennaio.csv` (RFC 4180, UTF-8, valori calcolati, date ISO 8601) |
| altri tipi | non copiati, segnalati come "non supportato" |

- **Markdown**: titoli, paragrafi, elenchi e tabelle ricostruiti. In testa c'è la riga `Origine:` con il link al file originale.
- **`index.md`**, nella radice della destinazione: elenca ogni file generato con il link al file e al suo originale.
  - Viene cancellato all'avvio di ogni conversione e riscritto alla fine.
  - I link agli originali sono percorsi assoluti, es. `C:/dati/report.pdf`, navigabili da un editor Markdown.
- **Nessun OCR**: i documenti senza testo estraibile (scansioni) vengono saltati e segnalati come "scansionato".

## Ricompilare da sorgente

Serve solo se modifichi il codice o vuoi rigenerare `dist\`.

Prerequisiti:
- Windows 10 o 11;
- [Rust](https://rustup.rs/) (toolchain MSVC, installata da `rustup`);
- accesso a Internet alla prima compilazione: `pdfium.dll` viene scaricata da [pdfium-binaries](https://github.com/bblanchon/pdfium-binaries) e verificata con SHA-256.

Dalla radice del repository, in PowerShell:

```
.\package.ps1
```

Lo script:
1. scarica PDFium, se manca;
2. compila in release `doc-ripper`, `gui`, `search-rm3` e `rocket-tag`;
3. ricrea `dist\` con `gui.exe`, `doc-ripper.exe`, `search-rm3.exe`, `rocket-tag.exe`, `pdfium.dll` con la sua licenza, `start.cmd`, `stop.cmd` e `rocket-tag-config\`.

Se la gui di `dist\` è in esecuzione lo script si ferma: chiudila prima con `dist\stop.cmd`.

Test di ciascun tool:

```
cargo test --manifest-path doc-ripper\Cargo.toml
cargo test --manifest-path gui\Cargo.toml
cargo test --manifest-path rocket-tag\Cargo.toml
```

I test della gui usano Microsoft Edge in modalità headless.

## Preparare una release

`dist\` si aggiorna solo nei commit di release:

```
dist\stop.cmd
.\package.ps1
git add dist
git commit -m "feat(REQ-NNN): release vX.Y.Z"
git tag -a vX.Y.Z -m "RocketData X.Y.Z"
git push origin main vX.Y.Z
```

## Problemi noti

- **Eseguibili non firmati.** `gui.exe`, `doc-ripper.exe` e `pdfium.dll` non hanno ancora una firma Authenticode.
  - Con **Smart App Control** attivo Windows può bloccarli.
  - Se li scarichi come zip dal browser, SmartScreen può chiedere conferma. In quel caso sblocca lo zip prima di estrarlo: *Proprietà → Sblocca*.
  - Clonare con `git` evita la segnalazione del download.
- **Porta.** Il server ascolta solo su `127.0.0.1`, su una porta libera scelta all'avvio, e non è raggiungibile da altri computer.
- **Impostazioni.** Le ultime cartelle usate sono salvate in `%APPDATA%\rocketdata-gui\settings.json`.

## Consultare i requisiti

`req-html.ps1` trasforma i requisiti in un ipertesto da sfogliare nel browser:

```
.\req-html.ps1 -Repo .
```

Le pagine finiscono in `docs\html\`, che non è versionata. Apri `docs\html\index.html`: non servono server né connessione a Internet. Per scriverle altrove usa `-Out`:

```
.\req-html.ps1 -Repo . -Out C:\temp\requisiti
```

- **Pagine.** C'è una pagina per il progetto e una per ogni tool o sotto-tool. `index.html` le elenca con il numero di requisiti per stato.
- **Collegamenti.** Ogni `REQ-NNN` o `BUG-NNN` porta alla riga di quell'ID, anche se sta nella pagina di un altro tool.
- **Tabelle.** Sopra ogni tabella ci sono un filtro per testo e uno per stato. Un clic sull'intestazione ordina la colonna.
- **Altre repository.** Lo script funziona su qualunque repository con la stessa struttura: `docs/REQUIREMENTS.md` nella radice e `<cartella>/docs/REQUIREMENTS.md` per ogni tool. Esempio: `.\req-html.ps1 -Repo C:\repo\altro-progetto`.

## Struttura del repository

| Percorso | Contenuto |
|---|---|
| `doc-ripper/` | Il convertitore (progetto Cargo autonomo) |
| `gui/` | L'interfaccia web locale (progetto Cargo autonomo) |
| `rocket-tag/` | Il classificatore dei paragrafi (progetto Cargo autonomo) |
| `dist/` | Eseguibili pronti all'uso della release corrente |
| `package.ps1` | Compila i tool e prepara `dist\` |
| `req-html.ps1` | Genera l'ipertesto HTML dei requisiti in `docs\html\` |
| `docs/` | Requisiti di progetto e tabella dei bug (`REQUIREMENTS.md`); i requisiti di ogni tool sono in `<tool>/docs/REQUIREMENTS.md` |
| `CHANGELOG.md` | Registro delle richieste e delle loro chiusure |
