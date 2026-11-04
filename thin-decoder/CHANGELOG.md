# Changelog

Richieste tracciate (requisiti e bug). Eventi: `richiesto`, `implementato`, `annullato`, `segnalato`, `risolto`.

REQ-001 – REQ-050 importati dal progetto claude.ai "Thin Decoder" (2026-10-31).

| Data | ID | Evento | Nota |
|---|---|---|---|
| 2026-10-31 | REQ-002 | richiesto | importato dal progetto claude.ai |
| 2026-10-31 | REQ-003 | richiesto | importato dal progetto claude.ai |
| 2026-10-31 | REQ-005 | richiesto | importato dal progetto claude.ai |
| 2026-10-31 | REQ-006 | richiesto | importato dal progetto claude.ai |
| 2026-10-31 | REQ-012 | richiesto | importato dal progetto claude.ai |
| 2026-10-31 | REQ-014 | richiesto | importato dal progetto claude.ai |
| 2026-10-31 | REQ-015 | richiesto | importato dal progetto claude.ai |
| 2026-10-31 | REQ-018 | richiesto | importato dal progetto claude.ai |
| 2026-10-31 | REQ-019 | richiesto | importato dal progetto claude.ai |
| 2026-10-31 | REQ-020 | richiesto | importato dal progetto claude.ai |
| 2026-10-31 | REQ-021 | richiesto | importato dal progetto claude.ai |
| 2026-10-31 | REQ-022 | richiesto | importato dal progetto claude.ai |
| 2026-10-31 | REQ-023 | richiesto | importato dal progetto claude.ai |
| 2026-10-31 | REQ-024 | richiesto | importato dal progetto claude.ai |
| 2026-10-31 | REQ-026 | richiesto | importato dal progetto claude.ai |
| 2026-10-31 | REQ-027 | richiesto | importato dal progetto claude.ai |
| 2026-10-31 | REQ-028 | richiesto | importato dal progetto claude.ai |
| 2026-10-31 | REQ-029 | richiesto | importato dal progetto claude.ai |
| 2026-10-31 | REQ-030 | richiesto | importato dal progetto claude.ai |
| 2026-10-31 | REQ-033 | richiesto | importato dal progetto claude.ai |
| 2026-10-31 | REQ-034 | richiesto | importato dal progetto claude.ai |
| 2026-10-31 | REQ-035 | richiesto | importato dal progetto claude.ai |
| 2026-10-31 | REQ-037 | richiesto | importato dal progetto claude.ai |
| 2026-10-31 | REQ-038 | richiesto | importato dal progetto claude.ai |
| 2026-10-31 | REQ-041 | richiesto | importato dal progetto claude.ai |
| 2026-10-31 | REQ-042 | richiesto | importato dal progetto claude.ai |
| 2026-10-31 | REQ-043 | richiesto | importato dal progetto claude.ai |
| 2026-10-31 | REQ-046 | richiesto | importato dal progetto claude.ai |
| 2026-10-31 | REQ-047 | richiesto | importato dal progetto claude.ai |
| 2026-10-31 | REQ-048 | richiesto | importato dal progetto claude.ai |
| 2026-10-31 | REQ-049 | richiesto | importato dal progetto claude.ai |
| 2026-11-02 | REQ-051 | richiesto | |
| 2026-11-02 | REQ-052 | richiesto | |
| 2026-11-02 | REQ-053 | richiesto | |
| 2026-11-02 | REQ-054 | richiesto | |
| 2026-11-02 | REQ-055 | richiesto | |
| 2026-11-02 | REQ-056 | richiesto | |
| 2026-11-02 | REQ-057 | richiesto | |
| 2026-11-02 | REQ-058 | richiesto | |
| 2026-11-02 | REQ-059 | richiesto | |
| 2026-11-02 | REQ-060 | richiesto | |
| 2026-11-02 | REQ-061 | richiesto | |
| 2026-11-02 | REQ-062 | richiesto | |
| 2026-11-02 | REQ-051 | implementato | eseguibile Rust a riga di comando senza LLM |
| 2026-11-02 | REQ-052 | implementato | lettura del file JSON di input (questions, text) |
| 2026-11-02 | REQ-053 | implementato | scrittura del file JSON di output con le coppie domanda-risposta |
| 2026-11-02 | REQ-054 | implementato | stampa a terminale di domanda e risposta |
| 2026-11-02 | REQ-055 | implementato | estrazione dei claim con regole su copula, possesso, passivo, SVO e complementi |
| 2026-11-02 | REQ-013 | implementato | claim con ID, soggetto, relazione, oggetto, span, documento e frase |
| 2026-11-02 | REQ-016 | implementato | scarto dei claim con span non letterale e registrazione nel log |
| 2026-11-02 | REQ-056 | implementato | negazione e modalità come attributi dei claim |
| 2026-11-02 | REQ-062 | implementato | pronomi soggetto sostituiti con il soggetto della frase precedente |
| 2026-11-02 | REQ-017 | implementato | normalizzazione di valori e unità in SI |
| 2026-11-02 | REQ-059 | implementato | ontologia al volo: acronimi, Hearst, copula e teste dei composti |
| 2026-11-02 | REQ-060 | implementato | ontologia a priori opzionale da file JSON e lessico predefinito |
| 2026-11-02 | REQ-057 | implementato | analisi delle domande in pattern di tripla con variabile e tipo atteso |
| 2026-11-02 | REQ-058 | implementato | confronto pattern-claim con livelli in ordine fisso e risposta da template |
| 2026-11-02 | REQ-061 | implementato | fallback su frase con massima sovrapposizione lessicale, altrimenti lacuna |
| 2026-11-02 | REQ-036 | implementato | test di riproducibilità su due esecuzioni con esempio e ontologia |
| 2026-11-02 | BUG-001 | segnalato | traccia dei sinonimi con radici |
| 2026-11-02 | BUG-001 | risolto | parole originali nella traccia dei sinonimi |
| 2026-11-03 | REQ-063 | richiesto | |
| 2026-11-03 | REQ-064 | richiesto | |
| 2026-11-03 | REQ-065 | richiesto | |
| 2026-11-03 | REQ-066 | richiesto | |
| 2026-11-03 | REQ-067 | richiesto | |
| 2026-11-03 | REQ-068 | richiesto | |
| 2026-11-03 | REQ-069 | richiesto | |
| 2026-11-03 | REQ-070 | richiesto | |
| 2026-11-03 | REQ-071 | richiesto | |
| 2026-11-03 | REQ-072 | richiesto | |
| 2026-11-03 | REQ-073 | richiesto | |
| 2026-11-03 | REQ-074 | richiesto | |
| 2026-11-03 | REQ-075 | richiesto | |
| 2026-11-03 | REQ-076 | richiesto | |
| 2026-11-03 | REQ-077 | richiesto | |
| 2026-11-03 | REQ-078 | richiesto | |
| 2026-11-03 | REQ-079 | richiesto | |
| 2026-11-03 | REQ-080 | richiesto | |
| 2026-11-03 | REQ-081 | richiesto | |
| 2026-11-03 | REQ-082 | richiesto | |
| 2026-11-03 | REQ-004 | annullato | |
| 2026-11-03 | REQ-007 | annullato | |
| 2026-11-03 | REQ-008 | annullato | |
| 2026-11-03 | REQ-009 | annullato | |
| 2026-11-03 | REQ-010 | annullato | |
| 2026-11-03 | REQ-011 | annullato | |
| 2026-11-03 | REQ-025 | annullato | |
| 2026-11-03 | REQ-031 | annullato | |
| 2026-11-03 | REQ-032 | annullato | |
| 2026-11-03 | REQ-039 | annullato | |
| 2026-11-03 | REQ-040 | annullato | |
| 2026-11-03 | REQ-044 | annullato | |
| 2026-11-03 | REQ-045 | annullato | |
| 2026-11-03 | REQ-051 | annullato | |
| 2026-11-03 | REQ-066 | annullato | |
| 2026-11-03 | REQ-083 | richiesto | |
| 2026-11-03 | REQ-075 | implementato | colore di origine sui claim: blu per le regole |
| 2026-11-03 | REQ-077 | implementato | colore dei risultati del motore a regole: blu o viola |
| 2026-11-03 | REQ-079 | implementato | colore di claim e risultati nel JSON di output e a terminale |
| 2026-11-03 | REQ-081 | implementato | controllo delle citazioni sulle risposte answered di ogni motore |
| 2026-11-03 | REQ-070 | implementato | opzione --config; senza configurazione LLM nessun LLM |
| 2026-11-03 | REQ-069 | implementato | client Ollama su /v1/chat/completions con schema JSON |
| 2026-11-03 | REQ-065 | implementato | cache su file delle risposte LLM indicizzata da sha256 |
| 2026-11-03 | REQ-068 | implementato | errore esplicito se il server LLM non risponde e la risposta non è in cache |
| 2026-11-03 | REQ-074 | implementato | estrazione con regole, LLM o entrambi; predefinito regole |
| 2026-11-03 | REQ-067 | implementato | frasi per l'estrazione LLM: uncovered (predefinito) o all |
| 2026-11-03 | REQ-076 | implementato | elaborazione con motore a regole, LLM o entrambi; output con results per motore |
| 2026-11-03 | REQ-078 | implementato | colori del motore LLM dalle premesse: rosso, giallo, arancione |
| 2026-11-03 | REQ-080 | implementato | flag separati question_llm e ontology_llm |
| 2026-11-03 | REQ-063 | implementato | pattern della domanda proposto dall'LLM se le regole non la riconoscono, con controllo delle parole |
| 2026-11-03 | REQ-083 | implementato | risultato rosso se il pattern della domanda viene dall'LLM |
| 2026-11-03 | REQ-064 | implementato | comando propose-ontology: ontologia proposta dall'LLM da rivedere a mano |
| 2026-11-03 | REQ-071 | implementato | specifica di compito nella traccia di ogni risultato |
