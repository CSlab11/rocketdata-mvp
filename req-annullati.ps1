# Toglie i requisiti annullati dal file dei requisiti di una cartella, del progetto o di un tool (REQ-169).
# Lo script non appartiene a nessun tool (REQ-005).
# Uso: .\req-annullati.ps1 -Path <cartella del progetto o di un tool>
param(
    [Parameter(Mandatory = $true)][string]$Path
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
$Doc.Lines = [Collections.Generic.List[string]]@(for ($i = 0; $i -lt $Doc.Lines.Count; $i++) { if (-not $Gone[$i]) { $Doc.Lines[$i] } })
Write-Lines $Doc
