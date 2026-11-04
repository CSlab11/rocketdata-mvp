---
name: bugfix
description: Implementa la correzione dei bug già registrati nella tabella di tracciabilità, con un commit per bug. Usare quando l'utente lancia /bugfix o al termine di bugtext.
argument-hint: "[BUG-NNN | <parte>_BUG-NNN ...]"
---

# /bugfix — correzione tracciata

Leggi prima `${CLAUDE_SKILL_DIR}/../req/conventions.md` e applicane i controlli preliminari.

Bug da correggere: $ARGUMENTS

Senza argomenti, correggi i bug con stato `aperto` in `docs/REQUIREMENTS.md` e nei `<cartella>/docs/REQUIREMENTS.md`. Un bug non ancora registrato passa prima dalla skill `bugtext`.

## Per ogni bug

1. **Correggi** secondo la colonna "Modifiche necessarie", limitandoti al difetto: niente refactoring o migliorie nello stesso commit.
2. **Verifica** che il comportamento atteso sia ripristinato. Se il progetto ha una suite di test, aggiungi un test che fallisce senza la correzione.
3. **Aggiorna la tabella**: stato `risolto`, "File impattati" con i file realmente toccati, e "Modifiche necessarie" se la correzione è risultata diversa da quella descritta.
4. **Aggiorna il changelog** con la riga `risolto`.
5. **Committa** codice, tabella e changelog insieme: `fix(BUG-NNN): <sintesi>`, con l'ID qualificato per una parte (`fix(<parte>_BUG-NNN): …`).

## Regole

- Se la correzione richiede più di quanto descritto (altri file, cambio di interfaccia), fermati e spiegalo prima di procedere.
- Se i file corretti sono elencati da requisiti `implementato`, controlla che quei requisiti restino soddisfatti.

## Dopo

Esegui lo script di verifica della skill `checkreq` e riporta, per ogni bug: ID, file impattati, hash del commit ed esito della verifica.
