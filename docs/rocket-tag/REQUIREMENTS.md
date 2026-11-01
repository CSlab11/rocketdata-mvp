# Requisiti e tracciabilità — rocket-tag

Pacchetto Rust del progetto `rocketdata-mvp` che classifica i paragrafi di documenti RFQ/RFP e offerte dei fornitori del settore spaziale. Prende file Markdown già prodotti dagli strumenti precedenti, li scorre paragrafo per paragrafo e assegna a ogni paragrafo un'etichetta per faccetta (natura, dominio, destinazione, ecc.) con due classificatori indipendenti: uno a regole e uno basato su un LLM locale. Valuta anche le prestazioni dei classificatori contro etichette umane.

Requisiti del tool `rocket-tag`. I requisiti di progetto e la tabella dei bug sono in [`../REQUIREMENTS.md`](../REQUIREMENTS.md); gli ID `REQ-NNN` sono unici in tutto il repository.

Provengono da [`classifier-handoff/requirements.md`](../../classifier-handoff/requirements.md), che conserva gli ID locali originali: il REQ-n locale corrisponde qui a REQ-(n + 78). Le appendici A–G sono normative: i requisiti vi rimandano. L'appendice I elenca i file di riferimento, che si trovano in `classifier-handoff/reference/`.

## Confini

- **In ambito**: iteratore sui file Markdown, classificatore a regole, classificatore LLM, file di uscita, valutazione, manutenzione dello schema.
- **Fuori ambito** (strumenti precedenti o successivi): conversione PDF→Markdown, pulizia di intestazioni e piè di pagina, estrazione con Docling, visualizzazione, strumento di etichettatura, corrispondenza tra righe Markdown e posizione nel PDF.
- **Dipendenza esterna**: lo strumento di etichettatura deve produrre le etichette umane nel formato dell'appendice F (abbinamento per documento e righe Markdown).

## Ambito e separazione

| ID | Requisito | Stato | File impattati |
| --- | --- | --- | --- |
| REQ-079 | Il pacchetto deve essere implementato in Rust come parte del progetto `rocketdata-mvp`, in un crate dedicato compilabile ed eseguibile senza le altre parti della pipeline. | implementato | `rocket-tag/.gitignore`, `rocket-tag/Cargo.lock`, `rocket-tag/Cargo.toml`, `rocket-tag/src/lib.rs`, `rocket-tag/src/main.rs`, `rocket-tag/tests/cli.rs` |
| REQ-080 | Il pacchetto deve presupporre che i file Markdown esistano già e non deve convertire, pulire o estrarre documenti. | implementato | `rocket-tag/tests/cli.rs` |
| REQ-081 | Il pacchetto deve funzionare interamente in locale e non deve inviare il contenuto dei documenti a servizi diversi dal server LLM configurato dall'utente. | aperto | — |
| REQ-082 | Il classificatore a regole e il classificatore LLM devono essere componenti separati, eseguibili in modo indipendente, ciascuno con il proprio file di uscita. | aperto | — |
| REQ-083 | Il classificatore LLM non deve dipendere dal codice del classificatore a regole; può usarne l'uscita solo come file in ingresso facoltativo. | aperto | — |
| REQ-084 | L'iteratore deve essere separato dai classificatori: ciascun classificatore deve esporre una funzione di libreria che classifica una singola unità e un comando che classifica un testo passato da riga di comando. | implementato | `rocket-tag/src/main.rs`, `rocket-tag/tests/cli.rs`, `rocket-tag/tests/llm.rs` |

## Iteratore sul Markdown

| ID | Requisito | Stato | File impattati |
| --- | --- | --- | --- |
| REQ-085 | L'iteratore deve ricevere un file Markdown oppure una cartella, di cui legge ricorsivamente i file `*.md`, e chiamare il classificatore scelto per ogni unità. | implementato | `rocket-tag/Cargo.lock`, `rocket-tag/Cargo.toml`, `rocket-tag/src/iterate.rs`, `rocket-tag/src/lib.rs` |
| REQ-086 | L'iteratore deve scomporre il Markdown in unità (paragrafi, voci di elenco, righe dati di tabella) secondo l'appendice A, usare i titoli solo per percorso dei titoli e clausola, ed escludere blocchi di codice, immagini, commenti HTML, separatori e righe vuote. | implementato | `rocket-tag/Cargo.lock`, `rocket-tag/Cargo.toml`, `rocket-tag/src/lib.rs`, `rocket-tag/src/markdown.rs` |
| REQ-087 | Per ogni unità l'iteratore deve determinare la riga iniziale e la riga finale nel file Markdown, numerate da 1 e incluse. | implementato | `rocket-tag/src/markdown.rs` |
| REQ-088 | Per ogni unità l'iteratore deve passare al classificatore testo, testo dell'unità precedente e successiva dello stesso file, percorso dei titoli, clausola, tipo di unità, lato e tipo del documento. | implementato | `rocket-tag/src/iterate.rs` |
| REQ-089 | L'iteratore deve ricavare lato e tipo del documento dalle etichette documento del file di etichette umane (appendice F), se fornito, abbinandole per nome del documento; altrimenti dalla sezione `documents` di `rules.yaml` applicata al nome del documento; altrimenti deve lasciarli sconosciuti. | implementato | `rocket-tag/src/classifier.rs`, `rocket-tag/src/iterate.rs`, `rocket-tag/src/labels.rs`, `rocket-tag/src/lib.rs`, `rocket-tag/src/main.rs`, `rocket-tag/src/rules.rs`, `rocket-tag/tests/cli.rs` |
| REQ-090 | Il nome del documento deve essere il percorso del file Markdown relativo alla cartella di ingresso, con separatore `/`, oppure il nome del file quando l'ingresso è un singolo file. | implementato | `rocket-tag/src/iterate.rs` |
| REQ-128 | I comandi che classificano un file o una cartella devono accettare un'opzione `--exclude <percorso>`, ripetibile, con un percorso relativo alla cartella di ingresso: il file indicato, o la cartella indicata con tutto il suo contenuto, non viene letto dall'iteratore. Il pacchetto non deve contenere nomi di esclusioni predefiniti legati ad altri tool. | implementato | `rocket-tag/src/iterate.rs`, `rocket-tag/src/main.rs`, `rocket-tag/tests/cli.rs` |

## Ingresso dei classificatori

| ID | Requisito | Stato | File impattati |
| --- | --- | --- | --- |
| REQ-091 | Ciascun classificatore deve ricevere come input obbligatori lo schema delle etichette e il testo di una singola unità, e restituire le etichette di quell'unità senza richiedere il documento intero. | aperto | — |
| REQ-092 | Lo schema deve essere fornito come input (percorso del file o oggetto già caricato) e non letto da un percorso fisso, così che lo stesso file sia condiviso con lo strumento di etichettatura. | implementato | `rocket-tag/src/schema.rs` |
| REQ-093 | Ciascun classificatore deve accettare come input facoltativi le unità vicine e il contesto (percorso dei titoli, clausola, tipo di unità, lato e tipo del documento) e funzionare anche quando mancano. | aperto | — |
| REQ-094 | Il classificatore LLM deve includere nel prompt le unità vicine quando sono fornite. | implementato | `rocket-tag/src/llm.rs` |

## Schema delle etichette

| ID | Requisito | Stato | File impattati |
| --- | --- | --- | --- |
| REQ-095 | Il pacchetto deve ricavare dallo schema fornito (appendice B) faccette, valori ammessi, descrizioni, valori proposti (`x-default`) e vincoli per lato documento (`x-fixedWhen`), senza valori di faccetta scritti nel codice. | implementato | `rocket-tag/Cargo.lock`, `rocket-tag/Cargo.toml`, `rocket-tag/config/label_schema.json`, `rocket-tag/src/lib.rs`, `rocket-tag/src/schema.rs` |
| REQ-096 | Il pacchetto deve applicare i vincoli `x-fixedWhen` a ogni uscita (es. `required_action = none` per i documenti `supplier`). | implementato | `rocket-tag/config/guide_examples.json`, `rocket-tag/src/classifier.rs`, `rocket-tag/src/rules.rs`, `rocket-tag/tests/reference.rs` |
| REQ-097 | Una modifica dello schema non deve richiedere modifiche al codice del classificatore LLM, dell'iteratore né della valutazione. | aperto | — |
| REQ-098 | Il pacchetto deve controllare lo schema all'avvio e fermarsi con un messaggio che indica l'errore quando non è conforme all'appendice B. | implementato | `rocket-tag/src/schema.rs` |

## Classificatore a regole

| ID | Requisito | Stato | File impattati |
| --- | --- | --- | --- |
| REQ-099 | Il classificatore a regole deve leggere le regole da un file `rules.yaml` fornito come input (appendice C), come dati e non come codice. | implementato | `rocket-tag/Cargo.lock`, `rocket-tag/Cargo.toml`, `rocket-tag/config/rules.yaml`, `rocket-tag/src/lib.rs`, `rocket-tag/src/rules.rs` |
| REQ-100 | Le regole devono supportare le modalità `first` (vince la prima regola che corrisponde), `score` (vince il valore con più corrispondenze), `multi` (faccette a scelta multipla) e la mappatura da un'altra faccetta (`then_map`). | implementato | `rocket-tag/src/classifier.rs`, `rocket-tag/src/lib.rs`, `rocket-tag/src/rules.rs` |
| REQ-101 | Le regole devono poter porre condizioni sul testo (`any`, `all`, `unless`, `count_min`, `min_len`, `max_len`, `case`), sul contesto (`side`, `doc_type`, `kind`, `heading`) e sulle faccette già decise. | implementato | `rocket-tag/src/rules.rs` |
| REQ-102 | Le espressioni regolari di `rules.yaml` devono usare la sintassi del crate Rust `regex`; il controllo delle regole deve segnalare come errore ogni espressione non compilabile. | implementato | `rocket-tag/src/rules.rs` |
| REQ-103 | Il classificatore a regole deve offrire un controllo di `rules.yaml` contro lo schema che blocca l'esecuzione per faccette o valori inesistenti ed espressioni non valide, e segnala come avvisi i valori mai prodotti, i valori non mappati in `then_map` e le faccette senza regole. | implementato | `rocket-tag/src/main.rs`, `rocket-tag/src/rules.rs`, `rocket-tag/tests/cli.rs` |
| REQ-104 | Il classificatore a regole deve omettere una faccetta (astensione) quando nessuna regola si applica e non è definito un valore predefinito. | implementato | `rocket-tag/src/rules.rs` |
| REQ-105 | Il classificatore a regole deve registrare per ogni faccetta la regola che l'ha decisa (`rule_ids`) e una confidenza, secondo l'appendice C. | implementato | `rocket-tag/src/rules.rs` |

## Classificatore LLM

| ID | Requisito | Stato | File impattati |
| --- | --- | --- | --- |
| REQ-106 | Il classificatore LLM deve funzionare con qualsiasi server locale compatibile con l'API OpenAI chat completions, con URL, modello e chiave facoltativa configurabili. | implementato | `rocket-tag/Cargo.lock`, `rocket-tag/Cargo.toml`, `rocket-tag/src/classifier.rs`, `rocket-tag/src/iterate.rs`, `rocket-tag/src/llm.rs`, `rocket-tag/src/main.rs`, `rocket-tag/tests/llm.rs` |
| REQ-107 | Il classificatore LLM deve vincolare la risposta al formato delle faccette derivato dallo schema (appendice D), con modalità `json_schema`, `guided_json` o `json_object` selezionabile. | implementato | `rocket-tag/src/llm.rs` |
| REQ-108 | Il classificatore LLM deve costruire il prompt secondo l'appendice D da `prompt_guide.md`, dalle descrizioni dello schema e da `guide_examples.json`, ignorando negli esempi le faccette e i valori assenti dallo schema corrente. | implementato | `rocket-tag/config/prompt_guide.md`, `rocket-tag/src/lib.rs`, `rocket-tag/src/llm.rs` |
| REQ-109 | Il classificatore LLM deve validare ogni risposta contro lo schema, ritentare una volta in caso di risposta non valida e, al secondo fallimento, scartare l'unità registrandola tra le scartate con il motivo. | implementato | `rocket-tag/src/llm.rs`, `rocket-tag/src/schema.rs`, `rocket-tag/tests/llm.rs` |
| REQ-110 | L'esecuzione con il classificatore LLM deve poter riprendere dopo un'interruzione, saltando le unità già presenti nel file di uscita con la stessa versione dello schema, lo stesso `prompt_id` e lo stesso `text_hash`. | implementato | `rocket-tag/src/llm.rs`, `rocket-tag/src/main.rs`, `rocket-tag/tests/llm.rs` |
| REQ-111 | Il classificatore LLM deve fermarsi con un messaggio chiaro quando il server non risponde (5 errori di connessione senza alcun successo). | implementato | `rocket-tag/src/llm.rs`, `rocket-tag/src/main.rs`, `rocket-tag/tests/llm.rs` |
| REQ-112 | L'esecuzione con il classificatore LLM deve supportare un numero configurabile di richieste parallele e un limite al numero di unità. | implementato | `rocket-tag/src/llm.rs`, `rocket-tag/src/main.rs`, `rocket-tag/tests/llm.rs` |
| REQ-113 | Se riceve il file di uscita del classificatore a regole, il classificatore LLM deve assegnare a ogni faccetta confidenza 0,9 se concorda con le regole sulla stessa unità e 0,5 se non concorda; senza quel file, 0,7. | implementato | `rocket-tag/src/llm.rs`, `rocket-tag/src/main.rs`, `rocket-tag/tests/llm.rs` |
| REQ-132 | Il classificatore LLM deve leggere la chiave facoltativa del server (REQ-106) dalla variabile d'ambiente `ROCKET_TAG_API_KEY` e non deve richiederla come argomento da riga di comando. | implementato | `rocket-tag/src/main.rs`, `rocket-tag/tests/llm.rs` |

## Uscita

| ID | Requisito | Stato | File impattati |
| --- | --- | --- | --- |
| REQ-114 | Ogni esecuzione di un classificatore deve scrivere un file JSON nel formato dell'appendice E. | implementato | `rocket-tag/Cargo.lock`, `rocket-tag/Cargo.toml`, `rocket-tag/src/lib.rs`, `rocket-tag/src/main.rs`, `rocket-tag/src/output.rs`, `rocket-tag/tests/cli.rs` |
| REQ-115 | Ogni elemento dell'uscita deve contenere il nome del documento, la riga iniziale e la riga finale dell'unità nel file Markdown e le etichette assegnate, un valore per faccetta. | implementato | `rocket-tag/src/iterate.rs`, `rocket-tag/src/main.rs`, `rocket-tag/src/output.rs`, `rocket-tag/tests/cli.rs` |
| REQ-116 | Ogni elemento dell'uscita deve contenere anche tipo di unità, `text_hash`, confidenza per faccetta e confidenza dell'unità, pari al minimo delle confidenze per faccetta; il file deve riportare classificatore e versione dello schema. | implementato | `rocket-tag/Cargo.lock`, `rocket-tag/Cargo.toml`, `rocket-tag/src/markdown.rs`, `rocket-tag/src/output.rs`, `rocket-tag/tests/cli.rs` |
| REQ-117 | Il file di uscita deve restare un JSON valido anche se l'esecuzione si interrompe (scrittura su file temporaneo e rinomina). | implementato | `rocket-tag/src/output.rs` |
| REQ-129 | I comandi che classificano un file o una cartella devono scrivere su stdout una riga `[<unità elaborate>/<unità totali>] <documento>` per ogni unità elaborata, così che chi li esegue possa seguirne l'avanzamento. | implementato | `rocket-tag/src/iterate.rs`, `rocket-tag/src/main.rs`, `rocket-tag/tests/cli.rs` |
| REQ-130 | Ogni comando del pacchetto deve terminare con codice di uscita 0 se riesce e diverso da 0 in caso di errore, con il messaggio di errore su stderr; le unità scartate (REQ-109) non sono un errore del comando. | aperto | — |
| REQ-131 | Durante l'esecuzione il classificatore LLM deve aggiornare il file di uscita almeno ogni 20 unità elaborate, con la scrittura di REQ-117, così che dopo un'interruzione forzata del processo la ripresa (REQ-110) trovi le unità già classificate. | implementato | `rocket-tag/src/llm.rs`, `rocket-tag/src/main.rs`, `rocket-tag/tests/llm.rs` |

## Valutazione

| ID | Requisito | Stato | File impattati |
| --- | --- | --- | --- |
| REQ-118 | La valutazione deve abbinare un file di uscita e un file di etichette umane (appendice F) per nome del documento, riga iniziale e riga finale; se entrambi hanno `text_hash` e differiscono, l'etichetta deve essere esclusa e contata come non allineata. | aperto | — |
| REQ-119 | La valutazione deve calcolare per ogni faccetta accuratezza, macro-F1, copertura, precisione e richiamo per valore, confusioni più frequenti e accuratezza per fascia di confidenza (appendice G). | aperto | — |
| REQ-120 | La valutazione deve escludere e contare le etichette umane non valide per lo schema corrente. | aperto | — |
| REQ-121 | La valutazione deve produrre un report Markdown e aggiungere una riga di metriche a uno storico JSONL (appendice G). | aperto | — |

## Manutenzione dello schema

| ID | Requisito | Stato | File impattati |
| --- | --- | --- | --- |
| REQ-122 | Il pacchetto deve offrire un rapporto d'impatto di una modifica dello schema: differenze tra due versioni (faccette e valori aggiunti o rimossi, obbligatorietà), etichette umane da rivedere e incoerenze di `rules.yaml`. | aperto | — |
| REQ-123 | Il pacchetto deve offrire la migrazione di un file di etichette umane a una nuova versione dello schema con una mappa di rinomina di faccette e valori ed eliminazione di faccette; le etichette valide passano alla nuova versione, le altre restano alla vecchia. | aperto | — |

## Materiale di riferimento

| ID | Requisito | Stato | File impattati |
| --- | --- | --- | --- |
| REQ-124 | Il repository deve conservare in `classifier-handoff/` il pacchetto di passaggio del classificatore, senza modifiche: `requirements.md` e `CHANGELOG.md` con gli ID locali originali e, in `reference/`, i file di riferimento dell'appendice I. (sostituito da REQ-126) | annullato | — |
| REQ-126 | Il branch di lavoro di `rocket-tag` deve contenere `classifier-handoff/` finché i requisiti del tool non sono implementati; prima del merge in `main` la cartella deve essere rimossa, dopo aver portato in `rocket-tag/` i file di riferimento che servono al tool e aver tolto da questo documento i rimandi alla cartella, così che `main` non la contenga. (sostituisce REQ-124) | aperto | — |

---

## Appendice A – Scomposizione del Markdown

Ingresso: Markdown in stile CommonMark con tabelle GFM, come quello prodotto dagli strumenti precedenti (export Docling dopo la pulizia).

| Elemento | Trattamento |
| --- | --- |
| Titolo ATX (`#` … `######`) | Non è un'unità. Aggiorna il percorso dei titoli: rimuove i titoli di livello ≥ al nuovo e aggiunge il nuovo. |
| Paragrafo (righe consecutive non vuote) | Un'unità `paragraph`. Le righe sono unite con uno spazio. |
| Voce di elenco (`-`, `*`, `+`, `1.`, `1)`) | Un'unità `list_item` per voce, senza il marcatore; le righe di continuazione indentate appartengono alla voce. |
| Tabella GFM | La riga d'intestazione e il separatore `---` non sono unità; ogni riga dati è un'unità `table_row` con testo `intestazione: valore; intestazione: valore` (celle vuote omesse; senza intestazione solo i valori separati da `; `). |
| Blocco di codice, immagine (`![…](…)`), segnaposto `<!-- image -->`, commento HTML, separatore `---`, riga vuota | Esclusi. |

- **Righe**: `line_start` e `line_end` sono la prima e l'ultima riga del file occupate dall'unità, numerate da 1 e incluse. Per una riga di tabella coincidono.
- **Clausola**: il numero iniziale del titolo più vicino che ne ha uno (`4.7.4 Interfaces` → `4.7.4`; regex `^(\d+(?:\.\d+)*)\.?\s+\S`), altrimenti assente.
- **Unità vicine**: la precedente e la successiva nello stesso file, nell'ordine del file, di qualsiasi tipo.
- **text_hash**: primi 16 caratteri esadecimali dello SHA-1 del testo dell'unità con spazi consecutivi ridotti a uno e senza spazi iniziali e finali.

Esempio:

```markdown
1  # 4 Verification
2
3  ## 4.2 Thermal tests
4
5  Thermal cycling will be performed on each flight unit,
6  with 8 cycles between -30 °C and +60 °C.
7
8  - Unit level: 8 cycles
9  - Board level: 
10   100 cycles
11
12 | Test | Level |
13 | --- | --- |
14 | TVAC | Unit |
```

| line_start | line_end | kind | text | heading_path | clause |
| --- | --- | --- | --- | --- | --- |
| 5 | 6 | paragraph | Thermal cycling will be performed on each flight unit, with 8 cycles between -30 °C and +60 °C. | 4 Verification › 4.2 Thermal tests | 4.2 |
| 8 | 8 | list_item | Unit level: 8 cycles | idem | 4.2 |
| 9 | 10 | list_item | Board level: 100 cycles | idem | 4.2 |
| 14 | 14 | table_row | Test: TVAC; Level: Unit | idem | 4.2 |

## Appendice B – Schema delle etichette (`label_schema.json`)

JSON Schema draft 2020-12. Il file attuale (v0.1.0) è in appendice I. Struttura usata dal pacchetto:

- `version`: stringa, versione dello schema, riportata in ogni uscita.
- `$defs.ParagraphFacets.properties`: le faccette, nell'ordine in cui vanno valutate e mostrate.
  - Faccetta a scelta singola: `{"$ref": "#/$defs/<Enum>"}`.
  - Faccetta a scelta multipla: `{"type": "array", "items": {"$ref": "#/$defs/<Enum>"}, "uniqueItems": true}`.
  - `x-default` (facoltativo): valore proposto (stringa, o lista per le multiple).
  - `x-fixedWhen` (facoltativo): `{"side": "<lato>", "value": "<valore>"}`, valore obbligato quando il documento ha quel lato.
- `$defs.ParagraphFacets.required`: faccette obbligatorie.
- `$defs.<Enum>`: `{"type": "string", "enum": [...], "x-enumDescriptions": {"<valore>": "<descrizione>"}}`.
- `$defs.DocumentLabel.properties.side` e `.doc_type`: `$ref` agli enum dei lati e dei tipi di documento.

Faccette attuali: `nature`, `domain`, `destination`, `required_action` (fissa a `none` per `supplier`), `extraction_quality` (default `ok`), `flags` (multipla, default `[]`).

Errori da segnalare all'avvio (REQ-098): JSON non valido, `ParagraphFacets` mancante, faccetta che non punta a un enum, `x-fixedWhen` con lato o valore non ammessi, `x-default` con valore non ammesso.

## Appendice C – Regole (`rules.yaml`)

```yaml
version: "0.1.0"
documents:                 # lato e tipo dal nome del documento (prima corrispondenza)
  - {match: '^\[TD\]', side: supplier, doc_type: technical_description}
facets:                    # valutate in quest'ordine
  <faccetta>:
    mode: first | score | multi        # default first
    rules:
      - id: <nome>                     # per rule_ids; default "<faccetta>#<indice>"
        value: <valore>
        any: [<regex>, ...]            # almeno una corrisponde (con count_min: occorrenze totali ≥ N)
        all: [<regex>, ...]            # tutte corrispondono
        unless: [<regex>, ...]         # nessuna corrisponde
        count_min: 2
        min_len: 10                    # lunghezza del testo in caratteri
        max_len: 200
        case: true                     # distingue maiuscole (default: no)
        when:                          # tutte le condizioni devono valere
          side: [prime, null]          # null = lato sconosciuto
          doc_type: [...]
          kind: [paragraph, list_item, table_row]
          heading: [<regex>, ...]      # sul percorso dei titoli unito con " › ", senza distinzione maiuscole
          <faccetta già decisa>: [<valori>]
        confidence: 0.7
    then_map:                          # solo mode first: se nessuna regola corrisponde
      from: <faccetta già decisa>
      map: {<valore>: <valore>}
      confidence: 0.6
    default: <valore> | null           # null = astensione
    default_confidence: 0.4
```

Semantica e confidenza:

- **first**: la prima regola con `when` e testo soddisfatti decide; confidenza = `confidence` della regola (default 0,5); altrimenti `then_map`, poi `default`.
- **score**: per ogni regola con `when` soddisfatto si sommano le occorrenze di `any` per valore; vince il punteggio più alto, a parità il valore che compare prima nelle regole; confidenza = 0,4 + 0,5 × punteggio vincente ÷ punteggio totale (arrotondata a 2 decimali); senza corrispondenze, `default`.
- **multi**: l'uscita è la lista ordinata dei valori di tutte le regole soddisfatte; confidenza = minima delle regole soddisfatte, o `default_confidence` se nessuna.
- Dopo le regole si applica `x-fixedWhen` dello schema con confidenza 1,0 e `rule_ids` = `schema x-fixedWhen`.
- `rule_ids`: id della regola, `score {valore: punteggio, …}`, `map <faccetta>`, `default` o `schema x-fixedWhen`; per `multi`, lista di id.
- Nota di migrazione: il `rules.yaml` attuale (appendice I) usa `kind: [text]` e `docling_label`, che vanno sostituiti con i tipi di unità di questa specifica.

## Appendice D – Prompt e richiesta LLM

**Messaggio di sistema**, nell'ordine:
1. il contenuto di `prompt_guide.md`;
2. `Facets and allowed values:` seguito, per ogni faccetta, da `<nome> (one of|list of):` e da una riga `  - <valore>: <descrizione>` per valore;
3. `Examples (side | text -> labels):` seguito da una riga `  [<side>] <text> -> <faccetta>=<valore>, …` per ogni esempio di `guide_examples.json`, tenendo solo coppie faccetta/valore presenti nello schema;
4. `Answer with a single JSON object containing exactly these keys: <faccette separate da virgola>.`

**Messaggio utente**, una riga per voce:
```
Document: <nome documento>
Document side: <side|unknown>; type: <doc_type|unknown>
Heading path: <titoli uniti con " > " | (none)>
Unit kind: <kind>
Previous unit (context): <primi 400 caratteri>        (solo se presente)
UNIT TO LABEL: <primi 2500 caratteri>
Next unit (context): <primi 400 caratteri>            (solo se presente)
```

**Formato di uscita vincolato**: oggetto con una proprietà per faccetta; singola → `{"type": "string", "enum": [...]}`; multipla → `{"type": "array", "items": {"type": "string", "enum": [...]}, "uniqueItems": true}`; `required` = faccette obbligatorie; `additionalProperties: false`. Gli enum sono inseriti in linea (niente `$ref`).

- `json_schema`: `response_format = {"type": "json_schema", "json_schema": {"name": "facets", "strict": true, "schema": <formato>}}`
- `guided_json`: campo `guided_json = <formato>`
- `json_object`: `response_format = {"type": "json_object"}`; la validazione avviene solo dopo.

`temperature` configurabile, default 0. Prima della validazione si applicano `x-fixedWhen` e il default `[]` per le faccette multiple mancanti. `prompt_id` = primi 10 caratteri esadecimali dello SHA-1 di messaggio di sistema + formato di uscita serializzato + nome del modello.

## Appendice E – File di uscita

```json
{
  "schema_version": "0.1.0",
  "classifier": "rules-v0.1.0",
  "created_at": "2026-11-02T10:00:00Z",
  "prompt_id": "f681ea8fcf",
  "items": [
    {
      "document": "pa_plan.md",
      "line_start": 5,
      "line_end": 6,
      "kind": "paragraph",
      "text_hash": "e3e95f58db330ac8",
      "labels": {"nature": "commitment", "domain": "verification", "destination": "P",
                 "required_action": "none", "extraction_quality": "ok", "flags": []},
      "confidence": {"nature": 0.65, "domain": 0.73, "destination": 0.6,
                     "required_action": 1.0, "extraction_quality": 0.7, "flags": 0.6},
      "unit_confidence": 0.6,
      "rule_ids": {"nature": "commit-will", "domain": "score {verification: 3, technical: 1}"}
    }
  ],
  "skipped": [
    {"document": "pa_plan.md", "line_start": 40, "line_end": 41, "error": "domain: valore non ammesso"}
  ]
}
```

- `classifier`: `rules-v<versione di rules.yaml>` oppure `llm:<modello>`.
- `prompt_id`: solo per l'LLM. `rule_ids`: solo per le regole.
- Le faccette in astensione sono assenti sia da `labels` sia da `confidence`.
- Ordine degli elementi: per documento, poi per `line_start`.

## Appendice F – Etichette umane (ingresso della valutazione)

File JSONL, una riga per etichetta; se la stessa unità o lo stesso documento compare più volte vale l'ultima riga.

```json
{"level": "document", "document": "pa_plan.md", "side": "supplier", "doc_type": "pa_plan",
 "provenance": {"source": "human", "annotator": "cs", "timestamp": "…", "schema_version": "0.1.0"}}
{"level": "paragraph", "document": "pa_plan.md", "line_start": 5, "line_end": 6, "text_hash": "e3e95f58db330ac8",
 "facets": {"nature": "commitment", "domain": "verification", "destination": "P",
            "required_action": "none", "extraction_quality": "ok", "flags": []},
 "provenance": {"source": "human", "annotator": "cs", "timestamp": "…", "schema_version": "0.1.0", "note": "…"}}
```

Un'etichetta di paragrafo è valida se ogni faccetta obbligatoria è presente con un valore ammesso, non ci sono faccette in più e i vincoli `x-fixedWhen` sono rispettati per il lato del documento.

Mappa di migrazione (REQ-123): `{"rename_facets": {"vecchio": "nuovo"}, "values": {"<faccetta>": {"vecchio": "nuovo"}}, "drop_facets": ["<faccetta>"]}`. Alle faccette nuove con `x-default` si assegna il default; alle etichette migrate e valide si aggiorna `schema_version` e si aggiunge `migrated from v<vecchia>` alla nota.

## Appendice G – Metriche

- **Accuratezza**: corrette ÷ etichette abbinate; l'astensione conta come errore.
- **Copertura**: unità con la faccetta presente ÷ etichette abbinate.
- **Precisione, richiamo, F1** per valore; **macro-F1** = media delle F1 dei valori con supporto > 0. Per le faccette multiple: accuratezza = corrispondenza esatta degli insiemi, precisione e richiamo per valore.
- **Confusioni**: le 8 coppie (umano → predetto) più frequenti.
- **Fasce di confidenza**: [0; 0,5), [0,5; 0,7), [0,7; 0,85), [0,85; 1], con numero di unità e accuratezza.
- **Report Markdown**: tabella riassuntiva per faccetta, poi per ogni faccetta tabella per valore, confusioni e fasce.
- **Storico JSONL**, una riga per esecuzione: `{"timestamp", "schema_version", "classifier", "n_eval", "facets": {"<faccetta>": {"accuracy", "macro_f1", "coverage"}}}`.

## Appendice H – Comandi (indicativi)

Un binario con sottocomandi; nomi e opzioni sono indicativi, il comportamento è definito dai requisiti.

```
classify rules  <md|cartella> -o out.json --schema S --rules R [--labels L]
classify llm    <md|cartella> -o out.json --schema S --base-url U --model M [--rules-out out_rules.json]
                [--labels L] [--format json_schema|guided_json|json_object] [--workers N] [--limit N]
classify one    --text "…" --schema S (--rules R | --base-url U --model M) [--side supplier] [--prev "…"] [--next "…"]
rules check     --schema S --rules R
evaluate        <etichette.jsonl> <out.json> --schema S [--report r.md] [--metrics m.jsonl]
schema check|diff|status|impact|migrate …
```

## Appendice I – File di riferimento

Da copiare nel progetto e usare come riferimento di comportamento:

| File | Uso |
| --- | --- |
| `label_schema.json` | Schema v0.1.0 (appendice B). |
| `rules.yaml` | Regole attuali (appendice C), da adattare ai tipi di unità. |
| `prompt_guide.md` | Regole di decisione per l'LLM. |
| `guide_examples.json` | 13 esempi inventati: `[{"side", "text", "labels": {…}}]`. |
| `classify_rules.py`, `classify_llm.py`, `evaluate.py`, `schema_tool.py`, `schema_util.py` | Implementazione Python di riferimento (lavora su JSONL estratti dal PDF invece che su Markdown). |

Casi di prova di riferimento: con le regole attuali adattate, i 13 esempi di `guide_examples.json` devono dare 13/13 su `nature`, `domain`, `destination` e `required_action`; l'esempio dell'appendice A deve produrre esattamente le 4 unità indicate.
