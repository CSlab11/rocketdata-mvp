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
  const result = el("div", { id: "result", class: "alert", hidden: "" });
  const output = el("pre", { id: "output", class: "output", hidden: "" });
  content.append(alert, src.row, dst.row, el("div", { class: "actions" }, start), result, output);

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
    output.textContent = "";
    output.hidden = false;
    const events = new EventSource("/api/doc-ripper/events");
    events.addEventListener("line", (e) => {
      output.textContent += (output.textContent ? "\n" : "") + e.data;
      output.scrollTop = output.scrollHeight;
    });
    events.addEventListener("end", (e) => {
      events.close();
      const code = JSON.parse(e.data);
      result.className = code === 0 ? "alert ok" : "alert error";
      result.textContent = `doc-ripper terminato con codice di uscita ${code ?? "sconosciuto"}`;
      result.hidden = false;
      start.disabled = false;
    });
  };

  start.addEventListener("click", async () => {
    alert.hidden = true;
    result.hidden = true;
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

  // Se un'esecuzione è in corso (es. pagina ricaricata), si ricollega all'output.
  const snapshot = await fetch("/api/doc-ripper/output").then((r) => r.json());
  if (snapshot.running) follow();
  else start.disabled = false;
}

window.addEventListener("hashchange", show);
show();
