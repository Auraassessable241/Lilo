# Lilo

Lilo is a compact, Windows-first note-taking widget built with Rust and egui. It keeps notes as ordinary Markdown files and provides a small knowledge graph for navigating connections between them.

> **MVP status:** version `0.0.1` is an early, functional build. Core note-taking and storage features work, but the interface is intentionally minimal and still needs substantial UI/UX refinement.

Lilo is non-commercial source-available software.

## Current features

- Markdown notes with YAML frontmatter;
- one live editor instead of separate edit and preview modes;
- headings, lists, task checkboxes, links, inline code, code blocks and other common Markdown formatting;
- Obsidian-style `[[wiki links]]`, aliases and backlinks;
- nested vault folders;
- note search, sorting and pinning;
- safe deletion to Trash with restoration;
- automatic creation and modification dates;
- local, folder and global graph scopes;
- graph zooming, panning and movable nodes;
- a temporary graph overlay for switching notes without leaving the editor;
- automatic saving, rotating backups and external file-change detection;
- configurable theme, accent colour, editor font size, shortcuts, always-on-top mode and Windows autostart.

## Storage

Lilo stores note content as `.md` files. Application preferences and UI state remain in `settings.json`.

A vault contains three application-managed directories:

```text
Vault/
├── Notes/       Markdown notes and user folders
├── Trash/       Recoverable deleted notes
└── Backups/     Rotating note backups
```

The vault location can be changed in Settings. Changing it takes effect after restarting Lilo. Because notes are regular Markdown files, they can also be inspected or edited with another text editor; Lilo detects external changes and avoids silently overwriting conflicting local edits.

## Default shortcuts

| Action | Shortcut |
| --- | --- |
| Create note | `Ctrl+N` |
| Search notes | `Ctrl+P` |
| Open graph | `Ctrl+G` |
| Toggle graph overlay | `Ctrl+Shift+G` |
| Save immediately | `Ctrl+S` |
| Return to editor or close an overlay | `Escape` |

Shortcuts can be changed in Settings.

## Building from source

Lilo `0.0.1` does not yet provide a polished installer or signed binary. To build it locally, install the stable Rust toolchain and run:

```powershell
git clone https://github.com/HellterEnjoy/Lilo.git
cd Lilo
cargo run --release
```

Run the project checks with:

```powershell
cargo fmt -- --check
cargo test
cargo clippy --all-targets -- -D warnings
```

## MVP limitations

- the visual design is temporary and several controls use text labels instead of final icons;
- some workflows require more clicks than intended for later versions;
- Windows is the current primary platform;
- there is no finished installer, automatic updater or signed release package;
- the global graph intentionally displays only the 80 most recently updated notes to keep the compact widget responsive.

See [ROADMAP.md](ROADMAP.md) for the planned direction. The roadmap describes intent rather than fixed deadlines or guaranteed release scope.

## Contributions

Bug reports, feature requests, UI/UX feedback and technical suggestions are welcome through GitHub Issues.

Lilo is intentionally maintained as a single-author project, so code pull requests are not accepted. Example code and pseudocode may still be shared as suggestions or references.

See [CONTRIBUTING.md](CONTRIBUTING.md) for the full contribution policy.

## License

Lilo is source-available under the [PolyForm Noncommercial License 1.0.0](LICENSE).

You may use, study, modify, fork and redistribute Lilo and modified versions for non-commercial purposes, subject to the license terms. Commercial use of Lilo or any modified or derived version is not permitted under this license and requires separate permission from the copyright holder.

The copyright holder retains the right to sell Lilo, issue separate commercial licenses, offer future versions under a different licensing model and distribute Lilo under other terms. Redistribution must preserve the license and all required copyright and attribution notices.

Copyright 2026 Kyrylo Yazynin. Project: https://github.com/HellterEnjoy/Lilo
