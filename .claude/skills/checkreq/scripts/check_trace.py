#!/usr/bin/env python3
"""Controllo meccanico della tracciabilita' requisiti -> file -> commit.

Confronta docs/REQUIREMENTS.md, gli eventuali <tool>/docs/REQUIREMENTS.md
(un file per ogni tool, nella cartella docs/ del tool), CHANGELOG.md e la storia git.
Nella storia git conta come file di tracciabilita' ogni REQUIREMENTS.md dentro una
cartella docs/, compresi i vecchi docs/<tool>/REQUIREMENTS.md. Un ID che corregge il
testo di un REQUIREMENTS.md (introduzione, note, appendici) puo' elencarlo tra i file
impattati: per quell'ID il file conta come contenuto.

Percorsi di "File impattati": in docs/REQUIREMENTS.md dalla radice del repository; in un
file di parte dalla cartella della parte, salvo che la sua introduzione contenga la riga
"Percorsi dei file: dalla radice del repository."

Numerazione degli ID: ogni file dei requisiti ha la propria. Dentro il file gli ID sono
REQ-NNN / BUG-NNN; fuori, gli ID di una parte sono qualificati con il nome della
cartella che contiene docs/ (<parte>_REQ-NNN), quelli di progetto restano senza prefisso.
Un ID senza prefisso in un commit o nel changelog indica il progetto se esiste;
altrimenti (ID storici della numerazione unica) l'elemento della parte il cui file
esisteva gia' al momento del commit o alla data della voce del changelog, oppure, se
nessuno esisteva, quello del file creato per primo.

Gli ID di un commit si leggono dal solo oggetto del messaggio. Un commit senza ID che
non si puo' piu' riscrivere si attribuisce a un ID con una voce del changelog
"- <hash> attribuito a REQ-NNN — motivo".

La sezione "DA RIVEDERE" considera solo i commit successivi all'ultimo
commit marcatore di revisione (messaggio che inizia con "review:").

Uso:    python3 check_trace.py [radice-del-repository]
Uscita: 0 nessun errore (possibili avvisi), 1 errori, 2 prerequisiti mancanti.
Solo libreria standard.
"""
import re
import subprocess
import sys
from pathlib import Path

REQ_FILE = "docs/REQUIREMENTS.md"
LOG_FILE = "CHANGELOG.md"
TRACE_RE = re.compile(r"(^|/)docs/(.+/)?REQUIREMENTS\.md$")
TOOL_REQ_RE = re.compile(r"^.+/docs/REQUIREMENTS\.md$")
LOCAL_RE = re.compile(r"(REQ|BUG)-\d{3,}")
# ID citato: qualificato (<parte>_REQ-NNN) o senza prefisso. Gruppo 1: parte, gruppo 2: ID locale.
ID_RE = re.compile(r"(?<![\w.-])(?:([A-Za-z0-9][A-Za-z0-9.-]*)_)?((?:REQ|BUG)-\d{3,})\b")
STATES = {"REQ": {"aperto", "implementato", "annullato"}, "BUG": {"aperto", "risolto"}}
DONE = {"implementato": "implementato", "risolto": "risolto"}
EMPTY = {"", "-", "—", "–"}
REVIEW_PREFIX = "review:"
ATTR_RE = re.compile(r"^- ([0-9a-f]{7,40}) attribuito a ((?:[A-Za-z0-9][A-Za-z0-9.-]*_)?(?:REQ|BUG)-\d{3,})\b",
                     re.M)
DATE_RE = re.compile(r"^##\s+(\d{4}-\d{2}-\d{2})\b")
# Riga dell'introduzione con cui un file di parte dichiara i percorsi dalla radice del repository.
ROOT_PATHS_RE = re.compile(r"^Percorsi dei file: dalla radice del repository\.?\s*$", re.M)


def part_of(name):
    """Nome della parte di un file dei requisiti: '' per il progetto, altrimenti la cartella che contiene docs/."""
    return "" if name == REQ_FILE else name.split("/")[-3]


def part_dir(name):
    """Cartella della parte ('thin-decoder/', 'search/search-rm3/'); '' per il progetto."""
    return "" if name == REQ_FILE else name[:-len(REQ_FILE)]


def path_base(name, text):
    """Prefisso dei percorsi di "File impattati": la cartella della parte, salvo dichiarazione contraria."""
    return "" if ROOT_PATHS_RE.search(text) else part_dir(name)


def qualify(part, local):
    return "%s_%s" % (part, local) if part else local


def git(root, *args):
    r = subprocess.run(
        ["git", "-c", "core.quotepath=false", *args],
        cwd=root, capture_output=True, text=True, encoding="utf-8", errors="replace",
    )
    if r.returncode != 0:
        raise RuntimeError(r.stderr.strip() or "git " + " ".join(args))
    return r.stdout


def split_row(line):
    cells = re.split(r"(?<!\\)\|", line.strip())
    if cells and not cells[0].strip():
        cells = cells[1:]
    if cells and not cells[-1].strip():
        cells = cells[:-1]
    return [c.strip().replace("\\|", "|") for c in cells]


def parse_files(cell):
    found = re.findall(r"`([^`]+)`", cell)
    if not found and cell.strip() not in EMPTY:
        found = cell.split(",")
    out = []
    for f in found:
        f = f.strip().replace("\\", "/")
        if f.startswith("./"):
            f = f[2:]
        if f and f not in EMPTY and f not in out:
            out.append(f)
    return out


def is_trace(f):
    """File di tracciabilita': CHANGELOG.md e i REQUIREMENTS.md dentro una cartella docs/."""
    return f == LOG_FILE or bool(TRACE_RE.search(f))


def is_req_doc(f):
    """REQUIREMENTS.md dentro una cartella docs/: tracciabilita', ma anche documento (testo, note, appendici)."""
    return bool(TRACE_RE.search(f))


def req_files(root):
    """docs/REQUIREMENTS.md seguito dai <tool>/docs/REQUIREMENTS.md (esclusi i file ignorati da git)."""
    listed = git(root, "ls-files", "--cached", "--others", "--exclude-standard").splitlines()
    tools = sorted({f for f in listed if TOOL_REQ_RE.match(f) and (root / f).is_file()})
    return [REQ_FILE] + tools


def parse_items(text, errors, name):
    """Legge le righe delle tabelle il cui header contiene la colonna ID; l'ID e' qualificato con la parte."""
    part = part_of(name)
    base = path_base(name, text)
    items, header = [], None
    for n, line in enumerate(text.splitlines(), 1):
        if not line.lstrip().startswith("|"):
            header = None
            continue
        cells = split_row(line)
        if cells and all(re.fullmatch(r":?-+:?", c) for c in cells):
            continue
        if header is None:
            header = [c.lower() for c in cells]
            continue
        if "id" not in header:
            continue
        row = dict(zip(header, cells))
        local = row.get("id", "")
        if not LOCAL_RE.fullmatch(local):
            errors.append(("%s:%d" % (name, n), "ID non valido (dentro il file va senza prefisso): '%s'" % local))
            continue
        ident = qualify(part, local)
        if len(cells) != len(header):
            errors.append((ident, "%s:%d: %d celle, attese %d (un '|' nel testo?)"
                           % (name, n, len(cells), len(header))))
        ref = row.get("req", "").strip()
        items.append({
            "id": ident,
            "local": local,
            "part": part,
            "file": name,
            "kind": local.split("-")[0],
            "state": row.get("stato", "").strip().lower(),
            # Percorsi riportati alla radice del repository per i confronti con file e commit.
            "files": [base + f for f in parse_files(row.get("file impattati", ""))],
            "ref": "" if ref in EMPTY else ref,
        })
    return items


def file_births(root, files):
    """Per ogni file dei requisiti: (hash, data) del commit che lo ha creato, seguendo i rinomini; None se mai committato."""
    births = {}
    for name in files:
        try:
            out = git(root, "log", "--follow", "--diff-filter=A", "--format=%H %cs", "--", name).split()
        except RuntimeError:
            out = []
        births[name] = (out[-2][:7], out[-1]) if len(out) >= 2 else None
    return births


class Resolver:
    """Risolve un ID citato fuori dal proprio file nell'ID qualificato di un elemento in tabella."""

    def __init__(self, items, births, commit_pos):
        self.by_id = {it["id"]: it for it in items}
        self.by_local = {}
        for it in items:
            if it["part"]:
                self.by_local.setdefault(it["local"], []).append(it)
        self.births = births
        self.commit_pos = commit_pos  # hash -> posizione nella storia (0 = piu' recente)

    def resolve(self, part, local, pos=None, date=None):
        """pos: posizione del commit che cita l'ID; date: data della voce del changelog."""
        if part:
            return qualify(part, local)
        if local in self.by_id or local not in self.by_local:
            return local
        cands = self.by_local[local]
        if len(cands) == 1:
            return cands[0]["id"]

        def born(it):
            b = self.births.get(it["file"])
            if not b:
                return False
            if pos is not None:
                p = self.commit_pos.get(b[0])
                return p is not None and p >= pos
            return date is not None and b[1] <= date

        alive = [it for it in cands if born(it)]
        if len(alive) == 1:
            return alive[0]["id"]
        if not alive:
            # Nessun file esisteva ancora (es. requisiti spostati da un file all'altro): il file creato per primo.
            def key(it):
                b = self.births.get(it["file"])
                return (b is None, -self.commit_pos.get(b[0], -1) if b else 0)
            return sorted(cands, key=key)[0]["id"]
        return None  # ambiguo


def load_commits(root):
    """Commit dal piu' recente al piu' vecchio: hash, messaggio, ID citati, file toccati."""
    try:
        out = git(root, "log", "--name-only", "--format=%x1e%H%x1f%B%x1f")
    except RuntimeError:
        return []
    commits = []
    for rec in out.split("\x1e")[1:]:
        parts = rec.split("\x1f")
        if len(parts) < 3:
            continue
        msg = parts[1].strip()
        subject = msg.splitlines()[0] if msg else ""
        commits.append({
            "hash": parts[0].strip()[:7],
            "subject": subject,
            # Solo l'oggetto: il corpo puo' citare altri ID come semplice rimando.
            # Gli ID citati si risolvono dopo aver letto le tabelle (Resolver).
            "refs": [(m.group(1), m.group(2)) for m in ID_RE.finditer(subject)],
            "ids": set(),
            "files": [f.strip() for f in parts[2].splitlines() if f.strip()],
        })
    return commits


def main():
    if hasattr(sys.stdout, "reconfigure"):
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    start = Path(sys.argv[1] if len(sys.argv) > 1 else ".").resolve()
    try:
        root = Path(git(start, "rev-parse", "--show-toplevel").strip())
    except (RuntimeError, OSError):
        print("check_trace: %s non e' un repository git" % start)
        return 2
    req_path = root / REQ_FILE
    if not req_path.is_file():
        print("check_trace: manca %s" % REQ_FILE)
        return 2

    errors, warnings, review = [], [], []
    files = req_files(root)
    items = []
    for name in files:
        items += parse_items((root / name).read_text(encoding="utf-8"), errors, name)
    by_id = {}
    for it in items:
        if it["id"] in by_id:
            errors.append((it["id"], "ID duplicato in tabella"))
        by_id[it["id"]] = it
    parts = {}
    for name in files[1:]:
        parts.setdefault(part_of(name), []).append(name)
    for part, names in sorted(parts.items()):
        if len(names) > 1:
            errors.append((part, "stesso nome di parte per piu' file: %s" % ", ".join(names)))

    tracked = set(git(root, "ls-files").splitlines())
    commits = load_commits(root)
    resolver = Resolver(items, file_births(root, files), {c["hash"]: i for i, c in enumerate(commits)})
    for i, c in enumerate(commits):
        for part, local in c["refs"]:
            ident = resolver.resolve(part, local, pos=i)
            if ident is None:
                warnings.append((c["hash"], "ID senza prefisso ambiguo tra piu' parti: %s" % local))
            else:
                c["ids"].add(ident)
    # Il campo Req di un bug: senza prefisso indica lo stesso file, poi il progetto (o un ID storico).
    for it in items:
        m = ID_RE.fullmatch(it["ref"]) if it["ref"] else None
        if m and not m.group(1):
            same = qualify(it["part"], m.group(2))
            it["ref"] = same if same in by_id else (resolver.resolve(None, m.group(2)) or it["ref"])
    # Ultima revisione di merito: le modifiche precedenti sono gia' state controllate.
    last_review = next((i for i, c in enumerate(commits) if c["subject"].startswith(REVIEW_PREFIX)),
                       len(commits))
    log_path = root / LOG_FILE
    log_text = log_path.read_text(encoding="utf-8") if log_path.is_file() else ""
    if not log_text:
        errors.append((LOG_FILE, "file mancante o vuoto"))
    # Commit che non si possono riscrivere, attribuiti a un ID con una voce del changelog:
    # "- <hash> attribuito a REQ-NNN — motivo".
    for m in ATTR_RE.finditer(log_text):
        found = [i for i, c in enumerate(commits)
                 if m.group(1).startswith(c["hash"]) or c["hash"].startswith(m.group(1))]
        if found:
            r = ID_RE.fullmatch(m.group(2))
            ident = resolver.resolve(r.group(1), r.group(2), pos=found[0])
            if ident:
                commits[found[0]]["ids"].add(ident)
        else:
            warnings.append((m.group(1), "attribuito a %s in %s ma assente dalla storia" % (m.group(2), LOG_FILE)))
    # Voci del changelog per ID qualificato; gli ID senza prefisso si risolvono con la data della voce.
    log_lines = {}
    date = None
    for ln in log_text.splitlines():
        d = DATE_RE.match(ln)
        if d:
            date = d.group(1)
            continue
        for m in ID_RE.finditer(ln):
            ident = resolver.resolve(m.group(1), m.group(2), date=date)
            if ident:
                log_lines.setdefault(ident, []).append(ln)

    # Stati, riferimenti, esistenza dei file, changelog
    for it in items:
        ident, state = it["id"], it["state"]
        if state not in STATES[it["kind"]]:
            errors.append((ident, "stato non valido: '%s'" % state))
        if it["ref"] and it["ref"] not in by_id:
            errors.append((ident, "riferimento a requisito inesistente: %s" % it["ref"]))
        if state in DONE and not it["files"]:
            errors.append((ident, "stato '%s' senza file impattati" % state))
        if state != "annullato":
            for f in it["files"]:
                if f == LOG_FILE:
                    warnings.append((ident, "i file di tracciabilita' non vanno elencati: %s" % f))
                elif is_req_doc(f):
                    pass  # ammesso se l'ID ne corregge il testo: controllato con i commit, sotto
                elif not (root / f).is_file():
                    errors.append((ident, "file elencato inesistente: %s" % f))
                elif f not in tracked:
                    errors.append((ident, "file elencato non versionato: %s" % f))
        lines = log_lines.get(ident, [])
        if not lines:
            errors.append((ident, "nessuna voce in %s" % LOG_FILE))
        elif state in DONE and not any(DONE[state] in ln.lower() for ln in lines):
            warnings.append((ident, "manca la voce '%s' in %s" % (DONE[state], LOG_FILE)))

    for ident in sorted(set(log_lines) - set(by_id)):
        warnings.append((ident, "citato in %s ma assente dalla tabella" % LOG_FILE))

    # Corrispondenza tra tabella e commit
    for it in items:
        ident = it["id"]
        own = [i for i, c in enumerate(commits) if ident in c["ids"]]
        # Un REQUIREMENTS.md elencato in tabella conta come contenuto dell'ID: l'ID ne corregge il testo.
        docs = {f for f in it["files"] if is_req_doc(f)}

        def content(f):
            return not is_trace(f) or f in docs

        touched = set()
        for i in own:
            touched.update(f for f in commits[i]["files"] if content(f))
        if it["state"] not in DONE:
            continue
        code_commits = [i for i in own if any(content(f) for f in commits[i]["files"])]
        if not code_commits:
            errors.append((ident, "stato '%s' ma nessun commit con questo ID modifica il codice"
                           % it["state"]))
            continue
        for f in it["files"]:
            if f not in touched:
                warnings.append((ident, "in tabella ma mai modificato da un commit dell'ID: %s" % f))
        for f in sorted(touched - set(it["files"])):
            if (root / f).is_file():
                warnings.append((ident, "modificato da un commit dell'ID ma assente in tabella: %s" % f))
        first = max(code_commits)  # primo commit di codice dell'ID (il piu' vecchio)
        for f in it["files"]:
            if f in docs:
                continue  # ogni commit di tracciabilita' lo tocca: non e' un segnale utile
            for c in commits[:min(first, last_review)]:
                if f in c["files"] and ident not in c["ids"]:
                    who = ", ".join(sorted(c["ids"])) or "senza ID"
                    review.append((ident, "%s modificato dopo da %s (%s)" % (f, c["hash"], who)))

    # Commit senza ID, a partire dall'introduzione della tabella
    with_table = [i for i, c in enumerate(commits) if any(is_trace(f) and f != LOG_FILE for f in c["files"])]
    if not with_table:
        warnings.append((REQ_FILE, "non ancora presente in alcun commit"))
    else:
        for c in commits[:max(with_table)]:
            code = [f for f in c["files"] if not is_trace(f)]
            if code and not c["ids"]:
                warnings.append((c["hash"], "commit senza ID che modifica %d file: %s"
                                 % (len(code), c["subject"])))
            for ident in sorted(c["ids"] - set(by_id)):
                warnings.append((c["hash"], "commit cita un ID assente dalla tabella: %s" % ident))

    dirty = [ln for ln in git(root, "status", "--porcelain").splitlines() if ln.strip()]
    if dirty:
        warnings.append(("working tree", "%d modifiche non committate" % len(dirty)))

    # Rapporto
    def count(kind):
        sel = [it for it in items if it["kind"] == kind]
        states = {}
        for it in sel:
            states[it["state"]] = states.get(it["state"], 0) + 1
        detail = ", ".join("%d %s" % (v, k) for k, v in sorted(states.items()))
        return "%d%s" % (len(sel), " (%s)" % detail if detail else "")

    print("check_trace: %s" % root)
    print("File requisiti: %s" % ", ".join(files))
    print("Requisiti: %s · Bug: %s" % (count("REQ"), count("BUG")))
    if last_review < len(commits):
        c = commits[last_review]
        print("Ultima revisione: %s %s" % (c["hash"], c["subject"]))
    for title, rows in (("ERRORI", errors), ("AVVISI", warnings),
                        ("DA RIVEDERE nel merito", review)):
        if rows:
            print("\n%s (%d)" % (title, len(rows)))
            for who, what in rows:
                print("  %-12s %s" % (who, what))
    verdict = "DISALLINEATO" if errors else ("ALLINEATO con avvisi" if warnings else "ALLINEATO")
    print("\nEsito: %s" % verdict)
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main())
