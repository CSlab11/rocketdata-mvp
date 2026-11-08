# ECSS embeddings – ricerca di termini e classificazione di paragrafi (offline)

Il sistema indicizza i 144 standard ECSS attivi in un archivio locale di embeddings,
organizzato secondo le categorie di `categorie.yaml`. Le categorie sono di due tipi:

- **strutturali**, che seguono l'albero ECSS: rami (M, E, Q…), discipline (M-60, E-40…) e trasversali (T-COSTI…);
- **temi generici**, per argomento e trasversali a tutti gli standard: cost, customer, procurement, risk, quality…
Su questo archivio permette di:

- trovare i **termini ECSS correlati** a un argomento;
- estrarre il **lessico specifico** di una categoria;
- **classificare paragrafi** di altri documenti rispetto alle categorie.

Tutto gira in locale. Internet serve solo una volta, per installare i pacchetti e scaricare il modello.

---

## Panoramica

Il sistema ha due fasi:

```
PREPARAZIONE (una volta)                       USO (ogni volta che serve)
────────────────────────                       ──────────────────────────
setup_model.py   scarica il modello            query.py     termini e lessici
      │                                        classify.py  classifica paragrafi
ingest.py        PDF ECSS → index/  ──────────►     (leggono l'indice)
```

**L'idea di base.** Il modello di embedding trasforma un testo in un vettore di 768 numeri.
Testi con significato simile producono vettori vicini, anche se usano parole diverse o lingue diverse
("cost estimate" ≈ "stima dei costi"). Tutto il sistema confronta vettori:

- *query*: trova le unità ECSS più vicine alla domanda;
- *classificazione*: trova le categorie più vicine al paragrafo.

---

## Installazione (una tantum, online)

```bat
cd rocket-filter
python -m venv .venv
.venv\Scripts\activate
pip install torch --index-url https://download.pytorch.org/whl/cpu
pip install -r requirements.txt
python setup_model.py
```

La riga `torch ... /cpu` è facoltativa: installa la versione solo-CPU, molto più leggera.
Se si dispone di una GPU NVIDIA, la si può saltare.

---

## 1. `setup_model.py` – scaricare il modello

Scarica il modello di embedding (`intfloat/multilingual-e5-base`) e lo salva in `models/`.
È l'unico passaggio che richiede internet. Da qui in poi gli script caricano il modello
dalla cartella locale e non effettuano alcuna chiamata di rete.

---

## 2. `ingest.py` – costruire l'indice

```bat
python ingest.py --pdf "C:\Users\anbon\Documents\ecss_crawler\ecss_docs\standards\active"
```

`ingest.py` svolge due lavori diversi, uno dopo l'altro.

| Parte | Passaggi | Cosa produce | Durata | Quando va rifatta |
|---|---|---|---|---|
| **1. Lettura degli standard** (pesante) | a) b) c) | il *contenuto* dell'indice: circa 46.000 unità con i loro vettori | 15–60 min la prima volta | solo per PDF nuovi o modificati, o se cambia il modello |
| **2. Organizzazione per categorie** (leggera) | d) e) | temi, prototipi, vocabolario di parole singole, lessici | pochi minuti | ogni volta che cambiano `categorie.yaml` o gli script |

Un'analogia: la parte 1 è digitalizzare i libri di una biblioteca, la parte 2 è decidere scaffali
ed etichette. Riorganizzare gli scaffali non richiede di riscansionare i libri.

La parte 2 viene sempre eseguita. La parte 1 viene eseguita solo dove serve:
lo script riconosce da solo i PDF già indicizzati e invariati, e li salta.

### a) Selezione dei PDF
Legge l'elenco degli standard da `categorie.yaml` e, per ognuno, cerca il PDF della **versione attiva**
nella cartella. I nomi dei file sono molto variabili: ad esempio
`ECSS-E-ST-32C_Rev.115November2008.pdf` è la Rev.1 del 15 novembre 2008.
Vengono scartati bozze (DIR, DFR), handbook, TM, slide di training, vecchie revisioni e PSS.

### b) Segmentazione in unità
Ogni PDF viene letto riga per riga, tenendo conto di font, dimensione e posizione,
e diviso in pezzi di significato compiuto:

| Tipo | Come viene riconosciuto | Esempio |
|---|---|---|
| definizione | titoli numerati nella clausola 3 | *critical path: chain of activities critical to…* |
| abbreviazione | tabella sigla / significato | *CBS: cost breakdown structure* |
| requisito | ID `ECSS-…_nnnnnnn` o lettera `a.`, `b.` | *The supplier shall…* |
| testo | testo descrittivo di una clausola | principi, spiegazioni |
| drd | testo negli annex DRD | contenuto atteso di un documento |

Vengono scartati intestazioni e piè di pagina, indice, riferimenti normativi e tabelle di tailoring.
Ogni unità conserva standard, clausola, percorso dei titoli, pagina e, se presente, l'ID del requisito.
Sui 144 standard si ottengono circa 46.000 unità.

### c) Embedding
Ogni unità viene trasformata in un vettore. Al testo viene anteposto il percorso dei titoli
(es. *Cost management requirements › Cost estimating: …*), così il modello capisce il contesto.

- I vettori vanno in `index/emb/`, un file per standard.
- Testi e metadati vanno in `index/ecss.sqlite`.

L'indicizzazione è **incrementale**: rilanciando lo script, vengono rielaborati solo
i PDF nuovi o modificati, e tutti se si cambia modello.

### d) Temi e prototipi
- **Temi generici.** Ogni tema è definito in `categorie.yaml` da una descrizione, alcuni termini
  seme (es. *procurement*: purchase order, supplier selection, lot acceptance…) e, facoltativamente,
  delle **ancore**: standard o clausole dedicati proprio a quell'argomento (es. *cost* → capitoli 6 e 9
  e annex di costo della M-ST-60C). I membri del tema sono:
  1. **ancora**: tutte le unità delle ancore, che forniscono un nucleo affidabile;
  2. **lessicale / semantica**: le 300 unità più vicine ai semi, prese da qualunque standard
     (`temi.unita_per_tema`). Se un seme compare alla lettera nel titolo della clausola o nel termine,
     l'unità riceve un bonus (`temi.bonus_lessicale`).

  Così *cost* ha il nucleo della M-ST-60C e in più raccoglie i paragrafi sui costi di Q-60, I-30, E-10…
  Con `python query.py membri --categoria cost --origine semantica` si controllano i membri meno sicuri.
- **Prototipi.** Per ogni categoria, i vettori delle sue unità vengono riassunti in 1–5
  "vettori rappresentativi" con k-means. Se ne usa più di uno perché una categoria
  (es. software) ha anime diverse: sviluppo, test, documentazione.
  Vengono salvati in `index/prototipi.npz`.

### e) Vocabolario di parole singole
I lessici e i termini restituiti dal sistema sono **parole singole** (es. *cost*, *procurement*,
*baseline*, *FMECA*), estratte da tre fonti:

| Fonte | Da dove | Esempio |
|---|---|---|
| definizione | parole dei termini definiti (glossario e clausola 3 di ogni standard) | *cost reimbursement contract* → cost, reimbursement, contract |
| titolo | parole dei titoli di clausole e annex | *Cost estimating and planning* → cost, estimating, planning |
| sigla | abbreviazioni maiuscole | CBS, EAC, FMECA |

Parole vuote (*the*, *of*…) e parole di servizio della struttura ECSS (*general*, *annex*,
*requirements*, *scope*…) vengono scartate; i plurali vengono ridotti al singolare.
Una parola presente solo nei titoli deve comparire in almeno 2 titoli distinti
(`vocabolario.min_occorrenze_titoli`).

Il vettore di ogni parola combina l'embedding della parola da sola e la media delle unità ECSS
in cui compare. Il contesto disambigua: *baseline* finisce vicino alle clausole di configuration
management, *margin* a quelle termiche e strutturali.

Per ogni parola e categoria vengono calcolati:
- **occorrenze**: in quante unità della categoria compare la parola (titoli, termini definiti, sigle);
- **concentrazione**: quota delle occorrenze totali della parola che cade nella categoria.
  *cost* compare quasi solo in M-60, quindi ha concentrazione alta in M-60; *plan* compare ovunque,
  quindi ha concentrazione bassa. La concentrazione è smussata (`vocabolario.smussamento`), così una
  parola vista 2 volte non scavalca una vista 30 volte;
- **specificità semantica**: score nella categoria − score della categoria concorrente più vicina
  dello stesso livello. Le categorie che si sovrappongono non sono concorrenti: T-COSTI non viene
  confrontata con T-PM, che la contiene;
- **punteggio**: concentrazione + `peso_semantico` × specificità normalizzata. È il criterio con cui
  `query.py lessico` ordina le parole: prevale l'evidenza di dove la parola compare davvero,
  e la vicinanza semantica corregge.

Per le sigle, il contesto comprende anche le clausole il cui titolo contiene l'espansione
(CBS → *Cost breakdown structure - DRD*).

### Quale comando usare

| Situazione | Comando | Cosa esegue |
|---|---|---|
| prima volta, o PDF nuovi o modificati | `python ingest.py --pdf "<cartella>"` | parte 1 solo sui PDF nuovi o cambiati, poi parte 2 |
| modifica a `categorie.yaml` (categorie, temi, semi) o aggiornamento degli script | `python ingest.py --solo-prototipi` | solo parte 2 |
| correzione del parser che riguarda standard specifici | `python ingest.py --pdf "<cartella>" --solo "ECSS-S-ST-00-01C Rev.1,ECSS-E-ST-20-06C Rev.1"` | parte 1 su quegli standard, poi parte 2 |
| cambio del modello di embedding, o modifica del parser che tocca molti standard | `python ingest.py --pdf "<cartella>" --forza` | parte 1 su tutto, poi parte 2 |

Nel dubbio, `python ingest.py --pdf "<cartella>"` va sempre bene: rielabora solo ciò che è cambiato
e ricalcola la parte 2.

---

## 3. `query.py` – interrogare l'indice

Trasforma la domanda in un vettore e cerca le unità più vicine.
Le domande possono essere scritte anche in italiano.

| Comando | Cosa restituisce |
|---|---|
| `python query.py categorie` | l'albero delle categorie con il numero di unità di ciascuna |
| `python query.py termini "cost management" --k 20` | le parole singole più vicine all'argomento |
| `python query.py termini "..." --categoria M-60` | idem, solo parole che compaiono nella categoria |
| `python query.py lessico --categoria cost --n 50` | le parole più caratteristiche della categoria, ordinate per punteggio, con concentrazione, occorrenze ed esempi |
| `python query.py lessico --categoria M-60 --fonte titolo` | idem, solo parole estratte dai titoli |
| `python query.py membri --categoria procurement --n 30` | le unità ECSS assegnate a un tema e il motivo (ancora, lessicale, semantica) |
| `python query.py unita "software validation" --tipo requisito` | requisiti, testo o DRD più pertinenti, con ID e clausola |

Opzioni utili:
- `--fonte definizione|titolo|sigla`: limita a una fonte;
- `--min-occorrenze N` (lessico, predefinito 2): solo parole che compaiono almeno N volte nella categoria;
- `--min-specificita`: filtro aggiuntivo sulla specificità semantica, ad esempio `0`;
- `--json`: output strutturato.

---

## 4. `classify.py` – classificare paragrafi

```bat
python classify.py documento.pdf --out risultati.csv
python classify.py relazione.docx note.txt --livelli discipline,trasversali
python classify.py --testo "The supplier shall provide a cost breakdown structure..."
```

### Cosa fa, passo per passo
1. **Estrae i paragrafi** dal documento (PDF, DOCX, TXT o MD), scartando quelli troppo corti
   (`min_caratteri_paragrafo`).
2. **Calcola l'embedding** di ogni paragrafo. I vettori sono in cache: un paragrafo già visto
   non viene ricalcolato.
3. **Calcola gli score.** Per ogni livello (temi, rami, discipline, trasversali) confronta il paragrafo
   con i prototipi di ogni categoria. Lo score di una categoria è la vicinanza al suo prototipo più simile.
4. **Decide, livello per livello e in modo indipendente:**
   - assegna la categoria migliore se supera la `soglia`;
   - assegna anche le categorie entro il `margine` dalla migliore, fino a `max_categorie`;
   - se nessuna supera la soglia, il paragrafo resta "non classificato" per quel livello.
5. **Aggiunge i termini di supporto:** le parole singole del vocabolario più vicine al paragrafo,
   che spiegano perché è finito in quella categoria.
6. **Scrive l'output:** CSV con separatore `;` (si apre direttamente in Excel), oppure JSONL.

Classificare per livello evita che categorie annidate si facciano concorrenza:
lo stesso paragrafo può risultare, ad esempio, temi **cost, customer** / ramo **M** / disciplina **M-60**.

### Opzioni
- `--livelli`: quali livelli classificare: `temi`, `rami`, `discipline`, `trasversali`,
  separati da virgola, oppure `tutti` (predefinito). Ad esempio `--livelli temi` usa solo i temi generici.
- `--soglia`, `--margine`: un unico valore per tutti i livelli, utile per le prove
  (i valori stabili vanno in `config.yaml`).
- `--out file.csv` o `--out file.jsonl`: file di output.

### Colonne del CSV
`file`, `pagina`, poi per ogni livello `<livello>`, `<livello>_score` e `<livello>_top3`,
infine `termini` e `testo`.

### Calibrazione (importante)
Le soglie in `config.yaml` (sezione `classificazione.livelli`) sono valori di partenza e
vanno tarate **per livello**: i rami, più ampi, hanno score più "medi" delle discipline.

1. Classificare un documento rappresentativo con `--out risultati.csv`.
2. Etichettare a mano 20–30 paragrafi per categoria.
3. Per ogni livello, confrontare la colonna `<livello>_score` dei paragrafi corretti con quella
   degli errati, e scegliere la soglia che li separa meglio.
4. Se due categorie si confondono spesso, ridefinirle in `categorie.yaml` e rilanciare
   `python ingest.py --solo-prototipi`.

Con qualche centinaio di paragrafi etichettati si può poi addestrare un classificatore leggero
sugli stessi embeddings, al posto dei prototipi.

---

### Temi generici: come definirli bene
- **Ancore**: se esiste uno standard o una clausola dedicati all'argomento, indicarli. È il modo più
  efficace per avere un tema pulito.
- **Semi in inglese**, come il testo degli standard, e specifici: *purchase order* è meglio di *order*.
  Evitare semi ambigui (*budget* richiama anche i *technical budgets* di massa e potenza).
- **Da 5 a 10 semi per tema**, che coprano i diversi aspetti dell'argomento.
- **Controllo**: `query.py membri --categoria <tema> --origine semantica` mostra i membri entrati
  solo per vicinanza semantica, cioè quelli da verificare.
- **Temi distinti tra loro.** Se due temi condividono molti membri, conviene fonderli o rendere i semi più netti.
- Dopo ogni modifica: `python ingest.py --solo-prototipi`.

## File di supporto

| File | Ruolo |
|---|---|
| `categorie.yaml` | **cosa** classificare: albero rami › discipline › standard, categorie trasversali (che possono puntare a categorie intere, a standard o a singole clausole) e temi generici (descrizione + semi) |
| `config.yaml` | **come** classificare: modello, dimensione delle unità, prototipi, soglie per livello |
| `ecss_parser.py` | selezione dei PDF attivi (2a) e segmentazione in unità (2b) |
| `vocabolario.py` | estrazione delle parole singole (passaggio e) e calcolo dei lessici |
| `core.py` | servizi comuni: caricamento del modello in modalità offline, database SQLite, risoluzione delle categorie, calcolo degli score |
| `index/` | archivio generato: `ecss.sqlite`, vettori per standard in `emb/`, `prototipi.npz` |

## Ordine tipico di lavoro
1. `setup_model.py`, poi `ingest.py`.
2. `query.py lessico` e `query.py termini`, per verificare che i termini estratti abbiano senso.
3. `classify.py` su un documento noto, poi correzione delle soglie in `config.yaml` guardando il CSV.
4. Eventuali modifiche a `categorie.yaml`, seguite da `ingest.py --solo-prototipi`.

---

## Modello

Il modello predefinito è `intfloat/multilingual-e5-base`: buon compromesso tra qualità
e velocità su CPU, e accetta query in italiano.

Per una qualità maggiore si può usare `BAAI/bge-m3`, che è più lento:

1. In `config.yaml` impostare `nome_hf` e `cartella`, e lasciare vuoti i due prefissi.
2. Rieseguire `python setup_model.py`.
3. Rieseguire `python ingest.py --pdf ...`. Il cambio di modello forza la rielaborazione.

## Collaudo senza modello

`set ECSS_TEST_EMBEDDER=1` attiva un embedder fittizio (bag of words), utile solo per
verificare che la pipeline funzioni. **Non va usato per il lavoro reale.**
