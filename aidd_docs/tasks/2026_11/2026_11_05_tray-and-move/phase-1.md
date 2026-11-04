---
status: done
---

# Instruction: Logique pure : position bornée à l'écran et libellés du menu

## Architecture projection

> Tree of the final files. ✅ create · ✏️ modify · ❌ delete

```txt
.
└── crates/overlay/src/
    ├── ✏️ lib.rs
    ├── ✅ placement.rs
    └── ✅ tray_menu.rs
```

## Test Scope

| Scenario | Action | Observable outcome | Actor |
| --- | --- | --- | --- |
| Happy path | Position sauvegardée entièrement sur un écran connu | Position conservée | cli |
| Edge case | Position sur un écran qui n'existe plus | Position ramenée dans l'écran principal, fenêtre entièrement visible | cli |
| Edge case | Fenêtre à cheval sur le bord droit | Décalée pour être entièrement visible | cli |
| Edge case | Aucun écran connu | Aucune position imposée (`None`) | cli |
| Happy path | Libellés du menu pour chaque couple mode × visibilité | « Mode interactif » en click-through, « Mode click-through » en interactif ; « Masquer » si visible, sinon « Afficher » | cli |

## Tasks to do

### `1)` Placement

> Ramener toute position sauvegardée dans un écran existant.

1. `placement.rs` : types `Point`, `Size` et `Rect` en pixels physiques.
2. `fit_on_screens(saved, window_size, screens, primary) -> Option<Point>` : garde la position si la fenêtre tient entièrement dans un écran, sinon la ramène dans l'écran le plus proche, et à défaut dans l'écran principal.

### `2)` Libellés du menu

> Faire refléter l'état courant par le menu.

1. `tray_menu.rs` : `mode_item_label(OverlayMode)` et `visibility_item_label(visible: bool)`, plus la constante du libellé « Quitter ».

## Test acceptance criteria

| Task | Acceptance criteria |
| --- | --- |
| 1 | Les tests couvrent les 4 scénarios de placement et passent avec `cargo test -p overlay`. |
| 2 | Les tests couvrent les 4 combinaisons de libellés. `architecture-tests` passe toujours. |
