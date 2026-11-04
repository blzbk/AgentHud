---
status: pending
---

# Instruction: Adapter Codex sur fixtures

## Architecture projection

> Tree of the final files. ✅ create · ✏️ modify · ❌ delete

```txt
.
└── crates/adapter-codex/
    ├── ✏️ Cargo.toml
    ├── src/
    │   ├── ✏️ lib.rs
    │   └── ✅ token_count.rs
    └── tests/
        ├── ✅ codex_usage_source.rs
        └── fixtures/
            ├── ✅ current-resets-at.jsonl
            ├── ✅ legacy-resets-in-seconds.jsonl
            ├── ✅ null-rate-limits.jsonl
            └── ✅ no-token-count.jsonl
```

## Test Scope

| Scenario | Action | Observable outcome | Actor |
| --- | --- | --- | --- |
| Setup | Fixtures anonymisées tirées d'une vraie session (structure conservée, contenu des messages remplacé) | Fixtures prêtes, sans donnée perso ni token | cli |
| Happy path | `current-resets-at.jsonl` | 5 h à 1 %, hebdo à 51 %, `resets_at` repris tel quel | cli |
| Edge case | `legacy-resets-in-seconds.jsonl` | `resets_at` égal à l'horodatage de l'événement plus `resets_in_seconds` | cli |
| Edge case | `null-rate-limits.jsonl` : le dernier `token_count` a `rate_limits: null` | Le `token_count` non nul précédent est utilisé | cli |
| Edge case | `no-token-count.jsonl`, ou une ligne JSON invalide | `Ok(None)`, sans panique | cli |
| Edge case | Deux racines, la plus récente côté WSL | Le snapshot de la racine la plus récente est retenu | cli |

## Tasks to do

### `1)` Parsing défensif de `token_count`

> Extraire l'usage quelle que soit la version de Codex.

1. `token_count.rs` : structs `serde`, avec des champs optionnels et `#[serde(default)]`.
2. Une ligne non `event_msg/token_count` ou invalide est ignorée.
3. Une fenêtre est classée par `window_minutes` : environ 300 donne `FiveHours`, 10080 donne `Weekly`. Sinon, elle est classée par position (`primary`, puis `secondary`).
4. `resets_at` est lu en priorité, sinon `timestamp` plus `resets_in_seconds`.

### `2)` `CodexUsageSource`

> Implémenter le port `UsageSource` pour Codex.

1. Construit à partir des racines (`adapter-fs::home_roots(".codex", lister)`).
2. `latest()` : fichier le plus récent par racine. **Si le chemin et le mtime de ce fichier n'ont pas changé depuis le tick précédent, réutiliser le snapshot en cache sans relire le fichier** (polling sur mtime). Sinon, puis lecture de la fin, puis premier `token_count` exploitable en partant de la fin. Garder le snapshot le plus récent toutes racines confondues.
3. `observed_at` reprend l'horodatage de l'événement.

### `3)` Fixtures et tests

> Protéger l'adapter contre les changements de schéma.

1. Créer les 4 fixtures anonymisées. Aucun texte de prompt, aucun chemin perso, aucun identifiant de compte.
2. `tests/codex_usage_source.rs` couvre chaque scénario du Test Scope.

## Test acceptance criteria

| Task | Acceptance criteria |
| --- | --- |
| 1 | Les tests de parsing passent sur les 4 fixtures. Une ligne invalide est ignorée. |
| 2 | Le test à 2 racines retient la plus récente. `latest()` ne panique jamais. Un 2e appel sans changement de mtime ne relit pas le fichier, ce qui est vérifié par un compteur de lectures. |
| 3 | `grep` sur les fixtures ne trouve ni `/home/`, ni `Users\\`, ni e-mail, ni token. |
