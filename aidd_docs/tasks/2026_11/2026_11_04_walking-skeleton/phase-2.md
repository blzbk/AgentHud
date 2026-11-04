---
status: done
---

# Instruction: App Tauri + UI React et gates TS

## Architecture projection

> Tree of the final files. ✅ create · ✏️ modify · ❌ delete

```txt
.
├── ✅ Cargo.toml (racine, phase 1 : ajout du membre src-tauri)
├── ✅ package.json
├── ✅ pnpm-workspace.yaml
├── ✅ eslint.config.js
├── apps/desktop/
│   ├── ✅ package.json · index.html · vite.config.ts · tsconfig.json · app-icon.png
│   ├── src/
│   │   ├── ✅ main.tsx · App.tsx · App.test.tsx
│   │   └── shared/ ✅ theme.css
│   └── src-tauri/
│       ├── ✅ Cargo.toml · build.rs · tauri.conf.json
│       ├── ✅ capabilities/default.json
│       ├── ✅ icons/
│       └── src/ ✅ main.rs · lib.rs
└── aidd_docs/memory/
    ├── ✏️ coding-assertions.md
    └── ✏️ codebase-map.md
```

## Test Scope

| Scenario | Action | Observable outcome | Actor |
| --- | --- | --- | --- |
| Setup | `pnpm install` à la racine | Dépendances installées, lockfile créé | cli |
| Happy path | `pnpm typecheck && pnpm lint && pnpm test` | Les 3 commandes sortent à 0. Le test Vitest voit le texte « AgentHud ». | cli |
| Happy path | `cargo xwin clippy -p agenthud-desktop --target x86_64-pc-windows-msvc -- -D warnings` | Sortie 0, sans warning | cli |
| Edge case | `pnpm tauri build --runner cargo-xwin --target x86_64-pc-windows-msvc` | L'exe sort dans `target/x86_64-pc-windows-msvc/release/` | cli |

## Tasks to do

### `1)` Workspace pnpm et scripts

> Avoir des gates TS appelables depuis la racine.

1. `pnpm-workspace.yaml` : `apps/*`.
2. `package.json` racine : scripts `typecheck`, `lint`, `test` et `tauri`, délégués à `apps/desktop`.
3. `eslint.config.js` : flat config, `typescript-eslint` et `eslint-plugin-react-hooks`. Le support du flat config par la version installée n'est pas vérifié : à contrôler à l'installation.

### `2)` Front Vite + React + TS

> Afficher le placeholder « AgentHud ».

1. Écrire `apps/desktop` à la main, sans `create-tauri-app` qui est interactif. Ajouter les dépendances `react`, `react-dom`, `@tauri-apps/api`, et en dev `vite`, `@vitejs/plugin-react`, `typescript`, `@tauri-apps/cli`, `vitest`, `jsdom`, `@testing-library/react` et `@testing-library/jest-dom`.
2. `App.tsx` affiche « AgentHud ». `theme.css` porte les tokens : variables CSS et fond semi-opaque.
3. `App.test.tsx` : Vitest (`environment: "jsdom"` dans `vite.config.ts`) et Testing Library vérifient le texte rendu.

### `3)` Composition root `src-tauri`

> Avoir un shell Tauri membre du workspace, qui assemble les crates.

1. Package `agenthud-desktop` (`tauri` et `tauri-build` en majeure 2, versions résolues à l'installation), avec des dépendances vers `application` et les adapters. Ajouter `apps/desktop/src-tauri` aux `members` du `Cargo.toml` racine.
2. `lib.rs` contient `run()` (Builder). `main.rs` appelle `run()` avec `windows_subsystem = "windows"`.
3. `capabilities/default.json` minimal (`core:default`).
4. Créer une icône source provisoire `apps/desktop/app-icon.png` (1024×1024, générée par script), puis lancer `pnpm tauri icon apps/desktop/app-icon.png`.

### `4)` Gates documentés

> Faire refléter les commandes réelles par la mémoire.

1. `coding-assertions.md` : `cargo clippy --workspace --exclude agenthud-desktop` sur l'hôte, plus `cargo xwin clippy -p agenthud-desktop --target x86_64-pc-windows-msvc`. Même séparation pour les tests.
2. `codebase-map.md` : confirmer les chemins d'entrée réels.

## Test acceptance criteria

| Task | Acceptance criteria |
| --- | --- |
| 1 | `pnpm typecheck`, `pnpm lint` et `pnpm test` sortent à 0 depuis la racine. |
| 2 | Le test Vitest passe et trouve « AgentHud ». |
| 3 | `cargo xwin clippy -p agenthud-desktop --target x86_64-pc-windows-msvc -- -D warnings` sort à 0. Le build cross-compilé produit un `.exe`. |
| 4 | Chaque commande listée dans `coding-assertions.md` s'exécute et sort à 0. |
