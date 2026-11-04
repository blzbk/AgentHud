---
status: done
---

# Instruction: Accès fichiers : racines Windows/WSL et lecture de fin de fichier

## Architecture projection

> Tree of the final files. ✅ create · ✏️ modify · ❌ delete

```txt
.
└── crates/adapter-fs/
    ├── ✏️ Cargo.toml
    └── src/
    ├── ✏️ lib.rs
    ├── ✅ roots.rs
    ├── ✅ newest.rs
    └── ✅ tail.rs
```

## Test Scope

| Scenario | Action | Observable outcome | Actor |
| --- | --- | --- | --- |
| Setup | Dossier temporaire avec plusieurs `*.jsonl` imbriqués, de mtimes différents | Arborescence prête | cli |
| Happy path | `newest_file(root, "sessions", "jsonl")` | Renvoie le fichier au mtime le plus récent | cli |
| Happy path | `read_tail_lines(file, max_bytes)` sur un fichier plus gros que `max_bytes` | Renvoie seulement les lignes complètes de la fin, la dernière en premier | cli |
| Edge case | Racine absente, ou dossier `sessions` vide | `Ok(None)`, sans erreur | cli |
| Edge case | `DistroLister` factice qui renvoie `["Ubuntu"]` et un home `thomas` | Racines `%USERPROFILE%\.codex` et `\\wsl.localhost\Ubuntu\home\thomas\.codex` | cli |
| Teardown | Suppression du dossier temporaire | État initial | cli |

## Tasks to do

### `1)` Racines de données

> Savoir où chercher les données d'un CLI, sans réveiller WSL.

1. `roots.rs` : `home_roots(dir_name, lister) -> Vec<PathBuf>`, qui renvoie le home Windows (`USERPROFILE`) plus, pour chaque distro démarrée, `\\wsl.localhost\<distro>\home\*\<dir_name>`.
2. Trait `DistroLister` avec 2 implémentations :
   - `WslCli` : `wsl.exe --list --running --quiet`, sortie UTF-16 décodée, sous `cfg(windows)` ;
   - `NoDistros` : pour l'hôte et les tests.
3. Sur un hôte non Windows, le home courant (`HOME`) remplace `USERPROFILE`, ce qui rend le code testable dans WSL.

### `2)` Fichier le plus récent

> Trouver la session active sans surveiller le disque.

1. `newest.rs` : parcours récursif borné de `<root>/sessions`, puis sélection du fichier au mtime maximal, sans crate de watcher.
2. Les erreurs d'I/O sur une racine sont journalisées et ignorées.

### `3)` Lecture de fin de fichier

> Lire le dernier événement sans charger toute la session.

1. `tail.rs` : lit les `max_bytes` finaux (64 Kio par défaut) et renvoie les lignes complètes, de la dernière à la première. Agrandir la fenêtre si aucune ligne utile n'est trouvée, jusqu'à 1 Mio.

## Test acceptance criteria

| Task | Acceptance criteria |
| --- | --- |
| 1 | Le test avec `DistroLister` factice produit les racines attendues. Sans distro, seul le home est renvoyé. |
| 2 | Le test renvoie le fichier le plus récent. Une racine absente donne `None`. |
| 3 | Le test sur un fichier de plus de 64 Kio ne renvoie que des lignes complètes, la dernière en premier. |
