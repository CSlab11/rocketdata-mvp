---
name: impl
description: Implementa i requisiti il cui piano è stato approvato, aggiorna i file impattati nella tabella dei requisiti e crea un commit per requisito. Usare quando l'utente lancia /impl o approva il piano prodotto da arc.
argument-hint: "[REQ-NNN | <parte>_REQ-NNN ...]"
---

# /impl — implementazione tracciata

Leggi prima `${CLAUDE_SKILL_DIR}/../req/conventions.md` e applicane i controlli preliminari.

Requisiti da implementare: $ARGUMENTS

Senza argomenti, implementa i requisiti del piano approvato in questa conversazione. Se non c'è un piano approvato, esegui prima la skill `arc` e attendi l'approvazione.

## Per ogni requisito, nell'ordine del piano

1. **Implementa** solo ciò che serve al requisito, seguendo il piano approvato.
2. **Verifica** come previsto dal piano (test, comando, controllo). Se la verifica fallisce, non committare: correggi o fermati e riferisci.
3. **Aggiorna la tabella**: stato `implementato` e, in "File impattati", i file realmente toccati, ricavati da `git status` dopo lo staging e non dal piano.
4. **Aggiorna il changelog** con la riga `implementato`.
5. **Committa** codice, tabella e changelog insieme: `feat(REQ-NNN): <sintesi>`, con l'ID qualificato per una parte (`feat(<parte>_REQ-NNN): …`).

## Regole

- Se serve scostarsi dal piano in modo sostanziale (altri file, altro approccio), fermati e spiegalo prima di procedere.
- Se trovi un difetto nel codice esistente, non correggerlo dentro questo commit: registralo con la skill `bugtext`.
- Se il requisito si rivela più ampio di quanto registrato, la parte in più è un nuovo requisito: passa da `req`.

## Dopo

Esegui lo script di verifica della skill `checkreq` e riporta, per ogni requisito: ID, file impattati, hash del commit ed esito della verifica.
