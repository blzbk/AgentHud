---
objective: "Un exe Windows cross-compilé depuis WSL affiche l'overlay AgentHud (transparent, au premier plan, click-through, bascule par raccourci) sur un workspace hexagonal compilable, avec tous les gates verts et la RAM mesurée."
status: in-progress
---

# Plan: Walking skeleton AgentHud

Source: demande de la feature 1 (session SDLC du 2026-11-04), `aidd_docs/INSTALL.md`, `aidd_docs/memory/`, `aidd_docs/adr/0001-cross-compile-windows-from-wsl.md`

## Phases

| #   | Phase                                  | File                         | Depends on |
| --- | -------------------------------------- | ---------------------------- | ---------- |
| 1   | Toolchain WSL et workspace Cargo hexagonal | [`phase-1.md`](./phase-1.md) | none |
| 2   | App Tauri + UI React et gates TS         | [`phase-2.md`](./phase-2.md) | 1 |
| 3   | Fenêtre overlay et raccourci global      | [`phase-3.md`](./phase-3.md) | 2 |
| 4   | Exe cross-compilé et mesure de la RAM    | [`phase-4.md`](./phase-4.md) | 3 |

## External sources

| URL | What it settled |
| --- | --- |
| https://v2.tauri.app/distribute/windows-installer/ | Cross-compilation : paquets `nsis lld llvm` (`clang` si du C est compilé), cible `x86_64-pc-windows-msvc`, `cargo-xwin`, commande `tauri build --runner cargo-xwin`, sortie `target/x86_64-pc-windows-msvc/release/bundle/nsis/`, NSIS uniquement. |
| https://v2.tauri.app/plugin/global-shortcut/ | `tauri-plugin-global-shortcut` : `Builder::new().with_handler(...)`, `ShortcutState::Pressed`, `app.global_shortcut().register(...)`. Les permissions JS ne servent que si l'on enregistre depuis le front. |
| https://docs.rs/tauri/latest/tauri/webview/struct.WebviewWindow.html | `set_ignore_cursor_events(bool)`, `set_always_on_top`, `set_skip_taskbar`, `set_decorations` : disponibles sur desktop. docs.rs « latest » indique 2.12.x, mais aucune version n'est épinglée : la majeure 2 est résolue à l'installation. |

## Impact

| Category | Verdict and evidence | Handled in |
| --- | --- | --- |
| runtime | Non vérifié : la toolchain WSL est absente (`cargo` et `pnpm` introuvables, observé). L'humain l'installe avec `sudo`. WebView2 156 est présent côté Windows : observé via une lecture du registre avec `powershell.exe`, avant l'ADR 0001. Cette méthode est désormais exclue, la présence est à reconfirmer par l'humain. | phase 1, phase 4 |
| runtime | Non vérifié : clippy et check de `src-tauri` sur la cible Windows via `cargo xwin clippy`. Sans librairies webkit2gtk, `src-tauri` ne compile pas sur l'hôte Linux. | phase 2 |
| runtime | Non vérifié : comportement réel du click-through et de la transparence d'un exe cross-compilé. L'humain vérifie sous Windows. | phase 3, phase 4 |
| architecture | Repo vide (seulement `README.md` et `aidd_docs/`). On part de l'architecture hexagonale documentée dans `aidd_docs/memory/architecture.md`. Le sens des dépendances est verrouillé par un test. | phase 1 |
| regression | Aucune : aucun code existant. | none |
| documentation | `coding-assertions.md` : commandes réelles des gates, avec la cible xwin pour `src-tauri`. `codebase-map.md` : chemins d'entrée réels. Mesure de la RAM consignée dans le dossier de la feature. | phase 2, phase 4 |

## Rules

| Rule | Why it applies |
| --- | --- |
| `architecture.md` : `domain` pur, `application` → `domain`, adapters → ports, `src-tauri` seul composition root | C'est la structure que crée la feature. |
| ADR 0001 : l'agent ne lance aucun outil Windows | Le build se fait par cross-compilation. L'exécution et la mesure de la RAM sont à la main de l'humain. |
| `coding-assertions.md` : fmt, clippy `-D warnings`, typecheck, lint, tests, build | Chaque phase se termine gates verts. |
| `vcs.md` : Conventional Commits, scopes par crate, auto-commit `after phase` | Un commit par phase. |
| `testing.md` : unitaires sans I/O, Vitest côté UI | Tests minimaux par crate et par composant. |

## Decisions

| Decision | Why |
| --- | --- |
| Raccourci global `Ctrl+Alt+H` enregistré côté Rust, sans permission JS | Le mode overlay reste un état natif. Le front n'a aucun accès système, conformément à `desktop.md`. |
| `src-tauri` est membre du workspace Cargo, mais vérifié par `cargo xwin` sur la cible Windows. Les autres crates sont vérifiés sur l'hôte. | Évite d'installer webkit2gtk dans WSL. L'app est Windows uniquement. |
