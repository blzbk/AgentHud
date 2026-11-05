---
status: pending
---

# Instruction: Modèle : origine des données, quota épuisé, planificateur de polling et sources interrogées

## Architecture projection

> Tree of the final files. ✅ create · ✏️ modify · ❌ delete

```txt
.
└── crates/
    ├── domain/src/ ✏️ lib.rs · usage.rs
    ├── application/src/
    │   ├── ✏️ lib.rs · ports.rs · refresh_usage.rs · usage_report.rs
    │   └── ✅ poll_schedule.rs · polled_usage.rs · freshest_usage.rs
    ├── adapter-claude/src/ ✏️ usage_source.rs
    └── adapter-codex/src/ ✏️ token_count.rs
```

## Test Scope

| Scenario | Action | Observable outcome | Actor |
| --- | --- | --- | --- |
| Happy path | Sondage réussi, agents qui travaillent | Prochain sondage après l'intervalle « actif », jamais avant le plancher | cli |
| Edge case | Quota à 100 % avec `resets_at` proche | Sondage juste après `resets_at` ; sinon période longue | cli |
| Edge case | Quota épuisé puis un agent passe d'inactif à actif | Sondage avancé au plancher | cli |
| Edge case | Tous les agents idle/offline | Intervalle « calme », plus long | cli |
| Edge case | Auth expirée | Période d'auth (Codex 30 min, OAuth 60 min) | cli |
| Edge case | 429 avec `Retry-After` plus long que le recul | `Retry-After` respecté | cli |
| Edge case | Source exacte plus ancienne qu'une estimation | La source exacte gagne | cli |
| Edge case | Statusline écrite il y a 16 min | Repli OAuth autorisé | cli |

## Tasks to do

### `1)` Domaine

> Dire d'où vient une donnée et si un quota est épuisé.

1. `domain::Origin` : `Sessions`, `AppServer`, `Statusline`, `OAuth`, `Transcripts`, avec un libellé de fil (`sessions`, `app-server`, `statusline`, `oauth`, `estimate`).
2. Champ `origin` dans `UsageSnapshot` ; mise à jour des constructeurs (`token_count.rs`, `usage_source.rs`, tests).
3. `UsageSnapshot::exhaustion(now)` : `None`, ou épuisé jusqu'à un reset connu ou inconnu (une fenêtre en % à 100 et pas encore reset).

### `2)` Planificateur pur

> Décider quand interroger une source distante.

1. `PollPolicy { floor, busy, quiet, exhausted, error_backoff, auth_backoff }`, avec `PollPolicy::CODEX_APP_SERVER` et `PollPolicy::CLAUDE_OAUTH`.
2. `PollScheduler` : `is_due(now)`, `record(now, PollOutcome)`, `observe_activity(now, Activity)`. `PollOutcome` : `Fresh { exhaustion }`, `Failed { retry_after }`, `AuthExpired`. `Activity` : `Busy` / `Quiet`.
3. Règles : plancher toujours respecté ; épuisé = période longue et sondage juste après `resets_at` ; épuisé + passage `Quiet` → `Busy` = plancher ; intervalle selon l'activité ; recul sur erreur (`Retry-After` respecté) ; période d'auth.
4. `Activity::from_reports(&[AgentReport])` : `Busy` si un agent est `working` ou `waiting`.

### `3)` Sources interrogées et combinaison

> Garder le dernier résultat d'une sonde et choisir la meilleure donnée.

1. Port `UsageFetcher { provider(), fetch(now) -> Result<UsageSnapshot, FetchError> }`, `FetchError` : `AuthExpired`, `RateLimited { retry_after }`, `Unavailable(String)`.
2. `PolledUsage` : fetcher + planificateur + dernier résultat + option activée. `poll_if_due(now, activity) -> bool`, `set_enabled`, `auth_expired()`. Désactivée, elle ne sonde plus et ne fournit plus rien. Implémente `UsageSource`.
3. `FreshestUsage` (`UsageSource`) : une donnée exacte l'emporte sur une estimée, puis la plus récente ; une erreur n'est remontée que si aucune source n'a de donnée.
4. `statusline_is_stale(snapshot, now)` avec `STATUSLINE_FRESH_FOR` = 15 min.
5. `UsageReport` : `source` (libellé de l'origine) et `authExpired` (sérialisé seulement si vrai, posé par `with_auth_expired`).

## Test acceptance criteria

| Task | Acceptance criteria |
| --- | --- |
| 1 | Tests `domain` : libellés d'origine distincts ; quota épuisé détecté, et plus après le reset. |
| 2 | Tests `application` : chacune des règles 1, 2, 3, 4 et 6 a un test ; le plancher n'est jamais violé, même après `Retry-After` court ou un réveil. |
| 3 | Tests : `PolledUsage` ne sonde pas avant l'échéance ni désactivée ; `FreshestUsage` préfère l'exact puis le récent ; le rapport sérialise `source` et `authExpired`. `cargo test --workspace --exclude agenthud-desktop` passe. |
