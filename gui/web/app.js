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

async function renderDocRipper(content) {
  const alert = el("div", { class: "alert error", role: "alert", hidden: "" });
  const start = el("button", { id: "start", class: "primary", type: "button", disabled: "" }, "Avvia conversione");
  content.append(alert, el("div", { class: "actions" }, start));

  // doc-ripper.exe deve trovarsi accanto a gui.exe (REQ-029).
  const status = await fetch("/api/doc-ripper/status").then((r) => r.json());
  if (status.available) {
    start.disabled = false;
  } else {
    alert.textContent = status.message;
    alert.hidden = false;
  }
}

window.addEventListener("hashchange", show);
show();
