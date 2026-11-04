# Coding Assertions

`apps/desktop/src-tauri` (package `agenthud-desktop`) ne compile que pour Windows : il est vérifié par `cargo xwin` sur `x86_64-pc-windows-msvc`. Les autres crates sont vérifiés sur l'hôte Linux, `agenthud-desktop` exclu. Prérequis des commandes `cargo xwin` : `apps/desktop/dist` existe (`pnpm --filter @agenthud/desktop build`).

## Before commit

| Order | Command | Checks |
| ----- | ------- | ------ |
| 1 | `cargo fmt --check` | format Rust |
| 2 | `cargo clippy --workspace --exclude agenthud-desktop --all-targets -- -D warnings` | lint Rust (crates hôte) |
| 3 | `cargo xwin clippy -p agenthud-desktop --target x86_64-pc-windows-msvc --all-targets -- -D warnings` | lint Rust (`src-tauri`) |
| 4 | `pnpm typecheck` | types TS |
| 5 | `pnpm lint` | lint TS |

## Before push

| Order | Command | Checks |
| ----- | ------- | ------ |
| 1 | `cargo test --workspace --exclude agenthud-desktop` | tests Rust (crates hôte, dont le sens des dépendances) |
| 2 | `cargo xwin test -p agenthud-desktop --target x86_64-pc-windows-msvc --no-run` | compilation des tests `src-tauri`. Ils ne s'exécutent que sous Windows, jamais depuis WSL (ADR 0001). |
| 3 | `pnpm test` | tests UI (Vitest) |
| 4 | `pnpm tauri build --runner cargo-xwin --target x86_64-pc-windows-msvc` | build de l'exe Windows (cross-compilé) |

## Behavior

Quand une correction est nécessaire, lancer 1 agent par assertion en échec (par exemple : types, tests, lint = 3 agents).
