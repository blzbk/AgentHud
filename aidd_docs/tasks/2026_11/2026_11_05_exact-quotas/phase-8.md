---
status: pending
---

# Instruction: Exe et tests manuels Windows

## Architecture projection

> Tree of the final files. ✅ create · ✏️ modify · ❌ delete

```txt
.
└── target/x86_64-pc-windows-msvc/release/ ✅ agenthud.exe (non versionné)
```

## Test Scope

| Scenario | Action | Observable outcome | Actor |
| --- | --- | --- | --- |
| Happy path | Build cross-compilé | `pnpm tauri build --runner cargo-xwin --target x86_64-pc-windows-msvc` sort 0 | cli |
| Happy path | Statusline installée, session Claude active | Carte Claude en %, source « statusline » | system |
| Happy path | Codex installé (Windows ou WSL démarrée) | Carte Codex source « app-server » | system |
| Edge case | Option OAuth cochée, statusline périmée | Carte Claude source « OAuth ⚠ » au plus toutes les 13 min | system |

## Tasks to do

### `1)` Build et vérification humaine

> Livrer l'exe à tester.

1. Toutes les gates de `coding-assertions.md` passent.
2. L'humain lance l'exe et vérifie les scénarios (ADR 0001).

## Test acceptance criteria

| Task | Acceptance criteria |
| --- | --- |
| 1 | Gates à 0 ; tests manuels faits par l'humain. |
