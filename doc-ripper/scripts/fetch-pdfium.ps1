# Scarica pdfium.dll (build bblanchon/pdfium-binaries, chromium/7881, compatibile con
# pdfium-render 0.9 "pdfium_latest") in doc-ripper\vendor\pdfium\.
# build.rs la copia poi accanto a doc-ripper.exe a ogni build.
$ErrorActionPreference = 'Stop'

$Release = 'chromium%2F7881'
$Sha256 = '73CC0DE638AC2095E7445BF56A38200A5B7C7CA0E9F4BA144598F2457377AC08'
$Url = "https://github.com/bblanchon/pdfium-binaries/releases/download/$Release/pdfium-win-x64.tgz"

$Vendor = Join-Path $PSScriptRoot '..\vendor\pdfium'
$Tmp = Join-Path ([System.IO.Path]::GetTempPath()) ("pdfium-" + [guid]::NewGuid())
New-Item -ItemType Directory -Force $Tmp | Out-Null
try {
    $Archive = Join-Path $Tmp 'pdfium-win-x64.tgz'
    Invoke-WebRequest $Url -OutFile $Archive -UseBasicParsing
    $Actual = (Get-FileHash $Archive -Algorithm SHA256).Hash
    if ($Actual -ne $Sha256) {
        throw "SHA-256 inatteso per pdfium-win-x64.tgz: $Actual"
    }
    # tar di Windows: un tar GNU nel PATH (es. Git Bash) leggerebbe "C:" come host remoto.
    & (Join-Path $env:SystemRoot 'System32\tar.exe') -xzf $Archive -C $Tmp
    if ($LASTEXITCODE -ne 0) { throw "estrazione fallita" }
    New-Item -ItemType Directory -Force $Vendor | Out-Null
    Copy-Item (Join-Path $Tmp 'bin\pdfium.dll') $Vendor -Force
    Copy-Item (Join-Path $Tmp 'LICENSE') (Join-Path $Vendor 'LICENSE') -Force
    Copy-Item (Join-Path $Tmp 'licenses') $Vendor -Recurse -Force
    Write-Output "pdfium.dll installata in $((Resolve-Path $Vendor).Path)"
}
finally {
    Remove-Item -Recurse -Force $Tmp
}
