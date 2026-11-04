---
status: done
---

# Instruction: Toolchain WSL et workspace Cargo hexagonal

## Architecture projection

> Tree of the final files. ✅ create · ✏️ modify · ❌ delete

```txt
.
├── ✅ Cargo.toml
├── ✅ rust-toolchain.toml
├── ✅ .gitignore
└── crates/
    ├── domain/            ✅ Cargo.toml · src/lib.rs
    ├── application/       ✅ Cargo.toml · src/lib.rs
    ├── adapter-fs/        ✅ Cargo.toml · src/lib.rs
    ├── adapter-claude/    ✅ Cargo.toml · src/lib.rs
    ├── adapter-codex/     ✅ Cargo.toml · src/lib.rs
    └── architecture-tests/ ✅ Cargo.toml · tests/dependency_direction.rs
```

## Test Scope

| Scenario | Action | Observable outcome | Actor |
| --- | --- | --- | --- |
| Setup | L'humain installe la toolchain (commandes de l'ADR 0001) | `cargo -V`, `pnpm -v` et `cargo xwin --version` répondent dans WSL | cli |
| Happy path | `cargo test --workspace` | Tous les tests passent, y compris le test de sens des dépendances | cli |
| Edge case | Ajout temporaire d'une dépendance `domain` → `application` | Le test `dependency_direction` échoue en nommant l'arête interdite | cli |
| Teardown | Retrait de la dépendance temporaire | Les tests repassent au vert | cli |

## Tasks to do

### `1)` Toolchain WSL (MANUAL, humain)

> Disposer de Rust, pnpm et cargo-xwin dans WSL.

1. Exécuter `sudo apt install nsis lld llvm clang`.
2. Installer `rustup`, puis exécuter `rustup target add x86_64-pc-windows-msvc`.
3. Exécuter `cargo install --locked cargo-xwin` et `corepack enable pnpm`.

### `2)` Workspace Cargo

> Avoir un workspace racine qui partage une même configuration.

1. `Cargo.toml` racine : `members = ["crates/*"]`. Le membre `apps/desktop/src-tauri` sera ajouté en phase 2. Y déclarer `[workspace.package]` (edition 2021, license MIT), `[workspace.lints]` (clippy `pedantic` en warn) et `[workspace.dependencies]`. Avec `-D warnings`, les lints pedantic deviennent bloquants, et c'est voulu (exigence de clean code). Toute exception passe par un `allow` justifié dans `[workspace.lints]`.
2. `rust-toolchain.toml` : canal stable, cible `x86_64-pc-windows-msvc`.
3. `.gitignore` : `target/`, `node_modules/`, `dist/`, `.xwin-cache/`.

### `3)` Crates hexagonaux

> Avoir 5 crates compilables, chacun avec une responsabilité documentée.

1. `domain` : aucune dépendance. Une doc `//!` de responsabilité et l'enum `AgentState` (`Waiting`, `Working`, `Idle`, `Offline`), plus un test unitaire.
2. `application` : dépend de `domain`. Doc `//!` uniquement (ports et cas d'usage viendront aux features suivantes).
3. `adapter-fs` : dépend de `application` et `domain`. Doc `//!` uniquement.
4. `adapter-claude` et `adapter-codex` : dépendent de `application`, `domain` et `adapter-fs`. Doc `//!` uniquement.

### `4)` Test de sens des dépendances

> Verrouiller les règles hexagonales par un test.

1. Crate `architecture-tests` (`publish = false`) : lit `cargo metadata` via la crate `cargo_metadata`.
2. Il échoue si `domain` dépend d'un crate interne, si `application` dépend d'autre chose que `domain`, ou si un adapter dépend d'un autre adapter (sauf de `adapter-fs`).

## Test acceptance criteria

| Task | Acceptance criteria |
| --- | --- |
| 1 | `cargo -V`, `pnpm -v` et `cargo xwin --version` affichent une version dans WSL. |
| 2 | `cargo metadata` liste les 6 crates de `crates/`. |
| 3 | `cargo fmt --check`, `cargo clippy --workspace -- -D warnings` et `cargo test --workspace` sortent à 0. |
| 4 | Le test `dependency_direction` passe, et échoue quand on ajoute une arête interdite. |
