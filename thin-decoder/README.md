# Thin Decoder — risposte deterministiche

Risponde a domande su un testo in inglese con regole, claim e ontologie.
Senza configurazione LLM non usa alcun LLM né la rete: stesso input, stesso output.

## Uso

```bash
cargo build --release
./target/release/thin-decoder examples/satellite.json risposte.json [--ontology examples/ontology.json] [--config config.json]
```

**Configurazione LLM** (opzionale): senza `--config` il sistema usa solo le regole.

```json
{ "llm": { "url": "http://localhost:11434", "model": "qwen2.5:7b", "temperature": 0, "seed": 42 },
  "extract": "rules", "reason": "rules", "question_llm": false, "ontology_llm": false }
```

`extract` (estrazione dei claim): `rules` (predefinito), `llm` oppure `both`.
`extract_scope`: frasi inviate all'LLM, `uncovered` (predefinito: solo quelle in cui
le regole non trovano claim) oppure `all`.
`reason` (risposte): `rules` (predefinito), `llm` oppure `both`; ogni motore attivo
riceve tutti i claim. Il motore LLM deve citare gli ID dei claim usati, altrimenti la
sua risposta diventa una lacuna.
`question_llm` e `ontology_llm` (predefiniti `false`) attivano separatamente
l'interpretazione delle domande con LLM e la proposta di ontologia.
Con `question_llm`, una domanda che le regole non riconoscono è tradotta dall'LLM in
un pattern (soggetto, relazione, oggetto, variabile, tipo atteso); il pattern si
scarta se una sua parola non compare nella domanda né nell'ontologia. Un risultato
ottenuto da un pattern dell'LLM è sempre `rosso`.
I claim delle regole sono `blu`, quelli dell'LLM `rosso`; l'LLM propone, le regole
verificano che span, soggetto e oggetto compaiano letteralmente nel testo.

Backend: Ollama tramite l'endpoint compatibile con OpenAI (`/v1/chat/completions`),
con output vincolato da schema JSON (`response_format`).
Le risposte sono salvate in `cache_dir` (predefinito `.td-cache`), indicizzate
dall'hash di modello, `version`, prompt e schema, e riusate quando la chiave coincide.

**Input** (`questions`, `text`):

```json
{ "questions": ["What is the mass of the reaction wheel?"],
  "text": "The reaction wheel (RW) has a mass of 2,500 g." }
```

**Output**: per ogni domanda, `question` e `results`, un risultato per motore attivo
(`engine`: `rules`, `llm`) con `answer`, `status` (`answered`, `textual_evidence`,
`not_found`), `color`, `task` (specifica di compito: tipo, pattern della domanda,
tipo atteso, origine del pattern), evidenza (claim, colore del claim, documento,
frase, span) e regole applicate. Le stesse risposte sono stampate a terminale.

**Proposta di ontologia con LLM** (comando separato, richiede `ontology_llm`):

```bash
./target/release/thin-decoder propose-ontology examples/satellite.json proposta.json --config config.json
```

Scrive sinonimi, sottoclassi e verbi-attributo nel formato dell'ontologia a priori;
il file va rivisto a mano e poi passato con `--ontology`. Le domande non chiamano
mai l'LLM per l'ontologia.

**Ontologia a priori** (opzionale, tutti i campi facoltativi):

```json
{ "synonyms": [["spacecraft", "satellite"]],
  "subclass_of": [["magnetorquer", "actuator"]],
  "attribute_verbs": [["draw", "power"]] }
```

## Pipeline

| Passo | Modulo | Cosa fa |
|---|---|---|
| Testo | `text.rs` | token con offset, frasi, radici |
| Claim | `extract.rs` | copula, attributi, SVO, passivo, complementi; negazione, modalità, coreferenza |
| Quantità | `quantity.rs` | valori e unità normalizzati in SI |
| Ontologia | `ontology.rs` | acronimi, Hearst, copula, teste dei composti, file JSON |
| Domanda | `question.rs` | tipo → tripla con variabile e tipo atteso |
| Risposta | `answer.rs` | livelli fissi: esatto, lemma, acronimo, sinonimo, testa; poi frase più vicina o lacuna |

Requisiti e tracciabilità: [docs/REQUIREMENTS.md](docs/REQUIREMENTS.md).
