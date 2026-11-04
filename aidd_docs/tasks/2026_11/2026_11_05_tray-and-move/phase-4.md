---
status: done
---

# Instruction: Exe et tests manuels Windows

## Architecture projection

> Tree of the final files. ✅ create · ✏️ modify · ❌ delete

```txt
.
└── aidd_docs/tasks/2026_11/2026_11_05_tray-and-move/
    └── ✅ manual-tests.md
```

## Test Scope

| Scenario | Action | Observable outcome | Actor |
| --- | --- | --- | --- |
| Setup | Build cross-compilé, puis fermeture de toute ancienne instance | Exe prêt | cli |
| Happy path | Scénarios des phases 2 et 3 sous Windows | Chaque check est ✅ ou décrit | system |
| Teardown | « Quitter » depuis le tray | Plus aucun processus | system |

## Tasks to do

### `1)` Build

> Produire l'exe.

1. Lancer `pnpm tauri build --runner cargo-xwin --target x86_64-pc-windows-msvc`.

### `2)` Tests (MANUAL, humain)

> Valider sur la vraie machine.

1. Dérouler les scénarios « system » des phases 2 et 3.

### `3)` Consigner

> Tracer le résultat.

1. `manual-tests.md` : date, commit et verdict de chaque check.

## Test acceptance criteria

| Task | Acceptance criteria |
| --- | --- |
| 1 | L'exe existe. |
| 2 | L'humain a donné un verdict pour chaque check. |
| 3 | `manual-tests.md` est écrit. |
