"""Vocabolario di parole singole estratte dagli standard ECSS.

Fonti:
  definizione  parole dei termini definiti (glossario e clausola 3 di ogni standard)
  titolo       parole dei titoli di clausole e annex
  sigla        abbreviazioni (CBS, EAC, FMECA, ...)

Ogni parola riceve un vettore che combina:
  - l'embedding della parola da sola;
  - la media degli embeddings delle unità in cui compare (contesto ECSS).
Il contesto disambigua: "baseline" vicino alle clausole di configuration management,
"margin" vicino a quelle termiche e strutturali, ecc.
"""
from __future__ import annotations

import re
from collections import defaultdict

import numpy as np

import core

STOPWORDS = set("""
a about above after again against all also an and any are as at be been before being below between both but by
can could did do does doing down during each either etc few for from further had has have having here how i if in
into is it its itself more most must no nor not of off on once only or other our out over own per same shall should
so some such than that the their them then there these they this those through thus to too under until up upon via
was we were what when where which while who whom why will with within without would yet versus vs
""".split())

# parole di servizio della struttura dei documenti ECSS: non dicono nulla sull'argomento
BOILERPLATE = set("""
requirement requirements general annex scope term terms definition definitions abbreviated normative informative
overview introduction purpose objective objectives content contents special remark remarks identification expected
response source clause present specific table figure example examples note notes standard standards drd ecss
applicability reference references relevant related principle principles description descriptions
provision provisions aspect aspects case cases
""".split())

_SING_EXCEPT = {"mass", "gas", "bus", "status", "analysis", "basis", "process", "access", "address", "stress",
                "series", "species", "class", "glass", "loss", "pass", "radius", "focus", "thesis", "axis",
                "chassis", "bias", "corpus", "lens", "news", "physics", "electronics", "dynamics", "logistics",
                "mechanics", "optics", "avionics", "statistics", "ergonomics", "diagnostics", "aeronautics"}


def singular(w: str) -> str:
    if w in _SING_EXCEPT or len(w) <= 3:
        return w
    if w.endswith("yses"):
        return w[:-4] + "ysis"
    if w.endswith("ies") and len(w) > 4:
        return w[:-3] + "y"
    if w.endswith(("sses", "shes", "ches", "xes", "zes")):
        return w[:-2]
    if w.endswith("s") and not w.endswith(("ss", "us", "is", "ics", "ous")):
        return w[:-1]
    return w


_TOKEN = re.compile(r"[A-Za-z][A-Za-z0-9]*(?:-[A-Za-z0-9]+)*")


_SIGLA = re.compile(r"[A-Z][A-Z0-9]{1,7}")


def words(text: str, sigle: set[str] | frozenset = frozenset()) -> list[str]:
    """Parole singole normalizzate di un termine o di un titolo.
    Le parole in maiuscolo restano sigle solo se sono sigle note (es. EMC); altrimenti
    vengono trattate come parole normali (es. 'PACKET' nei nomi di primitive -> 'packet')."""
    out = []
    for tok in _TOKEN.findall(text or ""):
        if tok in sigle:
            out.append(tok)
            continue
        w = singular(tok.lower())
        if len(w) < 3 or w in STOPWORDS or w in BOILERPLATE or w.replace("-", "").isdigit():
            continue
        out.append(w)
    return out


def build(emb: core.Embedder) -> None:
    con = core.db()
    cfg = core.config().get("vocabolario", {})
    min_occ_titoli = cfg.get("min_occorrenze_titoli", 2)
    max_ctx = cfg.get("max_contesti", 60)
    w_word = cfg.get("peso_parola", 0.5)
    w_sem = cfg.get("peso_semantico", 0.3)
    alpha = cfg.get("smussamento", 10)

    rows, mat = core.load_matrix(con)
    if not rows:
        return
    pos = {r["id"]: i for i, r in enumerate(rows)}

    # parola -> fonti, TUTTE le unità in cui compare, elementi distinti (standard, testo), esempi
    fonti: dict[str, set[str]] = defaultdict(set)
    ctx: dict[str, set[int]] = defaultdict(set)
    items: dict[str, set[tuple[str, str]]] = defaultdict(set)
    esempi: dict[str, list[str]] = defaultdict(list)

    def add(w: str, fonte: str, uid: int, std: str, src_text: str):
        fonti[w].add(fonte)
        key = (std, src_text)
        if key not in items[w]:
            items[w].add(key)
            if len(esempi[w]) < 3:
                esempi[w].append(src_text)
        ctx[w].add(uid)

    # sigle vere: solo maiuscole/cifre (esclude simboli fisici come Vop, Irev)
    expansions: dict[str, set[str]] = defaultdict(set)  # sigla -> espansioni ("cost breakdown structure")
    for r in rows:
        t = (r["termine"] or "").strip()
        if r["tipo"] == "abbreviazione" and _SIGLA.fullmatch(t) and t.lower() not in BOILERPLATE:
            exp = r["testo"].split(":", 1)[-1].strip().lower()
            if len(exp) >= 6:
                expansions[t].add(exp)
    sigle = set(expansions)

    title_units: dict[str, list[int]] = defaultdict(list)  # titolo (minuscolo) -> unità sotto quel titolo
    for r in rows:
        if r["tipo"] == "definizione" and r["termine"]:
            for w in words(r["termine"], sigle):
                add(w, "definizione", r["id"], r["standard"], r["termine"])
        elif r["tipo"] == "abbreviazione" and (r["termine"] or "").strip() in sigle:
            add(r["termine"].strip(), "sigla", r["id"], r["standard"], r["testo"])
        if r["tipo"] in ("requisito", "testo", "drd") and r["percorso"]:
            for title in r["percorso"].split(" › "):
                title_units[title.lower()].append(r["id"])
                for w in words(title, sigle):
                    add(w, "titolo", r["id"], r["standard"], title)

    # contesto delle sigle: anche le clausole il cui titolo contiene l'espansione
    # (CBS -> "Cost breakdown structure - DRD")
    for sig, exps in expansions.items():
        for title, uids in title_units.items():
            if any(e in title for e in exps):
                ctx[sig].update(uids)

    vocab = [w for w in fonti
             if fonti[w] & {"definizione", "sigla"} or len(items[w]) >= min_occ_titoli]
    vocab.sort()
    print(f"Vocabolario: {len(vocab)} parole singole "
          f"({sum('definizione' in fonti[w] for w in vocab)} da definizioni, "
          f"{sum('titolo' in fonti[w] for w in vocab)} da titoli, "
          f"{sum('sigla' in fonti[w] for w in vocab)} sigle)")

    # vettori: parola da sola + contesto ECSS (campione casuale riproducibile delle unità)
    rng = np.random.default_rng(0)
    wv = emb.queries(vocab, progress=len(vocab) > 2000)
    cv = np.zeros((len(vocab), mat.shape[1]), dtype=np.float32)
    for i, w in enumerate(vocab):
        ids = sorted(ctx[w])
        if len(ids) > max_ctx:
            ids = list(rng.choice(ids, size=max_ctx, replace=False))
        cv[i] = mat[[pos[u] for u in ids]].mean(0)
    cv /= np.linalg.norm(cv, axis=1, keepdims=True) + 1e-9
    vecs = w_word * wv + (1 - w_word) * cv
    vecs /= np.linalg.norm(vecs, axis=1, keepdims=True) + 1e-9
    np.save(core.index_dir() / "parole.npy", vecs.astype(np.float32))

    con.execute("DELETE FROM parole")
    con.executemany(
        "INSERT INTO parole (id, parola, fonti, occorrenze, n_standard, esempi) VALUES (?,?,?,?,?,?)",
        [(i, w, ",".join(sorted(fonti[w])), len(items[w]), len({s for s, _ in items[w]}), " | ".join(esempi[w]))
         for i, w in enumerate(vocab)])

    # occorrenze di ogni parola in ogni categoria (su tutte le unità in cui compare)
    unit_cats = core.unit_categories(rows)
    occ_cat: list[dict[str, int]] = []
    for w in vocab:
        occ: dict[str, int] = defaultdict(int)
        for u in ctx[w]:
            for c in unit_cats.get(u, ()):
                occ[c] += 1
        occ_cat.append(occ)
    n_ctx = np.array([max(1, len(ctx[w])) for w in vocab], dtype=np.float32)

    # tabella ricreata a ogni esecuzione (lo schema può cambiare tra versioni)
    con.execute("DROP TABLE IF EXISTS parola_categoria")
    con.execute("""CREATE TABLE parola_categoria (
        parola_id INTEGER, categoria TEXT, score REAL, specificita REAL, occorrenze INTEGER,
        concentrazione REAL, punteggio REAL, PRIMARY KEY (parola_id, categoria))""")

    cats = core.categories()
    n_units = len(rows)
    cat_size: dict[str, int] = defaultdict(int)
    for cs in unit_cats.values():
        for c in cs:
            cat_size[c] += 1
    for level in ("discipline", "trasversali", "rami", "temi"):
        cids, scores = core.category_scores(vecs, core.categories_for_level(level))
        if scores.shape[1] < 2:
            continue
        data = []
        for j, cid in enumerate(cids):
            rivals = [k for k, o in enumerate(cids) if k != j and not core.categories_overlap(cats[cid], cats[o])]
            spec = scores[:, j] - (scores[:, rivals].max(axis=1) if rivals else 0)
            occ = np.array([o.get(cid, 0) for o in occ_cat], dtype=np.float32)
            # quota delle occorrenze della parola che cade in questa categoria, smussata verso la quota
            # media della categoria: una parola vista 2 volte non batte una vista 30 volte
            prior = cat_size.get(cid, 0) / n_units
            conc = (occ + alpha * prior) / (n_ctx + alpha)
            z = (spec - spec.mean()) / (spec.std() + 1e-9)
            punt = conc + w_sem * np.clip(z, -3, 3) / 3
            data += [(i, cid, float(scores[i, j]), float(spec[i]), int(occ[i]), float(conc[i]), float(punt[i]))
                     for i in range(len(vocab))]
        con.executemany("INSERT INTO parola_categoria VALUES (?,?,?,?,?,?,?)", data)
    con.commit()
    print("Lessici di parole singole calcolati per tutte le categorie")


def load() -> tuple[list[dict], np.ndarray]:
    con = core.db()
    rows = [dict(r) for r in con.execute("SELECT * FROM parole ORDER BY id")]
    p = core.index_dir() / "parole.npy"
    if not rows or not p.exists():
        raise SystemExit("Vocabolario non trovato: eseguire python ingest.py --solo-prototipi")
    return rows, np.load(p)
