---
status: pending
---

# Instruction: Exe cross-compilé et mesure de la RAM

## Architecture projection

> Tree of the final files. ✅ create · ✏️ modify · ❌ delete

```txt
.
└── aidd_docs/tasks/2026_11/2026_11_04_walking-skeleton/
    └── ✅ ram-measurement.md
```

## Test Scope

| Scenario | Action | Observable outcome | Actor |
| --- | --- | --- | --- |
| Setup | `pnpm tauri build --runner cargo-xwin --target x86_64-pc-windows-msvc` dans WSL | L'exe et l'installateur NSIS sont produits | cli |
| Happy path | L'humain lance l'exe et attend 60 s au repos, puis relève le working set de `agenthud.exe` et de tous les `msedgewebview2.exe` enfants | Total inférieur à 100 Mo, CPU au repos proche de 0 % | system |
| Edge case | Total supérieur ou égal à 100 Mo | Le dépassement est consigné. L'humain décide entre un réglage de `MemoryUsageTargetLevel` dans une nouvelle feature, ou le maintien de la cible. | system |
| Teardown | Fermer l'app | Plus aucun processus lié | system |

## Tasks to do

### `1)` Build final

> Produire l'exe de la feature.

1. Lancer le build cross-compilé, puis vérifier la présence de l'exe et de l'installateur.
2. Donner à l'humain le chemin Windows de l'exe (`\\wsl.localhost\<distro>\...`).

### `2)` Mesure de la RAM (MANUAL, humain)

> Valider la cible de moins de 100 Mo.

1. Sous Windows, relever dans le Gestionnaire des tâches (onglet Détails) le working set de `agenthud.exe` et de tous les `msedgewebview2.exe` dont le parent est l'app.
2. Communiquer les valeurs.

### `3)` Consigner la mesure

> Garder une trace datée de la mesure.

1. Créer `ram-measurement.md` : date, version de WebView2, valeur par processus, total et verdict.

## Test acceptance criteria

| Task | Acceptance criteria |
| --- | --- |
| 1 | L'exe existe dans `target/x86_64-pc-windows-msvc/release/`, et l'installateur dans `.../bundle/nsis/`. |
| 2 | L'humain a fourni les valeurs par processus. |
| 3 | `ram-measurement.md` contient le total et un verdict clair par rapport à 100 Mo. |
