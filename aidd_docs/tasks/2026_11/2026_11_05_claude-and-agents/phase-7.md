---
status: done
---

# Instruction: Agents réellement ouverts (marqueurs d'exécution des CLI)

## Architecture projection

> Tree of the final files. ✅ create · ✏️ modify · ❌ delete

```txt
.
├── crates/adapter-claude/
│   ├── src/ ✏️ agent_sessions.rs · ✅ running_sessions.rs
│   └── tests/ ✏️ claude_agent_sessions.rs · fixtures/ ✅ running-session.json
└── crates/adapter-codex/
    ├── src/ ✏️ agent_sessions.rs · ✅ open_threads.rs
    └── tests/ ✏️ codex_agent_sessions.rs
```

## Test Scope

| Scenario | Action | Observable outcome | Actor |
| --- | --- | --- | --- |
| Happy path | Claude : 1 fichier `~/.claude/sessions/<pid>.json` (`sessionId`, `cwd`, `status: busy`) et 3 transcripts récents | 1 seul agent, nommé d'après le dernier segment de `cwd`, `working` | cli |
| Happy path | Claude : `status: idle` | `waiting` (`idle` après 5 min sans écriture, règle existante) | cli |
| Edge case | Claude : aucun dossier `sessions/`, ou JSON illisible | Repli sur le comportement actuel (transcripts de moins de 60 min) | cli |
| Happy path | Codex : `thread-writer-locks/<id>.lock` sans rollout correspondant | 1 agent `waiting`, nommé d'après l'id court | cli |
| Happy path | Codex : lock et rollout du même id | État déduit du rollout (règle existante) | cli |
| Edge case | Codex : rollout récent sans lock | Agent absent : le thread est fermé | cli |
| Edge case | Codex : dossier de locks absent | Repli sur le comportement actuel | cli |
| Happy path | Sous Windows, avec 1 Claude et 1 Codex ouverts | Exactement 1 agent par provider | system |

## Tasks to do

### `1)` Sessions Claude ouvertes

> Ne lister que les sessions Claude réellement en cours.

1. `running_sessions.rs` : lire `<root>/sessions/*.json`, avec un parsing défensif de `sessionId`, `cwd` et `status`. Ignorer `.key` et tout autre fichier.
2. Si au moins un marqueur est lu : les agents sont exactement ces sessions.
   - L'état de base vient de `status` : `busy` donne `working`, `idle` donne `waiting`.
   - La dernière écriture du transcript correspondant s'applique toujours pour `idle` (5 min).
   - Le projet est le dernier segment de `cwd`, qu'il soit écrit avec `/` ou avec `\`.
3. Sinon : repli sur la règle actuelle.

### `2)` Threads Codex ouverts

> Voir une conversation Codex dès son ouverture.

1. `open_threads.rs` : lister `<root>/thread-writer-locks/*.lock`, en ignorant `.coordination.lock`. Id = nom du fichier sans extension.
2. Si le dossier de locks existe : les agents sont les threads verrouillés.
   - Avec un rollout dont le nom contient l'id : état déduit du rollout.
   - Sans rollout : `waiting`, avec la date du lock comme dernière écriture.
3. Sinon : repli sur la règle actuelle.

### `3)` Mémoire

> Documenter les nouvelles sources.

1. `architecture.md` (règle d'état des agents) : préciser que les marqueurs sont internes aux CLI, non documentés, lus passivement, avec repli.

## Test acceptance criteria

| Task | Acceptance criteria |
| --- | --- |
| 1 | Les tests couvrent `busy`, `idle`, le repli, un `cwd` Windows ou Unix, et le cas « 3 transcripts, 1 seul marqueur » (1 seul agent). |
| 2 | Les tests couvrent un lock sans rollout, un lock avec rollout, un rollout sans lock, et le repli. |
| 3 | `architecture.md` décrit les 2 marqueurs. |
