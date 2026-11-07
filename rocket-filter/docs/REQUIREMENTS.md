# Requisiti e tracciabilità — rocket-filter

Tool Python del progetto `rocketdata-mvp` che indicizza in locale gli standard ECSS attivi in un archivio di embeddings organizzato per categorie (rami, discipline, categorie trasversali, temi generici). Sull'archivio permette di cercare termini e unità ECSS correlati a un argomento, di estrarre il lessico di una categoria e di classificare i paragrafi di altri documenti rispetto alle categorie.

Requisiti del tool `rocket-filter`. I requisiti di progetto e la tabella dei bug di progetto sono in [`docs/REQUIREMENTS.md`](../../docs/REQUIREMENTS.md). Gli ID di questo file sono locali: fuori da qui si scrivono `rocket-filter_REQ-NNN`. I percorsi dei file sono relativi alla cartella `rocket-filter/`.

## Confini

- **In ambito**: scaricamento del modello di embedding, selezione e segmentazione dei PDF ECSS, indice locale (SQLite e vettori), temi, prototipi, vocabolario di parole singole, interrogazione dell'indice, classificazione di paragrafi.
- **Fuori ambito**: scaricamento dei PDF degli standard, conversione di documenti in Markdown, OCR, addestramento di classificatori.

## Ambito e configurazione

| ID | Requisito | Stato | File impattati |
|----|-----------|-------|----------------|
| REQ-001 | `rocket-filter` deve essere un tool Python autonomo nella cartella `rocket-filter/`, con le dipendenze elencate in `requirements.txt`, senza dipendere da codice o file di altri tool; i dati generati (`index/`, `models/`, `__pycache__/`) devono essere esclusi da git. | aperto | — |
| REQ-002 | `setup_model.py` deve scaricare una sola volta il modello di embedding indicato in `config.yaml` (`modello.nome_hf`) e salvarlo nella cartella locale `modello.cartella`; è l'unico passaggio che richiede la rete. | aperto | — |
| REQ-003 | Gli altri script devono lavorare offline, caricando il modello solo dalla cartella locale senza chiamate di rete; se la cartella del modello non esiste devono terminare con un messaggio che indica di eseguire `python setup_model.py`. | aperto | — |
| REQ-004 | Il tool deve leggere da `config.yaml` il modello (nome, cartella, prefissi di query e di documento, dimensione del batch, numero massimo di token), la cartella dell'indice, il file delle categorie e i parametri di segmentazione, prototipi, temi, vocabolario e classificazione. | aperto | — |
| REQ-005 | Il tool deve leggere da `categorie.yaml` l'albero delle categorie (id, nome, `parent`, standard con codice e titolo), le categorie trasversali (che includono categorie intere, standard o clausole di uno standard) e i temi generici (id, nome, descrizione, semi e ancore facoltative). | aperto | — |
| REQ-006 | Ogni categoria dell'albero deve appartenere a un gruppo: `ramo` se non ha `parent`, `disciplina` se ha standard propri, `nodo` altrimenti; i membri di una categoria sono i suoi standard più quelli delle categorie discendenti. | aperto | — |
| REQ-007 | Con la variabile d'ambiente `ECSS_TEST_EMBEDDER=1` il tool deve usare un embedder fittizio a bag of words al posto del modello, per collaudare la pipeline senza il modello scaricato. | aperto | — |

## Ingestione degli standard

| ID | Requisito | Stato | File impattati |
|----|-----------|-------|----------------|
| REQ-008 | `ingest.py --pdf <cartella>` deve cercare nella cartella, per ogni standard di `categorie.yaml`, il PDF della versione attiva confrontando codice base, lettera di issue e revisione (anche quando la revisione è seguita da una data, es. `Rev.115November2008` = Rev.1), scartando bozze (DIR, DFR, draft, salvo standard dichiarati Draft), handbook, TM e materiale di training, e deve elencare a fine esecuzione gli standard senza PDF corrispondente. | aperto | — |
| REQ-009 | `ingest.py` deve segmentare ogni PDF in unità di tipo `definizione`, `abbreviazione`, `requisito`, `testo` e `drd`, conservando per ciascuna standard, file, clausola, percorso dei titoli, pagina, termine (per definizioni e abbreviazioni) e ID del requisito `ECSS-…_nnnnnnn` quando presente. | aperto | — |
| REQ-010 | La segmentazione deve scartare intestazioni e piè di pagina (righe nell'8% superiore e nel 7% inferiore della pagina), front matter, indice e i capitoli di tailoring, tracciabilità, bibliografia, change log e riferimenti normativi. | aperto | — |
| REQ-011 | Le unità più lunghe di `max_caratteri_unita` devono essere spezzate sui confini di frase e, dove manca la punteggiatura, sugli spazi tra le parole. | aperto | — |
| REQ-012 | `ingest.py` deve calcolare l'embedding di ogni unità con il prefisso di documento, anteponendo il percorso dei titoli al testo di requisiti, testo e DRD, e salvare i vettori in `index/emb/<standard>.npy` e testi e metadati in `index/ecss.sqlite`. | aperto | — |
| REQ-013 | L'ingestione deve essere incrementale: uno standard viene rielaborato solo se cambia l'impronta SHA-1 del PDF o il modello, o se manca il suo file di vettori; `--forza` rielabora tutti gli standard e `--solo "<codici>"` rielabora quelli indicati, terminando con errore se un codice non è in `categorie.yaml`. | aperto | — |
| REQ-014 | `ingest.py` deve togliere dall'indice unità, file e vettori degli standard non più presenti in `categorie.yaml`. | aperto | — |
| REQ-015 | Se un PDF non si riesce a leggere o non produce unità, `ingest.py` deve segnalarlo e proseguire con gli standard successivi. | aperto | — |
| REQ-016 | A ogni esecuzione `ingest.py` deve rigenerare temi, prototipi e vocabolario; con `--solo-prototipi` deve rigenerare solo questa parte, senza leggere i PDF. | aperto | — |

## Temi, prototipi e vocabolario

| ID | Requisito | Stato | File impattati |
|----|-----------|-------|----------------|
| REQ-017 | I membri di ogni tema generico devono essere tutte le unità delle sue ancore (origine `ancora`) più le `temi.unita_per_tema` unità dei tipi `temi.tipi` più vicine alla descrizione e ai semi, con un bonus `temi.bonus_lessicale` se un seme compare alla lettera nel percorso dei titoli o nel termine (origine `lessicale`, altrimenti `semantica`). | aperto | — |
| REQ-018 | Per ogni categoria con unità indicizzate il tool deve calcolare con k-means sferico sulle sue unità (abbreviazioni escluse) da 1 a `prototipi.max_per_categoria` prototipi, con k = numero di unità / `prototipi.unita_per_prototipo` (almeno 1), e salvarli in `index/prototipi.npz`. | aperto | — |
| REQ-019 | Il tool deve registrare nell'indice la descrizione di ogni categoria: la descrizione del tema per i temi, il testo dello Scope (clausola 1) dei primi cinque standard membri per le altre. | aperto | — |
| REQ-020 | Il tool deve costruire un vocabolario di parole singole da tre fonti (termini definiti, titoli di clausole e annex, sigle), scartando parole vuote, parole di servizio della struttura ECSS, parole di meno di 3 caratteri e numeri, riducendo i plurali al singolare e tenendo una parola presente solo nei titoli se compare in almeno `vocabolario.min_occorrenze_titoli` titoli distinti. | aperto | — |
| REQ-021 | Sono sigle solo le abbreviazioni di 2–8 caratteri maiuscoli o cifre con espansione di almeno 6 caratteri; le parole maiuscole che non sono sigle note devono essere trattate come parole normali. | aperto | — |
| REQ-022 | Il vettore di ogni parola deve combinare, con peso `vocabolario.peso_parola`, l'embedding della parola da sola e la media di al più `vocabolario.max_contesti` unità in cui compare (campione riproducibile); per le sigle il contesto comprende anche le clausole il cui titolo contiene l'espansione. I vettori vanno in `index/parole.npy`. | aperto | — |
| REQ-023 | Per ogni parola e ogni categoria di ciascun livello il tool deve calcolare occorrenze, concentrazione smussata con `vocabolario.smussamento`, specificità semantica (score nella categoria meno il migliore score tra le categorie dello stesso livello che non si sovrappongono) e punteggio = concentrazione + `vocabolario.peso_semantico` × specificità normalizzata. | aperto | — |

## Interrogazione (`query.py`)

| ID | Requisito | Stato | File impattati |
|----|-----------|-------|----------------|
| REQ-024 | `query.py categorie` deve mostrare l'albero delle categorie con gruppo e numero di unità di ciascuna. | aperto | — |
| REQ-025 | `query.py termini "<testo>"` deve restituire le `--k` parole del vocabolario più vicine al testo (predefinito 20), con filtri facoltativi `--categoria` (solo parole che compaiono nella categoria) e `--fonte` (`definizione`, `titolo` o `sigla`). | aperto | — |
| REQ-026 | `query.py lessico --categoria <id>` deve restituire le `--n` parole della categoria con punteggio più alto (predefinito 50), con concentrazione, occorrenze, specificità, fonti ed esempi, e filtri `--min-occorrenze` (predefinito 2), `--min-specificita` e `--fonte`. | aperto | — |
| REQ-027 | `query.py membri --categoria <tema>` deve restituire le `--n` unità del tema con score più alto (predefinito 40) e il motivo di appartenenza, con filtri `--tipo` e `--origine` (`ancora`, `lessicale` o `semantica`), e terminare con errore se la categoria non è un tema generico. | aperto | — |
| REQ-028 | `query.py unita "<testo>"` deve restituire le `--k` unità più vicine al testo (predefinito 10) tra i tipi `--tipo` (predefinito requisito, testo, drd, definizione), con filtro facoltativo `--categoria`, mostrando score, tipo, standard e ID del requisito o clausola. | aperto | — |
| REQ-029 | I comandi `termini`, `unita`, `lessico` e `membri` devono accettare `--json` per un'uscita strutturata; una categoria sconosciuta deve terminare con un errore che rimanda a `python query.py categorie`. | aperto | — |
| REQ-030 | Se mancano i prototipi o il vocabolario, `query.py` e `classify.py` devono terminare con un messaggio che indica di eseguire `ingest.py`. | aperto | — |

## Classificazione (`classify.py`)

| ID | Requisito | Stato | File impattati |
|----|-----------|-------|----------------|
| REQ-031 | `classify.py` deve accettare uno o più file PDF, DOCX, TXT o MD e un testo passato con `--testo`, ed estrarne i paragrafi (PDF: blocchi di testo con numero di pagina; DOCX: paragrafi, con `python-docx`; TXT e MD: blocchi separati da righe vuote), scartando quelli più corti di `classificazione.min_caratteri_paragrafo`; senza paragrafi deve terminare con errore. | aperto | — |
| REQ-032 | `classify.py` deve conservare in `index/ecss.sqlite` gli embeddings dei paragrafi, con chiave l'impronta SHA-1 di modello e testo, e ricalcolare solo i paragrafi non ancora visti. | aperto | — |
| REQ-033 | `classify.py` deve classificare sui livelli scelti con `--livelli` tra `temi`, `rami`, `discipline` e `trasversali` (separati da virgola, oppure `tutti`; predefinito `classificazione.livelli_attivi`) e terminare con errore su un livello non valido. | aperto | — |
| REQ-034 | Lo score di un paragrafo verso una categoria deve essere il massimo coseno con i prototipi della categoria; per ogni livello, in modo indipendente, il paragrafo riceve le categorie, tra le prime `max_categorie`, con score non inferiore alla `soglia` e alla migliore meno il `margine`, ed è non classificato se nessuna supera la soglia; il risultato conserva le cinque categorie migliori e segnala come ambiguo un livello con un numero di categorie diverso da uno. | aperto | — |
| REQ-035 | `--soglia` e `--margine` devono sovrascrivere i valori di `config.yaml` per tutti i livelli. | aperto | — |
| REQ-036 | Per ogni paragrafo `classify.py` deve indicare le `classificazione.termini_supporto` parole del vocabolario più vicine al paragrafo. | aperto | — |
| REQ-037 | Con `--out <file>.jsonl` `classify.py` deve scrivere un oggetto JSON per paragrafo; con un'altra estensione un CSV UTF-8 con BOM e separatore `;`, con colonne `file`, `pagina`, per ogni livello `<livello>`, `<livello>_score` e `<livello>_top3`, poi `termini` e `testo`. | aperto | — |
| REQ-038 | `classify.py` deve mostrare a video il dettaglio di ogni paragrafo quando manca `--out` o i paragrafi sono al più 20, e sempre un riepilogo per livello con paragrafi classificati, sotto soglia, soglia e margine. | aperto | — |

## Documentazione

| ID | Requisito | Stato | File impattati |
|----|-----------|-------|----------------|
| REQ-039 | `README.md` deve descrivere installazione, scaricamento del modello, costruzione dell'indice, comandi di interrogazione e di classificazione, calibrazione delle soglie, definizione dei temi e cambio del modello. | aperto | — |
