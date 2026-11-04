---
status: done
---

# Instruction: Câblage Tauri et carte Codex dans l'UI

## Architecture projection

> Tree of the final files. ✅ create · ✏️ modify · ❌ delete

```txt
.
├── apps/desktop/
│   ├── src/
│   │   ├── ✏️ App.tsx
│   │   ├── ✏️ App.test.tsx
│   │   └── features/usage/
│   │       ├── ✅ ProviderCard.tsx · ProviderCard.module.css · ProviderCard.test.tsx
│   │       ├── ✅ useUsage.ts · useUsage.test.ts
│   │       └── ✅ format.ts · format.test.ts
│   └── src-tauri/
│       ├── ✏️ Cargo.toml
│       └── src/
│           ├── ✏️ lib.rs
│           └── ✅ usage_loop.rs
└── aidd_docs/memory/
    ├── ✏️ architecture.md
    ├── ✏️ codebase-map.md
    └── ✏️ desktop.md
```

## Test Scope

| Scenario | Action | Observable outcome | Actor |
| --- | --- | --- | --- |
| Happy path | Event `usage` simulé, avec 5 h à 12 % et hebdo à 51 % | La carte affiche 2 jauges, leurs % et l'heure de reset | cli |
| Edge case | Provider `Unknown` | La carte affiche « unknown », jamais 0 % ni un champ vide | cli |
| Edge case | `resets_at` passé côté UI entre deux events | La jauge affiche 0 % | cli |
| Happy path | Âge de la donnée | Affiché « il y a 3 min » à partir de `observed_at` | cli |
| Happy path | L'exe sous Windows avec Codex utilisé récemment | Les valeurs de la carte correspondent au `/status` de Codex | system |

## Wireframe

```txt
┌────────────────────────────────┐
│ (1) AgentHud           (2) ●   │
├────────────────────────────────┤
│ (3) Codex          (4) il y a 3 min │
│ 5h     [████░░░░░░]  41 %  ↻ 17:40 │
│ hebdo  [█████░░░░░]  51 %  ↻ lun.  │
└────────────────────────────────┘
```

1. Titre « AgentHud ».
2. Indicateur de mode overlay (existant).
3. Carte provider : nom, puis une ligne par fenêtre avec jauge, %, et heure ou jour de reset.
4. Âge de la donnée (`observed_at`).

## Tasks to do

### `1)` Boucle de polling (`src-tauri`)

> Pousser l'usage à l'UI sans bloquer la fenêtre.

1. `usage_loop.rs` : tâche `tokio` (runtime Tauri) toutes les 3 s, qui appelle `RefreshUsage` avec `CodexUsageSource` et émet l'event `usage` si le résultat change.
2. Command `usage` qui renvoie le dernier résultat, lue par l'UI au chargement.
3. DTO sérialisé `{ provider, status: "known" | "unknown", fiveHours?, weekly?, observedAt? }`, avec des champs en camelCase. Les types TS sont générés par `tauri-specta`, comme le prévoit `architecture.md`. Si `tauri-specta` est incompatible avec la version de Tauri installée, écrire les types à la main et corriger `architecture.md` en conséquence.

### `2)` Carte provider (UI)

> Afficher l'usage lisiblement.

1. `useUsage` lit la command au chargement, puis suit l'event `usage`, sur le même schéma que `useOverlayMode`.
2. `ProviderCard` affiche les jauges avec libellé et %, l'heure de reset et l'âge. Afficher « unknown » si l'usage est absent, et 0 % une fois `resets_at` passé, recalculé chaque minute.
3. `format.ts` : formatage de l'heure de reset, du jour si plus de 24 h, et de l'âge.

### `3)` Mémoire

> Garder la carte du code à jour.

1. `codebase-map.md` : ajouter `features/usage`.
2. `desktop.md` : l'app lance `wsl.exe --list --running` à l'exécution, uniquement pour lister les distros démarrées.
3. `architecture.md` : préciser la règle « lecture passive ». L'app ne lance aucun agent ni CLI surveillé ; son seul processus externe est `wsl.exe --list --running --quiet`, en lecture.

## Test acceptance criteria

| Task | Acceptance criteria |
| --- | --- |
| 1 | `cargo xwin clippy -p agenthud-desktop` passe. Le DTO est testé (sérialisation) dans un crate de l'hôte, ou via `application`. |
| 2 | Les tests Vitest couvrent connu, unknown, reset passé et âge. |
| 3 | La mémoire liste `features/usage` et l'appel à `wsl.exe`. |
