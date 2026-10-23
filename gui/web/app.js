"use strict";

// Funzioni offerte dall'interfaccia (REQ-027): per aggiungerne una basta una voce qui.
const FUNCTIONS = [
  {
    id: "doc-ripper",
    title: "doc-ripper",
    description: "Converte PDF, DOCX e fogli di calcolo di una cartella in Markdown e CSV.",
    render: renderDocRipper,
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
        if (path) input.value = path;
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
    preview.hidden = true;
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

  // Anteprima di index.md (REQ-048): titoli, tabella e link nel formato scritto da doc-ripper.
  // I nodi si creano con textContent, così i nomi dei file non possono iniettare HTML; i link
  // li apre il server, che accetta solo quelli presenti nell'indice.
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
      const label = unescape(rest.slice(start + 1, close));
      const href = rest.slice(close + 2, end);
      nodes.push(unescape(rest.slice(0, start)));
      nodes.push(el("a", { href, onclick: (e) => (e.preventDefault(), openResult("link", href)) }, label));
      rest = rest.slice(end + 1);
    }
    nodes.push(unescape(rest));
    return nodes;
  };
  const renderIndex = (markdown) => {
    const blocks = [];
    const lines = markdown.split(/\r?\n/);
    for (let i = 0; i < lines.length; i++) {
      const line = lines[i];
      const heading = /^(#{1,5}) (.*)$/.exec(line);
      if (heading) {
        blocks.push(el(`h${heading[1].length + 2}`, {}, ...inline(heading[2])));
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
      } else if (line.trim()) {
        blocks.push(el("p", {}, ...inline(line)));
      }
    }
    previewBody.replaceChildren(...blocks);
  };
  const loadPreview = async () => {
    const response = await fetch("/api/doc-ripper/index");
    if (!response.ok) {
      preview.hidden = true;
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
