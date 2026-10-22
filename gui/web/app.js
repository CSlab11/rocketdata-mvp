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

function folderField(id, label) {
  const input = el("input", { id, type: "text", spellcheck: "false", autocomplete: "off" });
  return { input, row: el("div", { class: "field" }, el("label", { for: id }, label), el("div", { class: "field-row" }, input)) };
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
  content.append(alert, src.row, dst.row, el("div", { class: "actions" }, start, cancel), result, summary, output);

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

  // Output in tempo reale (REQ-031) e codice di uscita a fine esecuzione (REQ-028).
  const follow = () => {
    start.disabled = true;
    cancel.hidden = false;
    cancel.disabled = false;
    output.textContent = "";
    output.hidden = false;
    const events = new EventSource("/api/doc-ripper/events");
    openStreams.add(events);
    events.addEventListener("line", (e) => {
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
    output.textContent = snapshot.lines.join("\n");
    output.hidden = false;
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
