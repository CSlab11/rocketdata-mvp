# Requisiti — Thin Decoder

Concept: [CONCEPT.md](CONCEPT.md). Stati: `aperto`, `implementato`, `annullato`.

## Requisiti v1

### Input e interfaccia

| ID | Requisito | Stato | File impattati |
|---|---|---|---|
| REQ-001 | Il sistema deve accettare una richiesta Q con un tipo esplicito tra `domanda` e `verifica`. | aperto | — |
| REQ-002 | Il sistema deve rifiutare con un messaggio esplicito qualunque tipo di richiesta diverso da `domanda` e `verifica`. | aperto | — |
| REQ-003 | Il sistema deve accettare in modalità 1 una richiesta Q insieme a un testo semplice in inglese, e usare solo quel testo come fonte, senza eseguire la ricerca. | aperto | — |
| REQ-004 | Il sistema deve accettare in modalità 2 una richiesta Q insieme a un riferimento al corpus, e ottenere il testo di input interrogando il pacchetto di ricerca BM25 da riga di comando. Ricerca rimossa dalla v1. | annullato | — |
| REQ-005 | Il sistema deve accettare opzionalmente un'ontologia esistente; senza ontologia deve usare solo il testo di input come fonte di informazione. | aperto | — |
| REQ-006 | Il sistema deve permettere di scegliere per ogni richiesta il livello di derivazione 0, 1 o 2, e rifiutare il livello 2 se non è fornita un'ontologia. | aperto | — |

### Ricerca (rimossa dalla v1)

| ID | Requisito | Stato | File impattati |
|---|---|---|---|
| REQ-007 | Il sistema deve generare una o più query BM25 a partire da un modello fisso associato al tipo di richiesta. Ricerca rimossa dalla v1. | annullato | — |
| REQ-008 | Per il tipo `verifica`, il sistema deve generare query che coprano l'entità soggetto, il parametro del requisito, il metodo di verifica e anomalie, deviazioni o non-conformità. Ricerca rimossa dalla v1. | annullato | — |
| REQ-009 | Il sistema deve espandere le query con un dizionario di acronimi configurabile e, se è fornita un'ontologia, con i sinonimi dei concetti dell'ontologia. Ricerca rimossa dalla v1. | annullato | — |
| REQ-010 | Il sistema deve usare come testo di input tutti i chunk restituiti dalle query, insieme ai chunk adiacenti a ciascuno di essi. Ricerca rimossa dalla v1. | annullato | — |
| REQ-011 | Il sistema deve registrare nella traccia dell'output le query eseguite e i chunk ottenuti, con il documento di provenienza. Ricerca rimossa dalla v1. | annullato | — |

### Organizzazione dell'informazione

| ID | Requisito | Stato | File impattati |
|---|---|---|---|
| REQ-012 | Il sistema deve estrarre i claim da tutto il testo di input, senza filtrarli in base a Q. | aperto | — |
| REQ-013 | Ogni claim deve contenere ID, soggetto, relazione, oggetto, span di testo sorgente e riferimento al documento e alla posizione di provenienza. | implementato | `src/extract.rs`, `src/main.rs` |
| REQ-014 | Il sistema deve estrarre con regole deterministiche quantità, unità di misura, intervalli, identificativi e frasi normative contenenti "shall". | aperto | — |
| REQ-015 | Il sistema deve estrarre le relazioni non coperte dalle regole con un LLM locale il cui output è vincolato da uno schema JSON. | aperto | — |
| REQ-016 | Il sistema deve scartare ogni claim il cui span non compare letteralmente nel testo di input, e registrarlo nel log. | implementato | `src/extract.rs` |
| REQ-017 | Il sistema deve normalizzare valori numerici e unità di misura scritti in forme diverse in una rappresentazione unica. | implementato | `src/extract.rs`, `src/main.rs`, `src/quantity.rs` |
| REQ-018 | Se è fornita un'ontologia, il sistema deve allineare entità e relazioni dei claim ai concetti dell'ontologia. | aperto | — |
| REQ-019 | Il sistema deve salvare i claim estratti in un file JSON Lines. | aperto | — |

### Ragionamento

| ID | Requisito | Stato | File impattati |
|---|---|---|---|
| REQ-020 | Al livello di derivazione 0 il sistema non deve produrre fatti derivati. | aperto | — |
| REQ-021 | Al livello di derivazione 1 il sistema deve applicare regole generiche: confronto di valori e intervalli con conversione delle unità, identità tra menzioni della stessa entità, transitività e relazioni inverse. | aperto | — |
| REQ-022 | Ai livelli di derivazione 1 e 2 il sistema deve rilevare le contraddizioni tra claim sullo stesso attributo della stessa entità. | aperto | — |
| REQ-023 | Al livello di derivazione 2 il sistema deve applicare, oltre alle regole generiche, le regole dell'ontologia tramite un reasoner. | aperto | — |
| REQ-024 | Ogni fatto derivato deve riportare la regola applicata e gli ID dei claim usati come premesse. | aperto | — |
| REQ-025 | Il sistema non deve produrre fatti derivati tramite l'LLM. Sostituito da REQ-076 – REQ-078 (fatti LLM ammessi, distinti dal colore). | annullato | — |
| REQ-026 | Il sistema deve selezionare i claim rilevanti per Q e aggiungere i claim collegati alle loro entità entro due passi nel grafo. | aperto | — |

### Risposta

| ID | Requisito | Stato | File impattati |
|---|---|---|---|
| REQ-027 | Per il tipo `verifica`, il sistema deve produrre un esito tra `conforme`, `non conforme` e `non dimostrato`. | aperto | — |
| REQ-028 | Il sistema deve assegnare l'esito `conforme` solo se ogni condizione del requisito è soddisfatta da claim o fatti derivati e non esiste evidenza contraria; altrimenti deve assegnare `non conforme` o `non dimostrato`. | aperto | — |
| REQ-029 | Per il tipo `verifica`, la risposta deve elencare separatamente l'evidenza a favore, l'evidenza contraria e le lacune. | aperto | — |
| REQ-030 | Per il tipo `domanda`, la risposta deve riportare il fatto richiesto con la sua citazione oppure, se il fatto non è presente, dichiarare la lacuna. | aperto | — |
| REQ-031 | Il sistema deve generare la risposta da un template deterministico, in cui ogni affermazione cita un claim o un fatto derivato. Sostituito da REQ-081. | annullato | — |
| REQ-032 | L'output deve includere una traccia con la specifica di compito, le query (in modalità 2), i claim e i fatti derivati usati e le regole applicate. Sostituito da REQ-071. | annullato | — |
| REQ-071 | L'output deve includere una traccia con la specifica di compito, i claim e i fatti derivati usati e le regole applicate. | implementato | `README.md`, `src/answer.rs`, `src/output.rs` |

### Esecuzione

| ID | Requisito | Stato | File impattati |
|---|---|---|---|
| REQ-033 | Il sistema deve funzionare interamente in locale, senza accesso alla rete. | aperto | — |
| REQ-034 | Il sistema deve usare un modello pre-addestrato con al più 8 miliardi di parametri, quantizzato, senza fine-tuning. | aperto | — |
| REQ-035 | Il sistema deve leggere da configurazione modello, versione, quantizzazione, temperatura e seed, con temperatura 0 e seed fisso come valori predefiniti. | aperto | — |
| REQ-036 | Eseguito due volte sullo stesso input con la stessa configurazione, il sistema deve produrre gli stessi esiti. | implementato | `examples/ontology.json`, `examples/satellite.json`, `tests/reproducibility.rs` |
| REQ-037 | Il sistema deve fornire all'LLM, per l'estrazione, esempi few-shot non tratti dalla ground truth. | aperto | — |

### Validazione

| ID | Requisito | Stato | File impattati |
|---|---|---|---|
| REQ-038 | Il sistema deve fornire uno strumento di valutazione che legge la ground truth dal template `validation/ground_truth_template.xlsx`. Annullato da REQ-084 (validazione rimossa dal concept). | annullato | — |
| REQ-039 | Lo strumento di valutazione deve eseguire la modalità 1 usando i testi del foglio Contesti e la modalità 2 usando il pacchetto di ricerca, e produrre un report per ciascuna con la differenza tra le due. Sostituito da REQ-072. | annullato | — |
| REQ-040 | Lo strumento di valutazione deve calcolare falsi "conforme", accuratezza dell'esito, recall della ricerca, recall e precisione delle evidenze, accuratezza delle risposte, tracciabilità e riproducibilità, e confrontarle con le soglie del concept. Sostituito da REQ-073. | annullato | — |
| REQ-072 | Lo strumento di valutazione deve eseguire il sistema sui testi del foglio Contesti e produrre un report dei risultati. Annullato da REQ-084 (validazione rimossa dal concept). | annullato | — |
| REQ-073 | Lo strumento di valutazione deve calcolare falsi "conforme", accuratezza dell'esito, recall e precisione delle evidenze, accuratezza delle risposte, tracciabilità e riproducibilità, e confrontarle con le soglie del concept. Annullato da REQ-084 (validazione rimossa dal concept). | annullato | — |
| REQ-041 | Il report di valutazione deve indicare, per ogni metrica sotto soglia, il passo della pipeline in cui si originano gli errori. Annullato da REQ-084 (validazione rimossa dal concept). | annullato | — |
| REQ-042 | Lo strumento di valutazione deve esportare l'elenco dei fatti derivati per la revisione manuale. Annullato da REQ-084 (validazione rimossa dal concept). | annullato | — |
| REQ-084 | Il concept (`docs/CONCEPT.md`) non deve contenere criteri di successo con soglie di validazione né un criterio di uscita della v1 basato su di esse; deve conservare l'assunzione che la ground truth non si usa per addestrare né per i prompt few-shot. | implementato | `docs/CONCEPT.md` |

### Programma deterministico in Rust (`domanda`, modalità 1)

| ID | Requisito | Stato | File impattati |
|---|---|---|---|
| REQ-051 | Il sistema deve fornire un eseguibile a riga di comando scritto in Rust che risponde a domande su un testo in inglese senza usare LLM. Sostituito da REQ-070. | annullato | `Cargo.lock`, `Cargo.toml`, `src/main.rs`, `README.md` |
| REQ-052 | L'eseguibile deve leggere da un file JSON di input un oggetto con la lista delle domande (`questions`) e il testo in inglese (`text`). | implementato | `Cargo.lock`, `Cargo.toml`, `src/input.rs`, `src/main.rs` |
| REQ-053 | L'eseguibile deve salvare in un file JSON di output la lista delle coppie domanda-risposta. | implementato | `src/main.rs`, `src/output.rs` |
| REQ-054 | Per ogni domanda l'eseguibile deve stampare a terminale la domanda e la risposta. | implementato | `src/main.rs`, `src/output.rs` |
| REQ-055 | Il sistema deve estrarre i claim con regole deterministiche sulle frasi del testo: copula, possesso con attributo ("has a mass of"), "the A of X", soggetto-verbo-oggetto, passivo e complementi preposizionali. | implementato | `src/extract.rs`, `src/lexicon.rs`, `src/main.rs`, `src/text.rs` |
| REQ-056 | Ogni claim deve registrare come attributi la negazione e la modalità (shall, must, may, should, can, will). | implementato | `src/extract.rs` |
| REQ-057 | Il sistema deve classificare ogni domanda con regole (what, which, who, when, where, how much, how many, how + aggettivo, sì/no) e tradurla in un pattern di tripla con una variabile e un tipo di risposta atteso. | implementato | `src/main.rs`, `src/question.rs` |
| REQ-058 | Il sistema deve confrontare il pattern con i claim applicando, in ordine fisso, corrispondenza esatta, per lemma, per acronimo, per sinonimo e per testa del nome composto, e a parità scegliere il claim che compare prima nel testo. | implementato | `src/answer.rs`, `src/extract.rs`, `src/lexicon.rs`, `src/main.rs`, `src/ontology.rs`, `src/output.rs` |
| REQ-059 | Il sistema deve costruire dal testo un'ontologia al volo con definizioni di acronimi ("Long Name (LN)"), relazioni di sottoclasse da pattern di Hearst, da copula ("X is a Y") e dalle teste dei nomi composti. | implementato | `src/main.rs`, `src/ontology.rs` |
| REQ-060 | Il sistema deve accettare opzionalmente un'ontologia a priori in un file JSON con sinonimi, sottoclassi e corrispondenze verbo-attributo. | implementato | `src/main.rs`, `src/ontology.rs` |
| REQ-061 | Se nessun claim corrisponde, il sistema deve restituire la frase con la maggiore sovrapposizione di lemmi con la domanda, marcata come evidenza testuale, purché superi una soglia fissa; altrimenti deve dichiarare la lacuna. | implementato | `src/answer.rs`, `src/main.rs` |
| REQ-062 | Il sistema deve sostituire i pronomi soggetto (it, they, this, these) con il soggetto della frase precedente. | implementato | `src/extract.rs` |

### Integrazione LLM

| ID | Requisito | Stato | File impattati |
|---|---|---|---|
| REQ-063 | Se l'analisi a regole non riconosce una domanda, il sistema deve poter tradurla con un LLM locale in un pattern JSON vincolato da schema (soggetto, relazione, oggetto, variabile, tipo di risposta atteso), e scartare il pattern se una sua parola non compare nella domanda né nell'ontologia. | implementato | `README.md`, `src/answer.rs`, `src/llm_extract.rs`, `src/llm_question.rs`, `src/main.rs`, `src/ontology.rs` |
| REQ-064 | Il sistema deve fornire un comando separato che, dato un testo, fa proporre a un LLM locale sinonimi, sottoclassi e corrispondenze verbo-attributo e li scrive in un file nel formato dell'ontologia a priori (REQ-060), da rivedere a mano prima dell'uso; l'esecuzione delle domande non deve chiamare l'LLM per l'ontologia. | implementato | `README.md`, `src/llm_ontology.rs`, `src/main.rs`, `src/ontology.rs`, `tests/llm.rs` |
| REQ-065 | Il sistema deve salvare ogni risposta dell'LLM in una cache su file, indicizzata dall'hash di modello, versione, prompt e schema, e riusarla quando la chiave coincide. | implementato | `.gitignore`, `Cargo.lock`, `Cargo.toml`, `README.md`, `src/config.rs`, `src/llm.rs` |
| REQ-066 | Il sistema deve leggere dalla configurazione LLM l'elenco delle funzioni da attivare tra `extract`, `question` e `ontology`, oppure `all` per tutte. Sostituito da REQ-074, REQ-076 e REQ-080. | annullato | — |
| REQ-067 | Per l'estrazione, la configurazione LLM deve permettere di scegliere se inviare all'LLM solo le frasi in cui le regole non trovano claim (`uncovered`) oppure tutte le frasi (`all`). | implementato | `README.md`, `src/config.rs`, `src/llm_extract.rs`, `src/main.rs`, `tests/llm.rs` |
| REQ-068 | Se il server LLM non risponde e la risposta non è in cache, il sistema deve fermarsi con un messaggio di errore esplicito e un codice di uscita diverso da zero. | implementato | `src/llm.rs` |
| REQ-069 | Il sistema deve usare come backend LLM iniziale Ollama tramite l'endpoint compatibile con OpenAI (`/v1/chat/completions`), con indirizzo e modello letti dalla configurazione. | implementato | `Cargo.lock`, `Cargo.toml`, `README.md`, `src/config.rs`, `src/llm.rs`, `src/main.rs` |
| REQ-070 | Il sistema deve fornire un eseguibile a riga di comando scritto in Rust che risponde a domande su un testo in inglese; senza configurazione LLM non deve usare alcun LLM. | implementato | `Cargo.toml`, `README.md`, `src/config.rs`, `src/main.rs`, `tests/llm.rs` |

### Strategie di elaborazione e colori

| ID | Requisito | Stato | File impattati |
|---|---|---|---|
| REQ-074 | Il primo passaggio (estrazione dei claim) deve poter usare, secondo la configurazione, solo le regole, solo l'LLM oppure regole e LLM in parallelo; il valore predefinito è solo le regole. | implementato | `README.md`, `src/config.rs`, `src/extract.rs`, `src/llm_extract.rs`, `src/main.rs`, `tests/llm.rs` |
| REQ-075 | Ogni claim estratto deve avere un'etichetta di colore che ne indica l'origine: `blu` se estratto dalle regole, `rosso` se estratto dall'LLM. | implementato | `src/color.rs`, `src/extract.rs`, `src/main.rs` |
| REQ-076 | Il secondo passaggio (elaborazione: fatti derivati e risposte) deve poter usare, secondo la configurazione, il motore a regole, il motore LLM oppure entrambi; ogni motore attivo deve ricevere in input tutti i claim del primo passaggio, di qualunque colore. Il valore predefinito è solo il motore a regole. | implementato | `README.md`, `src/answer.rs`, `src/config.rs`, `src/llm_answer.rs`, `src/main.rs`, `src/output.rs`, `tests/colors.rs`, `tests/llm.rs` |
| REQ-077 | Il motore a regole deve etichettare ogni risultato `blu` se usa solo claim blu, `viola` se usa almeno un claim rosso. | implementato | `src/answer.rs`, `src/color.rs`, `src/output.rs` |
| REQ-078 | Il motore LLM deve etichettare ogni risultato `rosso` se usa solo claim rossi, `giallo` se usa solo claim blu, `arancione` se usa claim blu e rossi; il colore si calcola dagli ID dei claim citati come premesse. | implementato | `src/color.rs`, `src/llm_answer.rs` |
| REQ-079 | Il colore di claim e risultati deve comparire come etichetta nel JSON di output e nella stampa a terminale. | implementato | `src/answer.rs`, `src/output.rs`, `tests/colors.rs` |
| REQ-080 | La configurazione LLM deve permettere di attivare separatamente l'interpretazione delle domande con LLM (REQ-063) e la proposta di ontologia (REQ-064). | implementato | `README.md`, `src/config.rs` |
| REQ-081 | Il motore a regole deve generare la risposta da un template deterministico; ogni affermazione della risposta, di qualunque motore, deve citare un claim o un fatto derivato. | implementato | `src/answer.rs` |
| REQ-083 | Se il pattern della domanda è proposto dall'LLM (REQ-063), il risultato deve avere colore `rosso`, in deroga a REQ-077. | implementato | `README.md`, `src/answer.rs`, `src/llm_question.rs`, `tests/llm.rs` |

## Requisiti opzionali (fuori v1)

| ID | Requisito | Stato | File impattati |
|---|---|---|---|
| REQ-043 | Il sistema deve supportare i tipi di richiesta `sintesi`, `confronto` e `lacune`. | aperto | — |
| REQ-044 | Il sistema deve poter produrre inferenze tramite LLM con premesse citate, marcate come "ipotesi" e tenute separate dai fatti derivati deterministici. Sostituito da REQ-076 – REQ-078. | annullato | — |
| REQ-045 | Il sistema deve poter generare la risposta con un LLM, verificando che ogni frase citi un claim o un fatto derivato. Sostituito da REQ-076 e REQ-081. | annullato | — |
| REQ-046 | Il sistema deve poter espandere le query di ricerca con un LLM, registrando le espansioni nella traccia; la funzione deve essere attivabile da configurazione e spenta di default. | aperto | — |
| REQ-047 | Il sistema deve poter ottenere il testo di input con una ricerca che combina BM25 e ricerca per embedding; la funzione deve essere attivabile da configurazione e spenta di default. | aperto | — |
| REQ-048 | Il sistema deve poter ri-estrarre in modo più dettagliato gli span rilevanti per Q, in aggiunta all'estrazione completa. | aperto | — |
| REQ-049 | Il sistema deve poter usare un modello con fine-tuning LoRA addestrato sui claim corretti a mano; da attivare se la recall dell'estrazione è sotto soglia per cause di linguaggio di dominio. Annullato da REQ-084 (senza soglie la condizione di attivazione non è verificabile). | annullato | — |
| REQ-050 | Il sistema deve poter accettare documenti PDF e DOCX convertendoli in testo. | aperto | — |
| REQ-082 | Il sistema deve poter applicare un passo finale a regole che elimina i duplicati nell'unione dei risultati del motore LLM e del motore a regole. | aperto | — |

## Bug

| ID | Descrizione | Req | File impattati | Modifiche necessarie | Stato |
|---|---|---|---|---|---|
| BUG-001 | La traccia dei sinonimi mostra le radici ("satellit") invece delle parole originali. | REQ-060 | `src/ontology.rs` | In `add_synonyms` conservare le parole originali e usarle nel messaggio di provenienza. | risolto |
