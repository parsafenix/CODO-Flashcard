# CODO: Flashcard - Install Package

This package contains the final offline Windows release for `CODO: Flashcard`.

## Recommended install method

Run:

- `CODO-Flashcard-Setup-v0.2.0-x64.exe`

This is the main Windows installer. Because the installer is configured for machine-wide installation, Windows may ask for administrator approval during setup.

## Package contents

Top level:

- `CODO-Flashcard-Setup-v0.2.0-x64.exe`
  - Main NSIS installer
- `README.md`
  - This guide
- `Data/`
  - Extra release files for the same version

Inside `Data/`:

- `CODO-Flashcard-v0.2.0-x64.msi`
  - Alternative MSI installer
- `CODO-Flashcard-v0.2.0-x64.exe`
  - Standalone release executable

## Install steps

1. Open `CODO-Flashcard-Setup-v0.2.0-x64.exe`
2. Approve the administrator prompt if Windows shows it
3. Finish the installer steps
4. Launch `CODO: Flashcard` from Start Menu or Desktop shortcut

## Notes

- The app is fully local and works offline
- User data is stored locally on the installed machine
- No account, internet connection, or external backend is required
- The display name inside the app is `CODO: Flashcard`
