---
status: done
---

# Instruction: Exe et tests manuels Windows

## Architecture projection

> Tree of the final files. ✅ create · ✏️ modify · ❌ delete

```txt
.
└── aidd_docs/tasks/2026_11/2026_11_05_claude-and-agents/
    └── ✅ manual-tests.md
```

## Test Scope

| Scenario | Action | Observable outcome | Actor |
| --- | --- | --- | --- |
| Setup | Build complet cross-compilé, puis fermeture de toute ancienne instance via « Quitter » | Exe prêt | cli |
| Happy path | Une session Claude qui travaille | Le badge Claude passe à `working`, puis à `waiting` à la fin du tour | system |
| Happy path | Une session Codex qui travaille | Même cycle pour le badge Codex | system |
| Happy path | Aucune activité pendant 6 min | Les badges passent à `idle` | system |
| Happy path | Tokens Claude sur 5 h | L'ordre de grandeur est cohérent avec `ccusage blocks` ou `/usage` | system |
| Edge case | Plus aucune session depuis plus de 60 min | Le provider affiche `offline` | system |

## Tasks to do

### `1)` Build

> Produire l'exe.

1. Lancer `pnpm tauri build --runner cargo-xwin --target x86_64-pc-windows-msvc`.

### `2)` Tests (MANUAL, humain)

> Valider sur la vraie machine.

1. Dérouler les scénarios « system » ci-dessus.

### `3)` Consigner

> Tracer le résultat.

1. `manual-tests.md` : date, commit et verdict de chaque check.

## Test acceptance criteria

| Task | Acceptance criteria |
| --- | --- |
| 1 | L'exe existe. |
| 2 | L'humain a donné un verdict pour chaque check. |
| 3 | `manual-tests.md` est écrit. |
