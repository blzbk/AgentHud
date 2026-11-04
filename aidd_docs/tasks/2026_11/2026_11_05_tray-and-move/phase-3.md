---
status: done
---

# Instruction: Icône de notification et 2e lancement visible

## Architecture projection

> Tree of the final files. ✅ create · ✏️ modify · ❌ delete

```txt
.
├── apps/desktop/src-tauri/
│   ├── ✏️ Cargo.toml
│   └── src/
│       ├── ✏️ lib.rs · overlay_window.rs
│       └── ✅ tray.rs
└── aidd_docs/memory/
    ├── ✏️ desktop.md
    └── ✏️ codebase-map.md
```

## Test Scope

| Scenario | Action | Observable outcome | Actor |
| --- | --- | --- | --- |
| Happy path | Clic droit sur l'icône AgentHud | Menu avec « Mode interactif », « Masquer » et « Quitter » | system |
| Happy path | « Mode interactif » | L'overlay passe en interactif, l'indicateur change, et le libellé devient « Mode click-through » | system |
| Happy path | `Ctrl+Alt+H` | Le libellé du menu suit aussi le mode | system |
| Happy path | « Masquer », puis « Afficher » | L'overlay disparaît puis réapparaît, et le libellé alterne | system |
| Happy path | « Quitter » | L'app se ferme, l'icône disparaît, il ne reste aucun processus | system |
| Edge case | Overlay masqué, puis 2e lancement de l'exe | L'overlay réapparaît, sans 2e instance | system |

## Tasks to do

### `1)` Icône et menu

> Piloter l'app depuis la zone de notification.

1. Activer la fonctionnalité `tray-icon` de `tauri`.
2. `tray.rs` : icône de l'app, tooltip « AgentHud », menu de 3 entrées dont les libellés viennent de `overlay::tray_menu`.
3. Les actions passent par `overlay_window::toggle`, par `show`/`hide` de la fenêtre et par `app.exit(0)`.
4. Exposer `overlay_window::set_visible` et mettre à jour les libellés du menu après chaque changement de mode ou de visibilité, quelle qu'en soit la source (menu ou raccourci).

### `2)` 2e lancement visible

> Ne plus jamais avoir de lancement « sans effet ».

1. Dans le callback single-instance, afficher la fenêtre (`show`) et mettre à jour le menu.

### `3)` Mémoire

> Documenter le nouveau comportement.

1. `desktop.md` : tray, actions, position mémorisée dans `window.json` et permission `start-dragging`.
2. `codebase-map.md` : `tray.rs` et `window_position.rs`.

## Test acceptance criteria

| Task | Acceptance criteria |
| --- | --- |
| 1 | `cargo xwin clippy` passe. Sous Windows, chaque entrée du menu agit et son libellé reflète l'état. |
| 2 | Sous Windows, l'overlay masqué réapparaît au 2e lancement. |
| 3 | La mémoire décrit le tray et la position mémorisée. |
