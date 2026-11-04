# Genera l'ipertesto HTML dei requisiti di una repository (REQ-154), da aprire nel browser senza server né Internet:
#   una pagina per il progetto e per ogni tool o sotto-tool (REQ-155, REQ-156), index.html con i collegamenti fra
#   le pagine (REQ-157), un collegamento per ogni ID citato (REQ-158), filtro e ordinamento delle tabelle (REQ-160).
# Funziona su qualunque repository con la struttura di REQ-151: docs\REQUIREMENTS.md nella radice per il progetto,
# <cartella>\docs\REQUIREMENTS.md per ogni tool. Lo script non appartiene a nessun tool (REQ-005).
# Uso: .\req-html.ps1 -Repo <radice della repository> [-Out <cartella>]   (default: <Repo>\docs\html, REQ-159)
param(
    [Parameter(Mandatory = $true)][string]$Repo,
    [string]$Out
)
$ErrorActionPreference = 'Stop'

$Repo = (Resolve-Path -LiteralPath $Repo).Path.TrimEnd('\')
$ProjectFile = Join-Path $Repo 'docs\REQUIREMENTS.md'
if (-not (Test-Path -LiteralPath $ProjectFile -PathType Leaf)) { throw "requisiti di progetto non trovati: $ProjectFile" }
if (-not $Out) { $Out = Join-Path $Repo 'docs\html' }
New-Item -ItemType Directory -Force -Path $Out | Out-Null
$Out = (Resolve-Path -LiteralPath $Out).Path.TrimEnd('\')
$Utf8 = New-Object System.Text.UTF8Encoding $false

# --- File dei requisiti (REQ-155) ----------------------------------------------------------------------------------
# Ogni cartella con docs\REQUIREMENTS.md, a qualunque profondità; la radice è il progetto. Le cartelle di build,
# i collegamenti e la cartella di uscita non si esplorano.
$Skip = @('.git', 'target', 'node_modules')
function Find-ReqDirs([string]$Dir) {
    if (Test-Path -LiteralPath (Join-Path $Dir 'docs\REQUIREMENTS.md') -PathType Leaf) { $Dir }
    foreach ($Sub in Get-ChildItem -LiteralPath $Dir -Directory -Force -ErrorAction SilentlyContinue) {
        if ($Skip -contains $Sub.Name -or $Sub.FullName -eq $Out -or ($Sub.Attributes -band [IO.FileAttributes]::ReparsePoint)) { continue }
        Find-ReqDirs $Sub.FullName
    }
}

$Pages = @(foreach ($Dir in Find-ReqDirs $Repo) {
    $Rel = ($Dir.Substring($Repo.Length).TrimStart('\')) -replace '\\', '/'
    $File = Join-Path $Dir 'docs\REQUIREMENTS.md'
    $Label = 'progetto'; $Page = 'progetto.html'
    if ($Rel) { $Label = $Rel; $Page = ($Rel -replace '/', '.') + '.html' }
    [pscustomobject]@{
        Rel = $Rel; Label = $Label; Page = $Page; File = $File
        Lines = [IO.File]::ReadAllLines($File, $Utf8)
        Counts = [ordered]@{}
    }
})
$PageOfFile = @{}
foreach ($P in $Pages) { $PageOfFile[$P.File.ToLowerInvariant()] = $P.Page }

# --- Pagina di ogni ID (REQ-158) -----------------------------------------------------------------------------------
# Un ID appartiene alla pagina della tabella in cui è nella prima colonna.
$IdPage = @{}
foreach ($P in $Pages) {
    foreach ($L in $P.Lines) {
        if ($L -match '^\s*\|\s*((?:REQ|BUG)-\d+)\s*\|' -and -not $IdPage.ContainsKey($Matches[1])) { $IdPage[$Matches[1]] = $P.Page }
    }
}

# --- Markdown → HTML (REQ-156) -------------------------------------------------------------------------------------
# Solo ciò che compare nei file dei requisiti: titoli, paragrafi, elenchi, tabelle GFM, blocchi di codice,
# separatori, commenti HTML e, nel testo, codice, grassetto, corsivo e link.
function Enc([string]$s) { [System.Net.WebUtility]::HtmlEncode($s) }

# Un link a un file dei requisiti porta alla sua pagina (REQ-158); gli altri link relativi sono ricalcolati dalla
# cartella di uscita, così restano validi.
function Resolve-Href([string]$Url, [string]$SrcDir) {
    if ($Url -match '^(#|[a-zA-Z][a-zA-Z0-9+.-]*:)') { return $Url }
    $Parts = $Url -split '#', 2
    $Abs = [IO.Path]::GetFullPath((Join-Path $SrcDir ([Uri]::UnescapeDataString($Parts[0]))))
    $Href = $PageOfFile[$Abs.ToLowerInvariant()]
    if (-not $Href) { $Href = ([Uri]"$Out\").MakeRelativeUri([Uri]$Abs).ToString() }
    if ($Parts.Count -gt 1) { $Href += '#' + $Parts[1] }
    $Href
}

function Format-Plain([string]$s, [bool]$Ids) {
    $s = Enc ($s -replace '\\([!-/:-@\[-`{-~])', '$1')
    $s = $s -replace '\*\*(.+?)\*\*', '<strong>$1</strong>'
    $s = $s -replace '(?<![\w*])\*(?=\S)(.+?)(?<=\S)\*(?![\w*])', '<em>$1</em>'
    if ($Ids) {
        $s = [regex]::Replace($s, '\b(?:REQ|BUG)-\d+\b', {
            param($m)
            $Target = $IdPage[$m.Value]
            if ($Target) { '<a class="id" href="' + $Target + '#' + $m.Value + '">' + $m.Value + '</a>' } else { $m.Value }
        })
    }
    $s
}

$InlineTokens = '`([^`]+)`|\[((?:`[^`]*`|[^\]`])+)\]\(([^)\s]+)\)'
function Format-Inline([string]$s, [string]$SrcDir, [bool]$Ids = $true) {
    $sb = New-Object System.Text.StringBuilder
    $pos = 0
    foreach ($m in [regex]::Matches($s, $InlineTokens)) {
        [void]$sb.Append((Format-Plain $s.Substring($pos, $m.Index - $pos) $Ids))
        if ($m.Groups[1].Success) {
            [void]$sb.Append('<code>' + (Enc $m.Groups[1].Value) + '</code>')
        } else {
            $Href = Enc (Resolve-Href $m.Groups[3].Value $SrcDir)
            [void]$sb.Append('<a href="' + $Href + '">' + (Format-Inline $m.Groups[2].Value $SrcDir $false) + '</a>')
        }
        $pos = $m.Index + $m.Length
    }
    [void]$sb.Append((Format-Plain $s.Substring($pos) $Ids))
    $sb.ToString()
}

function Split-Row([string]$Line) {
    $l = $Line.Trim()
    if ($l.StartsWith('|')) { $l = $l.Substring(1) }
    if ($l.EndsWith('|') -and -not $l.EndsWith('\|')) { $l = $l.Substring(0, $l.Length - 1) }
    $Esc = [string][char]1
    , @(($l -replace '\\\|', $Esc) -split '\|' | ForEach-Object { $_.Trim().Replace($Esc, '|') })
}

$ListItem = '^(\s*)([-*+]|\d+[.)])\s+(.*)$'
$TableSep = '^\s*\|?\s*:?-+:?\s*(\|\s*:?-+:?\s*)*\|?\s*$'
$BlockStart = '^(#{1,6})\s|^\s*(```|~~~)|^\s*\|'

function Convert-Body($P) {
    $L = $P.Lines; $n = $L.Count; $i = 0
    $Src = Split-Path $P.File
    $Seen = @{}
    $sb = New-Object System.Text.StringBuilder
    while ($i -lt $n) {
        $Line = $L[$i]
        if ($Line -match '^\s*(```|~~~)') {
            $Fence = $Matches[1]; $i++
            $Code = New-Object System.Collections.Generic.List[string]
            while ($i -lt $n -and $L[$i] -notmatch "^\s*$Fence") { $Code.Add($L[$i]); $i++ }
            $i++
            [void]$sb.Append('<pre><code>' + (Enc ($Code -join "`n")) + "</code></pre>`n")
        }
        elseif ($Line -match '^\s*$') { $i++ }
        elseif ($Line -match '^\s*<!--') {
            while ($i -lt $n -and $L[$i] -notmatch '-->') { $i++ }
            $i++
        }
        elseif ($Line -match '^(#{1,6})\s+(.*?)\s*#*\s*$') {
            $h = $Matches[1].Length
            [void]$sb.Append("<h$h>" + (Format-Inline $Matches[2] $Src) + "</h$h>`n")
            $i++
        }
        elseif ($Line -match '^\s*([-*_])(\s*\1){2,}\s*$') { [void]$sb.Append("<hr>`n"); $i++ }
        elseif ($Line -match '^\s*\|' -and $i + 1 -lt $n -and $L[$i + 1] -match $TableSep) {
            $Head = Split-Row $Line
            $i += 2
            $Rows = New-Object System.Collections.Generic.List[object]
            while ($i -lt $n -and $L[$i] -match '^\s*\|') { $Rows.Add((Split-Row $L[$i])); $i++ }
            $St = [array]::IndexOf([string[]]$Head, 'Stato')
            [void]$sb.Append('<div class="tbl"><table><thead><tr>')
            foreach ($c in $Head) { [void]$sb.Append('<th>' + (Format-Inline $c $Src) + '</th>') }
            [void]$sb.Append("</tr></thead><tbody>`n")
            foreach ($r in $Rows) {
                $Id = ''
                if ($r[0] -match '^(?:REQ|BUG)-\d+$' -and -not $Seen.ContainsKey($r[0])) { $Seen[$r[0]] = 1; $Id = ' id="' + $r[0] + '"' }
                if ($St -ge 0 -and $St -lt $r.Count -and $r[0] -match '^REQ-\d+$') { $P.Counts[$r[$St]] = 1 + $P.Counts[$r[$St]] }
                [void]$sb.Append("<tr$Id>")
                for ($c = 0; $c -lt $Head.Count; $c++) {
                    $v = ''; if ($c -lt $r.Count) { $v = $r[$c] }
                    $Cls = ''
                    if ($c -eq $St) { $Cls = ' class="st st-' + (Enc $v) + '"' }
                    elseif ($c -eq 0 -and $v -match '^(?:REQ|BUG)-\d+$') { $Cls = ' class="idc"' }
                    [void]$sb.Append("<td$Cls>" + (Format-Inline $v $Src) + '</td>')
                }
                [void]$sb.Append("</tr>`n")
            }
            [void]$sb.Append("</tbody></table></div>`n")
        }
        elseif ($Line -match $ListItem) {
            $Items = New-Object System.Collections.Generic.List[object]
            while ($i -lt $n) {
                $Cur = $L[$i]
                if ($Cur -match $ListItem) {
                    $Ind = $Matches[1].Length; $Mark = $Matches[2]; $Text = $Matches[3]
                    $Items.Add([pscustomobject]@{ Ind = $Ind; Tag = $(if ($Mark -match '\d') { 'ol' } else { 'ul' }); Text = $Text })
                    $i++
                }
                elseif ($Cur -match '^\s+\S') { $Items[$Items.Count - 1].Text += ' ' + $Cur.Trim(); $i++ }
                elseif ($Cur -match '^\s*$' -and $i + 1 -lt $n -and ($L[$i + 1] -match '^\s+\S' -or $L[$i + 1] -match $ListItem)) { $i++ }
                else { break }
            }
            $Stack = New-Object System.Collections.Stack
            foreach ($It in $Items) {
                while ($Stack.Count -and $It.Ind -lt $Stack.Peek()[0]) { [void]$sb.Append('</li></' + $Stack.Pop()[1] + ">`n") }
                if ($Stack.Count -and $It.Ind -eq $Stack.Peek()[0]) { [void]$sb.Append("</li>`n") }
                else { [void]$sb.Append('<' + $It.Tag + ">`n"); $Stack.Push(@($It.Ind, $It.Tag)) }
                [void]$sb.Append('<li>' + (Format-Inline $It.Text $Src))
            }
            while ($Stack.Count) { [void]$sb.Append('</li></' + $Stack.Pop()[1] + ">`n") }
        }
        else {
            $Para = New-Object System.Collections.Generic.List[string]
            $Para.Add($Line.Trim()); $i++
            while ($i -lt $n -and $L[$i] -notmatch '^\s*$' -and $L[$i] -notmatch $BlockStart -and $L[$i] -notmatch $ListItem) { $Para.Add($L[$i].Trim()); $i++ }
            [void]$sb.Append('<p>' + (Format-Inline ($Para -join ' ') $Src) + "</p>`n")
        }
    }
    $sb.ToString()
}

# --- Pagine --------------------------------------------------------------------------------------------------------
$Css = @'
:root{--bg:#fff;--fg:#1d2330;--muted:#5d6675;--line:#d9dee6;--head:#f2f4f8;--link:#1f5fbf;--hl:#fff3bf;--ok:#1e7a3c;--open:#a35c00;--off:#80868f}
@media (prefers-color-scheme:dark){:root{--bg:#15181e;--fg:#e3e7ee;--muted:#9aa3b2;--line:#2e3440;--head:#1e232b;--link:#7fb0ff;--hl:#4a3f12;--ok:#5fcf86;--open:#f0a851;--off:#8b929c}}
*{box-sizing:border-box}
body{margin:0;background:var(--bg);color:var(--fg);font:15px/1.5 system-ui,-apple-system,"Segoe UI",sans-serif}
nav{position:sticky;top:0;z-index:3;display:flex;flex-wrap:wrap;gap:4px 16px;padding:10px 16px;background:var(--head);border-bottom:1px solid var(--line);font-size:14px}
nav .here{font-weight:600}
main{max-width:1400px;margin:0 auto;padding:16px 16px 64px}
a{color:var(--link)}
h1{font-size:24px;margin:8px 0 16px}
h2{font-size:19px;margin-top:32px;padding-bottom:4px;border-bottom:1px solid var(--line)}
h3{font-size:16px;margin-top:24px}
code{font:13px/1.4 Consolas,ui-monospace,monospace;background:var(--head);padding:1px 4px;border-radius:3px}
pre{background:var(--head);padding:12px;overflow-x:auto;border-radius:4px}
pre code{padding:0;background:none}
.tbl{margin:16px 0}
.tools{display:flex;flex-wrap:wrap;gap:8px;align-items:center;margin-bottom:6px;font-size:14px}
.tools input{flex:1 1 220px;max-width:420px}
input,select{font:inherit;padding:4px 8px;color:var(--fg);background:var(--bg);border:1px solid var(--line);border-radius:4px}
.tools .count{color:var(--muted)}
table{border-collapse:collapse;width:100%}
th,td{border:1px solid var(--line);padding:6px 8px;text-align:left;vertical-align:top;overflow-wrap:anywhere}
th{position:sticky;top:var(--nav,40px);z-index:2;background:var(--head);cursor:pointer;user-select:none}
th[data-dir=asc]::after{content:" \25B2";font-size:10px}
th[data-dir=desc]::after{content:" \25BC";font-size:10px}
tr{scroll-margin-top:calc(var(--nav,40px) + 48px)}
tr:target td{background:var(--hl)}
td.idc,td.st{white-space:nowrap;overflow-wrap:normal}
td.st{font-weight:600}
.st-implementato,.st-risolto{color:var(--ok)}
.st-aperto{color:var(--open)}
.st-annullato{color:var(--off)}
td.num{text-align:right}
'@

# L'altezza della barra (REQ-157) serve alle intestazioni fisse delle tabelle. Ogni tabella ha il filtro per testo
# e stato e l'ordinamento per colonna (REQ-160).
$Js = @'
(function () {
  var nav = document.querySelector('nav');
  function navHeight() { document.documentElement.style.setProperty('--nav', nav.offsetHeight + 'px'); }
  navHeight(); window.addEventListener('resize', navHeight);
  function text(row, i) { return row.cells[i] ? row.cells[i].textContent.trim() : ''; }
  var tables = [];
  document.querySelectorAll('.tbl').forEach(function (box) {
    var table = box.querySelector('table'), body = table.tBodies[0];
    var ths = [].slice.call(table.tHead.rows[0].cells), rows = [].slice.call(body.rows), st = -1;
    ths.forEach(function (th, i) { if (th.textContent.trim() === 'Stato') st = i; });
    var tools = document.createElement('div'); tools.className = 'tools';
    var q = document.createElement('input'); q.type = 'search'; q.placeholder = 'Filtra le righe'; tools.appendChild(q);
    var sel = null;
    if (st >= 0) {
      sel = document.createElement('select');
      sel.add(new Option('Tutti gli stati', ''));
      var seen = {};
      rows.forEach(function (r) { var v = text(r, st); if (v && !seen[v]) { seen[v] = 1; sel.add(new Option(v, v)); } });
      tools.appendChild(sel);
    }
    var count = document.createElement('span'); count.className = 'count'; tools.appendChild(count);
    box.insertBefore(tools, table);
    function apply() {
      var t = q.value.trim().toLowerCase(), s = sel ? sel.value : '', n = 0;
      rows.forEach(function (r) {
        var ok = (!t || r.textContent.toLowerCase().indexOf(t) >= 0) && (!s || text(r, st) === s);
        r.hidden = !ok; if (ok) n++;
      });
      count.textContent = n + ' di ' + rows.length + ' righe';
    }
    q.addEventListener('input', apply);
    if (sel) sel.addEventListener('change', apply);
    ths.forEach(function (th, i) {
      th.title = 'Ordina per questa colonna';
      th.addEventListener('click', function () {
        var dir = th.dataset.dir === 'asc' ? 'desc' : 'asc', k = dir === 'asc' ? 1 : -1;
        ths.forEach(function (h) { delete h.dataset.dir; });
        th.dataset.dir = dir;
        rows.slice().sort(function (a, b) {
          return k * text(a, i).localeCompare(text(b, i), 'it', { numeric: true, sensitivity: 'base' });
        }).forEach(function (r) { body.appendChild(r); });
      });
    });
    apply();
    tables.push({ rows: rows, reset: function () { q.value = ''; if (sel) sel.value = ''; apply(); } });
  });
  // Una riga raggiunta da un collegamento ma nascosta dal filtro torna visibile.
  function showTarget() {
    var el = location.hash && document.getElementById(decodeURIComponent(location.hash.slice(1)));
    if (el && el.hidden) {
      tables.forEach(function (t) { if (t.rows.indexOf(el) >= 0) t.reset(); });
      el.scrollIntoView();
    }
  }
  window.addEventListener('hashchange', showTarget); showTarget();
})();
'@

function Write-Page([string]$Name, [string]$Title, [string]$Body) {
    $Nav = New-Object System.Text.StringBuilder
    $Links = @(@{ Page = 'index.html'; Label = 'Indice' }) + @($Pages | ForEach-Object { @{ Page = $_.Page; Label = $_.Label } })
    foreach ($Link in $Links) {
        if ($Link.Page -eq $Name) { [void]$Nav.Append('<span class="here">' + (Enc $Link.Label) + '</span>') }
        else { [void]$Nav.Append('<a href="' + $Link.Page + '">' + (Enc $Link.Label) + '</a>') }
    }
    $Html = @"
<!doctype html>
<html lang="it">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>$(Enc $Title)</title>
<style>
$Css</style>
</head>
<body>
<nav>$($Nav.ToString())</nav>
<main>
$Body</main>
<script>
$Js</script>
</body>
</html>
"@
    [IO.File]::WriteAllText((Join-Path $Out $Name), $Html, $Utf8)
}

foreach ($P in $Pages) {
    $Title = $P.Label
    foreach ($L in $P.Lines) { if ($L -match '^#\s+(.*)$') { $Title = $Matches[1].Trim(); break } }
    Write-Page $P.Page $Title (Convert-Body $P)
}

# --- Indice (REQ-157) ----------------------------------------------------------------------------------------------
$RepoName = Split-Path $Repo -Leaf
$States = New-Object System.Collections.Generic.List[string]
foreach ($P in $Pages) { foreach ($s in $P.Counts.Keys) { if (-not $States.Contains($s)) { $States.Add($s) } } }
$Idx = New-Object System.Text.StringBuilder
[void]$Idx.Append('<h1>Requisiti — ' + (Enc $RepoName) + "</h1>`n")
[void]$Idx.Append('<p>Generato da <code>req-html.ps1</code> il ' + (Get-Date -Format 'yyyy-MM-dd HH:mm') + ' da <code>' + (Enc $Repo) + "</code>.</p>`n")
[void]$Idx.Append('<div class="tbl"><table><thead><tr><th>Parte</th><th>File</th>')
foreach ($s in $States) { [void]$Idx.Append('<th>' + (Enc $s) + '</th>') }
[void]$Idx.Append("<th>Requisiti</th></tr></thead><tbody>`n")
foreach ($P in $Pages) {
    $Path = 'docs/REQUIREMENTS.md'; if ($P.Rel) { $Path = $P.Rel + '/docs/REQUIREMENTS.md' }
    [void]$Idx.Append('<tr><td><a href="' + $P.Page + '">' + (Enc $P.Label) + '</a></td><td><code>' + (Enc $Path) + '</code></td>')
    $Tot = 0
    foreach ($s in $States) { $c = 0 + $P.Counts[$s]; $Tot += $c; [void]$Idx.Append('<td class="num">' + $c + '</td>') }
    [void]$Idx.Append('<td class="num">' + $Tot + "</td></tr>`n")
}
[void]$Idx.Append("</tbody></table></div>`n")
Write-Page 'index.html' "Requisiti — $RepoName" $Idx.ToString()

Write-Output "$($Pages.Count + 1) pagine scritte in $Out"
