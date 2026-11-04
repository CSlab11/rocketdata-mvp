# Convenzioni di tracciabilità

Condivise dalle skill `req`, `arc`, `impl`, `checkreq`, `bugtext`, `bugfix`.

## Regola base

Ogni modifica ai file del progetto appartiene a un ID (`REQ-NNN` o `BUG-NNN`; per una parte del progetto, `<parte>_REQ-NNN` o `<parte>_BUG-NNN`, vedi "Numerazione degli ID") e finisce in un commit che cita quell'ID. Nessuna modifica resta fuori da un commit.

## Prima di scrivere qualsiasi file

1. La cartella deve essere un repository git. Se non lo è, fermati e proponi `git init`.
2. Il working tree deve essere pulito (`git status --porcelain` vuoto). Se non lo è, fermati e chiedi come trattare le modifiche pendenti: non includerle in silenzio nel tuo commit.

## File di tracciabilità

| File | Contenuto |
|---|---|
| `docs/REQUIREMENTS.md` | Tabelle Requisiti e Bug di progetto: fonte di verità della tracciabilità |
| `<cartella>/docs/REQUIREMENTS.md` | Facoltativi: tabelle Requisiti e Bug di una parte del progetto (es. un tool), nella cartella `docs/` della parte |
| `CHANGELOG.md` | Registro cronologico delle richieste (requisiti e bug) di progetto e delle parti non autonome, e della loro chiusura |
| `<cartella>/CHANGELOG.md` | Registro cronologico di una parte autonoma (vedi "File impattati"), nella cartella della parte |

Se mancano, creali dai modelli qui sotto.

### Struttura ad albero

Se il progetto è diviso in parti (tool, moduli) con cartelle proprie, i loro requisiti possono stare nella cartella `docs/` della parte, in `<cartella>/docs/REQUIREMENTS.md`: ad esempio i requisiti di `doc-ripper/` in `doc-ripper/docs/REQUIREMENTS.md`, quelli di `search/search-rm3/` in `search/search-rm3/docs/REQUIREMENTS.md`. Ogni file tratta solo la sua parte; l'integrazione tra parti va nel file della parte che la realizza.

- `docs/REQUIREMENTS.md` resta obbligatorio: contiene i requisiti e i bug di progetto.
- I file `<cartella>/docs/REQUIREMENTS.md` contengono la tabella Requisiti e, se servono, la tabella Bug della parte, con le stesse colonne di `docs/REQUIREMENTS.md`. Sotto `docs/` nella radice non ci sono altri `REQUIREMENTS.md`.
- Un nuovo requisito o bug va nel file della parte che riguarda; se riguarda più parti o nessuna, in `docs/REQUIREMENTS.md`.
- Una parte autonoma (percorsi dalla propria cartella, vedi "File impattati") ha il proprio `<cartella>/CHANGELOG.md`, con le voci dei soli suoi ID scritti senza prefisso; le sue voci non stanno nel `CHANGELOG.md` della radice, che resta per il progetto e per le parti non autonome. La lingua del contenuto segue quella del progetto; intestazioni di colonna e stati restano quelli indicati, perché lo script di verifica li legge.

### Numerazione degli ID

- **Numerazione per file.** Ogni file dei requisiti ha una numerazione propria, per tipo. Dentro il file gli ID si scrivono `REQ-NNN` e `BUG-NNN`, senza il nome della parte; un nuovo ID prende il numero successivo al più alto già presente nel file per quel tipo (`001` se non ce ne sono).
- **ID qualificato.** Fuori dal proprio file (oggetto dei commit, `CHANGELOG.md` della radice, rimandi da altri file dei requisiti) un ID di una parte si scrive `<parte>_REQ-NNN` o `<parte>_BUG-NNN`, dove `<parte>` è il nome della cartella che contiene `docs/` (es. `thin-decoder_REQ-001` per `thin-decoder/docs/REQUIREMENTS.md`, `search-rm3_REQ-001` per `search/search-rm3/docs/REQUIREMENTS.md`). Due parti non possono avere cartelle con lo stesso nome.
- **Progetto senza prefisso.** Gli ID di `docs/REQUIREMENTS.md` restano `REQ-NNN` e `BUG-NNN` ovunque. Un ID senza prefisso fuori da un file di parte indica sempre il progetto.
- **Nessuna rinumerazione.** Gli ID già registrati non si rinumerano, nemmeno quando un progetto passa a questa numerazione. Prima la numerazione era unica nel repository: gli ID esistenti nei file delle parti mantengono il loro numero, che diventa locale al file (es. `REQ-011` di `doc-ripper/docs/REQUIREMENTS.md` è `doc-ripper_REQ-011`), e i bug già registrati in `docs/REQUIREMENTS.md` restano dove sono.
- **ID storici.** Nei commit e nelle voci di `CHANGELOG.md` scritti con la numerazione unica, un ID senza prefisso indica l'elemento che aveva quell'ID in quel momento: quello di progetto se esiste, altrimenti quello della parte il cui file esisteva già. Commit e changelog passati non si riscrivono. Da quando un progetto adotta questa numerazione, gli ID delle parti si scrivono sempre qualificati.

### Modello di `docs/REQUIREMENTS.md`

```markdown
# Requisiti e tracciabilità

## Requisiti

| ID | Requisito | Stato | File impattati |
|----|-----------|-------|----------------|
| REQ-001 | Il sistema deve esportare il report in CSV. | implementato | `src/export.py`, `tests/test_export.py` |
| REQ-002 | Il sistema deve validare l'input prima dell'export. | aperto | — |

## Bug

| ID | Descrizione | Req | Stato | File impattati | Modifiche necessarie |
|----|-------------|-----|-------|----------------|----------------------|
| BUG-001 | L'export fallisce con campi vuoti: atteso campo vuoto nel CSV, ottenuto TypeError. | REQ-001 | risolto | `src/export.py` | Gestire `None` in `format_row`. |
```

Regole delle celle:

- **ID**: progressivo per tipo e per file, tre cifre, mai riusato né rinumerato; sempre senza prefisso dentro il proprio file.
- **Requisito**: una frase verificabile ("Il sistema deve …"). Il testo di un requisito registrato non si riscrive: se cambia, il vecchio passa ad `annullato` con la nota "sostituito da REQ-NNN" e se ne registra uno nuovo.
- **Stato requisito**: `aperto`, `implementato`, `annullato`.
- **Stato bug**: `aperto`, `risolto`.
- **Req** (solo bug): il requisito violato, oppure `—`. Senza prefisso se sta nello stesso file o nel progetto, qualificato se sta nel file di un'altra parte.
- **Rimandi nel testo**: come per Req, un ID di un altro file di parte si scrive qualificato.
- **File impattati**: percorsi con `/`, relativi alla radice del repository in `docs/REQUIREMENTS.md` e alla cartella della parte (quella che contiene `docs/`) in `<cartella>/docs/REQUIREMENTS.md`, così che la parte resti autonoma: `src/main.rs`, non `thin-decoder/src/main.rs`. Una parte che integra le altre e tocca file fuori dalla sua cartella dichiara nell'introduzione del suo file la riga `Percorsi dei file: dalla radice del repository.` e usa percorsi dalla radice; ciascuno tra backtick, separati da virgola; `—` se nessuno. Sono i file realmente toccati dai commit dell'ID (da `git diff --cached --name-only`), non quelli previsti. Non elencare `CHANGELOG.md`, né i file `REQUIREMENTS.md` toccati solo per aggiornare le tabelle; elenca un `REQUIREMENTS.md` solo quando l'ID ne corregge il testo (introduzione, note, appendici), ad esempio un bug in un rimando.
- Nessun carattere `|` dentro le celle.

### Modello di `CHANGELOG.md`

```markdown
# Changelog

## 2026-10-11
- REQ-002 richiesto — Validazione dell'input prima dell'export
- BUG-001 segnalato — Export fallisce con campi vuoti (`src/export.py`)
- BUG-001 risolto — Gestiti i valori None in format_row

## 2026-10-10
- REQ-001 richiesto — Export del report in CSV
- REQ-001 implementato — Aggiunto modulo di export (`src/export.py`, `tests/test_export.py`)
```

Data più recente in alto; dentro la stessa data le righe si aggiungono in fondo. Le parole chiave sono `richiesto` / `implementato` / `annullato` per i requisiti e `segnalato` / `risolto` per i bug. Nel `CHANGELOG.md` della radice gli ID delle parti non autonome sono qualificati (`- gui_REQ-001 richiesto — …`); nel `<cartella>/CHANGELOG.md` di una parte autonoma gli ID della parte sono senza prefisso (`- REQ-001 richiesto — …`) e i percorsi partono dalla cartella della parte.

## Commit

| Fase | Messaggio |
|---|---|
| `req` | `req(REQ-NNN): …` (più ID se registrati insieme: `req(REQ-003, REQ-004): …`) |
| `impl` | `feat(REQ-NNN): …` |
| `bugtext` | `bug(BUG-NNN): …` |
| `bugfix` | `fix(BUG-NNN): …` |
| riallineamento della sola tabella | `trace(REQ-NNN): …` |
| chiusura di una revisione di merito (`checkreq`) | `review: …`, commit vuoto senza ID |

Per un ID di una parte si usa l'ID qualificato: `feat(thin-decoder_REQ-001): …`, `fix(thin-decoder_BUG-002): …`.

- In `impl` e `bugfix`: un commit per ID, che contiene insieme codice, riga della tabella e riga del changelog.
- I commit di un ID di una parte autonoma (percorsi dalla propria cartella, vedi "File impattati") toccano solo file dentro la cartella della parte, compresi i suoi file di tracciabilità; le modifiche fuori dalla cartella vanno in un requisito o bug di progetto. `check_trace.py` lo controlla dal primo commit della parte che tocca il suo `CHANGELOG.md`.
- `git add` solo dei file di quel passo, per percorso esplicito.
- Niente `--amend`, rebase o force push; niente push se non richiesto. Si committa sul branch corrente.
- Se il progetto ha già una propria convenzione per i messaggi, rispettala ma mantieni l'ID nell'oggetto del messaggio: è l'ID che regge la tracciabilità.
- Gli ID del commit sono quelli nell'oggetto (la prima riga). Gli ID citati nel corpo sono solo rimandi e non attribuiscono il commit.
- Un commit senza ID che non si può più riscrivere (già pubblicato o con tag) si attribuisce a un ID con una voce nel changelog in cui stanno le voci di quell'ID, sotto la data del giorno: `- <hash> attribuito a REQ-NNN — <motivo> (<file>)`, in un commit `trace(REQ-NNN): …`.
