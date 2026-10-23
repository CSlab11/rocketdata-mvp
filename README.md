# RocketData

RocketData converte i documenti di una cartella (PDF, DOCX e fogli di calcolo) in Markdown e CSV, pronti da leggere, cercare o elaborare. È composto da due tool indipendenti:

- **`doc-ripper`**: il convertitore, usabile anche da riga di comando;
- **`gui`**: un'interfaccia web locale che avvia `doc-ripper` e ne mostra avanzamento e risultati.

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
2. compila in release `doc-ripper` e `gui`;
3. ricrea `dist\` con `gui.exe`, `doc-ripper.exe`, `pdfium.dll` con la sua licenza, `start.cmd` e `stop.cmd`.

Se la gui di `dist\` è in esecuzione lo script si ferma: chiudila prima con `dist\stop.cmd`.

Test di ciascun tool:

```
cargo test --manifest-path doc-ripper\Cargo.toml
cargo test --manifest-path gui\Cargo.toml
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

## Struttura del repository

| Percorso | Contenuto |
|---|---|
| `doc-ripper/` | Il convertitore (progetto Cargo autonomo) |
| `gui/` | L'interfaccia web locale (progetto Cargo autonomo) |
| `dist/` | Eseguibili pronti all'uso della release corrente |
| `package.ps1` | Compila i tool e prepara `dist\` |
| `docs/` | Requisiti e tracciabilità (`REQUIREMENTS.md`, uno per tool) |
| `CHANGELOG.md` | Registro delle richieste e delle loro chiusure |
