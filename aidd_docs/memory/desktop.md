# Desktop

## Framework

- Tauri, avec l'UI dans la WebView2 de Windows. Le process principal est en Rust (`apps/desktop/src-tauri`).
- Fenêtre overlay transparente, `alwaysOnTop`, sans décoration. Elle est click-through via `set_ignore_cursor_events`, et un raccourci clavier la rend interactive.

## Native access

- Lecture seule des dossiers `~/.claude` et `~/.codex`, côté Windows natif ou WSL (`\\wsl.localhost\...`, `\\wsl$\...`).
- Tout accès aux fichiers passe par Rust, jamais par les plugins fs côté JS.
- Aucun accès réseau en v1.

## Build and release

- Cible : Windows uniquement, un seul `.exe`.
- En dev, l'exe est cross-compilé depuis WSL : `pnpm tauri build --runner cargo-xwin --target x86_64-pc-windows-msvc`. Voir l'ADR 0001.
- Publication sur GitHub Releases via GitHub Actions (`windows-latest`).
- Pas de signature en v1 : Windows affiche l'avertissement SmartScreen. SignPath (gratuit pour l'OSS) est prévu plus tard.
