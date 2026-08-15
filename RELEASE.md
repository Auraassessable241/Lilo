# Lilo release guide

## Install on Windows

1. Download `Lilo-<version>-windows-x64.zip` from the GitHub release.
2. Optionally verify the archive against the accompanying `.sha256` file.
3. Extract the complete archive to a directory owned by your Windows account.
4. Run `Lilo.exe`. Windows SmartScreen may warn because the preview binary is not code-signed.

Lilo creates its default vault as `LiloVault` in the user's Documents directory. The active path is visible and can be changed immediately in **Settings → Storage**.

## Update

Close Lilo, extract the new archive, and replace the old application files. The executable is separate from the vault, so replacing it does not remove notes or settings. Keep a vault export before an update when the data matters.

## Backup and recovery

- **Recovery → Trash** restores notes deleted inside Lilo.
- **Recovery → Backups** previews and restores rotating versions created during saving.
- **Recovery → Diagnostics** reports Markdown files whose metadata could not be read normally.
- **Settings → Storage → Export** copies Notes, Trash, and settings to a timestamped folder outside the active vault.

Lilo never requires a proprietary database: notes remain Markdown files under the vault's `Notes` directory. If the application cannot start, copy the whole vault before manually repairing any file.

## Build and package

Install the stable Rust toolchain, then run the checks and packaging script from PowerShell:

```powershell
cargo fmt -- --check
cargo test
cargo clippy --all-targets -- -D warnings
powershell -ExecutionPolicy Bypass -File .\scripts\package-windows.ps1
```

The script performs a locked release build and creates the ZIP plus its SHA-256 checksum under `dist/`. Release artifacts are intentionally ignored by Git.
