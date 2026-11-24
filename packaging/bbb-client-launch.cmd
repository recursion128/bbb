@echo off
setlocal
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0bbb-client-launch.ps1" %*
exit /b %ERRORLEVEL%
