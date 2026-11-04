---
objective: "L'utilisateur déplace l'overlay en mode interactif, retrouve sa position au lancement suivant, et pilote l'app (mode, afficher/masquer, quitter) depuis une icône de la zone de notification ; un 2e lancement réaffiche l'overlay."
status: in-progress
---

# Plan: Déplacement de l'overlay et icône de notification

Source: demande de la feature 3 (session SDLC du 2026-11-05), `aidd_docs/memory/`, ADR 0001

## Phases

| #   | Phase | File | Depends on |
| --- | ----- | ---- | ---------- |
| 1   | Logique pure : position bornée à l'écran et libellés du menu | [`phase-1.md`](./phase-1.md) | none |
| 2   | Déplacement par la barre de titre et position mémorisée | [`phase-2.md`](./phase-2.md) | 1 |
| 3   | Icône de notification et 2e lancement visible | [`phase-3.md`](./phase-3.md) | 1 |
| 4   | Exe et tests manuels Windows | [`phase-4.md`](./phase-4.md) | 2, 3 |

## External sources

| URL | What it settled |
| --- | --- |
| https://v2.tauri.app/learn/system-tray/ | La fonctionnalité `tray-icon` de `tauri`, `TrayIconBuilder` et `Menu`/`MenuItem`, avec `on_menu_event`. À vérifier à l'implémentation. |
| https://v2.tauri.app/reference/javascript/api/namespacewindow/ | `data-tauri-drag-region` et `startDragging` exigent la permission `core:window:allow-start-dragging`. À vérifier à l'implémentation. |

## Impact

| Category | Verdict and evidence | Handled in |
| --- | --- | --- |
| runtime | **Non vérifié :** le comportement réel du tray, du glisser-déposer et de la restauration de position sous Windows. Seul l'humain peut le tester. | phase 4 |
| architecture | La logique pure va dans le crate `overlay` (lu : il contient `OverlayMode`, sans dépendance). La glue Tauri va dans `src-tauri`, déjà lu : `lib.rs`, `overlay_window.rs`. `architecture-tests` interdit toute dépendance interne à `overlay`. | phase 1, 2, 3 |
| regression | Moyen. On modifie le raccourci `Ctrl+Alt+H`, le callback single-instance et `App.tsx`. Ils sont couverts par les tests du crate `overlay`, par Vitest (`App.test.tsx`, `useOverlayMode.test.ts`) et par les tests manuels de la feature 1. | phase 2, 3, 4 |
| documentation | `desktop.md` (tray, position mémorisée, permission de déplacement) et `codebase-map.md` (nouveaux modules). | phase 3 |

## Rules

| Rule | Why it applies |
| --- | --- |
| `testing.md` : toute logique à tester va dans un crate compilable sur l'hôte | Le bornage de la position et les libellés du menu sont testés dans `crates/overlay`. |
| `desktop.md` : pas d'accès système côté JS en dehors du nécessaire | Le seul ajout côté JS est la permission de déplacement. Le tray et la position sont gérés côté Rust. |
| `design.md` : un libellé en plus de la couleur | Les entrées du menu reflètent le mode courant. |
| ADR 0001 | L'humain teste l'exe. |
| `vcs.md` | Un commit par phase, avec les scopes `overlay`, `desktop` et `ui`. |

## Decisions

| Decision | Why |
| --- | --- |
| La position est sauvegardée par l'app dans `<app_config_dir>/window.json`, plutôt que par `tauri-plugin-window-state` | On garde la main sur le bornage à l'écran (fonction pure testée), et on ne persiste que la position. |
| Le déplacement n'est possible qu'en mode interactif | En click-through, la fenêtre ignore la souris. Le mode interactif est donc la seule façon de la saisir. |
