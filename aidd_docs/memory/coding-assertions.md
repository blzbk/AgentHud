# Coding Assertions

## Before commit

| Order | Command                        | Checks              |
| ----- | ------------------------------ | ------------------- |
| 1     | `cargo fmt --check`            | format Rust         |
| 2     | `cargo clippy -- -D warnings`  | lint Rust           |
| 3     | `pnpm typecheck`               | types TS            |
| 4     | `pnpm lint`                    | lint TS             |

## Before push

| Order | Command                  | Checks                 |
| ----- | ------------------------ | ---------------------- |
| 1     | `cargo test --workspace` | tests Rust             |
| 2     | `pnpm test`              | tests UI (Vitest)      |
| 3     | `pnpm tauri build --runner cargo-xwin --target x86_64-pc-windows-msvc` | build de l'exe Windows (cross-compilé) |

## Behavior

Quand une correction est nécessaire, lancer 1 agent par assertion en échec (par exemple : types, tests, lint = 3 agents).
