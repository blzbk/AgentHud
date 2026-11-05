---
status: done
---

# Instruction: Claude : endpoint OAuth en mode dégradé

## Architecture projection

> Tree of the final files. ✅ create · ✏️ modify · ❌ delete

```txt
.
├── ✏️ Cargo.toml · Cargo.lock
└── crates/adapter-claude/
    ├── ✏️ Cargo.toml
    ├── src/ ✏️ lib.rs · ✅ oauth_usage.rs · credentials.rs
    └── tests/
        ├── ✅ claude_oauth_usage.rs
        └── fixtures/ ✅ oauth-usage.json · oauth-usage-limits.json · oauth-usage-nulls.json
```

## Test Scope

| Scenario | Action | Observable outcome | Actor |
| --- | --- | --- | --- |
| Setup | Transport HTTP factice, credentials factices en dossier temporaire | Aucun accès réseau | cli |
| Happy path | Réponse 200 `five_hour` / `seven_day` | % exacts, resets ISO convertis, origine `OAuth` | cli |
| Edge case | Forme `limits[]`, champs `null` ou en trop | Repli champ par champ, `None` si inconnu | cli |
| Edge case | 401 / 403 | `FetchError::AuthExpired` | cli |
| Edge case | 429 avec `Retry-After` (secondes) | `RateLimited { retry_after }` | cli |
| Edge case | Credentials absents ou sans `accessToken` | `Unavailable`, aucune requête envoyée | cli |
| Edge case | Erreur réseau, 5xx, JSON invalide | `Unavailable` sans token dans le message | cli |

## Tasks to do

### `1)` Credentials

> Relire le token de Claude Code à chaque appel, sans le garder.

1. `credentials.rs` : `AccessToken` (newtype sans `Debug` révélateur) lu dans `<racine>/.credentials.json` (`claudeAiOauth.accessToken`), première racine qui en a un.

### `2)` Fetcher

> Interroger l'endpoint de façon défensive.

1. Trait `HttpTransport` (une requête GET → statut, `Retry-After`, corps) ; implémentation `ureq` : connexion 3 s, total 8 s, aucune redirection, corps limité à 64 Kio.
2. `ClaudeOAuthUsage` implémente `UsageFetcher` : headers `Authorization: Bearer`, `anthropic-beta: oauth-2025-04-20` ; statut → `FetchError`.
3. Parsing : `five_hour` / `seven_day` (`utilization`, `resets_at`), repli `limits[]` (`session`, `weekly_all`, `percent`).
4. Dépendance `ureq` dans le workspace ; `cargo xwin check -p adapter-claude --target x86_64-pc-windows-msvc` passe.

## Test acceptance criteria

| Task | Acceptance criteria |
| --- | --- |
| 1 | Tests : token lu depuis un fichier factice temporaire ; absent → `Unavailable` sans appel ; `{:?}` du token ne le montre pas. |
| 2 | Tests avec transport factice pour 200, 401, 403, 429, 500, JSON invalide, `limits[]`, `null` ; headers vérifiés ; `cargo xwin check` passe. |
