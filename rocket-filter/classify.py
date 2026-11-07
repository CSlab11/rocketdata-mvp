"""Classificazione di paragrafi rispetto alle categorie ECSS.

Esempi:
    python classify.py documento.pdf
    python classify.py note.txt relazione.md --out risultati.csv
    python classify.py documento.pdf --livelli discipline,trasversali
    python classify.py documento.pdf --livelli discipline --soglia 0.82
    python classify.py --testo "The supplier shall provide a cost breakdown structure..."

Ogni paragrafo viene classificato separatamente su ogni livello dell'albero
(rami, discipline, trasversali), con soglia e margine propri per livello.
Per ogni livello: categorie assegnate e score delle migliori; in più i termini ECSS
più vicini al paragrafo.
Gli embeddings dei paragrafi sono in cache: rielaborando un documento modificato
si ricalcolano solo i paragrafi cambiati.
"""
from __future__ import annotations

import argparse
import csv
import json
import re
import sys
from pathlib import Path

import numpy as np

import core
import vocabolario


def paragraphs_from_file(path: Path, min_chars: int) -> list[dict]:
    out = []
    if path.suffix.lower() == ".pdf":
        import pymupdf
        pymupdf.TOOLS.mupdf_display_errors(False)
        with pymupdf.open(path) as doc:
            for pno, page in enumerate(doc, start=1):
                for b in page.get_text("blocks"):
                    text = re.sub(r"\s+", " ", b[4]).strip()
                    if len(text) >= min_chars:
                        out.append({"file": path.name, "pagina": pno, "testo": text})
    elif path.suffix.lower() == ".docx":
        try:
            import docx
        except ImportError:
            sys.exit("Per i .docx installare python-docx (pip install python-docx)")
        for p in docx.Document(str(path)).paragraphs:
            text = p.text.strip()
            if len(text) >= min_chars:
                out.append({"file": path.name, "pagina": None, "testo": text})
    else:
        raw = path.read_text(encoding="utf-8", errors="replace")
        for block in re.split(r"\n\s*\n", raw):
            text = re.sub(r"\s+", " ", block).strip()
            if len(text) >= min_chars:
                out.append({"file": path.name, "pagina": None, "testo": text})
    return out


def embed_cached(texts: list[str], emb: core.Embedder) -> np.ndarray:
    con = core.db()
    model = core.config()["modello"]["nome_hf"]
    keys = [core.sha1_text(model + "\x00" + t) for t in texts]
    found = {}
    for i in range(0, len(keys), 500):
        chunk = keys[i:i + 500]
        q = f"SELECT sha1, vettore FROM cache_paragrafi WHERE sha1 IN ({','.join('?' * len(chunk))})"
        for r in con.execute(q, chunk):
            found[r["sha1"]] = np.frombuffer(r["vettore"], dtype=np.float32)
    todo = [i for i, k in enumerate(keys) if k not in found]
    if todo:
        vecs = emb.queries([texts[i] for i in todo], progress=len(todo) > 50)
        con.executemany("INSERT OR REPLACE INTO cache_paragrafi VALUES (?,?)",
                        [(keys[i], v.tobytes()) for i, v in zip(todo, vecs)])
        con.commit()
        for i, v in zip(todo, vecs):
            found[keys[i]] = v
    return np.array([found[k] for k in keys], dtype=np.float32)


LIVELLI = ["temi", "rami", "discipline", "trasversali"]


def level_settings(cfg: dict, levels: list[str], soglia: float | None, margine: float | None) -> dict:
    """Soglia, margine e numero massimo di categorie per ogni livello (override da riga di comando)."""
    out = {}
    for lv in levels:
        s = dict(cfg["livelli"].get(lv, {}))
        if soglia is not None:
            s["soglia"] = soglia
        if margine is not None:
            s["margine"] = margine
        out[lv] = s
    return out


def classify(paras: list[dict], settings: dict, n_terms: int) -> list[dict]:
    """Classifica ogni paragrafo su ogni livello in modo indipendente."""
    if not paras:
        return []
    emb = core.Embedder()
    vecs = embed_cached([p["testo"] for p in paras], emb)
    cats = core.categories()

    per_level = {lv: core.category_scores(vecs, core.categories_for_level(lv)) for lv in settings}

    wrows, wvecs = vocabolario.load()
    tsims = vecs @ wvecs.T

    results = []
    for i, p in enumerate(paras):
        livelli = {}
        for lv, (cids, scores) in per_level.items():
            st = settings[lv]
            if not cids:
                livelli[lv] = {"categorie": [], "top": []}
                continue
            order = np.argsort(-scores[i])
            best = float(scores[i, order[0]])
            assigned = [cids[j] for j in order[:st["max_categorie"]]
                        if scores[i, j] >= st["soglia"] and scores[i, j] >= best - st["margine"]]
            top = [{"categoria": cids[j], "nome": cats[cids[j]]["nome"], "score": round(float(scores[i, j]), 3)}
                   for j in order[:5]]
            livelli[lv] = {"categorie": assigned, "ambiguo": len(assigned) != 1, "top": top}
        terms = [wrows[j]["parola"] for j in np.argsort(-tsims[i])[:n_terms]]
        results.append({**p, "livelli": livelli, "termini": terms})
    return results


def _top_str(top: list[dict], n: int = 3) -> str:
    return " | ".join(f"{t['categoria']} {t['score']:.3f}" for t in top[:n])


def write_csv(path: Path, res: list[dict], levels: list[str]) -> None:
    with open(path, "w", encoding="utf-8-sig", newline="") as f:
        w = csv.writer(f, delimiter=";")
        header = ["file", "pagina"]
        for lv in levels:
            header += [lv, f"{lv}_score", f"{lv}_top3"]
        w.writerow(header + ["termini", "testo"])
        for r in res:
            row = [r["file"], r["pagina"]]
            for lv in levels:
                L = r["livelli"][lv]
                best = L["top"][0]["score"] if L["top"] else ""
                row += [", ".join(L["categorie"]), best, _top_str(L["top"])]
            w.writerow(row + [", ".join(r["termini"]), r["testo"]])


def main() -> None:
    cfg = core.config()["classificazione"]
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("file", nargs="*", type=Path, help="PDF, DOCX, TXT o MD")
    ap.add_argument("--testo", help="classifica direttamente un testo")
    ap.add_argument("--livelli", default=",".join(cfg.get("livelli_attivi", LIVELLI)),
                    help="livelli separati da virgola: temi, rami, discipline, trasversali (oppure 'tutti')")
    ap.add_argument("--soglia", type=float, help="soglia unica per tutti i livelli (sovrascrive config.yaml)")
    ap.add_argument("--margine", type=float, help="margine unico per tutti i livelli (sovrascrive config.yaml)")
    ap.add_argument("--out", type=Path, help="file di output .csv o .jsonl")
    a = ap.parse_args()

    levels = LIVELLI if a.livelli.strip() in ("tutti", "tutte") else [x.strip() for x in a.livelli.split(",") if x.strip()]
    bad = [lv for lv in levels if lv not in LIVELLI]
    if bad:
        sys.exit(f"Livelli non validi: {', '.join(bad)}. Valori ammessi: {', '.join(LIVELLI)}, tutti")
    settings = level_settings(cfg, levels, a.soglia, a.margine)

    paras = []
    if a.testo:
        paras.append({"file": "-", "pagina": None, "testo": a.testo})
    for f in a.file:
        paras += paragraphs_from_file(f, cfg["min_caratteri_paragrafo"])
    if not paras:
        sys.exit("Nessun paragrafo da classificare.")

    res = classify(paras, settings, cfg["termini_supporto"])

    if a.out and a.out.suffix.lower() == ".jsonl":
        with open(a.out, "w", encoding="utf-8") as f:
            for r in res:
                f.write(json.dumps(r, ensure_ascii=False) + "\n")
    elif a.out:
        write_csv(a.out, res, levels)

    if not a.out or len(res) <= 20:
        for r in res:
            print(f"\n[{r['file']} p.{r['pagina']}] {r['testo'][:140]}{'…' if len(r['testo']) > 140 else ''}")
            for lv in levels:
                L = r["livelli"][lv]
                cat = ", ".join(L["categorie"]) or "—"
                print(f"  {lv:<12s} → {cat:<16s} ({_top_str(L['top'])})")
            print(f"  termini: {', '.join(r['termini'])}")

    print(f"\n{len(res)} paragrafi." + (f" Risultati in {a.out}" if a.out else ""))
    for lv in levels:
        n_ok = sum(1 for r in res if r["livelli"][lv]["categorie"])
        print(f"  {lv:<12s} classificati {n_ok:5d}   sotto soglia {len(res) - n_ok:5d}   "
              f"(soglia {settings[lv]['soglia']}, margine {settings[lv]['margine']})")


if __name__ == "__main__":
    main()
