# Genera l'ipertesto HTML dei requisiti di una repository (REQ-154), da aprire nel browser senza server né Internet:
#   una pagina per il progetto e per ogni tool o sotto-tool (REQ-155).
# Funziona su qualunque repository con la struttura di REQ-151: docs\REQUIREMENTS.md nella radice per il progetto,
# <cartella>\docs\REQUIREMENTS.md per ogni tool. Lo script non appartiene a nessun tool (REQ-005).
# Uso: .\req-html.ps1 -Repo <radice della repository>   (pagine in <Repo>\docs\html)
param(
    [Parameter(Mandatory = $true)][string]$Repo
)
$ErrorActionPreference = 'Stop'

$Repo = (Resolve-Path -LiteralPath $Repo).Path.TrimEnd('\')
$ProjectFile = Join-Path $Repo 'docs\REQUIREMENTS.md'
if (-not (Test-Path -LiteralPath $ProjectFile -PathType Leaf)) { throw "requisiti di progetto non trovati: $ProjectFile" }
$Out = Join-Path $Repo 'docs\html'
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
    }
})

# --- Markdown → HTML -----------------------------------------------------------------------------------------------
function Enc([string]$s) { [System.Net.WebUtility]::HtmlEncode($s) }

function Format-Plain([string]$s) {
    $s = Enc ($s -replace '\\([!-/:-@\[-`{-~])', '$1')
    $s = $s -replace '\*\*(.+?)\*\*', '<strong>$1</strong>'
    $s = $s -replace '(?<![\w*])\*(?=\S)(.+?)(?<=\S)\*(?![\w*])', '<em>$1</em>'
    $s
}

$InlineTokens = '`([^`]+)`'
function Format-Inline([string]$s, [string]$SrcDir) {
    $sb = New-Object System.Text.StringBuilder
    $pos = 0
    foreach ($m in [regex]::Matches($s, $InlineTokens)) {
        [void]$sb.Append((Format-Plain $s.Substring($pos, $m.Index - $pos)))
        [void]$sb.Append('<code>' + (Enc $m.Groups[1].Value) + '</code>')
        $pos = $m.Index + $m.Length
    }
    [void]$sb.Append((Format-Plain $s.Substring($pos)))
    $sb.ToString()
}

function Split-Row([string]$Line) {
    $l = $Line.Trim()
    if ($l.StartsWith('|')) { $l = $l.Substring(1) }
    if ($l.EndsWith('|') -and -not $l.EndsWith('\|')) { $l = $l.Substring(0, $l.Length - 1) }
    $Esc = [string][char]1
    , @(($l -replace '\\\|', $Esc) -split '\|' | ForEach-Object { $_.Trim().Replace($Esc, '|') })
}

$TableSep = '^\s*\|?\s*:?-+:?\s*(\|\s*:?-+:?\s*)*\|?\s*$'
$BlockStart = '^(#{1,6})\s|^\s*\|'

function Convert-Body($P) {
    $L = $P.Lines; $n = $L.Count; $i = 0
    $Src = Split-Path $P.File
    $sb = New-Object System.Text.StringBuilder
    while ($i -lt $n) {
        $Line = $L[$i]
        if ($Line -match '^\s*$') { $i++ }
        elseif ($Line -match '^(#{1,6})\s+(.*?)\s*#*\s*$') {
            $h = $Matches[1].Length
            [void]$sb.Append("<h$h>" + (Format-Inline $Matches[2] $Src) + "</h$h>`n")
            $i++
        }
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
                [void]$sb.Append('<tr>')
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
        else {
            $Para = New-Object System.Collections.Generic.List[string]
            $Para.Add($Line.Trim()); $i++
            while ($i -lt $n -and $L[$i] -notmatch '^\s*$' -and $L[$i] -notmatch $BlockStart) { $Para.Add($L[$i].Trim()); $i++ }
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
main{max-width:1400px;margin:0 auto;padding:16px 16px 64px}
a{color:var(--link)}
h1{font-size:24px;margin:8px 0 16px}
h2{font-size:19px;margin-top:32px;padding-bottom:4px;border-bottom:1px solid var(--line)}
h3{font-size:16px;margin-top:24px}
code{font:13px/1.4 Consolas,ui-monospace,monospace;background:var(--head);padding:1px 4px;border-radius:3px}
.tbl{margin:16px 0}
table{border-collapse:collapse;width:100%}
th,td{border:1px solid var(--line);padding:6px 8px;text-align:left;vertical-align:top;overflow-wrap:anywhere}
th{position:sticky;top:0;z-index:2;background:var(--head)}
td.idc,td.st{white-space:nowrap;overflow-wrap:normal}
td.st{font-weight:600}
.st-implementato,.st-risolto{color:var(--ok)}
.st-aperto{color:var(--open)}
.st-annullato{color:var(--off)}
'@

function Write-Page([string]$Name, [string]$Title, [string]$Body) {
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
<main>
$Body</main>
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

Write-Output "$($Pages.Count) pagine scritte in $Out"
