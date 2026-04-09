@echo off
setlocal
set "ROOT=%~dp0"
set "APP=%ROOT%src-tauri\target\release\codo_flashcard.exe"

if not exist "%APP%" (
  echo Release build not found.
  echo Build the app first with: npm run tauri:build
  pause
  exit /b 1
)

start "" "%APP%"
