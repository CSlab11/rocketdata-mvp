# Prepara doc-ripper\vendor\docling\ con tutto il necessario per eseguire Docling senza Python installato
# (REQ-052):
#   python\   Python embeddable con Docling e le sue dipendenze (doc-ripper\docling\requirements.txt)
# package.ps1 lo esegue solo se la cartella manca e poi la copia in dist\docling\.
$ErrorActionPreference = 'Stop'

$PyVersion = '3.12.10'
$PySha256 = '4ACBED6DD1C744B0376E3B1CF57CE906F9DC9E95E68824584C8099A63025A3C3'
$PyUrl = "https://www.python.org/ftp/python/$PyVersion/python-$PyVersion-embed-amd64.zip"
$GetPipUrl = 'https://bootstrap.pypa.io/get-pip.py'

# pip scrive il log su stderr: in PowerShell 5.1, con 'Stop', una riga di stderr rediretto
# interromperebbe lo script. L'esito dei comandi esterni si giudica solo dal codice di uscita.
function Invoke-Native([string] $Exe, [string[]] $Arguments, [string] $Failure) {
    $Previous = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    try { & $Exe @Arguments 2>&1 | ForEach-Object { "$_" } }
    finally { $ErrorActionPreference = $Previous }
    if ($LASTEXITCODE -ne 0) { throw $Failure }
}

$Vendor = Join-Path $PSScriptRoot '..\vendor\docling'
$Requirements = Join-Path $PSScriptRoot '..\docling\requirements.txt'
$Tmp = Join-Path ([System.IO.Path]::GetTempPath()) ("docling-" + [guid]::NewGuid())
New-Item -ItemType Directory -Force $Tmp | Out-Null
try {
    if (Test-Path $Vendor) { Remove-Item -Recurse -Force $Vendor }
    $Python = Join-Path $Vendor 'python'
    New-Item -ItemType Directory -Force $Python | Out-Null

    $Zip = Join-Path $Tmp 'python.zip'
    Invoke-WebRequest $PyUrl -OutFile $Zip -UseBasicParsing
    $Actual = (Get-FileHash $Zip -Algorithm SHA256).Hash
    if ($Actual -ne $PySha256) { throw "SHA-256 inatteso per python-$PyVersion-embed-amd64.zip: $Actual" }
    Expand-Archive $Zip -DestinationPath $Python

    # Il Python embeddable ignora site-packages finché nel ._pth non si abilita "import site".
    $Pth = Get-ChildItem $Python -Filter 'python*._pth' | Select-Object -First 1
    (Get-Content $Pth.FullName) -replace '^#\s*import site', 'import site' | Set-Content $Pth.FullName -Encoding ascii

    $Exe = Join-Path $Python 'python.exe'
    $GetPip = Join-Path $Tmp 'get-pip.py'
    Invoke-WebRequest $GetPipUrl -OutFile $GetPip -UseBasicParsing
    Invoke-Native $Exe @($GetPip, '--no-warn-script-location', '--quiet') 'installazione di pip fallita'
    Invoke-Native $Exe @('-m', 'pip', 'install', '--no-warn-script-location', '--quiet', '-r', $Requirements) 'installazione di Docling fallita'

    Write-Output "Docling installato in $((Resolve-Path $Vendor).Path)"
}
finally {
    Remove-Item -Recurse -Force $Tmp
}
