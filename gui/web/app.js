"use strict";

// Funzioni offerte dall'interfaccia (REQ-027): per aggiungerne una basta una voce qui.
const FUNCTIONS = [
  {
    id: "doc-ripper",
    title: "doc-ripper",
    description: "Converte PDF, DOCX e fogli di calcolo di una cartella in Markdown e CSV.",
    render: renderDocRipper,
  },
  {
    id: "search",
    title: "Ricerca",
    description: "Cerca nei file Markdown di una cartella con search-rm3.",
    render: renderSearch,
  },
];

function el(tag, attrs = {}, ...children) {
  const node = document.createElement(tag);
  for (const [key, value] of Object.entries(attrs)) {
    if (key === "class") node.className = value;
    else if (key.startsWith("on")) node.addEventListener(key.slice(2), value);
    else node.setAttribute(key, value);
  }
  node.append(...children);
  return node;
}

function renderMenu(current) {
  const list = document.getElementById("functions");
  list.replaceChildren(
    ...FUNCTIONS.map((f) => {
      const link = el("a", { href: `#${f.id}` }, f.title);
      if (f.id === current) link.setAttribute("aria-current", "page");
      return el("li", {}, link);
    }),
  );
}

function show() {
  const id = location.hash.slice(1);
  const fn = FUNCTIONS.find((f) => f.id === id) || FUNCTIONS[0];
  renderMenu(fn.id);
  const content = document.getElementById("content");
  content.replaceChildren(el("h1", {}, fn.title), el("p", { class: "lead" }, fn.description));
  fn.render(content);
}

// Markdown reso come nodi DOM, per l'anteprima di index.md (REQ-048) e per il testo dei
// risultati della ricerca (REQ-072): titoli, paragrafi, tabelle, elenchi, blocchi di codice,
// link, `codice`, **grassetto** e *corsivo*. I nodi si creano con textContent, così il testo dei
// file non può iniettare HTML; `link(etichetta, href)` decide come mostrare i link.
function renderMarkdown(markdown, link) {
  const splitCells = (row) => {
    const cells = [];
    let cell = "";
    for (let i = 0; i < row.length; i++) {
      if (row[i] === "\\") cell += row[i] + (row[++i] ?? "");
      else if (row[i] === "|") cells.push(cell), (cell = "");
      else cell += row[i];
    }
    cells.push(cell);
    return cells.slice(1, -1).map((c) => c.trim());
  };
  const unescape = (text) => text.replace(/\\(.)/g, "$1");
  // Posizione del delimitatore di chiusura, saltando i caratteri preceduti da `\`.
  const closing = (text, marker, from) => {
    for (let i = from; i < text.length; i++) {
      if (text[i] === "\\") i++;
      else if (text.startsWith(marker, i)) return i;
    }
    return -1;
  };
  // Codice ed enfasi in un tratto di testo senza link; `\x` è il carattere x.
  const emphasis = (text) => {
    const nodes = [];
    let plain = "";
    const flush = () => {
      if (plain) nodes.push(plain);
      plain = "";
    };
    for (let i = 0; i < text.length; ) {
      if (text[i] === "\\" && i + 1 < text.length) {
        plain += text[i + 1];
        i += 2;
        continue;
      }
      if (text[i] === "`") {
        const end = text.indexOf("`", i + 1);
        if (end > i + 1) {
          flush();
          nodes.push(el("code", {}, text.slice(i + 1, end)));
          i = end + 1;
          continue;
        }
      }
      if (text[i] === "*") {
        const marker = text.startsWith("**", i) ? "**" : "*";
        const end = closing(text, marker, i + marker.length);
        if (end > i + marker.length) {
          flush();
          nodes.push(el(marker === "**" ? "strong" : "em", {}, ...emphasis(text.slice(i + marker.length, end))));
          i = end + marker.length;
          continue;
        }
      }
      plain += text[i++];
    }
    flush();
    return nodes;
  };
  // Testo con link `[testo](href)`; l'href può contenere parentesi bilanciate.
  const inline = (text) => {
    const nodes = [];
    let rest = text;
    for (let start; (start = rest.search(/\[(?:\\.|[^\]\\])*\]\(/)) !== -1; ) {
      const close = rest.indexOf("](", start);
      let end = close + 2;
      for (let depth = 0; end < rest.length && !(rest[end] === ")" && depth === 0); end++) {
        if (rest[end] === "(") depth++;
        else if (rest[end] === ")") depth--;
      }
      nodes.push(...emphasis(rest.slice(0, start)));
      nodes.push(link(unescape(rest.slice(start + 1, close)), rest.slice(close + 2, end)));
      rest = rest.slice(end + 1);
    }
    nodes.push(...emphasis(rest));
    return nodes;
  };
  const LIST_ITEM = /^\s*([-*+]|\d+[.)]) +(.*)$/;
  const blocks = [];
  const lines = markdown.split(/\r?\n/);
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const heading = /^(#{1,6}) +(.*)$/.exec(line);
    if (heading) {
      blocks.push(el(`h${Math.min(heading[1].length + 2, 6)}`, {}, ...inline(heading[2])));
    } else if (line.startsWith("```")) {
      const code = [];
      for (i++; i < lines.length && !lines[i].startsWith("```"); i++) code.push(lines[i]);
      blocks.push(el("pre", {}, el("code", {}, code.join("\n"))));
    } else if (line.startsWith("|")) {
      const rows = [];
      for (; i < lines.length && lines[i].startsWith("|"); i++) rows.push(splitCells(lines[i]));
      i--;
      const [head, , ...body] = rows;
      blocks.push(
        el(
          "table",
          {},
          el("thead", {}, el("tr", {}, ...head.map((c) => el("th", {}, ...inline(c))))),
          el("tbody", {}, ...body.map((r) => el("tr", {}, ...r.map((c) => el("td", {}, ...inline(c)))))),
        ),
      );
    } else if (LIST_ITEM.test(line)) {
      const ordered = /\d/.test(LIST_ITEM.exec(line)[1]);
      const items = [];
      for (; i < lines.length && LIST_ITEM.test(lines[i]); i++) items.push(el("li", {}, ...inline(LIST_ITEM.exec(lines[i])[2])));
      i--;
      blocks.push(el(ordered ? "ol" : "ul", {}, ...items));
    } else if (line.trim()) {
      // Righe consecutive di testo formano un solo paragrafo.
      const text = [line.trim()];
      for (i++; i < lines.length && lines[i].trim() && !/^(#|\||```)/.test(lines[i]) && !LIST_ITEM.test(lines[i]); i++) {
        text.push(lines[i].trim());
      }
      i--;
      blocks.push(el("p", {}, ...inline(text.join(" "))));
    }
  }
  return blocks;
}

// Campo cartella con "Sfoglia…": il selettore di Windows lo apre il server (REQ-030).
function folderField(id, label) {
  const input = el("input", { id, type: "text", spellcheck: "false", autocomplete: "off" });
  const browse = el("button", { id: `${id}-browse`, type: "button" }, "Sfoglia…");
  browse.addEventListener("click", async () => {
    browse.disabled = true;
    try {
      const response = await fetch("/api/pick-folder", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ title: label, initial: input.value }),
      });
      if (response.ok) {
        const { path } = await response.json();
        if (path) {
          input.value = path;
          input.dispatchEvent(new Event("input"));
        }
      }
    } finally {
      browse.disabled = false;
    }
  });
  return { input, row: el("div", { class: "field" }, el("label", { for: id }, label), el("div", { class: "field-row" }, input, browse)) };
}

async function renderDocRipper(content) {
  const alert = el("div", { class: "alert error", role: "alert", hidden: "" });
  const src = folderField("src", "Cartella di origine");
  const dst = folderField("dst", "Cartella di destinazione");
  const start = el("button", { id: "start", class: "primary", type: "button", disabled: "" }, "Avvia conversione");
  const cancel = el("button", { id: "cancel", type: "button", hidden: "" }, "Annulla");
  const result = el("div", { id: "result", class: "alert", hidden: "" });
  const summary = el("div", { id: "summary", class: "summary", hidden: "" });
  const output = el("pre", { id: "output", class: "output", hidden: "" });
  const progressBar = el("progress", { max: "100", value: "0" });
  const progressText = el("span");
  const progress = el("div", { id: "progress", class: "progress", hidden: "" }, progressBar, progressText);

  // Apertura dei risultati dell'ultima esecuzione (REQ-033).
  const openResult = async (target, href) => {
    const response = await fetch("/api/open", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ target, href }),
    });
    if (!response.ok) showError(await response.text());
  };
  const openDst = el("button", { id: "open-dst", type: "button", onclick: () => openResult("destination") }, "Apri cartella di destinazione");
  const openIndex = el("button", { id: "open-index", type: "button", onclick: () => openResult("index") }, "Apri index.md");
  const results = el("div", { id: "results", class: "actions", hidden: "" }, openDst, openIndex);
  const previewBody = el("div", { class: "preview-body" });
  const preview = el("section", { id: "preview", class: "preview", hidden: "" }, el("h2", {}, "Anteprima di index.md"), previewBody);

  content.append(alert, src.row, dst.row, el("div", { class: "actions" }, start, cancel), progress, result, results, summary, preview, output);

  const showError = (message) => {
    alert.textContent = message;
    alert.hidden = false;
  };

  // doc-ripper.exe deve trovarsi accanto a gui.exe (REQ-029).
  const status = await fetch("/api/doc-ripper/status").then((r) => r.json());
  if (!status.available) {
    showError(status.message);
    return;
  }

  // Barra di avanzamento dalle righe `[n/totale] <file>` e `[n/totale] pagina p/pagine` (REQ-044):
  // ogni file pesa uguale, dentro un pdf conta la frazione di pagine elaborate.
  const FILE_LINE = /^\[(\d+)\/(\d+)\] /;
  const PAGE_LINE = /^\[(\d+)\/(\d+)\] pagina (\d+)\/(\d+)$/;
  let position = null;
  const track = (line) => {
    const page = PAGE_LINE.exec(line);
    const file = page || FILE_LINE.exec(line);
    if (file) position = { file: +file[1], total: +file[2], page: page ? +page[3] : 0, pages: page ? +page[4] : 0 };
  };
  const drawProgress = (finished) => {
    progress.hidden = !position;
    if (!position) return;
    const { file, total, page, pages } = position;
    const fraction = finished ? 1 : (file - 1 + (pages ? page / pages : 0)) / total;
    const percent = Math.floor(fraction * 100);
    progressBar.setAttribute("value", percent);
    progressText.textContent = `File ${file} di ${total}${pages ? ` — pagina ${page} di ${pages}` : ""} (${percent}%)`;
  };
  // Le righe per pagina servono solo alla barra: non compaiono nell'output.
  const visible = (lines) => lines.filter((l) => !PAGE_LINE.test(l));

  // Output in tempo reale (REQ-031) e codice di uscita a fine esecuzione (REQ-028).
  const follow = () => {
    start.disabled = true;
    cancel.hidden = false;
    cancel.disabled = false;
    output.textContent = "";
    output.hidden = false;
    position = null;
    drawProgress(false);
    noIndex();
    const events = new EventSource("/api/doc-ripper/events");
    openStreams.add(events);
    events.addEventListener("line", (e) => {
      track(e.data);
      drawProgress(false);
      if (PAGE_LINE.test(e.data)) return;
      output.textContent += (output.textContent ? "\n" : "") + e.data;
      output.scrollTop = output.scrollHeight;
    });
    events.addEventListener("end", async () => {
      events.close();
      openStreams.delete(events);
      showResult(await fetch("/api/doc-ripper/output").then((r) => r.json()));
      start.disabled = false;
      cancel.hidden = true;
    });
  };

  // Interruzione dell'esecuzione in corso (REQ-034).
  cancel.addEventListener("click", async () => {
    cancel.disabled = true;
    const response = await fetch("/api/doc-ripper/cancel", { method: "POST" });
    if (!response.ok) showError(await response.text());
  });

  // Anteprima di index.md (REQ-048): i link li apre il server, che accetta solo quelli presenti
  // nell'indice.
  const renderIndex = (markdown) => {
    const link = (label, href) => el("a", { href, onclick: (e) => (e.preventDefault(), openResult("link", href)) }, label);
    previewBody.replaceChildren(...renderMarkdown(markdown, link));
  };
  // All'avvio e quando index.md manca l'anteprima lo dice (REQ-049).
  const noIndex = () => {
    previewBody.replaceChildren(el("p", { class: "muted" }, "Nessun indice disponibile"));
    preview.hidden = false;
  };
  const loadPreview = async () => {
    const response = await fetch("/api/doc-ripper/index");
    if (!response.ok) {
      noIndex();
      return;
    }
    renderIndex(await response.text());
    preview.hidden = false;
  };

  // Esito in base al codice di uscita e riepilogo di doc-ripper (REQ-032).
  const showResult = (snapshot) => {
    const code = snapshot.exit_code;
    if (snapshot.cancelled) {
      result.className = "alert warn";
      result.textContent = "Conversione annullata: i file già generati restano nella destinazione.";
    } else {
      result.className = code === 0 ? "alert ok" : "alert error";
      result.textContent =
        code === 0 ? "Conversione completata." : `Conversione terminata con errori (codice di uscita ${code ?? "sconosciuto"}).`;
    }
    result.hidden = false;
    results.hidden = !snapshot.dst;
    output.textContent = visible(snapshot.lines).join("\n");
    position = null;
    snapshot.lines.forEach(track);
    drawProgress(!snapshot.cancelled);
    output.hidden = false;
    loadPreview();
    const s = snapshot.summary;
    if (!s) {
      summary.hidden = true;
      return;
    }
    const section = (title, items, render) =>
      el("section", {}, el("h2", {}, `${title}: ${items.length}`), el("ul", {}, ...items.map((i) => el("li", {}, render(i)))));
    summary.replaceChildren(
      section("Convertiti", s.converted, (c) => `${c.file} → ${c.outputs.join(", ")}`),
      section("Saltati", s.skipped, (e) => `${e.file} — ${e.reason}`),
      section("Falliti", s.failed, (e) => `${e.file} — ${e.reason}`),
    );
    summary.hidden = false;
  };

  start.addEventListener("click", async () => {
    alert.hidden = true;
    result.hidden = true;
    results.hidden = true;
    summary.hidden = true;
    const response = await fetch("/api/doc-ripper/run", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ src: src.input.value, dst: dst.input.value }),
    });
    if (!response.ok) {
      showError(await response.text());
      return;
    }
    follow();
  });

  // Ultime cartelle usate, anche dopo un riavvio del server (REQ-035).
  const saved = await fetch("/api/settings").then((r) => r.json());
  if (!src.input.value) src.input.defaultValue = saved.src;
  if (!dst.input.value) dst.input.defaultValue = saved.dst;

  // Pagina ricaricata: si ricollega all'esecuzione in corso o mostra l'esito dell'ultima.
  const snapshot = await fetch("/api/doc-ripper/output").then((r) => r.json());
  if (snapshot.running) {
    follow();
  } else {
    if (snapshot.lines.length > 0) showResult(snapshot);
    start.disabled = false;
  }
}

// Ricerca con search-rm3, eseguito dal server come processo separato (REQ-069).
async function renderSearch(content) {
  const alert = el("div", { class: "alert error", role: "alert", hidden: "" });
  // Cartella da indicizzare con "Sfoglia…" (REQ-071); "Indicizza" sulla stessa riga.
  const { input: folder, row: folderRow } = folderField("search-src", "Cartella da indicizzare");
  const indexButton = el("button", { id: "search-index", type: "button", disabled: "" }, "Indicizza");
  folderRow.querySelector(".field-row").append(indexButton);
  const question = el("input", { id: "search-q", type: "search", autocomplete: "off" });
  const searchButton = el("button", { id: "search-run", class: "primary", type: "submit", disabled: "" }, "Cerca");
  // Numero di risultati: solo interi >= 1, passato a search-rm3 come --top (REQ-072).
  const top = el("input", { id: "search-top", class: "top", type: "number", min: "1", step: "1", required: "" });
  const form = el(
    "form",
    {},
    el("div", { class: "field" }, el("label", { for: "search-q" }, "Domanda"), el("div", { class: "field-row" }, question)),
    el("div", { class: "field" }, el("label", { for: "search-top" }, "Numero di risultati"), el("div", { class: "field-row" }, top, searchButton)),
  );
  const result = el("div", { id: "search-result", class: "alert", hidden: "" });
  const list = el("ol", { id: "search-results", class: "search-results" });
  content.append(alert, folderRow, form, result, list);

  const showError = (message) => {
    alert.textContent = message;
    alert.hidden = false;
  };
  const post = (url, body) =>
    fetch(url, { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify(body) });
  // Una richiesta alla volta: durante l'esecuzione di search-rm3 i pulsanti restano disabilitati.
  const busy = async (action) => {
    alert.hidden = true;
    result.hidden = true;
    indexButton.disabled = searchButton.disabled = true;
    try {
      await action();
    } finally {
      indexButton.disabled = searchButton.disabled = false;
    }
  };

  indexButton.addEventListener("click", () =>
    busy(async () => {
      const response = await post("/api/search/index", { src: folder.value });
      if (!response.ok) return showError(await response.text());
      result.className = "alert ok";
      result.textContent = "Indicizzazione completata.";
      result.hidden = false;
      refreshStale();
    }),
  );

  // "Indicizza" in rosso se l'indice della cartella scritta nel campo va rifatto (REQ-074).
  const refreshStale = async () => {
    const src = folder.value;
    const { stale } = await fetch(`/api/search/stale?src=${encodeURIComponent(src)}`).then((r) => r.json());
    if (folder.value !== src) return;
    if (stale) {
      indexButton.classList.add("stale");
      indexButton.title = "L'indice di questa cartella va rifatto";
    } else {
      indexButton.classList.remove("stale");
      indexButton.removeAttribute("title");
    }
  };
  folder.addEventListener("input", refreshStale);

  form.addEventListener("submit", (event) => {
    event.preventDefault();
    busy(async () => {
      const src = folder.value;
      const response = await post("/api/search/query", { src, q: question.value, top: Number(top.value) });
      if (!response.ok) return showError(await response.text());
      showResults(src, (await response.json()).results);
    });
  });

  // Per ogni risultato: file cliccabile, intestazione, riga, punteggio e testo della sezione
  // con il Markdown reso (REQ-072). L'intestazione è già sopra: dal testo si toglie la sua riga.
  const showResults = (src, results) => {
    const open = async (file) => {
      const response = await post("/api/search/open", { src, file });
      if (!response.ok) showError(await response.text());
    };
    const hit = (r) => {
      const body = /^#{1,6} /.test(r.text) ? r.text.slice(r.text.indexOf("\n") + 1 || r.text.length) : r.text;
      return el(
        "li",
        { class: "hit" },
        el(
          "div",
          { class: "hit-meta" },
          el("a", { href: "#", class: "hit-file", onclick: (e) => (e.preventDefault(), open(r.file)) }, r.file),
          el("span", {}, `riga ${r.line}`),
          el("span", {}, `punteggio ${r.score.toFixed(3)}`),
        ),
        el("h2", { class: "hit-heading" }, r.heading || "(senza intestazione)"),
        el("div", { class: "hit-text" }, ...renderMarkdown(body, (label) => label)),
      );
    };
    list.replaceChildren(...(results.length ? results.map(hit) : [el("li", { class: "muted" }, "Nessun risultato")]));
  };

  // Proposta: l'ultima cartella di destinazione usata da doc-ripper (REQ-071, REQ-035) e
  // l'ultimo numero di risultati, 10 al primo avvio (REQ-072).
  const saved = await fetch("/api/settings").then((r) => r.json());
  if (!folder.value) folder.defaultValue = saved.dst;
  if (!top.value) top.defaultValue = saved.top ?? 10;

  // Pagina ricaricata: torna l'ultima ricerca riuscita dall'avvio del server (REQ-077).
  const last = await fetch("/api/search/last").then((r) => r.json());
  if (last) {
    folder.defaultValue = last.src;
    question.defaultValue = last.q;
    top.defaultValue = last.top;
    showResults(last.src, last.response.results);
  }
  await refreshStale();

  // search-rm3.exe deve trovarsi accanto a gui.exe (REQ-070).
  const status = await fetch("/api/search/status").then((r) => r.json());
  if (!status.available) {
    showError(status.message);
    return;
  }
  indexButton.disabled = searchButton.disabled = false;
}

// Connessioni in tempo reale aperte: vanno chiuse prima di spegnere il server.
const openStreams = new Set();

// Pulsante "Esci" (REQ-039): spegne il server, interrompendo un doc-ripper in corso.
document.getElementById("quit").addEventListener("click", async () => {
  if (!confirm("Chiudere RocketData? Un'eventuale conversione in corso verrà interrotta.")) return;
  openStreams.forEach((s) => s.close());
  await fetch("/api/shutdown", { method: "POST" }).catch(() => {});
  document.body.replaceChildren(
    el("div", { class: "closed" }, el("h1", {}, "RocketData è stato chiuso"), el("p", { class: "lead" }, "Puoi chiudere questa scheda.")),
  );
});

window.addEventListener("hashchange", show);
show();
