"""Interrogazione dell'indice ECSS.

I "termini" sono parole singole estratte da termini definiti, titoli delle clausole e sigle.

Esempi:
    python query.py termini "cost management" --k 20
    python query.py termini "gestione dei costi" --categoria M-60
    python query.py lessico --categoria cost --n 40
    python query.py lessico --categoria T-SW --fonte titolo
    python query.py unita "software validation" --tipo requisito --k 10
    python query.py membri --categoria procurement --n 30
    python query.py categorie
Aggiungere --json per un output strutturato.
"""
from __future__ import annotations

import argparse

import numpy as np

import core
import vocabolario


def _filter(rows, mat, categoria: str | None):
    if not categoria:
        return rows, mat
    cats = core.categories()
    if categoria not in cats:
        raise SystemExit(f"Categoria sconosciuta: {categoria}. Vedi: python query.py categorie")
    mask = np.array([core.unit_in_category(r, cats[categoria]) for r in rows], dtype=bool)
    return [r for r, m in zip(rows, mask) if m], mat[mask]


def search(text: str, tipi: list[str], categoria: str | None, k: int, dedup_terms: bool) -> list[dict]:
    con = core.db()
    ph = ",".join("?" * len(tipi))
    rows, mat = core.load_matrix(con, f"WHERE tipo IN ({ph})", tuple(tipi))
    rows, mat = _filter(rows, mat, categoria)
    if not rows:
        return []
    q = core.Embedder().queries([text])[0]
    sims = mat @ q
    order = np.argsort(-sims)
    out, seen = [], {}
    for i in order:
        r = rows[i]
        key = (r["termine"] or "").lower() if dedup_terms and r["termine"] else None
        if key and key in seen:
            seen[key]["standard"].append(r["standard"])
            continue
        item = {"score": round(float(sims[i]), 3), "tipo": r["tipo"], "termine": r["termine"],
                "standard": [r["standard"]], "clausola": r["clausola"], "req_id": r["req_id"],
                "testo": r["testo"]}
        if key:
            seen[key] = item
        out.append(item)
        if len(out) >= k:
            break
    return out


def search_words(text: str, categoria: str | None, k: int, fonte: str | None) -> list[dict]:
    """Parole singole più vicine a un argomento (opzionalmente: solo quelle presenti nella categoria)."""
    rows, vecs = vocabolario.load()
    keep = np.ones(len(rows), dtype=bool)
    if fonte:
        keep &= np.array([fonte in r["fonti"].split(",") for r in rows])
    if categoria:
        if categoria not in core.categories():
            raise SystemExit(f"Categoria sconosciuta: {categoria}. Vedi: python query.py categorie")
        con = core.db()
        present = {r["parola_id"] for r in con.execute(
            "SELECT parola_id FROM parola_categoria WHERE categoria=? AND occorrenze>0", (categoria,))}
        keep &= np.array([r["id"] in present for r in rows])
    q = core.Embedder().queries([text])[0]
    sims = np.where(keep, vecs @ q, -np.inf)
    out = []
    for i in np.argsort(-sims)[:k]:
        if not np.isfinite(sims[i]):
            break
        r = rows[i]
        out.append({"score": round(float(sims[i]), 3), "parola": r["parola"], "fonti": r["fonti"],
                    "occorrenze": r["occorrenze"], "esempi": r["esempi"]})
    return out


def lexicon(categoria: str, n: int, min_spec: float | None, min_occ: int, fonte: str | None) -> list[dict]:
    vocabolario.load()  # senza vocabolario termina indicando di eseguire ingest.py
    con = core.db()
    q = """SELECT p.parola, p.fonti, p.esempi, pc.score, pc.specificita, pc.occorrenze, pc.concentrazione, pc.punteggio
           FROM parola_categoria pc JOIN parole p ON p.id = pc.parola_id
           WHERE pc.categoria = ? AND pc.occorrenze >= ?"""
    params: list = [categoria, min_occ]
    if min_spec is not None:
        q += " AND pc.specificita >= ?"
        params.append(min_spec)
    if fonte:
        q += " AND (',' || p.fonti || ',') LIKE ?"
        params.append(f"%,{fonte},%")
    q += " ORDER BY pc.punteggio DESC LIMIT ?"
    params.append(n)
    return [{"parola": r["parola"], "punteggio": round(r["punteggio"], 3),
             "concentrazione": round(r["concentrazione"], 3), "occorrenze": r["occorrenze"],
             "specificita": round(r["specificita"], 3), "fonti": r["fonti"], "esempi": r["esempi"]}
            for r in con.execute(q, params)]


def theme_members_list(tema: str, n: int, tipo: str | None, origine: str | None = None) -> list[dict]:
    con = core.db()
    q = """SELECT u.tipo, u.termine, u.standard, u.clausola, u.req_id, u.testo, tu.score, tu.origine
           FROM tema_unita tu JOIN unita u ON u.id = tu.unita_id WHERE tu.tema = ?"""
    params: list = [tema]
    if tipo:
        q += " AND u.tipo = ?"
        params.append(tipo)
    if origine:
        q += " AND tu.origine = ?"
        params.append(origine)
    q += " ORDER BY tu.score DESC LIMIT ?"
    params.append(n)
    return [dict(r) for r in con.execute(q, params)]


def list_categories() -> list[dict]:
    con = core.db()
    rows, _ = core.load_matrix(con)
    out = []
    for cid, c in core.categories().items():
        n = sum(1 for r in rows if core.unit_in_category(r, c))
        out.append({"id": cid, "nome": c["nome"], "gruppo": c["gruppo"], "parent": c["parent"], "unita": n})
    return out


def _short(t: str, n: int = 110) -> str:
    return t if len(t) <= n else t[: n - 1] + "…"


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest="cmd", required=True)

    t = sub.add_parser("termini", help="parole singole correlate a un argomento")
    t.add_argument("testo")
    t.add_argument("--categoria", help="solo parole che compaiono nella categoria")
    t.add_argument("--k", type=int, default=20)
    t.add_argument("--fonte", choices=["definizione", "titolo", "sigla"], help="solo parole da questa fonte")

    u = sub.add_parser("unita", help="unità (requisiti, testo, DRD, definizioni) più vicine")
    u.add_argument("testo")
    u.add_argument("--categoria")
    u.add_argument("--tipo", default="requisito,testo,drd,definizione")
    u.add_argument("--k", type=int, default=10)

    lx = sub.add_parser("lessico", help="parole singole più specifiche di una categoria")
    lx.add_argument("--categoria", required=True)
    lx.add_argument("--n", type=int, default=50)
    lx.add_argument("--min-specificita", type=float, help="filtro opzionale sulla specificità semantica")
    lx.add_argument("--min-occorrenze", type=int, default=2,
                    help="solo parole che compaiono almeno N volte nella categoria (predefinito 2)")
    lx.add_argument("--fonte", choices=["definizione", "titolo", "sigla"], help="solo parole da questa fonte")

    mb = sub.add_parser("membri", help="unità ECSS assegnate a un tema generico (per controllo)")
    mb.add_argument("--categoria", required=True)
    mb.add_argument("--n", type=int, default=40)
    mb.add_argument("--tipo", help="definizione | requisito | testo | drd")
    mb.add_argument("--origine", choices=["ancora", "lessicale", "semantica"],
                    help="solo membri entrati per questo motivo (utile per controllare quelli semantici)")

    sub.add_parser("categorie", help="elenco delle categorie")
    for p in (t, u, lx, mb):
        p.add_argument("--json", action="store_true")
    a = ap.parse_args()

    if a.cmd == "termini":
        res = search_words(a.testo, a.categoria, a.k, a.fonte)
        if a.json:
            print(core.dumps(res)); return
        for r in res:
            print(f"{r['score']:.3f}  {r['parola']:<24s} {r['fonti']:<26s} es.: {_short(r['esempi'], 70)}")
    elif a.cmd == "unita":
        res = search(a.testo, a.tipo.split(","), a.categoria, a.k, dedup_terms=False)
        if a.json:
            print(core.dumps(res)); return
        for r in res:
            ref = r["req_id"] or r["clausola"]
            print(f"{r['score']:.3f}  [{r['tipo']}] {r['standard'][0]} {ref}\n        {_short(r['testo'], 200)}")
    elif a.cmd == "lessico":
        if a.categoria not in core.categories():
            raise SystemExit(f"Categoria sconosciuta: {a.categoria}. Vedi: python query.py categorie")
        res = lexicon(a.categoria, a.n, a.min_specificita, a.min_occorrenze, a.fonte)
        if a.json:
            print(core.dumps(res)); return
        print(f"{'punt.':>6s} {'conc.':>5s} {'occ.':>5s} {'spec.':>6s}  {'parola':<24s} fonti")
        for r in res:
            print(f"{r['punteggio']:6.3f} {r['concentrazione'] * 100:4.0f}% {r['occorrenze']:5d} "
                  f"{r['specificita']:+6.3f}  {r['parola']:<24s} {r['fonti']:<24s} es.: {_short(r['esempi'], 50)}")
        print("\nconc. = quota delle occorrenze della parola che cade in questa categoria"
              "\nspec. = vicinanza semantica alla categoria rispetto alla concorrente più vicina"
              "\npunt. = conc. + peso_semantico × spec. normalizzata (ordinamento)")
    elif a.cmd == "membri":
        cat = core.categories().get(a.categoria)
        if not cat or cat["gruppo"] != "tema":
            raise SystemExit(f"{a.categoria} non è un tema generico. Vedi: python query.py categorie")
        res = theme_members_list(a.categoria, a.n, a.tipo, a.origine)
        if a.json:
            print(core.dumps(res)); return
        for r in res:
            ref = r["termine"] or r["req_id"] or r["clausola"]
            print(f"{r['score']:.3f}  [{r['tipo']}|{r['origine']}] {r['standard']} {ref}\n"
                  f"        {_short(r['testo'], 160)}")
    else:
        for r in list_categories():
            ind = "  " if r["parent"] else ""
            print(f"{ind}{r['id']:<18s} {r['gruppo']:<12s} {r['unita']:6d}  {r['nome']}")


if __name__ == "__main__":
    main()
