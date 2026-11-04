---
name: bugtext
description: Registra un bug nella tabella di tracciabilità con file impattati e modifiche necessarie, poi passa alla correzione. Usare quando l'utente lancia /bugtext, segnala un difetto o un malfunzionamento, oppure quando emerge un bug durante impl o checkreq.
argument-hint: "[descrizione del bug]"
---

# /bugtext — segnalazione e analisi del bug

Leggi prima `${CLAUDE_SKILL_DIR}/../req/conventions.md` e applicane i controlli preliminari.

Bug segnalato: $ARGUMENTS

Se non c'è descrizione, chiedila e fermati.

## Passi

1. **Riproduci o localizza** il difetto e risali alla causa, non al sintomo. Se non riesci a riprodurlo, dillo e riferisci cosa hai verificato.
2. **Distingui**: se il comportamento è corretto rispetto ai requisiti registrati e l'utente ne vuole uno diverso, non è un bug: passa alla skill `req`.
3. **Cerca bug collegati** con la stessa causa e registra ciascuno con il proprio ID.
4. **Registra** nella tabella Bug del file dei requisiti della parte in cui sta il difetto (`<cartella>/docs/REQUIREMENTS.md`, creando la tabella se manca), oppure di `docs/REQUIREMENTS.md` se riguarda più parti o nessuna; primo ID libero in quel file, stato `aperto`:
   - Descrizione: comportamento osservato e comportamento atteso.
   - Req: il requisito violato, o `—`.
   - File impattati: i file che contengono il difetto.
   - Modifiche necessarie: cosa va cambiato e dove, in modo che la correzione sia eseguibile senza rifare l'analisi.
5. **Aggiorna il changelog** con la riga `segnalato` e i file impattati.
6. **Committa** solo i due file di tracciabilità: `bug(BUG-NNN): <titolo>`, con l'ID qualificato per una parte (`bug(<parte>_BUG-NNN): …`).
7. **Riporta** all'utente: bug trovati, file impattati, modifiche necessarie.

Non correggere il codice in questa fase.

## Dopo

Prosegui con la skill `bugfix` sugli ID appena creati. Fermati prima e chiedi conferma solo se la correzione cambia un requisito registrato, oppure se l'utente ha chiesto la sola segnalazione.
