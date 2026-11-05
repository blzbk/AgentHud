---
status: pending
---

# Instruction: UI : jauges Claude, source des données et auth expirée

## Architecture projection

> Tree of the final files. ✅ create · ✏️ modify · ❌ delete

```txt
.
└── apps/desktop/
    ├── ✏️ src-tauri/tauri.conf.json
    └── src/features/usage/ ✏️ ProviderCard.tsx · ProviderCard.module.css · ProviderCard.test.tsx · useUsage.ts · useUsage.test.ts
```

## Test Scope

| Scenario | Action | Observable outcome | Actor |
| --- | --- | --- | --- |
| Happy path | Claude avec source `statusline` | Jauges 5 h et hebdo en %, mention « statusline » | cli |
| Happy path | Claude avec source `oauth` | Jauges, mention « OAuth ⚠ » | cli |
| Edge case | Claude avec source `estimate` | Tokens estimés comme avant, mention « estimation », pas de ligne hebdo | cli |
| Edge case | `authExpired` | Mention « auth expirée » lisible | cli |
| Happy path | Codex avec source `app-server` ou `sessions` | Mention de la source | cli |

## Wireframe

```txt
┌──────────────────────────────────────┐
│ (1) AgentHud                 ● mode  │
├──────────────────────────────────────┤
│ (2) Codex      app-server  il y a 1 m│
│ 5h     [██░░░░░░░░]  12 %  ↻ 17:40   │
│ hebdo  [█████░░░░░]  51 %  ↻ lun.    │
│ (4) ● working   AgentHud             │
├──────────────────────────────────────┤
│ (2) Claude     statusline  il y a 10s│
│ 5h     [███░░░░░░░]  30 %  ↻ 18:00   │
│ hebdo  [██████░░░░]  60 %  ↻ jeu.    │
│ (3) auth expirée                     │
│ (4) ● waiting   AgentHud             │
└──────────────────────────────────────┘
```

1. Titre et mode (existants).
2. En-tête de carte : nom, source de la donnée (`statusline`, `OAuth ⚠`, `estimation`, `app-server`, `session`), âge.
3. Avertissement d'auth expirée, seulement quand une sonde la signale.
4. Agents (existants).

## Tasks to do

### `1)` Types et carte

> Afficher la source et des jauges dès qu'un % existe.

1. `useUsage.ts` : `source?` et `authExpired?` en miroir de `UsageReport`.
2. `ProviderCard` : fenêtres affichées selon la donnée (Claude : 5 h et hebdo si la 5 h est en %, sinon 5 h en tokens) ; libellé de source ; ligne « auth expirée ».
3. Hauteur de la fenêtre ajustée si le contenu maximal dépasse 440 px (2 cartes, 2 fenêtres, 4 agents + `+N`, source, auth).

## Test acceptance criteria

| Task | Acceptance criteria |
| --- | --- |
| 1 | Vitest : jauges Claude en `statusline` et `oauth`, tokens en `estimate`, libellés de source, « auth expirée », `unknown` jamais 0 %. `pnpm typecheck`, `pnpm lint`, `pnpm test` passent. |
