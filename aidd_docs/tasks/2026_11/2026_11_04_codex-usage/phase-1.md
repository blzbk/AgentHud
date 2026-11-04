---
status: pending
---

# Instruction: Modèle d'usage et cas d'usage RefreshUsage

## Architecture projection

> Tree of the final files. ✅ create · ✏️ modify · ❌ delete

```txt
.
├── ✏️ Cargo.toml
└── crates/
    ├── domain/src/
    │   ├── ✏️ lib.rs
    │   └── ✅ usage.rs
    └── application/
        ├── ✏️ Cargo.toml
        └── src/
        ├── ✏️ lib.rs
        ├── ✅ ports.rs
        └── ✅ refresh_usage.rs
```

## Test Scope

| Scenario | Action | Observable outcome | Actor |
| --- | --- | --- | --- |
| Happy path | `UsageWindow` à 40 %, `resets_at` dans le futur, puis `used_at(now)` | 40 % | cli |
| Edge case | `resets_at` passé, puis `used_at(now)` | 0 % | cli |
| Edge case | `used_percent` hors de 0 à 100 | Valeur bornée à l'intervalle 0 à 100 | cli |
| Happy path | `RefreshUsage` avec une `UsageSource` factice qui renvoie un snapshot | Le snapshot du provider est retourné | cli |
| Edge case | La source renvoie `None` ou une erreur | Un usage `unknown` est retourné pour ce provider, sans panique | cli |

## Tasks to do

### `1)` Modèle d'usage (`domain`)

> Représenter l'usage d'un provider sans I/O.

1. `Provider` (`Codex`, `Claude`) avec `label()`.
2. `WindowKind` (`FiveHours`, `Weekly`).
3. `UsageWindow` : `kind`, `used_percent` (borné de 0 à 100), `resets_at` (optionnel). Méthode `used_at(now)`, qui renvoie 0 une fois `resets_at` passé.
4. `UsageSnapshot` : `provider`, `five_hours` et `weekly` (chacun optionnel), `observed_at`.
5. Temps représenté en secondes epoch (`i64`), sans dépendance externe.

### `2)` Port et cas d'usage (`application`)

> Obtenir l'usage d'un provider sans connaître sa source.

0. Déclarer dans `[workspace.dependencies]` du `Cargo.toml` racine : `thiserror`, `serde`, `serde_json`, `log` et `tokio`. Les crates les référencent avec `.workspace = true`.
1. `ports.rs` : trait `UsageSource { fn provider(&self) -> Provider; fn latest(&self) -> Result<Option<UsageSnapshot>, SourceError>; }`, avec une erreur typée (`thiserror`).
2. `refresh_usage.rs` : `RefreshUsage` prend des `Box<dyn UsageSource>` et renvoie un `ProviderUsage` par provider : `Known(snapshot)` ou `Unknown`.
3. Les tests utilisent des sources factices.

## Test acceptance criteria

| Task | Acceptance criteria |
| --- | --- |
| 1 | Les tests de `domain` couvrent le reset passé (0 %), le bornage et le cas nominal. |
| 2 | Les tests de `application` couvrent un snapshot connu, `None` et une erreur, chacun donnant le bon `ProviderUsage`. Le test `dependency_direction` passe. |
