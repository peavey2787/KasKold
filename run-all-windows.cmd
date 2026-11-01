@echo off
rem Complete KasKold validation suite on Windows.
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0qa\windows\run-all.ps1" %*
exit /b %ERRORLEVEL%
