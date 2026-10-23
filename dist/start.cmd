@echo off
rem Avvia il server di gui, se non e' gia' in esecuzione (REQ-037).
rem Il PID viene salvato in %LOCALAPPDATA%\rocketdata-gui\gui.pid per stop.cmd.
rem Gli argomenti passano a gui.exe, es.: start.cmd --no-browser
setlocal

set "EXE=%~dp0gui.exe"
if not exist "%EXE%" set "EXE=%~dp0target\release\gui.exe"
if not exist "%EXE%" (
    echo Errore: gui.exe non trovato in "%~dp0" ne' in "%~dp0target\release\".
    exit /b 1
)

set "PIDDIR=%LOCALAPPDATA%\rocketdata-gui"
set "PIDFILE=%PIDDIR%\gui.pid"

set "PID="
if exist "%PIDFILE%" set /p PID=<"%PIDFILE%"
if defined PID (
    tasklist /FI "PID eq %PID%" /FI "IMAGENAME eq gui.exe" /NH | findstr /I "gui.exe" >nul && (
        echo gui e' gia' in esecuzione ^(PID %PID%^).
        exit /b 0
    )
)

if not exist "%PIDDIR%" mkdir "%PIDDIR%"
for %%F in ("%EXE%") do set "EXEDIR=%%~dpF"

rem Argomenti passati tramite variabile d'ambiente: nel comando PowerShell le virgolette si romperebbero.
set "GUI_ARGS=%*"
set "PID="
for /f %%P in ('powershell -NoProfile -Command "$a = @{ FilePath = '%EXE%'; WorkingDirectory = '%EXEDIR%'; WindowStyle = 'Minimized'; PassThru = $true }; if ($env:GUI_ARGS) { $a.ArgumentList = $env:GUI_ARGS }; (Start-Process @a).Id"') do set "PID=%%P"
if not defined PID (
    echo Errore: impossibile avviare "%EXE%".
    exit /b 1
)

>"%PIDFILE%" echo %PID%
echo gui avviato ^(PID %PID%^).
