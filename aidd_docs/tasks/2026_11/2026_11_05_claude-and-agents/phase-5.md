---
status: pending
---

# Instruction: HUD multi-LLM : cartes Codex et Claude avec leurs agents

## Architecture projection

> Tree of the final files. ✅ create · ✏️ modify · ❌ delete

```txt
.
├── apps/desktop/
│   ├── src/
│   │   ├── ✏️ App.tsx · App.test.tsx · App.module.css
│   │   └── features/
│   │       ├── usage/ ✏️ ProviderCard.tsx · ProviderCard.test.tsx · useUsage.ts · format.ts · format.test.ts
│   │       └── agents/ ✅ AgentList.tsx · AgentList.module.css · AgentList.test.tsx · useAgents.ts · useAgents.test.ts
│   └── src-tauri/
│       ├── ✏️ Cargo.toml · tauri.conf.json
│       └── src/ ✏️ lib.rs · usage_loop.rs
└── aidd_docs/memory/
    └── ✏️ architecture.md · codebase-map.md · design.md · project-brief.md
```

## Test Scope

| Scenario | Action | Observable outcome | Actor |
| --- | --- | --- | --- |
| Happy path | Usage et agents connus pour les 2 providers | 2 cartes (Codex, Claude), chacune avec son usage et sa liste d'agents | cli |
| Happy path | Usage Claude en tokens | Exemple : « 5h  1,2 M tokens  ↻ ~17:00 (estimé) » | cli |
| Edge case | Provider `offline` | La carte affiche un badge « offline », sans liste | cli |
| Edge case | Usage `unknown` | « unknown », jamais 0 ni vide | cli |
| Happy path | Sous Windows, avec des sessions Claude et Codex actives | Badges cohérents avec l'activité réelle | system |

## Wireframe

```txt
┌──────────────────────────────────────┐
│ (1) AgentHud                 (2) ●   │
├──────────────────────────────────────┤
│ (3) Codex               il y a 2 min │
│ 5h     [██░░░░░░░░]  12 %  ↻ 17:40   │
│ hebdo  [█████░░░░░]  51 %  ↻ lun.    │
│ (4) ● working   rollout-…a1a126      │
├──────────────────────────────────────┤
│ (3) Claude              il y a 10 s  │
│ 5h     1,2 M tokens   ↻ ~17:00 estimé│
│ (4) ● waiting   AgentHud             │
│     ● idle      PowerKitchen         │
└──────────────────────────────────────┘
```

1. Titre (existant, déplaçable en mode interactif).
2. Indicateur de mode overlay (existant).
3. Carte provider : nom, âge, puis une ligne par fenêtre d'usage. Codex en %, Claude en tokens estimés.
4. Liste des agents actifs : badge d'état avec libellé, et projet ou identifiant court. Badge `offline` si aucune session.

## Tasks to do

### `1)` Polling et câblage

> Alimenter le HUD pour tous les providers.

1. `usage_loop.rs` (ou un module `hud_loop.rs`) :
   - un seul tick de 3 s exécute `RefreshUsage` (Codex et Claude) et `TrackAgents` (Codex et Claude) ;
   - il émet les events `usage` et `agents` seulement quand ils changent ;
   - command `agents` pour la lecture au chargement.
2. Un seul `CachedLister` est partagé par toutes les sources.
3. `tauri.conf.json` : la hauteur de la fenêtre s'adapte au contenu (taille calculée côté UI puis `setSize`), ou passe à une hauteur fixe suffisante. Choisir la solution la plus simple qui ne coupe rien, et la documenter.

### `2)` UI

> Présenter tous les LLM de la même façon.

1. `ProviderCard` gère `usedTokens` (format « 1,2 M tokens ») et la mention « estimé » / « ~ ».
2. `AgentList` et `useAgents` suivent le même schéma que `useUsage` (command au chargement, puis event).
3. `App.tsx` affiche une carte par provider, dans l'ordre Codex, puis Claude.

### `3)` Mémoire

> Garder la documentation alignée.

1. `architecture.md` : port `AgentStateSource` et règle d'état (seuils de 5 et 60 min).
2. `codebase-map.md` : `features/agents` et `adapter-claude`.
3. `design.md` : badges d'agent et format des tokens.
4. `project-brief.md` : l'usage Claude est exprimé en tokens estimés.

## Test acceptance criteria

| Task | Acceptance criteria |
| --- | --- |
| 1 | `cargo xwin clippy` passe. Les DTO sont testés dans `application`. |
| 2 | Les tests Vitest couvrent 2 cartes, les tokens, `estimé`, `offline`, `unknown` et la liste d'agents. |
| 3 | La mémoire décrit les agents et l'usage Claude. |
