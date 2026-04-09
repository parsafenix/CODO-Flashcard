@echo off
setlocal
set "ROOT=%~dp0"
set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
cd /d "%ROOT%"
npm run tauri:dev
