---
status: done
---

# Instruction: État des agents Codex

## Architecture projection

> Tree of the final files. ✅ create · ✏️ modify · ❌ delete

```txt
.
└── crates/adapter-codex/
    ├── src/
    │   ├── ✏️ lib.rs
    │   └── ✅ turn_signal.rs · agent_sessions.rs
    └── tests/
        ├── ✅ codex_agent_sessions.rs
        └── fixtures/ ✅ turn-running.jsonl · turn-complete.jsonl · turn-aborted.jsonl
```

## Test Scope

| Scenario | Action | Observable outcome | Actor |
| --- | --- | --- | --- |
| Happy path | Dernier événement de tour : `task_started` | `TurnStarted` | cli |
| Happy path | Dernier événement de tour : `task_complete` ou `turn_aborted` | `TurnEnded` | cli |
| Edge case | Aucun événement de tour dans la fin de fichier | `Unknown` | cli |
| Happy path | 2 sessions actives et 1 de plus de 60 min | 2 `AgentSession`, triées | cli |
| Edge case | Le mtime d'une session n'a pas changé | Le fichier n'est pas relu (compteur) | cli |

## Tasks to do

### `1)` Signal de tour Codex

> Savoir si un tour Codex est en cours.

1. `turn_signal.rs` : en partant de la fin, renvoyer le premier `event_msg` de type `task_started`, `task_complete` ou `turn_aborted`. Ignorer les lignes invalides et les autres types.

### `2)` `CodexAgentSessions`

> Implémenter `AgentStateSource` pour Codex.

1. Utiliser les mêmes racines que `CodexUsageSource`, puis `recent_files(sessions, "jsonl", now - 60 min)`.
2. Pour chaque fichier, prendre le signal de tour, en cache par chemin et mtime.
3. `session_id` = nom du fichier sans extension. `project` = `cwd` de `session_meta` s'il est lu à bas coût, sinon absent (prévu pour la v2).
4. Fixtures anonymisées.

## Test acceptance criteria

| Task | Acceptance criteria |
| --- | --- |
| 1 | Les tests couvrent les 3 fixtures et le cas `Unknown`. |
| 2 | Les tests couvrent les sessions actives et inactives, et le cache mtime. Le grep d'anonymisation est vide. |
