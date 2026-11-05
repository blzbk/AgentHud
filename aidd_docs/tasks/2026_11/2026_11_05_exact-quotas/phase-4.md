---
status: done
---

# Instruction: Codex : client `codex app-server` (`account/rateLimits/read`)

## Architecture projection

> Tree of the final files. ✅ create · ✏️ modify · ❌ delete

```txt
.
└── crates/adapter-codex/
    ├── src/ ✏️ lib.rs · ✅ app_server.rs · app_server_launch.rs
    └── tests/
        ├── ✅ codex_app_server.rs
        └── fixtures/ ✅ app-server-rate-limits.json · app-server-auth-required.json
```

## Test Scope

| Scenario | Action | Observable outcome | Actor |
| --- | --- | --- | --- |
| Setup | Faux serveur sur canal de lignes en mémoire | Aucun processus lancé | cli |
| Happy path | `initialize` → réponse, `initialized`, `account/rateLimits/read` → réponse | 5 h et hebdo exacts, origine `AppServer` | cli |
| Edge case | Notifications et lignes illisibles intercalées | Ignorées jusqu'à la réponse au bon `id` | cli |
| Edge case | Erreur `-32600` « authentication required » ou backend 401 | `FetchError::AuthExpired` | cli |
| Edge case | Pas de réponse avant le délai | `Unavailable` | cli |
| Edge case | `codex` introuvable, aucune distro démarrée | `Unavailable`, aucun `wsl.exe` lancé pour une distro arrêtée | cli |

## Tasks to do

### `1)` Protocole

> Parler JSON-RPC (sans champ `jsonrpc`) en JSONL sur stdio.

1. `app_server.rs` : écrit `initialize` (`clientInfo { name: "agenthud", title: "AgentHud", version }`), attend sa réponse, écrit `initialized` puis `account/rateLimits/read` (`excludeResetCreditDetails: true`), attend la réponse au bon `id`, avec un délai global.
2. Parsing défensif de `rateLimits.primary` / `secondary` (`usedPercent`, `windowDurationMins`, `resetsAt`), classement par durée puis par position, comme `token_count`.

### `2)` Lancement

> Lancer le client officiel sans jamais réveiller une distro.

1. `app_server_launch.rs` : candidats dans l'ordre : `codex.exe` puis `codex.cmd` trouvés dans le `PATH` Windows ; puis, pour chaque distro démarrée (`DistroLister` partagé), `wsl.exe -d <distro> -- sh -lc 'exec codex app-server'`. Windows seulement (`cfg(windows)`), `CREATE_NO_WINDOW`, stdin/stdout en pipes, processus tué après l'échange.
2. `CodexAppServer` implémente `UsageFetcher` : premier candidat qui répond ; l'auth expirée remonte telle quelle.

## Test acceptance criteria

| Task | Acceptance criteria |
| --- | --- |
| 1 | Tests sur canal en mémoire : requêtes écrites conformes (méthodes, `id`, pas de `jsonrpc`), réponse lue malgré les notifications, auth et délai classés. |
| 2 | `cargo xwin clippy -p agenthud-desktop` compile le lanceur Windows ; un test hôte vérifie l'ordre des candidats et qu'aucune distro arrêtée n'est candidate. |
