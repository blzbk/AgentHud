---
objective: "L'overlay affiche l'usage Codex des fenêtres 5 h et hebdomadaire, lu passivement dans les sessions JSONL de Windows et de WSL, avec l'heure de reset, l'âge de la donnée et « unknown » quand l'usage n'est pas connu."
status: in-progress
---

# Plan: Adapter Codex, usage 5 h et hebdomadaire

Source: demande de la feature 2 (session SDLC du 2026-11-04), `aidd_docs/memory/`, `aidd_docs/adr/0001-cross-compile-windows-from-wsl.md`

## Phases

| #   | Phase | File | Depends on |
| --- | ----- | ---- | ---------- |
| 1   | Modèle d'usage et cas d'usage RefreshUsage | [`phase-1.md`](./phase-1.md) | none |
| 2   | Accès fichiers : racines Windows/WSL et lecture de fin de fichier | [`phase-2.md`](./phase-2.md) | 1 |
| 3   | Adapter Codex sur fixtures | [`phase-3.md`](./phase-3.md) | 1, 2 |
| 4   | Câblage Tauri et carte Codex dans l'UI | [`phase-4.md`](./phase-4.md) | 3 |
| 5   | Exe et test manuel Windows | [`phase-5.md`](./phase-5.md) | 4 |

## External sources

| URL | What it settled |
| --- | --- |
| https://github.com/openai/codex/issues/5276 | Les `rate_limits` des événements `token_count` peuvent valoir `null` dans certaines versions. |
| https://lin-guanguo.github.io/llm-memory-research/agent-cli/codex-session-file-schema/ | Le champ de reset change selon les versions : `resets_at` dans les unes, `resets_in_seconds` dans les autres. |

## Impact

| Category | Verdict and evidence | Handled in |
| --- | --- | --- |
| runtime | **Observé** dans `~/.codex/sessions/2026/11/03/rollout-*.jsonl` (12 fichiers) : la dernière ligne `event_msg` / `token_count` porte `rate_limits.primary` (`used_percent` 1.0, `window_minutes` 300, `resets_at` en secondes epoch), `secondary` (`window_minutes` 10080) et `plan_type`. | phase 3 |
| runtime | **Observé :** `~/.codex` existe dans WSL et dans `C:\Users\Thomas\.codex`. **Non vérifié :** l'accès de l'exe Windows à `\\wsl.localhost\<distro>\home\<user>\.codex`, et la détection d'une distro démarrée sans la réveiller. | phase 2, phase 5 |
| runtime | **Non vérifié :** la taille des sessions. Elles peuvent être volumineuses, on ne lit donc que la fin de fichier. | phase 2 |
| architecture | On part du squelette hexagonal (crates vides). La feature ajoute le premier port, `UsageSource`, le premier cas d'usage, le premier adapter câblé dans `src-tauri`, et la boucle de polling. Le sens des dépendances reste verrouillé par `architecture-tests`. | phase 1, phase 4 |
| regression | Faible : le squelette (overlay, raccourci, single-instance) est couvert par Vitest, par le crate `overlay` et par `manual-tests.md`. On n'y touche que dans `lib.rs` et `App.tsx`. | phase 4 |
| documentation | `codebase-map.md` (nouvelle feature UI `features/usage`). `architecture.md` (boucle de polling et event `usage`) si la forme diffère. `desktop.md` (lancement de `wsl.exe` par l'app à l'exécution). | phase 4 |

## Rules

| Rule | Why it applies |
| --- | --- |
| `architecture.md` : lecture passive uniquement, polling sur mtime, jamais de watcher | C'est le cœur de l'adapter. La règle « ne lance aucun processus » est précisée en phase 4 pour autoriser `wsl.exe --list --running`. |
| `architecture.md` : schémas JSONL non garantis, parsing défensif, `unknown` plutôt qu'un plantage | Les champs varient selon les versions de Codex. |
| `testing.md` : fixtures JSONL anonymisées, sans donnée perso ni token | Les tests de l'adapter s'appuient sur des sessions réelles. |
| `design.md` : `unknown` jamais affiché vide ni à 0 %, libellé en plus de la couleur | Rendu de la carte. |
| ADR 0001 : l'agent ne lance aucun programme Windows | Le test d'exécution est fait par l'humain. Le code de l'app peut appeler `wsl.exe` à l'exécution, mais jamais l'agent. |
| `vcs.md` : un commit par phase, avec les scopes `domain`, `application`, `adapter-fs`, `adapter-codex`, `desktop`, `ui` | Suivi de l'avancement. |

## Decisions

| Decision | Why |
| --- | --- |
| Distros WSL démarrées détectées à l'exécution par `wsl.exe --list --running --quiet` (sortie UTF-16), derrière un trait `DistroLister`. Seules leurs racines sont scannées. | Accéder à `\\wsl.localhost\<distro>` démarre une distro arrêtée. Le trait garde `adapter-fs` testable sur l'hôte. |
| Usage d'un provider = le `token_count` non nul le plus récent, toutes racines confondues (horodatage de l'événement, sinon mtime) | Codex peut tourner côté Windows comme côté WSL. |
| La règle « 0 % une fois `resets_at` passé » est appliquée dans `domain` (`UsageWindow::used_at(now)`) | C'est une règle métier pure, testable sans I/O. |
