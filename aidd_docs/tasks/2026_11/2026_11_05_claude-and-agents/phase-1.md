---
status: done
---

# Instruction: Modèle : tokens d'usage, sessions d'agent et déduction de l'état

## Architecture projection

> Tree of the final files. ✅ create · ✏️ modify · ❌ delete

```txt
.
└── crates/
    ├── domain/src/
    │   ├── ✏️ lib.rs · usage.rs
    │   └── ✅ agent.rs
    └── application/src/
        ├── ✏️ lib.rs · ports.rs · usage_report.rs
        ├── ✅ track_agents.rs
        └── ✅ agent_report.rs
```

## Test Scope

| Scenario | Action | Observable outcome | Actor |
| --- | --- | --- | --- |
| Happy path | Dernier signal `TurnStarted`, écriture il y a 10 s | `working` | cli |
| Happy path | Dernier signal `TurnEnded`, écriture il y a 30 s | `waiting` | cli |
| Edge case | N'importe quel signal, sans écriture depuis 6 min | `idle` | cli |
| Edge case | Provider sans session active | Une seule entrée `offline` pour ce provider | cli |
| Happy path | Fenêtre Claude en tokens (`used_tokens`, `resets_at` estimé) | Le rapport sérialise `usedTokens` et `estimated: true`, sans `%` | cli |
| Regression | Snapshot Codex en % | Rapport identique à l'existant | cli |

## Tasks to do

### `1)` Usage en tokens

> Représenter un usage que l'on ne connaît qu'en tokens.

1. `UsageWindow` accepte une mesure `Percent(f64)` ou `Tokens(u64)`, plus un drapeau `estimated`.
2. `used_at(now)` renvoie la même mesure, remise à 0 une fois `resets_at` passé.
3. Adapter `UsageReport` : `usedPercent` ou `usedTokens`, plus `estimated`. Le cas Codex n'est pas modifié.

### `2)` Sessions d'agent

> Déduire l'état d'un agent d'après son dernier signal.

1. `agent.rs` :
   - `TurnSignal` (`TurnStarted`, `TurnEnded`, `Unknown`) ;
   - `AgentSession` (`provider`, `session_id`, `project` optionnel, `last_signal`, `last_write`) ;
   - `state_at(now)` qui renvoie un `AgentState`.
2. Constantes : `IDLE_AFTER` = 5 min et `ACTIVE_WITHIN` = 60 min.

### `3)` Port et cas d'usage

> Lister les agents de tous les providers.

1. Trait `AgentStateSource { provider(); sessions() -> Result<Vec<AgentSession>, SourceError> }`.
2. `TrackAgents` renvoie, pour chaque provider, ses sessions actives triées par dernière écriture, ou une entrée `offline`. Une erreur donne `unknown`, journalisée.
3. `AgentReport` est le DTO sérialisé en camelCase, du même style qu'`UsageReport`.

## Test acceptance criteria

| Task | Acceptance criteria |
| --- | --- |
| 1 | Les tests couvrent les tokens, le cas estimé et la non-régression Codex. |
| 2 | Les tests couvrent `working`, `waiting`, `idle` et les seuils exacts. |
| 3 | Les tests couvrent `offline`, l'erreur et le tri. `architecture-tests` passe. |
