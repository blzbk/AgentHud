---
objective: "Le HUD présente Codex et Claude côte à côte : l'usage de chacun (Codex en %, Claude en tokens sur une fenêtre de 5 h estimée) et l'état de chaque session d'agent active (waiting, working, idle, offline), lus passivement dans les fichiers locaux de Windows et de WSL."
status: in-progress
---

# Plan: Adapter Claude, état des agents et HUD multi-LLM

Source: stories #4 (état des agents Codex), #5 (état des agents Claude) et #6 (tokens Claude sur 5 h) ; demande de l'utilisateur du 2026-11-05 : « 3, et je veux que mon HUD présente tous mes LLM, Codex et Claude ». Mémoire : `aidd_docs/memory/` et ADR 0001.

## Phases

| #   | Phase | File | Depends on |
| --- | ----- | ---- | ---------- |
| 1   | Modèle : tokens d'usage, sessions d'agent et déduction de l'état | [`phase-1.md`](./phase-1.md) | none |
| 2   | Accès fichiers : sessions récentes et liste des distros en cache | [`phase-2.md`](./phase-2.md) | 1 |
| 3   | État des agents Codex | [`phase-3.md`](./phase-3.md) | 1, 2 |
| 4   | Adapter Claude : tokens sur 5 h et état des agents | [`phase-4.md`](./phase-4.md) | 1, 2 |
| 5   | HUD multi-LLM : cartes Codex et Claude avec leurs agents | [`phase-5.md`](./phase-5.md) | 3, 4 |
| 6   | Exe et tests manuels Windows | [`phase-6.md`](./phase-6.md) | 5 |
| 7   | Agents réellement ouverts (marqueurs d'exécution des CLI) | [`phase-7.md`](./phase-7.md) | 5 |

## External sources

| URL | What it settled |
| --- | --- |
| https://github.com/ryoppippi/ccusage | Méthode de référence pour les tokens Claude : dédoublonnage par `message.id` + `requestId`, blocs de 5 h démarrant à l'heure pleine du premier message après 5 h d'inactivité. À relire à l'implémentation. |

## Impact

| Category | Verdict and evidence | Handled in |
| --- | --- | --- |
| runtime | **Observé :** dans `~/.claude/projects/*/*.jsonl`, les lignes `assistant` portent `timestamp`, `message.id`, `message.model`, `message.stop_reason` (`tool_use` ou `end_turn`) et `message.usage` (`input_tokens`, `output_tokens`, `cache_creation_input_tokens`, `cache_read_input_tokens`). Beaucoup de lignes de métadonnées sont intercalées : `attachment`, `last-prompt`, `mode`, `permission-mode`, `ai-title`, `system`, etc. | phase 4 |
| runtime | **Observé :** les sessions Codex contiennent `event_msg/task_started`, `event_msg/task_complete`, `event_msg/turn_aborted`, `response_item/*` et `event_msg/token_count`. | phase 3 |
| runtime | **Observé :** `~/.claude/projects` existe dans WSL et dans `C:\Users\Thomas\.claude\projects`. | phase 4 |
| runtime | **Non vérifié :** un tool en attente d'approbation laisse la dernière ligne en `assistant/tool_use`, donc l'agent apparaît `working` au lieu de `waiting`. C'est une limite connue de la lecture passive. | phase 4 |
| architecture | Ajout du port `AgentStateSource`, du cas d'usage `TrackAgents` et du crate `adapter-claude`, déjà présent mais vide. Le sens hexagonal est inchangé et vérifié par `architecture-tests`. | phase 1, 4 |
| regression | Moyen. On modifie `UsageSnapshot`/`UsageWindow` (ajout des tokens), `usage_loop.rs`, `App.tsx` et la taille de la fenêtre. Ils sont couverts par les tests de `domain`, d'`application` et d'`adapter-codex`, et par Vitest. | phase 1, 5 |
| performance | Issue #9 : `wsl.exe` est lancé à chaque tick, et la feature double les racines (`.claude` en plus de `.codex`). On partage une liste de distros gardée 30 s en cache. | phase 2 |
| documentation | `codebase-map.md`, `architecture.md` (port `AgentStateSource`, règle d'état), `design.md` (cartes et badges) et `project-brief.md` (tokens Claude estimés). | phase 5 |

## Rules

| Rule | Why it applies |
| --- | --- |
| `architecture.md` : lecture passive, polling sur mtime, parsing défensif, `unknown` en cas de doute | Les 2 adapters. |
| `architecture.md` : aucun appel à l'endpoint OAuth d'Anthropic | L'usage Claude est calculé à partir des transcripts locaux uniquement. |
| `testing.md` : fixtures JSONL anonymisées | Les nouvelles fixtures Claude et Codex. |
| `design.md` : un libellé en plus de la couleur, jamais un champ vide | Les badges d'état et les tokens. |
| ADR 0001 | L'humain teste l'exe. |
| `vcs.md` | Un commit par phase, avec les scopes `domain`, `application`, `adapter-fs`, `adapter-codex`, `adapter-claude`, `desktop` et `ui`. |

## Decisions

| Decision | Why |
| --- | --- |
| Session active = fichier modifié depuis moins de 60 min. `idle` après 5 min sans écriture. `offline` quand un provider n'a aucune session active. | Fixe le seuil des stories #4 et #5. Une session plus ancienne disparaît du HUD. |
| La fenêtre de 5 h de Claude est **estimée**, à la façon de ccusage, et affichée avec la mention « estimé » | Anthropic ne publie pas le début de la fenêtre. |
| La story #4 (agents Codex) entre dans la feature | L'utilisateur veut que le HUD présente tous ses LLM de la même façon. |
