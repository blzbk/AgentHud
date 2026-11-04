---
status: pending
---

# Instruction: Déplacement par la barre de titre et position mémorisée

## Architecture projection

> Tree of the final files. ✅ create · ✏️ modify · ❌ delete

```txt
.
└── apps/desktop/
    ├── src/
    │   ├── ✏️ App.tsx · App.test.tsx · App.module.css
    └── src-tauri/
        ├── capabilities/ ✏️ default.json
        └── src/
            ├── ✏️ lib.rs
            └── ✅ window_position.rs
```

## Test Scope

| Scenario | Action | Observable outcome | Actor |
| --- | --- | --- | --- |
| Happy path | Mode interactif, glisser la barre de titre | La fenêtre suit la souris | system |
| Edge case | Mode click-through, clic sur la barre de titre | Le clic traverse, la fenêtre ne bouge pas | system |
| Happy path | Déplacer, quitter, relancer | La fenêtre réapparaît à la même position | system |
| Edge case | Fichier de position absent ou illisible | Position par défaut, erreur journalisée, pas de plantage | cli |
| Happy path | Vitest : en mode interactif, l'en-tête porte `data-tauri-drag-region` ; pas en click-through | Attribut présent ou absent selon le mode | cli |

## Tasks to do

### `1)` Barre de titre déplaçable

> Saisir l'overlay par son titre en mode interactif.

1. Dans `App.tsx`, poser `data-tauri-drag-region` sur l'en-tête, seulement en mode `interactive`, avec un curseur `move` pour le signaler.
2. Ajouter `core:window:allow-start-dragging` dans `capabilities/default.json`.

### `2)` Position mémorisée

> Retrouver l'overlay là où on l'a laissé.

1. `window_position.rs` : au déplacement (`WindowEvent::Moved`), sauvegarder la position dans `<app_config_dir>/window.json` avec un debounce d'environ 500 ms.
2. Au démarrage, lire la position, la passer à `overlay::fit_on_screens` avec les écrans disponibles, puis l'appliquer.
3. Toute erreur d'I/O ou de parsing est journalisée, et l'app garde la position par défaut.

## Test acceptance criteria

| Task | Acceptance criteria |
| --- | --- |
| 1 | Le test Vitest vérifie la présence de l'attribut selon le mode. La permission est déclarée. |
| 2 | `cargo xwin clippy` passe. La lecture et l'écriture du JSON sont testées via une fonction pure (sérialisation de `Point`) dans `crates/overlay`. Sous Windows, la position survit à un relancement (test manuel). |
