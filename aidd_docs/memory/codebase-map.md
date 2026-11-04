# Codebase Map

```mermaid
flowchart TD
  Desktop[apps/desktop] --> Crates[crates]
  Crates --> Domain[domain]
  Crates --> Application[application]
  Crates --> Adapters["adapter-*"]
```

## Areas

- `crates/domain` : entités et value objects, sans I/O.
- `crates/application` : cas d'usage et ports.
- `crates/adapter-<provider>` : lecture et interprétation des données d'un provider.
- `crates/adapter-fs` : polling sur la date de modification et résolution des chemins Windows et WSL.
- `crates/architecture-tests` : test du sens des dépendances hexagonales (`cargo metadata`).
- `apps/desktop/src-tauri` : composition root, commands, events et fenêtre overlay.
- `apps/desktop/src` : UI React, organisée par feature (`features/usage`, `features/agents`).

## Entry points

- `apps/desktop/src-tauri/src/main.rs` : démarrage de l'app (binaire `agenthud`), appelle `run()` de `src/lib.rs`.
- `apps/desktop/src/main.tsx` : démarrage de l'UI.

## Packages

- Workspace Cargo racine : `crates/*` et `apps/desktop/src-tauri`.
- Workspace pnpm : `apps/desktop` (package `@agenthud/desktop`). Scripts racine délégués : `typecheck`, `lint`, `test`, `tauri`.
