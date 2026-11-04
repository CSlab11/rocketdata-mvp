---
name: checkreq
description: Controlla che la tabella dei requisiti, i file impattati, il changelog e i commit siano ancora allineati. Usare quando l'utente lancia /checkreq o chiede di verificare la tracciabilità dei requisiti.
argument-hint: "[REQ-NNN | BUG-NNN | <parte>_REQ-NNN ...]"
allowed-tools: Bash(python3 ${CLAUDE_SKILL_DIR}/scripts/check_trace.py *)
disallowed-tools: Edit Write NotebookEdit
---

# /checkreq — verifica di allineamento

Leggi prima `${CLAUDE_SKILL_DIR}/../req/conventions.md`.

Ambito: $ARGUMENTS

Senza argomenti, controlla tutti i requisiti e i bug. Questa fase non modifica nulla.

## 1. Controllo meccanico

Dalla radice del repository esegui:

```bash
python3 ${CLAUDE_SKILL_DIR}/scripts/check_trace.py .
```

Su Windows usa `python` o `py` al posto di `python3`. Lo script controlla ID (per file, qualificati con il nome della parte; ID storici risolti come in "Numerazione degli ID"), stati, esistenza dei file elencati, corrispondenza tra file in tabella e file toccati dai commit di ciascun ID, voci del changelog, commit senza ID e modifiche non committate. Esce con 1 se ci sono errori.

## 2. Controllo di merito

Lo script non sa se il codice soddisfa ancora il requisito. La sezione "DA RIVEDERE" elenca i file di un requisito modificati da commit di altri ID dopo l'ultimo commit marcatore di revisione (`review: …`); senza marcatore, su tutta la storia. Per ogni requisito `implementato` nell'ambito, cominciando da quelli nella sezione "DA RIVEDERE":

1. Leggi il testo del requisito e i file elencati.
2. Stabilisci se il comportamento richiesto è ancora presente.
3. Stabilisci se l'elenco dei file è completo: cerca codice che realizza il requisito in file non elencati.

Se l'ambito è ampio, dichiara quali requisiti hai letto nel merito e quali hai controllato solo con lo script.

## 3. Rapporto

Una tabella con ID, esito (`allineato`, `disallineato`, `da rivedere`) e motivo, seguita dall'elenco degli errori e degli avvisi dello script non ancora spiegati.

## 4. Correzioni

Non correggere nulla senza conferma. Proponi, per ogni disallineamento:

- **Solo la tabella o il changelog sono sbagliati** → correzione dei due file e commit `trace(<ID>): …`.
- **Il codice non soddisfa più il requisito** → skill `bugtext`.
- **Il requisito è cambiato di fatto** → skill `req` (nuovo requisito, il vecchio passa ad `annullato`).
- **Commit senza ID o modifiche non committate** → chiedi all'utente a quale ID appartengono.

## 5. Chiusura della revisione

Se la revisione ha coperto tutti i requisiti (nessun argomento), non restano disallineamenti né errori dello script e i test del progetto passano, proponi il commit marcatore e crealo solo dopo conferma:

```bash
git commit --allow-empty -m "review: revisione di merito dei requisiti implementati"
```

Da quel commit in poi la sezione "DA RIVEDERE" riparte vuota e mostra solo le modifiche successive. Il messaggio non deve citare ID: il marcatore non appartiene a un requisito e non modifica file.
