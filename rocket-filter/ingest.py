"""Ingestione degli standard ECSS: PDF -> unità -> embeddings -> prototipi di categoria.

Uso:
    python ingest.py --pdf "C:\\...\\standards\\active"
    python ingest.py --pdf <cartella> --forza          # ricalcola tutto
    python ingest.py --solo-prototipi                  # rigenera solo temi, prototipi e lessici
    python ingest.py --pdf <cartella> --solo "ECSS-S-ST-00-01C Rev.1"   # rielabora standard specifici

Incrementale: uno standard viene rielaborato solo se il suo PDF (o il modello) è cambiato.
"""
from __future__ import annotations

import argparse
import sys
import time
from pathlib import Path

import numpy as np

import core
import ecss_parser as P
import vocabolario


def embed_text(u: dict) -> str:
    if u["tipo"] in ("definizione", "abbreviazione"):
        return u["testo"]
    return f"{u['percorso']}: {u['testo']}"


def ingest_standards(pdf_dir: Path, force: bool, emb: "core.Embedder | None",
                     only: set[str] | None = None) -> "core.Embedder | None":
    con = core.db()
    names = [p.name for p in pdf_dir.glob("*.pdf")]
    model_name = core.config()["modello"]["nome_hf"]
    max_chars = core.config().get("max_caratteri_unita", 1500)
    missing, done, skipped = [], 0, 0

    for code in core.all_standards():
        fname = P.match_file(code, names)
        if not fname:
            missing.append(code)
            continue
        path = pdf_dir / fname
        sha = core.sha1_file(path)
        row = con.execute("SELECT sha1, modello FROM files WHERE standard=?", (code,)).fetchone()
        forced = force or (only is not None and code in only)
        if row and row["sha1"] == sha and row["modello"] == model_name and core.emb_path(code).exists() and not forced:
            skipped += 1
            continue

        t0 = time.time()
        try:
            units = P.units(path, code, max_chars=max_chars)
        except Exception as e:  # PDF illeggibile o layout anomalo
            print(f"  ! {code}: errore di parsing ({e})")
            continue
        if not units:
            print(f"  ! {code}: nessuna unità estratta (PDF scansionato?)")
            continue
        if emb is None:
            print("Caricamento del modello...")
            emb = core.Embedder()
        vecs = emb.documents([embed_text(u) for u in units])

        con.execute("DELETE FROM termine_categoria WHERE unita_id IN (SELECT id FROM unita WHERE standard=?)", (code,))
        con.execute("DELETE FROM unita WHERE standard=?", (code,))
        con.executemany(
            "INSERT INTO unita (standard, file, tipo, clausola, percorso, pagina, termine, req_id, testo, riga) "
            "VALUES (?,?,?,?,?,?,?,?,?,?)",
            [(code, fname, u["tipo"], u["clausola"], u["percorso"], u["pagina"], u.get("termine"),
              u.get("req_id"), u["testo"], i) for i, u in enumerate(units)],
        )
        np.save(core.emb_path(code), vecs)
        con.execute("INSERT OR REPLACE INTO files VALUES (?,?,?,?,?)", (code, fname, sha, len(units), model_name))
        con.commit()
        done += 1
        print(f"  ✓ {code:34s} {len(units):5d} unità  ({time.time() - t0:.0f}s)")

    # standard non più presenti in categorie.yaml
    known = set(core.all_standards())
    for (std,) in con.execute("SELECT standard FROM files").fetchall():
        if std not in known:
            con.execute("DELETE FROM unita WHERE standard=?", (std,))
            con.execute("DELETE FROM files WHERE standard=?", (std,))
            core.emb_path(std).unlink(missing_ok=True)
    con.commit()

    print(f"\nElaborati: {done}   invariati: {skipped}   senza PDF: {len(missing)}")
    if missing:
        print("  Standard senza PDF corrispondente:", ", ".join(missing))
    return emb


# ───────────────────────── prototipi e lessici ─────────────────────────

def spherical_kmeans(x: np.ndarray, k: int, iters: int = 30, seed: int = 0) -> np.ndarray:
    rng = np.random.default_rng(seed)
    k = max(1, min(k, len(x)))
    if k == 1:
        c = x.mean(0, keepdims=True)
        return c / np.linalg.norm(c, axis=1, keepdims=True)
    # inizializzazione k-means++ (distanza = 1 - coseno)
    centers = [x[rng.integers(len(x))]]
    for _ in range(1, k):
        d = 1 - np.max(x @ np.array(centers).T, axis=1)
        d = np.clip(d, 1e-9, None)
        centers.append(x[rng.choice(len(x), p=d / d.sum())])
    c = np.array(centers)
    for _ in range(iters):
        lab = np.argmax(x @ c.T, axis=1)
        new = np.array([x[lab == j].mean(0) if np.any(lab == j) else c[j] for j in range(k)])
        new /= np.linalg.norm(new, axis=1, keepdims=True)
        if np.allclose(new, c, atol=1e-5):
            break
        c = new
    return c


def build_themes(emb: "core.Embedder | None") -> "core.Embedder | None":
    """Assegna a ogni tema generico i suoi membri:
    1. le unità delle ancore (standard o clausole dedicati all'argomento), incluse per intero;
    2. le unità più vicine ai semi, con un bonus se un seme compare letteralmente nel titolo o nel termine.
    """
    import re

    themes = {cid: c for cid, c in core.categories().items() if c["gruppo"] == "tema"}
    con = core.db()
    con.execute("DROP TABLE IF EXISTS tema_unita")
    con.execute("CREATE TABLE tema_unita (tema TEXT, unita_id INTEGER, score REAL, origine TEXT, "
                "PRIMARY KEY (tema, unita_id))")
    if not themes:
        con.commit()
        return emb
    cfg = core.config().get("temi", {})
    n_per = cfg.get("unita_per_tema", 300)
    bonus = cfg.get("bonus_lessicale", 0.05)
    tipi = cfg.get("tipi", ["definizione", "requisito", "testo", "drd"])
    rows, mat = core.load_matrix(con, f"WHERE tipo IN ({','.join('?' * len(tipi))})", tuple(tipi))
    if not rows:
        con.commit()
        return emb
    if emb is None:
        print("Caricamento del modello...")
        emb = core.Embedder()
    # testo su cui cercare i semi alla lettera: titoli della clausola + termine definito
    heads = [f"{r['percorso'] or ''} {r['termine'] or ''}".lower() for r in rows]
    data, report = [], []
    for cid, t in themes.items():
        seeds = [t["descrizione"]] + t["semi"] if t["descrizione"] else t["semi"]
        q = emb.queries(seeds)
        sims = (mat @ q.T).max(axis=1)  # vicinanza al seme più simile
        pats = [re.compile(r"\b" + re.escape(sd.lower()) + r"\b") for sd in t["semi"]]
        lex = np.array([any(p.search(h) for p in pats) for h in heads], dtype=np.float32)
        anchored = np.array([core.matches_members(r, t["ancore"]) for r in rows], dtype=bool) \
            if t["ancore"] else np.zeros(len(rows), dtype=bool)
        combined = sims + bonus * lex
        combined[anchored] = -np.inf  # le ancore entrano comunque: il top-N si calcola sulle altre unità
        top = [i for i in np.argsort(-combined)[:n_per] if np.isfinite(combined[i])]
        data += [(cid, rows[i]["id"], float(sims[i]), "ancora") for i in np.where(anchored)[0]]
        data += [(cid, rows[i]["id"], float(sims[i]), "lessicale" if lex[i] else "semantica") for i in top]
        report.append(f"{cid}={int(anchored.sum())}+{len(top)}")
    con.executemany("INSERT OR REPLACE INTO tema_unita VALUES (?,?,?,?)", data)
    con.commit()
    core.theme_members(reload=True)
    print(f"Temi: {len(themes)} temi (unità da ancore + vicine ai semi): " + ", ".join(report))
    return emb


def build_prototypes() -> None:
    con = core.db()
    cfg = core.config()["prototipi"]
    rows, mat = core.load_matrix(con, "WHERE tipo != 'abbreviazione'")
    if not rows:
        print("Nessuna unità indicizzata.")
        return
    cats = core.categories()
    ids, vecs = [], []
    for cid, cat in cats.items():
        mask = np.array([core.unit_in_category(r, cat) for r in rows])
        n = int(mask.sum())
        if n == 0:
            continue
        k = min(cfg["max_per_categoria"], max(1, n // cfg["unita_per_prototipo"]))
        for v in spherical_kmeans(mat[mask], k):
            ids.append(cid)
            vecs.append(v)
    np.savez(core.index_dir() / "prototipi.npz", categorie=np.array(ids), vettori=np.array(vecs, dtype=np.float32))
    print(f"Prototipi: {len(vecs)} vettori per {len(set(ids))} categorie")

    # tabella categorie con descrizione (Scope degli standard membri)
    con.execute("DELETE FROM categorie")
    for cid, cat in cats.items():
        if cat["gruppo"] == "tema":
            con.execute("INSERT INTO categorie VALUES (?,?,?,?,?)",
                        (cid, cat["nome"], None, "tema", cat["descrizione"]))
            continue
        stds = list(dict.fromkeys(s for s, _ in cat["membri"]))[:5]
        scopes = []
        for s in stds:
            r = con.execute("SELECT group_concat(testo, ' ') AS t FROM unita WHERE standard=? AND clausola='1'",
                            (s,)).fetchone()
            if r and r["t"]:
                scopes.append(r["t"][:600])
        con.execute("INSERT INTO categorie VALUES (?,?,?,?,?)",
                    (cid, cat["nome"], cat["parent"], cat["gruppo"], " ".join(scopes)))
    con.commit()


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--pdf", type=Path, help="cartella con i PDF degli standard")
    ap.add_argument("--forza", action="store_true", help="rielabora tutti gli standard")
    ap.add_argument("--solo-prototipi", action="store_true", help="rigenera solo temi, prototipi e lessici (dopo modifiche a categorie.yaml)")
    ap.add_argument("--solo", help="rielabora comunque questi standard (codici separati da virgola)")
    a = ap.parse_args()
    if not a.solo_prototipi:
        if not a.pdf or not a.pdf.is_dir():
            sys.exit("Indicare con --pdf la cartella dei PDF")
        only = {x.strip() for x in a.solo.split(",")} if a.solo else None
        if only:
            unknown = only - set(core.all_standards())
            if unknown:
                sys.exit(f"Standard non presenti in categorie.yaml: {', '.join(sorted(unknown))}")
        emb = ingest_standards(a.pdf, a.forza, None, only)
    else:
        emb = None
    emb = build_themes(emb)
    build_prototypes()
    if emb is None:
        emb = core.Embedder()
    vocabolario.build(emb)
    print("Fatto.")


if __name__ == "__main__":
    main()
