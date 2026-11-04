# Toglie i requisiti annullati dal file dei requisiti di una cartella, del progetto o di un tool (REQ-169).
# Lo script non appartiene a nessun tool (REQ-005).
# Con -WhatIf elenca le righe da togliere e le citazioni da aggiornare senza scrivere nulla (REQ-171).
# Uso: .\req-annullati.ps1 -Path <cartella del progetto o di un tool> [-WhatIf]
param(
    [Parameter(Mandatory = $true)][string]$Path,
    [switch]$WhatIf
)
$ErrorActionPreference = 'Stop'

$Dir = (Resolve-Path -LiteralPath $Path).Path.TrimEnd('\')
$File = Join-Path $Dir 'docs\REQUIREMENTS.md'
if (-not (Test-Path -LiteralPath $File -PathType Leaf)) { throw "file dei requisiti non trovato: $File" }

# --- Lettura e scrittura -------------------------------------------------------------------------------------------
# Il testo si divide solo sui LF: un eventuale CR resta nella riga, e il file si riscrive identico salvo le righe
# cambiate, con o senza BOM come l'originale.
function Read-Lines([string]$F) {
    $Bytes = [IO.File]::ReadAllBytes($F)
    $Bom = $Bytes.Length -ge 3 -and $Bytes[0] -eq 0xEF -and $Bytes[1] -eq 0xBB -and $Bytes[2] -eq 0xBF
    $Skip = 0; if ($Bom) { $Skip = 3 }
    $Text = (New-Object System.Text.UTF8Encoding $false).GetString($Bytes, $Skip, $Bytes.Length - $Skip)
    [pscustomobject]@{ File = $F; Bom = $Bom; Lines = [Collections.Generic.List[string]]$Text.Split("`n") }
}
function Write-Lines($Doc) {
    [IO.File]::WriteAllText($Doc.File, ($Doc.Lines -join "`n"), (New-Object System.Text.UTF8Encoding $Doc.Bom))
}

# --- Righe delle tabelle Requisiti ---------------------------------------------------------------------------------
# Una tabella Requisiti ha l'intestazione | ID | Requisito | Stato | …, a prescindere dal titolo della sezione; la
# tabella Bug (| ID | Descrizione | …) resta fuori. Restituisce indice di riga, ID, numero e stato di ogni requisito.
function Get-ReqRows($Lines) {
    $StateCol = -1
    for ($i = 0; $i -lt $Lines.Count; $i++) {
        $L = $Lines[$i].TrimEnd("`r")
        if (-not $L.TrimStart().StartsWith('|')) { $StateCol = -1; continue }
        $Cells = @($L.Split('|') | ForEach-Object { $_.Trim() })
        if ($Cells[1] -eq 'ID' -and $Cells[2] -eq 'Requisito') { $StateCol = [array]::IndexOf($Cells, 'Stato'); continue }
        if ($StateCol -lt 0 -or $Cells[1] -notmatch '^REQ-(\d+)$') { continue }
        [pscustomobject]@{ Index = $i; Id = $Cells[1]; Num = [int]$Matches[1]; State = $Cells[$StateCol] }
    }
}

# --- Righe da togliere (REQ-169) -----------------------------------------------------------------------------------
# La riga con l'ID più alto resta anche se annullata, perché il prossimo ID non riusi il suo numero (REQ-172).
$Doc = Read-Lines $File
$Rows = @(Get-ReqRows $Doc.Lines)
$Max = ($Rows | Measure-Object -Property Num -Maximum).Maximum
$Remove = @($Rows | Where-Object { $_.State -eq 'annullato' -and $_.Num -ne $Max })

Write-Host $File
foreach ($R in $Rows | Where-Object { $_.State -eq 'annullato' -and $_.Num -eq $Max }) { Write-Host "  conservato $($R.Id): ID più alto" }
foreach ($R in $Remove) { Write-Host "  tolto $($R.Id)" }
if (-not $Remove) { Write-Host '  nessun requisito annullato'; return }

$Gone = @{}; foreach ($R in $Remove) { $Gone[$R.Index] = $true }

# --- Citazioni rimaste (REQ-170) -----------------------------------------------------------------------------------
# Ogni citazione di un ID tolto riceve " (annullato)", se non l'ha già. Nel file stesso si cercano gli ID senza
# prefisso (non preceduti da lettera, cifra, _ o -, così thin-decoder_REQ-001 resta fuori); se la cartella è un tool,
# negli altri file dei requisiti della repository si cercano gli ID qualificati <tool>_REQ-NNN. Un ID senza prefisso
# in un altro file indica quel file o il progetto, mai il tool, e non si tocca.
$Alt = ($Remove | ForEach-Object { $_.Num.ToString('000') }) -join '|'
function Mark-Refs($D, [string]$Prefix, $SkipIndex) {
    $Re = '(?<![\w-])' + [regex]::Escape($Prefix) + 'REQ-(' + $Alt + ')(?!\d)(?! ?\(annullato\))'
    for ($i = 0; $i -lt $D.Lines.Count; $i++) {
        if ($SkipIndex -and $SkipIndex[$i]) { continue }
        $New = [regex]::Replace($D.Lines[$i], $Re, '$0 (annullato)')
        if ($New -ne $D.Lines[$i]) {
            $Ids = ([regex]::Matches($D.Lines[$i], $Re) | ForEach-Object { $_.Value }) -join ', '
            Write-Host "  riga $($i + 1): $Ids (annullato)"
            $D.Lines[$i] = $New
            $D.Changed = $true
        }
    }
}

$Doc | Add-Member Changed $true
Mark-Refs $Doc '' $Gone
$Doc.Lines = [Collections.Generic.List[string]]@(for ($i = 0; $i -lt $Doc.Lines.Count; $i++) { if (-not $Gone[$i]) { $Doc.Lines[$i] } })
$Others = @()

$Repo = git -C $Dir rev-parse --show-toplevel
if ($LASTEXITCODE -ne 0) { throw "la cartella non è in una repository git: $Dir" }
$Repo = (Resolve-Path -LiteralPath $Repo).Path.TrimEnd('\')
if ($Dir -ne $Repo) {
    $Prefix = (Split-Path -Leaf $Dir) + '_'
    $Skip = @('.git', 'target', 'node_modules')
    function Find-ReqFiles([string]$D) {
        $F = Join-Path $D 'docs\REQUIREMENTS.md'
        if ((Test-Path -LiteralPath $F -PathType Leaf) -and $F -ne $File) { $F }
        foreach ($Sub in Get-ChildItem -LiteralPath $D -Directory -Force -ErrorAction SilentlyContinue) {
            if ($Skip -contains $Sub.Name -or ($Sub.Attributes -band [IO.FileAttributes]::ReparsePoint)) { continue }
            Find-ReqFiles $Sub.FullName
        }
    }
    foreach ($F in Find-ReqFiles $Repo) {
        $O = Read-Lines $F
        $O | Add-Member Changed $false
        Write-Host $F
        Mark-Refs $O $Prefix $null
        if ($O.Changed) { $Others += $O } else { Write-Host '  nessuna citazione' }
    }
}

if ($WhatIf) { Write-Host 'Anteprima (-WhatIf): nessun file modificato.'; return }
Write-Lines $Doc
foreach ($O in $Others) { Write-Lines $O }
