---
status: done
---

# Instruction: Exe et test manuel Windows

## Architecture projection

> Tree of the final files. ✅ create · ✏️ modify · ❌ delete

```txt
.
└── aidd_docs/tasks/2026_11/2026_11_04_codex-usage/
    └── ✅ manual-tests.md
```

## Test Scope

| Scenario | Action | Observable outcome | Actor |
| --- | --- | --- | --- |
| Setup | Build cross-compilé dans WSL | Exe produit | cli |
| Happy path | L'humain lance l'exe après une session Codex récente | La carte Codex affiche des % identiques au `/status` de Codex (à 1 % près) | system |
| Edge case | La distro WSL est arrêtée au lancement | Elle reste arrêtée (`wsl --list --running` vide), seule la racine Windows est lue | system |
| Edge case | Aucune session Codex sur aucune racine | La carte affiche « unknown » | system |
| Happy path | RAM au repos après 60 s | Toujours sous 100 Mo | system |

## Tasks to do

### `1)` Build

> Produire l'exe de la feature.

1. Lancer `pnpm tauri build --runner cargo-xwin --target x86_64-pc-windows-msvc`.

### `2)` Test manuel (MANUAL, humain)

> Valider la feature sur la vraie machine.

1. Lancer l'exe et comparer la carte au `/status` de Codex.
2. Arrêter WSL (`wsl --shutdown`), relancer l'exe, puis vérifier que la distro ne redémarre pas.
3. Relever la RAM au repos.

### `3)` Consigner

> Garder la trace des tests.

1. `manual-tests.md` : date, commit et résultat de chaque check.

## Test acceptance criteria

| Task | Acceptance criteria |
| --- | --- |
| 1 | L'exe existe. |
| 2 | L'humain a fourni le résultat de chaque check. |
| 3 | `manual-tests.md` consigne chaque check avec son verdict. |
