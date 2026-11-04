# Architecture

Vision complète et justification des choix : `aidd_docs/INSTALL.md`.

## Stack

- **Rust + Tauri** pour le cœur : polling, lecture des fichiers, déduction des états. Le travail de fond hors WebView évite le ralentissement des timers et ne laisse aucun secret côté JS.
- **React + TypeScript + Vite** pour l'UI, dans la WebView2.
- `tokio` pour le polling, `serde` pour le parsing JSONL, `thiserror` pour les erreurs typées.
- Types TS des commands et events écrits à la main, en miroir des DTO Rust (`UsageReport` dans `application`, `useUsage.ts` côté UI). `tauri-specta` est écarté : il n'existe qu'en release candidate, et il génère les types en exécutant l'app, ce qui est impossible depuis WSL (ADR 0001).

## How it fits together

```mermaid
flowchart LR
  UI[UI React] -->|commands| Shell[src-tauri]
  Shell -->|events| UI
  Shell --> App[application]
  App --> Domain[domain]
  Adapters[adapters per provider] -.implements ports.-> App
  Adapters --> Fs[adapter-fs]
  Fs --> Files[("~/.claude, ~/.codex")]
```

## Key decisions

- **DDD et hexagonal :**
  - `domain` est pur, sans I/O.
  - `application` porte les cas d'usage et les ports (`UsageSource`, `AgentStateSource`).
  - Les adapters implémentent les ports.
  - Seul `src-tauri` assemble les modules (composition root).
  - `overlay` est hors de l'hexagone : c'est la logique pure de la fenêtre, sans dépendance interne, utilisée seulement par `src-tauri`.
- **Un adapter par provider.** Ajouter un LLM revient à créer `adapter-<llm>`, sans jamais toucher `domain`.
- **Lecture passive uniquement en v1.** L'app lit les fichiers que les CLI écrivent déjà. Elle n'installe aucun hook, ne demande aucune configuration et ne lance ni agent ni CLI surveillé. Son seul processus externe est `wsl.exe --list --running --quiet`, en lecture, pour lister les distros démarrées.
- **Aucun appel à l'endpoint OAuth d'Anthropic.** Il est non officiel, renvoie des 429 et son usage dans un outil tiers est interdit par les conditions d'Anthropic. L'usage Claude est donc exprimé en **tokens estimés** sur un bloc de 5 h calculé comme `ccusage` (`adapter-claude/src/usage_block.rs`) : dédoublonnage par `message.id` + `requestId`, bloc qui démarre à l'heure pleine du premier message et dure 5 h, nouveau bloc après 5 h depuis le début ou depuis le message précédent. Les transcripts des sous-agents comptent aussi.
- **État des agents (port `AgentStateSource`, cas d'usage `TrackAgents`) :** une session = un fichier JSONL modifié depuis moins de 60 min (`ACTIVE_WITHIN`). Son état vient du dernier événement de tour : tour en cours = `working`, tour fini = `waiting`, rien de lisible = `unknown`. Sans écriture depuis 5 min (`IDLE_AFTER`), elle est `idle`. Un provider sans session active est `offline`. Côté Claude, l'écriture d'un sous-agent garde sa session active.
- **Polling sur la date de modification (mtime) toutes les 2 à 5 s,** jamais de surveillance des fichiers. `ReadDirectoryChangesW` n'est pas fiable sur `\\wsl$`.
- **Boucle de polling :** `src-tauri/src/usage_loop.rs` lance une tâche `tokio` (runtime Tauri) toutes les 3 s. Elle exécute `RefreshUsage` et `TrackAgents` (Codex puis Claude) hors des workers async, garde le dernier résultat (commands `usage` et `agents`, lues par l'UI au chargement) et émet les events `usage` et `agents` seulement s'ils changent. Toutes les sources partagent un seul `CachedLister` : `wsl.exe` est lancé au plus une fois toutes les 30 s.
- **Build dans WSL, avec l'exe Windows cross-compilé par `cargo-xwin`.** L'agent ne lance jamais d'outil Windows : sandbox oblige. L'humain lance l'exe. Voir `aidd_docs/adr/0001-cross-compile-windows-from-wsl.md`.
- **Un provider peut cumuler plusieurs sources,** de la plus fiable à la moins fiable. Les sources optionnelles (hooks, `codex app-server`) viennent après les sources passives.

## Gotchas

- Les schémas JSONL de Claude et Codex changent d'une version à l'autre (par exemple `resets_at` ou `resets_in_seconds`, et des fenêtres qui peuvent valoir `null`). Le parsing doit être défensif : en cas de doute, afficher `unknown`, ne jamais planter.
- Les données des CLI peuvent se trouver dans WSL. Il faut résoudre `\\wsl.localhost\<distro>\home\<user>\...` comme `\\wsl$\...`. Le polling ne doit pas réveiller une distro arrêtée.
- La RAM se mesure sur **tous** les processus `msedgewebview2.exe`. La cible est moins de 100 Mo.
