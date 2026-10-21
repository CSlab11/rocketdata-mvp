@echo off
rem Avvia il server di gui, se non e' gia' in esecuzione (GUI-015).
rem Il PID viene salvato in %LOCALAPPDATA%\rocketdata-gui\gui.pid per stop.cmd.
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

set "PID="
for /f %%P in ('powershell -NoProfile -Command "(Start-Process -FilePath '%EXE%' -WorkingDirectory '%EXEDIR%' -WindowStyle Minimized -PassThru).Id"') do set "PID=%%P"
if not defined PID (
    echo Errore: impossibile avviare "%EXE%".
    exit /b 1
)

>"%PIDFILE%" echo %PID%
echo gui avviato ^(PID %PID%^).
