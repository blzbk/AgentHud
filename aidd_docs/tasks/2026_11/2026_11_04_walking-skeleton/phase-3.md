---
status: done
---

# Instruction: Fenêtre overlay et raccourci global

## Architecture projection

> Tree of the final files. ✅ create · ✏️ modify · ❌ delete

```txt
.
└── apps/desktop/
    ├── src/
    │   ├── ✅ App.tsx · App.test.tsx (phase 2, étendus)
    │   └── features/overlay/ ✅ useOverlayMode.ts · useOverlayMode.test.ts
    └── src-tauri/
        ├── ✅ Cargo.toml · tauri.conf.json (phase 2, étendus)
        └── src/
            ├── ✅ lib.rs (phase 2, étendu)
            └── ✅ overlay_window.rs (glue Tauri ; le mode pur est dans `crates/overlay`, déplacé en revue)
```

## Test Scope

| Scenario | Action | Observable outcome | Actor |
| --- | --- | --- | --- |
| Setup | Build cross-compilé, exe copié et lancé côté Windows par l'humain | La fenêtre apparaît, sans bordure ni entrée dans la barre des tâches | system |
| Happy path | Clic à travers l'overlay | Le clic atteint la fenêtre placée dessous | system |
| Happy path | `Ctrl+Alt+H` | L'overlay devient interactif, et l'indicateur de mode passe à « interactif » | system |
| Edge case | `Ctrl+Alt+H` déjà pris par une autre app | L'app démarre quand même en click-through, et l'erreur d'enregistrement est journalisée | system |
| Teardown | Fermer l'app | Plus aucun processus `agenthud` ni `msedgewebview2` lié | system |

## Wireframe

```txt
┌────────────────────────────┐
│ (1) AgentHud        (2) ●  │
│                            │
│ (3) zone vide (v1)         │
└────────────────────────────┘
```

1. Titre : le placeholder « AgentHud ».
2. Indicateur de mode : `click-through` ou `interactif`, avec libellé et couleur.
3. Zone réservée aux cartes provider des features suivantes.

## Tasks to do

### `1)` Configuration de la fenêtre

> Avoir une fenêtre overlay dès le lancement.

1. Dans `tauri.conf.json` : `transparent: true`, `decorations: false`, `alwaysOnTop: true`, `skipTaskbar: true`, `resizable: false`, une petite taille fixe, et `shadow: false`.

### `2)` Mode overlay côté Rust

> Donner au natif la maîtrise du click-through.

1. `crates/overlay` (déplacé en revue depuis `overlay.rs`) : type `OverlayMode` (`ClickThrough`, `Interactive`), avec `toggle()` testé unitairement.
2. Au `setup`, appliquer `set_ignore_cursor_events(true)`.
3. Émettre l'event `overlay-mode` vers l'UI à chaque changement.

### `3)` Raccourci global

> Basculer de mode sans souris.

1. Ajouter `tauri-plugin-global-shortcut`, puis enregistrer `Ctrl+Alt+H` côté Rust avec `with_handler`, sur `ShortcutState::Pressed`.
2. Si l'enregistrement échoue, journaliser l'erreur et continuer en click-through.

### `4)` Indicateur de mode côté UI

> Afficher le mode courant.

1. `useOverlayMode` écoute `overlay-mode` (`@tauri-apps/api/event`).
2. `App.tsx` affiche l'indicateur. Tests Vitest avec l'event mocké.

## Test acceptance criteria

| Task | Acceptance criteria |
| --- | --- |
| 1 | Sous Windows (humain) : fenêtre transparente, au premier plan, sans bordure, absente de la barre des tâches. |
| 2 | Le test unitaire de `toggle()` passe. Sous Windows, les clics traversent l'overlay au démarrage. |
| 3 | Sous Windows, `Ctrl+Alt+H` bascule le mode, et un deuxième appui revient en click-through. |
| 4 | Le test Vitest vérifie que l'indicateur passe à « interactif » à réception de l'event. |
