---
status: done
---

# Instruction: Accès fichiers : sessions récentes et liste des distros en cache

## Architecture projection

> Tree of the final files. ✅ create · ✏️ modify · ❌ delete

```txt
.
└── crates/adapter-fs/src/
    ├── ✏️ lib.rs · newest.rs · roots.rs
    └── ✅ recent.rs
```

## Test Scope

| Scenario | Action | Observable outcome | Actor |
| --- | --- | --- | --- |
| Happy path | `recent_files(dir, ext, since)` sur une arborescence de mtimes variés | Seuls les fichiers modifiés après `since`, du plus récent au plus ancien | cli |
| Edge case | Dossier absent | Liste vide, sans erreur | cli |
| Happy path | `CachedLister` sur un lister factice qui compte ses appels, 2 appels à moins de 30 s d'intervalle | Le lister réel n'est appelé qu'une fois | cli |
| Edge case | 2 appels à plus de 30 s d'intervalle (horloge injectée) | Le lister réel est rappelé | cli |

## Tasks to do

### `1)` Fichiers récents

> Lister les sessions actives sans tout relire.

1. `recent.rs` : `recent_files(dir, extension, since, max_depth)` renvoie les `DatedFile` modifiés après `since`, triés par mtime décroissant. Réutiliser le parcours de `newest.rs`.

### `2)` Liste des distros en cache

> Réduire les lancements de `wsl.exe` (issue #9).

1. `CachedLister<L: DistroLister>` garde la liste 30 s, avec une horloge injectable pour les tests.
2. Une seule instance est partagée par toutes les sources, et câblée en phase 5.

## Test acceptance criteria

| Task | Acceptance criteria |
| --- | --- |
| 1 | Les tests couvrent le filtre `since`, le tri et le dossier absent. |
| 2 | Le test du compteur prouve un seul appel dans les 30 s, puis un nouvel appel après. |
