---
status: pending
---

# Instruction: Desktop : options persistées, tray et boucle de polling distante

## Architecture projection

> Tree of the final files. ✅ create · ✏️ modify · ❌ delete

```txt
.
├── crates/overlay/src/ ✏️ lib.rs · tray_menu.rs · ✅ options.rs
└── apps/desktop/src-tauri/
    ├── ✏️ Cargo.toml
    └── src/ ✏️ lib.rs · tray.rs · usage_loop.rs · ✅ options_store.rs · remote_poll.rs
```

## Test Scope

| Scenario | Action | Observable outcome | Actor |
| --- | --- | --- | --- |
| Happy path | Premier lancement sans `options.json` | OAuth désactivé, app-server activé | cli |
| Happy path | Case du tray cochée | Option appliquée tout de suite et écrite dans `options.json` | system |
| Edge case | `options.json` illisible | Valeurs par défaut, erreur journalisée | cli |
| Edge case | OAuth activé mais statusline fraîche | Aucun appel OAuth | cli |
| Happy path | Sondes distantes lentes | La lecture des fichiers continue toutes les 3 s | system |

## Tasks to do

### `1)` Options

> Persister les choix de l'utilisateur.

1. `overlay::Options { claude_oauth: false, codex_app_server: true }`, `from_json` tolérant (champ manquant = défaut) et `to_json`.
2. `tray_menu` : libellés `CLAUDE_OAUTH_LABEL` (« ⚠ Quotas Claude via OAuth (non autorisé par Anthropic) ») et `CODEX_APP_SERVER_LABEL` (« Quotas Codex via app-server »).
3. `options_store.rs` : lecture au démarrage et écriture dans `<app_config_dir>/options.json`.

### `2)` Tray

> Basculer chaque sonde comme le mode click-through.

1. 2 `CheckMenuItem` dans `tray.rs` ; un clic bascule l'option, l'applique au `PolledUsage` correspondant, l'enregistre et rafraîchit la coche.

### `3)` Boucle distante

> Sonder sans bloquer la lecture des fichiers.

1. `usage_loop::compose` : Codex = `FreshestUsage(sessions, PolledUsage(app-server))` ; Claude = `FreshestUsage(statusline, PolledUsage(OAuth), estimation)`, avec le `CachedLister` partagé.
2. `remote_poll.rs` : tâche `tokio` toutes les 10 s, dans `spawn_blocking` : calcule l'`Activity` depuis les derniers `AgentReport`, la transmet aux planificateurs, sonde app-server si due, sonde OAuth si due et si `statusline_is_stale`.
3. Rapports : `authExpired` posé à partir de chaque `PolledUsage`.

## Test acceptance criteria

| Task | Acceptance criteria |
| --- | --- |
| 1 | Tests `overlay` : défauts, JSON partiel, JSON invalide, aller-retour ; libellés exacts. |
| 2 | `cargo xwin clippy -p agenthud-desktop --all-targets -- -D warnings` passe. |
| 3 | `cargo xwin test -p agenthud-desktop --no-run` compile ; la règle « OAuth seulement si statusline périmée » est testée dans `application` (phase 1). |
