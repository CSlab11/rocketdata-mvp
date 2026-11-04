# Thin Decoder — Concept v1

## Scopo

Dato un testo sorgente in inglese e una richiesta Q (domanda o compito), il Thin Decoder produce una risposta in cui **ogni affermazione è tracciabile**: a un passaggio del testo, oppure a una regola deterministica applicata ai fatti estratti (ed eventualmente a un'ontologia fornita). Evidenza contraria e lacune sono riportate esplicitamente.

Primo caso d'uso: **verifica di conformità di requisiti rispetto al testo di documenti**.

## Pipeline

```
Q (tipo esplicito: domanda | verifica)
 │
 ├─ 0. Specifica di compito   tipo, target, criterio                      deterministico   (+ LLM vincolato se le regole non riconoscono Q)
 ├─ 1. Organizzazione         estrazione di TUTTI i claim dal testo        regole + LLM locale vincolato
 │                            controllo letterale degli span               deterministico
 │                            allineamento all'ontologia (se presente)
 ├─ 2. Ragionamento           livelli di derivazione 0 / 1 / 2             deterministico
 │                            contraddizioni, selezione sottografo
 └─ 3. Risposta               template: esito, pro, contro, lacune, citazioni   deterministico
```

Il testo di input è fornito insieme a Q. La ricerca non fa parte della v1; resta come funzione opzionale, attivabile da configurazione (vedi "Fuori scope").

## Decisioni

| Tema | Decisione |
|---|---|
| Rapporto tra Q ed estrazione | L'estrazione dei claim è **indipendente da Q** e copre tutto il testo di input. Q guida selezione, ragionamento e risposta. |
| Tipi di richiesta | **Chiusi**. v1: `domanda`, `verifica`. Il tipo è un parametro esplicito. |
| Input | Q + testo semplice in inglese, fornito direttamente. Ontologia esistente **opzionale**: senza ontologia il testo è l'unica fonte. |
| Fatti derivati | Livello 0: nessuna derivazione. Livello 1: regole generiche (numeri, unità, intervalli, coreferenza, transitività, inverse, contraddizioni). Livello 2: livello 1 + regole dell'ontologia. |
| Ruolo dell'LLM | **Opzionale, spento di default.** Può intervenire nell'estrazione, nell'elaborazione (fatti derivati e risposte), nell'interpretazione delle domande e nella proposta di ontologia fuori linea. Output vincolato da schema JSON; ogni risultato cita i claim usati. Il **colore** distingue sempre ciò che è certo (blu) da ciò che passa per l'LLM. |
| Backend LLM | Ollama in locale, tramite l'endpoint compatibile con OpenAI; indirizzo e modello da configurazione, così il backend si può sostituire. |
| Modello | Locale, pre-addestrato, ≤ 8B parametri, quantizzato, **senza fine-tuning**. Scelto con un confronto a durata fissa tra 2–3 candidati. |
| Esito della verifica | `conforme` / `non conforme` / `non dimostrato`. Nel dubbio: `non dimostrato`. |

## Strategie e colori

Due passaggi, ciascuno configurabile: **regole**, **LLM** oppure **entrambi**. Predefinito: solo regole.

1. **Estrazione** dei claim dal testo.
2. **Elaborazione** (fatti derivati e risposte): ogni motore attivo riceve tutti i claim del passaggio 1.

| Origine | Input usati | Colore |
|---|---|---|
| Estrazione | regole | blu |
| Estrazione | LLM | rosso |
| Motore a regole | solo blu | blu |
| Motore a regole | almeno un rosso | viola |
| Motore a regole | pattern della domanda proposto dall'LLM | rosso |
| Motore LLM | solo rossi | rosso |
| Motore LLM | solo blu | giallo |
| Motore LLM | blu e rossi | arancione |

Solo il **blu** è certo al 100%; gli altri colori indicano da dove arriva l'incertezza. Il colore compare nel JSON di output e a terminale. Opzionale, fuori dalla prossima implementazione: un passo finale a regole che elimina i duplicati tra i risultati dei due motori.

## Integrazione LLM

L'LLM propone, le regole verificano: nulla entra nel grafo o nella risposta senza un controllo deterministico.

| Funzione | Cosa propone l'LLM | Controllo deterministico |
|---|---|---|
| Estrazione | Claim nello schema dei claim; con regole in parallelo, sulle frasi senza claim a regole (`uncovered`) o su tutte (`all`) | Span letterale nel testo; soggetto e oggetto contenuti nello span; a parità vince il claim a regole |
| Domande | Pattern della domanda (soggetto, relazione, oggetto, variabile, tipo atteso), solo se le regole non la riconoscono | Ogni parola del pattern compare nella domanda o nell'ontologia; altrimenti fallback lessicale |
| Ontologia | Sinonimi, sottoclassi, verbi-attributo, con un comando separato e fuori linea | Revisione a mano; il file si usa come ontologia a priori e non richiede l'LLM in esecuzione |

Estrazione ed elaborazione si scelgono come sopra; interpretazione delle domande e proposta di ontologia si attivano separatamente. Riproducibilità: temperatura 0, seed fisso e **cache su file** delle risposte, indicizzata dall'hash di modello, versione, prompt e schema. Se il server non risponde e la risposta non è in cache, l'esecuzione si ferma con un errore.

## Criteri di successo (soglie v1)

| Metrica | Soglia |
|---|---|
| Falsi "conforme" | 0 |
| Accuratezza esito verifica | ≥ 80% |
| Recall delle evidenze | ≥ 80% |
| Precisione delle evidenze | ≥ 70% |
| Accuratezza risposte a domande | ≥ 90% |
| Tracciabilità delle affermazioni | 100% |
| Correttezza dei fatti derivati (revisione manuale) | 100% |
| Riproducibilità degli esiti (due esecuzioni) | 100% |

Le soglie sono il **criterio di uscita** della v1: se una non è rispettata, la v1 si chiude con un report che indica metrica e passo della pipeline responsabili.


## Fuori scope v1 (requisiti opzionali)

- Tipi `sintesi`, `confronto`, `lacune`.
- Inferenze LLM validate, marcate come "ipotesi".
- Verbalizzazione fluida con LLM.
- Ricerca ibrida BM25 + embedding, attivabile da configurazione e spenta di default.
- Espansione delle query di ricerca con LLM, attivabile da configurazione e spenta di default.
- Approfondimento mirato sugli span rilevanti per Q.
- Fine-tuning LoRA.
- Conversione da PDF/DOCX.

## Assunzioni

- Hardware: singola macchina locale; il limite di 8B parametri quantizzati è il vincolo di riferimento finché non viene fissata una macchina specifica.
- La ground truth non viene mai usata per addestrare o per scrivere i prompt few-shot.
