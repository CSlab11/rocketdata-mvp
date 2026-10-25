# Prepara dist\ con tutto il necessario per usare RocketData (REQ-073):
#   gui.exe, doc-ripper.exe, search-rm3.exe, pdfium.dll, start.cmd, stop.cmd e la licenza di PDFium.
# Compila in release doc-ripper, gui e search-rm3, ciascuno nel proprio progetto Cargo: lo script non
# appartiene a nessun tool e i tool non condividono nulla (REQ-005).
# Uso: .\package.ps1   poi   dist\start.cmd
$ErrorActionPreference = 'Stop'

$Root = $PSScriptRoot
$Dist = Join-Path $Root 'dist'

$Cargo = (Get-Command cargo -ErrorAction SilentlyContinue).Source
if (-not $Cargo) { $Cargo = Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe' }
if (-not (Test-Path $Cargo)) { throw "cargo non trovato: installare Rust (rustup)" }

# doc-ripper copia pdfium.dll accanto al proprio eseguibile durante la build: va scaricata prima.
$Pdfium = Join-Path $Root 'doc-ripper\vendor\pdfium\pdfium.dll'
if (-not (Test-Path $Pdfium)) {
    & (Join-Path $Root 'doc-ripper\scripts\fetch-pdfium.ps1')
}

foreach ($Tool in 'doc-ripper', 'gui', 'search\search-rm3') {
    Write-Output "Compilazione di $Tool..."
    & $Cargo build --release --quiet --manifest-path (Join-Path $Root "$Tool\Cargo.toml")
    if ($LASTEXITCODE -ne 0) { throw "compilazione di $Tool fallita" }
}

# Una gui avviata da dist\ tiene bloccati i file: si controlla prima di cancellare qualsiasi cosa,
# così dist\ non resta a metà.
$Running = Get-Process gui, doc-ripper, search-rm3 -ErrorAction SilentlyContinue |
    Where-Object { $_.Path -and $_.Path.StartsWith("$Dist\", [StringComparison]::OrdinalIgnoreCase) }
if ($Running) {
    throw "la gui di $Dist è in esecuzione: fermarla con dist\stop.cmd e rilanciare lo script"
}
if (Test-Path $Dist) {
    Remove-Item $Dist -Recurse -Force
}
New-Item -ItemType Directory $Dist | Out-Null

$Files = @(
    'gui\target\release\gui.exe',
    'gui\start.cmd',
    'gui\stop.cmd',
    'doc-ripper\target\release\doc-ripper.exe',
    'doc-ripper\target\release\pdfium.dll',
    'search\search-rm3\target\release\search-rm3.exe'
)
foreach ($File in $Files) {
    Copy-Item (Join-Path $Root $File) $Dist
}
# Licenza di PDFium, da distribuire insieme alla DLL.
Copy-Item (Join-Path $Root 'doc-ripper\vendor\pdfium\LICENSE') (Join-Path $Dist 'pdfium-LICENSE.txt')

Write-Output "Pacchetto pronto in $Dist"
Get-ChildItem $Dist | ForEach-Object { Write-Output "  $($_.Name)" }
