---
status: done
---

# Instruction: Mémoire et README

## Architecture projection

> Tree of the final files. ✅ create · ✏️ modify · ❌ delete

```txt
.
├── ✏️ README.md
└── aidd_docs/memory/ ✏️ architecture.md · desktop.md · design.md · codebase-map.md · project-brief.md
```

## Test Scope

| Scenario | Action | Observable outcome | Actor |
| --- | --- | --- | --- |
| Happy path | Relecture croisée avec l'ADR 0002 | Plus aucune règle « aucun appel OAuth » ni « seul processus : wsl.exe » contradictoire | cli |

## Tasks to do

### `1)` Mémoire

> Aligner la mémoire sur l'ADR 0002.

1. `architecture.md` : sources par provider et priorités, OAuth en mode dégradé (option, 13 min, reculs, token jamais stocké), processus `codex app-server`, accès réseau unique, `PollScheduler`.
2. `desktop.md` : options du tray, `options.json`, processus et réseau autorisés, installation de la statusline.
3. `design.md` : jauges Claude, libellés de source, « auth expirée ».
4. `codebase-map.md` : nouveaux modules et `scripts/statusline`.
5. `project-brief.md` : % exacts Claude quand une source le fournit.

### `2)` README

> Permettre l'installation sans lire le code.

1. Installation de la statusline en une ligne (Windows natif et WSL), chaînage d'une statusline existante, avertissement explicite sur le risque de l'option OAuth.

## Test acceptance criteria

| Task | Acceptance criteria |
| --- | --- |
| 1 | `grep` : plus de phrase contredisant l'ADR 0002 dans `aidd_docs/memory`. |
| 2 | Le README contient les 2 commandes d'installation et l'avertissement OAuth. |
