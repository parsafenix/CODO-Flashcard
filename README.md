# CODO: Flashcard

CODO: Flashcard is a fully offline desktop flashcard app for multilingual vocabulary study. It uses:

- Tauri 2
- React + TypeScript + Vite
- Rust + SQLite (`rusqlite`)

Built by PARSA FALAHATI  
LinkedIn: www.linkedin.com/in/parsa-falahati  
به امید فردایی بهتر ♥️

## What ships in this app

- Local deck, card, review, and settings persistence
- UTF-8 TXT import with BOM and `\n` / `\r\n` support
- Strict duplicate rejection based on the normalized 3-language tuple per deck
- Persian-aware normalization and mixed RTL/LTR rendering
- YekanBakh Persian font family bundled locally with the app
- Quizlet-like spaced repetition implemented explicitly in Rust
- Deck library, card CRUD, study sessions, session summary, export, backup, and reset controls

## Data rules

- All timestamps are stored in UTC
- Due cards are defined strictly as `next_review_at <= now_utc`
- `new` cards are tracked separately and do not count as due until they enter scheduled review
- Original text is preserved exactly; search and duplicate detection use normalized shadow fields

## Project layout

- `src/`: React frontend
- `src/components/ui`: shared UI building blocks
- `src/features`: deck, card, import, study, and settings features
- `src/lib`: typed API wrappers, normalization helpers, formatting, and keyboard utilities
- `src/styles`: global theme and typography
- `src/assets/branding`: bundled logos and YekanBakh fonts
- `src-tauri/src/commands`: Tauri command handlers
- `src-tauri/src/db`: SQLite setup, migrations, repositories
- `src-tauri/src/services`: normalization, import parsing, SRS, export, and backup logic
- `src-tauri/migrations`: SQL schema
- `scripts/tauri-runner.mjs`: local Tauri runner that injects Cargo and Node paths
- `launch-flashcard-dev.bat`: Windows development launcher
- `launch-flashcard-app.bat`: Windows release launcher

## Install dependencies

```bash
npm install
```

## Run in development

```bash
npm run tauri:dev
```

Windows shortcut launcher:

```bat
launch-flashcard-dev.bat
```

## Run tests

```bash
npm test
cargo test
```

## Build the production desktop app

```bash
npm run tauri:build
```

The command produces:

- Standalone release executable:
  - `src-tauri/target/release/codo_flashcard.exe`
- Windows MSI installer:
  - `src-tauri/target/release/bundle/msi/CODO Flashcard_0.1.0_x64_en-US.msi`
- Windows NSIS installer:
  - `src-tauri/target/release/bundle/nsis/CODO Flashcard_0.1.0_x64-setup.exe`

Note:

- The app title inside the UI is `CODO: Flashcard`
- The Windows installer filename uses `CODO Flashcard` because `:` is not allowed in Windows bundle product names
- The NSIS installer is configured with `perMachine` install mode, so Windows will request administrator elevation during installation

Windows release launcher:

```bat
launch-flashcard-app.bat
```

This launcher starts `src-tauri/target/release/codo_flashcard.exe` after the release build exists.

## Notes

- SQLite is created automatically in the Tauri app data directory on first launch
- Imports expect UTF-8 `.txt` files and tolerate BOM, empty lines, comments, and Windows/Unix line endings
- Deck export supports `.txt` and `.json`
- Full app backup creates a local SQLite snapshot plus a JSON manifest
