"""Componenti comuni: configurazione, modello di embedding, archivio, categorie."""
from __future__ import annotations

import hashlib
import re
import json
import os
import sqlite3
from functools import lru_cache
from pathlib import Path

# Nessuna chiamata di rete: il modello deve essere già nella cartella locale.
os.environ.setdefault("HF_HUB_OFFLINE", "1")
os.environ.setdefault("TRANSFORMERS_OFFLINE", "1")

import sys

import numpy as np
import yaml

ROOT = Path(__file__).resolve().parent

# console di Windows: evita errori di codifica su caratteri come ✓ e ›
for _s in (sys.stdout, sys.stderr):
    try:
        _s.reconfigure(encoding="utf-8", errors="replace")
    except Exception:
        pass


# ───────────────────────── configurazione ─────────────────────────

@lru_cache(maxsize=1)
def config() -> dict:
    with open(ROOT / "config.yaml", encoding="utf-8") as f:
        return yaml.safe_load(f)


def index_dir() -> Path:
    d = ROOT / config()["indice"]
    d.mkdir(parents=True, exist_ok=True)
    (d / "emb").mkdir(exist_ok=True)
    return d


def sha1_file(path: Path) -> str:
    h = hashlib.sha1()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def sha1_text(t: str) -> str:
    return hashlib.sha1(t.encode("utf-8")).hexdigest()


# ───────────────────────── modello ─────────────────────────

class _HashModel:
    """Embedder fittizio (bag of words con hashing) per collaudare la pipeline senza modello.
    Si attiva con la variabile d'ambiente ECSS_TEST_EMBEDDER=1. Non usarlo per lavoro reale."""

    dim = 512

    def get_sentence_embedding_dimension(self):
        return self.dim

    def encode(self, texts, **_):
        out = np.zeros((len(texts), self.dim), dtype=np.float32)
        for i, t in enumerate(texts):
            for w in re.findall(r"[a-z]{3,}", t.lower()):
                out[i, int(hashlib.md5(w.encode()).hexdigest(), 16) % self.dim] += 1
        out /= np.linalg.norm(out, axis=1, keepdims=True) + 1e-9
        return out


class Embedder:
    def __init__(self):
        if os.environ.get("ECSS_TEST_EMBEDDER") == "1":
            self.model, self.q_prefix, self.d_prefix, self.batch = _HashModel(), "", "", 32
            return
        from sentence_transformers import SentenceTransformer

        m = config()["modello"]
        path = ROOT / m["cartella"]
        if not path.exists():
            raise SystemExit(f"Modello non trovato in {path}. Eseguire una volta (online): python setup_model.py")
        self.model = SentenceTransformer(str(path), device=None)
        self.model.max_seq_length = m.get("max_token", 512)
        self.q_prefix = m.get("prefisso_query", "")
        self.d_prefix = m.get("prefisso_documento", "")
        self.batch = m.get("batch", 32)

    def _enc(self, texts: list[str], prefix: str, progress: bool) -> np.ndarray:
        if not texts:
            return np.zeros((0, self.model.get_sentence_embedding_dimension()), dtype=np.float32)
        v = self.model.encode([prefix + t for t in texts], batch_size=self.batch,
                              normalize_embeddings=True, show_progress_bar=progress, convert_to_numpy=True)
        return v.astype(np.float32)

    def documents(self, texts: list[str], progress: bool = False) -> np.ndarray:
        return self._enc(texts, self.d_prefix, progress)

    def queries(self, texts: list[str], progress: bool = False) -> np.ndarray:
        return self._enc(texts, self.q_prefix, progress)


# ───────────────────────── archivio ─────────────────────────

SCHEMA = """
CREATE TABLE IF NOT EXISTS files (
    standard TEXT PRIMARY KEY, file TEXT, sha1 TEXT, n_unita INTEGER, modello TEXT
);
CREATE TABLE IF NOT EXISTS unita (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    standard TEXT, file TEXT, tipo TEXT, clausola TEXT, percorso TEXT, pagina INTEGER,
    termine TEXT, req_id TEXT, testo TEXT, riga INTEGER  -- riga nel file emb/<standard>.npy
);
CREATE INDEX IF NOT EXISTS ix_unita_std ON unita(standard);
CREATE INDEX IF NOT EXISTS ix_unita_tipo ON unita(tipo);
CREATE TABLE IF NOT EXISTS categorie (
    id TEXT PRIMARY KEY, nome TEXT, parent TEXT, gruppo TEXT, descrizione TEXT
);
CREATE TABLE IF NOT EXISTS termine_categoria (
    unita_id INTEGER, categoria TEXT, score REAL, specificita REAL, origine TEXT,
    PRIMARY KEY (unita_id, categoria)
);
CREATE TABLE IF NOT EXISTS cache_paragrafi (sha1 TEXT PRIMARY KEY, vettore BLOB);
CREATE TABLE IF NOT EXISTS parole (
    id INTEGER PRIMARY KEY, parola TEXT UNIQUE, fonti TEXT, occorrenze INTEGER, n_standard INTEGER, esempi TEXT
);
CREATE TABLE IF NOT EXISTS tema_unita (tema TEXT, unita_id INTEGER, score REAL, origine TEXT,
    PRIMARY KEY (tema, unita_id));
"""


def db() -> sqlite3.Connection:
    con = sqlite3.connect(index_dir() / "ecss.sqlite")
    con.row_factory = sqlite3.Row
    con.executescript(SCHEMA)
    return con


def emb_path(standard: str) -> Path:
    safe = standard.replace(" ", "_").replace("/", "-")
    return index_dir() / "emb" / f"{safe}.npy"


def load_matrix(con: sqlite3.Connection, where: str = "", params: tuple = ()) -> tuple[list[sqlite3.Row], np.ndarray]:
    """Righe di `unita` (filtrate) + matrice degli embeddings allineata."""
    rows = con.execute(f"SELECT * FROM unita {where} ORDER BY standard, riga", params).fetchall()
    if not rows:
        return [], np.zeros((0, 0), dtype=np.float32)
    cache: dict[str, np.ndarray] = {}
    vecs = []
    for r in rows:
        if r["standard"] not in cache:
            cache[r["standard"]] = np.load(emb_path(r["standard"]), mmap_mode="r")
        vecs.append(cache[r["standard"]][r["riga"]])
    return rows, np.asarray(vecs, dtype=np.float32)


# ───────────────────────── categorie ─────────────────────────

@lru_cache(maxsize=1)
def categories() -> dict:
    """Restituisce {id: {nome, parent, gruppo, membri}}.

    `membri` è una lista di (standard, prefissi_clausola | None): None = tutto lo standard.
    gruppo: 'ramo' (radici), 'disciplina' (con standard propri), 'nodo' (intermedi), 'trasversale'.
    """
    with open(ROOT / config()["categorie"], encoding="utf-8") as f:
        y = yaml.safe_load(f)
    cats = {c["id"]: dict(c) for c in y["categorie"]}
    children: dict[str, list[str]] = {}
    for c in cats.values():
        if c.get("parent"):
            children.setdefault(c["parent"], []).append(c["id"])

    def members(cid: str) -> list[tuple[str, list[str] | None]]:
        c = cats[cid]
        out = [(s["codice"], None) for s in c.get("standard", [])]
        for ch in children.get(cid, []):
            out += members(ch)
        return out

    res = {}
    for cid, c in cats.items():
        gruppo = "ramo" if not c.get("parent") else ("disciplina" if c.get("standard") else "nodo")
        res[cid] = {"id": cid, "nome": c["nome"], "parent": c.get("parent"), "gruppo": gruppo, "membri": members(cid)}
    for t in y.get("categorie_trasversali", []) or []:
        m: list[tuple[str, list[str] | None]] = []
        for inc in t.get("include", []) or []:
            m += res[inc]["membri"]
        for s in t.get("standard", []) or []:
            m.append((s, None))
        for cl in t.get("clausole", []) or []:
            m.append((cl["codice"], [str(x) for x in cl.get("clausole", [])] or None))
        res[t["id"]] = {"id": t["id"], "nome": t["nome"], "parent": None, "gruppo": "trasversale", "membri": m}
    for t in y.get("temi", []) or []:
        anc: list[tuple[str, list[str] | None]] = []
        for a in t.get("ancore", []) or []:
            if isinstance(a, str):
                anc.append((a, None))
            else:
                anc.append((a["codice"], [str(x) for x in a.get("clausole", [])] or None))
        res[t["id"]] = {"id": t["id"], "nome": t["nome"], "parent": None, "gruppo": "tema", "membri": [],
                        "descrizione": t.get("descrizione", ""), "semi": list(t.get("semi", [])), "ancore": anc}
    return res


_THEME_MEMBERS: dict[str, set[int]] | None = None


def theme_members(reload: bool = False) -> dict[str, set[int]]:
    """Unità assegnate a ogni tema (calcolate in ingestione, tabella tema_unita)."""
    global _THEME_MEMBERS
    if _THEME_MEMBERS is None or reload:
        _THEME_MEMBERS = {}
        for r in db().execute("SELECT tema, unita_id FROM tema_unita"):
            _THEME_MEMBERS.setdefault(r["tema"], set()).add(r["unita_id"])
    return _THEME_MEMBERS


def all_standards() -> list[str]:
    seen, out = set(), []
    for c in categories().values():
        for s, _ in c["membri"]:
            if s not in seen:
                seen.add(s)
                out.append(s)
    return out


def categories_overlap(a: dict, b: dict) -> bool:
    """True se due categorie condividono contenuto (stesso standard e clausole sovrapposte)."""
    def ov(pa, pb):
        if pa is None or pb is None:
            return True
        return any(x == y or x.startswith(y + ".") or y.startswith(x + ".") for x in pa for y in pb)
    for sa, pa in a["membri"]:
        for sb, pb in b["membri"]:
            if sa == sb and ov(pa, pb):
                return True
    return False


def matches_members(row, members) -> bool:
    """True se l'unità rientra in una lista di (standard, prefissi di clausola | None)."""
    cl = row["clausola"] or ""
    for std, prefixes in members:
        if row["standard"] == std and (prefixes is None or any(cl == p or cl.startswith(p + ".") for p in prefixes)):
            return True
    return False


def unit_in_category(row, cat: dict) -> bool:
    if cat["gruppo"] == "tema":
        return row["id"] in theme_members().get(cat["id"], set())
    for std, prefixes in cat["membri"]:
        if row["standard"] != std:
            continue
        if prefixes is None:
            return True
        cl = row["clausola"] or ""
        if any(cl == p or cl.startswith(p + ".") for p in prefixes):
            return True
    return False


def unit_categories(rows) -> dict[int, list[str]]:
    """Per ogni unità, le categorie (di tutti i livelli) a cui appartiene. Versione veloce di unit_in_category."""
    by_std: dict[str, list[tuple[str, list[str] | None]]] = {}
    for cid, c in categories().items():
        for std, prefixes in c["membri"]:
            by_std.setdefault(std, []).append((cid, prefixes))
    themes = theme_members()
    out: dict[int, list[str]] = {}
    for r in rows:
        cl = r["clausola"] or ""
        found = []
        for cid, prefixes in by_std.get(r["standard"], ()):
            if prefixes is None or any(cl == p or cl.startswith(p + ".") for p in prefixes):
                found.append(cid)
        found += [t for t, ids in themes.items() if r["id"] in ids]
        out[r["id"]] = sorted(set(found))
    return out


def categories_for_level(level: str) -> list[str]:
    cats = categories()
    groups = {
        "discipline": {"disciplina"},
        "rami": {"ramo"},
        "trasversali": {"trasversale"},
        "temi": {"tema"},
        "tutte": {"ramo", "nodo", "disciplina", "trasversale", "tema"},
    }[level]
    return [cid for cid, c in cats.items() if c["gruppo"] in groups]


def load_prototypes() -> tuple[list[str], np.ndarray]:
    p = index_dir() / "prototipi.npz"
    if not p.exists():
        raise SystemExit("Prototipi non trovati: eseguire prima ingest.py")
    z = np.load(p, allow_pickle=False)
    return [str(x) for x in z["categorie"]], z["vettori"]


def category_scores(vecs: np.ndarray, cat_ids: list[str] | None = None) -> tuple[list[str], np.ndarray]:
    """Score di ogni vettore verso ogni categoria = max coseno sui prototipi della categoria."""
    pc, pv = load_prototypes()
    available = set(pc)  # categorie senza unità indicizzate non hanno prototipi
    wanted = [c for c in (cat_ids or sorted(available)) if c in available]
    sims = vecs @ pv.T  # (n, n_prototipi)
    out = np.full((vecs.shape[0], len(wanted)), -1.0, dtype=np.float32)
    idx = {c: i for i, c in enumerate(wanted)}
    for j, c in enumerate(pc):
        if c in idx:
            k = idx[c]
            out[:, k] = np.maximum(out[:, k], sims[:, j])
    return wanted, out


def dumps(o) -> str:
    return json.dumps(o, ensure_ascii=False, indent=2)
