"""Parsing degli standard ECSS: selezione dei file e segmentazione in unità.

Unità prodotte (campo `tipo`):
  definizione   termine + definizione (clausola "Terms, definitions...")
  abbreviazione sigla + significato
  requisito     requisito normativo (a., b., ... con eventuale ID ECSS-..._nnnnnnn)
  testo         testo descrittivo di una clausola
  drd           testo di un annex DRD (Document Requirements Definition)
"""
from __future__ import annotations

import re
import unicodedata
from dataclasses import dataclass, field
from pathlib import Path

import pymupdf

pymupdf.TOOLS.mupdf_display_errors(False)

# ───────────────────────── selezione dei file ─────────────────────────

_EXCLUDE = re.compile(r"(DIR\d*|DFR|DRAFT|Training|training|-HB-|-TM-)", re.I)
_CODE = re.compile(r"^(ECSS-[A-Z](?:-(?:ST|AS))?-\d+(?:-\d+)*)([A-Z])(?:\s+(.*))?$")


def parse_code(code: str) -> dict:
    """'ECSS-E-ST-40C Rev.1' -> base, lettera, rev, draft."""
    m = _CODE.match(code.strip())
    if not m:
        raise ValueError(f"codice non riconosciuto: {code}")
    base, letter, rest = m.group(1), m.group(2), m.group(3) or ""
    rev = re.search(r"Rev\.?\s*(\d+)", rest)
    return {
        "base": base,
        "letter": letter,
        "rev": rev.group(1) if rev else None,
        "draft": bool(re.search(r"draft", rest, re.I)),
    }


def _file_rev(name: str, wanted: str | None) -> bool:
    """Verifica la revisione nel nome file (gestisce 'Rev.115November2008' = Rev.1 + data)."""
    m = re.search(r"Rev\.?\s*(\d+)", name)
    if wanted is None:
        return m is None
    if m is None:
        return False
    s = m.group(1)
    rest = name[m.start(1):]
    if s == wanted:
        return True
    # Rev.1 seguito subito da una data: "115November2008" -> rev "1" + "15November"
    return rest.startswith(wanted) and re.match(r"\d{1,2}[A-Za-z]", rest[len(wanted):]) is not None


def match_file(code: str, names: list[str]) -> str | None:
    """Trova il PDF della versione attiva di uno standard tra i nomi file."""
    c = parse_code(code)
    pat = re.compile(re.escape(c["base"]) + rf"(?:{c['letter']})?(?!-\d)(?![A-Za-z])", re.I)
    cands = []
    for n in names:
        norm = n.replace("_", "-")
        if not norm.upper().startswith("ECSS-") or not pat.match(norm):
            continue
        if _EXCLUDE.search(norm) and not (c["draft"] and re.search(r"draft", norm, re.I)):
            continue
        if not _file_rev(norm, c["rev"]):
            continue
        cands.append(n)
    # a parità, preferisce il nome con la lettera di issue esplicita
    cands.sort(key=lambda n: (c["base"] + c["letter"]).lower() not in n.replace("_", "-").lower())
    return cands[0] if cands else None


# ───────────────────────── lettura del PDF ─────────────────────────

@dataclass
class Line:
    page: int
    size: float
    bold: bool
    font: str
    x: float
    y: float
    text: str

    @property
    def is_heading_font(self) -> bool:
        return self.bold and "Arial" in self.font and self.size >= 10.5


_NUM = re.compile(r"^(\d+(\.\d+)*|[A-Z](\.\d+)+|Annex [A-Z]+(\s*\((normative|informative)\))?)$")
_REQ_ID = re.compile(r"^ECSS-[A-Z]-(?:ST|AS)-[\w-]+_\d{5,}$")
_LETTER = re.compile(r"^[a-z]\.$")
_TOC = re.compile(r"\.{5,}")
_FRONT = re.compile(r"^(change log|table of contents|foreword|introduction|disclaimer|figures|tables)$", re.I)
# annex/capitoli senza contenuto semantico utile (tabelle di tailoring, tracciabilità)
_SKIP_CHAPTER = re.compile(
    r"tailoring of this standard|pre-?tailoring|traceability|bibliography|change log|normative references", re.I)
_DRD_ITEM = re.compile(r"^<[\d.]+>")


def clean(t: str) -> str:
    t = unicodedata.normalize("NFKC", t)
    t = t.replace("‐", "-").replace("‑", "-").replace("ʺ", '"').replace("ʹ", "'").replace("’", "'").replace(" ", " ")
    return re.sub(r"\s+", " ", t).strip()


def read_lines(path: Path) -> list[Line]:
    out: list[Line] = []
    with pymupdf.open(path) as doc:
        for pno, page in enumerate(doc, start=1):
            h = page.rect.height
            for b in page.get_text("dict")["blocks"]:
                for ln in b.get("lines", []):
                    text = clean("".join(s["text"] for s in ln["spans"]))
                    if not text:
                        continue
                    y = ln["bbox"][1]
                    if y < 0.08 * h or y > 0.93 * h:  # intestazione / piè di pagina
                        continue
                    s = ln["spans"][0]
                    out.append(Line(pno, s["size"], bool(s["flags"] & 16), s["font"], ln["bbox"][0], y, text))
    return out


# ───────────────────────── struttura a clausole ─────────────────────────

@dataclass
class Section:
    number: str
    title: str
    level: int
    page: int
    path: list[str] = field(default_factory=list)
    body: list[Line] = field(default_factory=list)


def _level(num: str) -> int:
    if num.startswith("Annex"):
        return 1
    return num.count(".") + 1


def sections(lines: list[Line]) -> list[Section]:
    """Raggruppa le righe in clausole numerate; scarta il front matter e l'indice."""
    secs: list[Section] = []
    stack: list[Section] = []
    i, n = 0, len(lines)
    started = False
    chapter_no = 0
    while i < n:
        ln = lines[i]
        # capitolo senza numero estraibile (il numero è a volte un'immagine): numerazione sintetica
        if (ln.is_heading_font and ln.size >= 18 and not _NUM.match(ln.text)
                and (i == 0 or not (lines[i - 1].is_heading_font and _NUM.match(lines[i - 1].text)))
                and (started or ln.text.lower() == "scope")
                and not _FRONT.match(ln.text) and not ln.text.startswith("Annex")):
            title_parts, j = [ln.text], i + 1
            while j < n and lines[j].is_heading_font and not _NUM.match(lines[j].text) and abs(lines[j].size - ln.size) <= 1.5:
                title_parts.append(lines[j].text)
                j += 1
            chapter_no += 1
            started = True
            stack.clear()
            title = clean(" ".join(title_parts))
            sec = Section(str(chapter_no), title, 1, ln.page, [title])
            secs.append(sec)
            stack.append(sec)
            i = j
            continue
        if ln.is_heading_font and _NUM.match(ln.text):
            num = ln.text
            title_parts, j = [], i + 1
            while j < n and lines[j].is_heading_font and not _NUM.match(lines[j].text) and abs(lines[j].size - ln.size) <= 1.5:
                title_parts.append(lines[j].text)
                j += 1
            title = clean(" ".join(title_parts))
            if num.startswith("Annex"):
                title = re.sub(r"^\((normative|informative)\)\s*", "", title)
            if num == "1":
                started = True
            if num.isdigit():
                chapter_no = int(num)
            if started and title:
                lvl = _level(num)
                while stack and stack[-1].level >= lvl:
                    stack.pop()
                path = [s.title for s in stack] + [title]
                sec = Section(num, title, lvl, ln.page, path)
                secs.append(sec)
                stack.append(sec)
                i = j
                continue
        if started and stack and not _TOC.search(ln.text):
            stack[-1].body.append(ln)
        i += 1
    return secs


# ───────────────────────── unità ─────────────────────────

def _join(lines: list[Line]) -> str:
    txt = ""
    for ln in lines:
        t = ln.text
        if txt.endswith("-") and len(txt) > 1 and txt[-2].isalpha() and t[:1].islower():
            txt = txt[:-1] + t
        else:
            txt = (txt + " " + t) if txt else t
    return clean(txt)


def _split_long(text: str, max_chars: int) -> list[str]:
    if len(text) <= max_chars:
        return [text]
    parts, cur = [], ""
    for sent in re.split(r"(?<=[.;:])\s+", text):
        if cur and len(cur) + len(sent) > max_chars:
            parts.append(cur)
            cur = sent
        else:
            cur = f"{cur} {sent}".strip()
    if cur:
        parts.append(cur)
    # testo senza punteggiatura (tabelle): taglio duro sulle parole
    out = []
    for part in parts:
        while len(part) > max_chars:
            cut = part.rfind(" ", 0, max_chars)
            cut = cut if cut > 0 else max_chars
            out.append(part[:cut])
            part = part[cut:].strip()
        if part:
            out.append(part)
    return out


def _abbreviations(sec: Section) -> list[tuple[str, str]]:
    """Tabella sigla | significato: righe allineate sulla stessa y."""
    rows: dict[tuple[int, int], list[Line]] = {}
    for ln in sec.body:
        rows.setdefault((ln.page, round(ln.y / 3)), []).append(ln)
    pairs = []
    for key in sorted(rows):
        cells = sorted(rows[key], key=lambda l: l.x)
        if len(cells) < 2:
            # continuazione del significato sulla riga successiva
            if pairs and cells[0].x > 200:
                a, m = pairs[-1]
                pairs[-1] = (a, f"{m} {cells[0].text}")
            continue
        a, m = cells[0].text, " ".join(c.text for c in cells[1:])
        if a.lower() == "abbreviation":
            continue
        if len(a) <= 15 and not a.endswith(":"):
            pairs.append((a, m))
    return pairs


def units(path: Path, code: str, max_chars: int = 1500) -> list[dict]:
    secs = sections(read_lines(path))
    out: list[dict] = []

    def add(**u):
        u.setdefault("standard", code)
        u.setdefault("file", path.name)
        min_len = 5 if u["tipo"] in ("abbreviazione", "definizione") else 25
        if u.get("testo") and len(u["testo"]) >= min_len:
            out.append(u)

    for k, s in enumerate(secs):
        is_leaf = k + 1 >= len(secs) or secs[k + 1].level <= s.level
        chapter = s.path[0] if s.path else ""
        in_terms = re.search(r"^terms\b.*definitions|^definitions", chapter, re.I) is not None
        parent = s.path[-2] if len(s.path) >= 2 else ""
        base = {"clausola": s.number, "percorso": " › ".join(s.path), "pagina": s.page}

        if in_terms:
            if re.search(r"abbreviat", s.title, re.I):
                for a, m in _abbreviations(s):
                    add(tipo="abbreviazione", termine=a, testo=f"{a}: {m}", **base)
            elif (s.level >= 3 and is_leaf and s.body
                  and not re.search(r"^(definitions for|introduction|general)\b", s.title, re.I)
                  and not any("space system breakdown" in p.lower() for p in s.path[:-1])):
                add(tipo="definizione", termine=s.title, testo=f"{s.title}: {_join(s.body)}", **base)
            continue

        if _SKIP_CHAPTER.search(chapter):
            continue
        is_drd = re.search(r"\bDRD\b", chapter) is not None
        # spezza il corpo sui marcatori di requisito "a.", "b.", ...
        blocks: list[tuple[str | None, list[Line]]] = [(None, [])]
        for ln in s.body:
            if _REQ_ID.match(ln.text):
                # l'ID precede sempre il requisito (anche quando la lettera a., b. non è estraibile)
                blocks.append((ln.text, []))
                continue
            if _DRD_ITEM.match(ln.text):
                blocks.append((None, [ln]))
                continue
            if _LETTER.match(ln.text) and ln.x < 190:
                if not (blocks[-1][0] and not blocks[-1][1]):  # non duplicare il blocco aperto dall'ID
                    blocks.append(("", []))
                continue
            blocks[-1][1].append(ln)
        for req_id, blines in blocks:
            text = _join(blines)
            if not text:
                continue
            if req_id is None:
                for k, chunk in enumerate(_split_long(text, max_chars)):
                    add(tipo="drd" if is_drd else "testo", testo=chunk, **base)
            else:
                for chunk in _split_long(text, max_chars):
                    add(tipo="requisito", req_id=req_id or None, testo=chunk, **base)
    return out


def scope_text(units_list: list[dict]) -> str:
    """Testo della clausola 1 (Scope), usato come descrizione della categoria."""
    return " ".join(u["testo"] for u in units_list if u["clausola"] == "1" and u["tipo"] == "testo")
