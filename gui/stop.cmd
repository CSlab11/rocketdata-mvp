@echo off
rem Arresta il server di gui avviato da start.cmd, insieme ai processi figli
rem (es. un doc-ripper in corso) (GUI-015).
setlocal

set "PIDFILE=%LOCALAPPDATA%\rocketdata-gui\gui.pid"
if not exist "%PIDFILE%" (
    echo gui non risulta in esecuzione.
    exit /b 0
)

set "PID="
set /p PID=<"%PIDFILE%"
if not defined PID (
    del "%PIDFILE%"
    echo gui non risulta in esecuzione.
    exit /b 0
)

rem Controlla anche il nome del processo: il PID potrebbe essere stato riassegnato.
tasklist /FI "PID eq %PID%" /FI "IMAGENAME eq gui.exe" /NH | findstr /I "gui.exe" >nul
if errorlevel 1 (
    del "%PIDFILE%"
    echo gui non risulta in esecuzione.
    exit /b 0
)

taskkill /PID %PID% /T /F >nul
if errorlevel 1 (
    echo Errore: impossibile arrestare gui ^(PID %PID%^).
    exit /b 1
)

del "%PIDFILE%"
echo gui arrestato ^(PID %PID%^).
