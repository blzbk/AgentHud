---
status: done
---

# Instruction: Adapter Claude : tokens sur 5 h et état des agents

## Architecture projection

> Tree of the final files. ✅ create · ✏️ modify · ❌ delete

```txt
.
└── crates/adapter-claude/
    ├── ✏️ Cargo.toml
    ├── src/
    │   ├── ✏️ lib.rs
    │   └── ✅ transcript.rs · usage_block.rs · usage_source.rs · agent_sessions.rs
    └── tests/
        ├── ✅ claude_usage_source.rs · claude_agent_sessions.rs
        └── fixtures/ ✅ assistant-end-turn.jsonl · assistant-tool-use.jsonl · duplicated-messages.jsonl · metadata-only.jsonl
```

## Test Scope

| Scenario | Action | Observable outcome | Actor |
| --- | --- | --- | --- |
| Happy path | Messages `assistant` sur 2 h, dans 2 projets | Somme des tokens (`input`, `output`, `cache_creation`, `cache_read`) dans le bloc courant | cli |
| Edge case | Le même `message.id` + `requestId` répété sur plusieurs lignes | Compté une seule fois | cli |
| Edge case | Écart de plus de 5 h entre 2 messages | Nouveau bloc qui démarre à l'heure pleine du message, `resets_at` = début + 5 h, `estimated` | cli |
| Edge case | Aucun message depuis plus de 5 h | Bloc expiré : usage à 0, ou `unknown` sans aucune session | cli |
| Happy path | Dernière ligne significative : `assistant` avec `end_turn` | `TurnEnded`, donc `waiting` | cli |
| Happy path | Dernière ligne significative : `assistant` avec `tool_use`, ou `user` | `TurnStarted`, donc `working` | cli |
| Edge case | Seulement des lignes de métadonnées en fin de fichier | Remonter plus haut. Si rien n'est trouvé : `Unknown` | cli |

## Tasks to do

### `1)` Lecture des transcripts

> Extraire les messages utiles d'un transcript Claude.

1. `transcript.rs` : parsing défensif d'une ligne. Garder `type`, `timestamp`, `requestId`, `message.id`, `message.stop_reason` et `message.usage`. Toute autre ligne est une métadonnée et est ignorée.

### `2)` Bloc de 5 h estimé

> Approximer la fenêtre de 5 h d'Anthropic.

1. `usage_block.rs` : fonction pure. À partir des messages horodatés (dédoublonnés), elle renvoie le bloc courant : début à l'heure pleine, fin = début + 5 h, total de tokens.
2. `usage_source.rs` : `ClaudeUsageSource` lit les transcripts modifiés depuis moins de 5 h dans toutes les racines `.claude/projects`. Les fichiers sont relus en entier, mais seulement quand leur mtime change. Cache par fichier.
3. Le snapshot produit `five_hours` en `Tokens` avec `estimated`, et `weekly` absent.

### `3)` État des agents Claude

> Implémenter `AgentStateSource` pour Claude.

1. `agent_sessions.rs` : sessions = `projects/*/*.jsonl` modifiés depuis moins de 60 min. Le signal de tour vient de la dernière ligne significative.
2. `project` = nom du dossier du projet, décodé seulement s'il est trivial, sinon brut (prévu pour la v2).
3. Fixtures anonymisées (aucun prompt, aucun chemin perso, aucun e-mail).

## Test acceptance criteria

| Task | Acceptance criteria |
| --- | --- |
| 1 | Les tests de parsing passent sur les 4 fixtures. |
| 2 | Les tests couvrent la somme, le dédoublonnage, le début de bloc, l'expiration et le cache mtime. |
| 3 | Les tests couvrent `end_turn`, `tool_use`, `user` et les métadonnées seules. Le grep d'anonymisation est vide. |
