# Project Brief

## What it is

- Overlay Windows, toujours au premier plan et click-through, qui affiche le statut des LLM de code : usage des quotas et état des agents.
- Outil perso, destiné à l'open source.

## Why it exists

- Voir d'un coup d'œil le quota restant et l'activité de chaque agent, sans ouvrir de terminal ni de dashboard.
- Rester **indépendant** des CLI surveillées : aucun hook, aucune configuration, aucun token. Ne jamais enfreindre les règles d'usage des providers.

## Domain language

| Term | Meaning |
| ---- | ------- |
| Provider | Un LLM surveillé (Claude Code, Codex). Chacun a son propre adapter. |
| Session | Une conversation d'un agent, identifiée par son fichier JSONL. |
| UsageWindow | Une fenêtre de quota : `5h` (session) ou `weekly`. Elle porte un % utilisé et une heure de reset. |
| AgentState | L'état déduit d'un agent : `waiting`, `working`, `idle` ou `offline`. |
| Source | Ce qui fournit les données d'un provider. Les sources passives (lecture de fichiers) passent avant les sources optionnelles. |

## Key features

- Usage des fenêtres 5 h et hebdomadaire pour chaque provider.
- État de chaque agent : en attente, working, idle ou hors ligne.
- Valeurs en cache avec leur âge, et remise à 0 % une fois `resets_at` passé.
