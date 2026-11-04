---
name: req
description: Registra nuovi requisiti nella tabella di tracciabilità prima di qualsiasi modifica al codice. Usare quando l'utente lancia /req o chiede una nuova funzionalità o un cambio di comportamento in un progetto.
argument-hint: "[testo dei requisiti]"
---

# /req — registrazione dei requisiti

Leggi prima `${CLAUDE_SKILL_DIR}/conventions.md` e applicane i controlli preliminari.

Requisiti in ingresso: $ARGUMENTS

Se non c'è testo, chiedi all'utente i requisiti e fermati.

## Passi

1. **Scomponi** il testo in requisiti atomici e verificabili, uno per riga, formulati come "Il sistema deve …". Non aggiungere requisiti che l'utente non ha chiesto.
2. **Chiarisci** solo ciò che impedisce di scrivere un requisito verificabile: fai le domande in un unico messaggio e attendi. Altrimenti prosegui.
3. **Controlla i conflitti** con i requisiti già in tabella (duplicati, contraddizioni, requisiti che questo sostituisce) e segnalali. Un requisito sostituito passa ad `annullato`.
4. **Registra**: aggiungi le righe nel file dei requisiti della parte interessata (`<cartella>/docs/REQUIREMENTS.md`, oppure `docs/REQUIREMENTS.md` per i requisiti di progetto; vedi "Struttura ad albero" nelle convenzioni), con il primo ID libero in quel file (vedi "Numerazione degli ID"), stato `aperto` e file `—`, e una riga `richiesto` per ciascun ID in `CHANGELOG.md` (ID qualificato per una parte).
5. **Committa** solo i due file di tracciabilità: `req(REQ-NNN): <titolo>`, con l'ID qualificato per una parte (`req(<parte>_REQ-NNN): …`).
6. **Mostra** all'utente gli ID creati con il testo registrato.

Non toccare il codice in questa fase.

## Dopo

Prosegui subito con la skill `arc` sugli ID appena creati.
