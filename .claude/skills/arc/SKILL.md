---
name: arc
description: Analizza i requisiti aperti e spiega come verranno implementati, senza modificare nulla. Usare quando l'utente lancia /arc o subito dopo la registrazione di nuovi requisiti con req.
argument-hint: "[REQ-NNN | <parte>_REQ-NNN ...]"
disallowed-tools: Edit Write NotebookEdit
---

# /arc — analisi e piano di implementazione

Leggi prima `${CLAUDE_SKILL_DIR}/../req/conventions.md`.

Requisiti da analizzare: $ARGUMENTS

Senza argomenti, analizza tutti i requisiti con stato `aperto` in `docs/REQUIREMENTS.md` e nei `<cartella>/docs/REQUIREMENTS.md`.

Questa fase è di sola lettura: nessuna modifica ai file, nessun commit.

## Passi

1. Leggi i requisiti indicati e il codice che li riguarda.
2. Per ogni requisito presenta:
   - **Approccio**: come verrà soddisfatto, in poche righe.
   - **File**: quali verranno creati o modificati e perché.
   - **Impatti**: requisiti già `implementato` che elencano gli stessi file, con il rischio di regressione.
   - **Verifica**: come si dimostrerà che il requisito è soddisfatto (test, comando, controllo manuale).
   - **Alternative scartate**: solo se la scelta non è ovvia, una riga ciascuna.
3. Indica l'ordine di implementazione se i requisiti dipendono l'uno dall'altro.
4. Elenca le domande aperte che cambierebbero il piano.

Scegli la soluzione più semplice che soddisfa il requisito: niente astrazioni o funzioni non richieste.

## Dopo

Chiedi all'utente di approvare o correggere il piano e fermati. Dopo l'approvazione prosegui con la skill `impl`.
