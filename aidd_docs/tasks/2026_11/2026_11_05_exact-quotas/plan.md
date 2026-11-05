---
objective: "Le HUD affiche les % exacts 5 h et hebdo de Claude (statusline, puis OAuth en mode dégradé optionnel) et de Codex (app-server en plus des sessions JSONL), avec la source de chaque donnée, un polling adaptatif testé sur l'hôte et des options persistées dans le tray."
status: pending
---

# Plan: Quotas exacts Claude et Codex

Source: ADR 0002 (`aidd_docs/adr/0002-claude-oauth-usage-degraded-mode.md`), spike #3 (`aidd_docs/tasks/2026_11/2026_11_05_usage-spike/`), demande de l'utilisateur du 2026-11-05 (feature 5 « quotas exacts »).

## Phases

| #   | Phase | File | Depends on |
| --- | ----- | ---- | ---------- |
| 1   | Modèle : origine des données, quota épuisé, planificateur de polling et sources interrogées | [`phase-1.md`](./phase-1.md) | none |
| 2   | Claude : script de statusline et lecture du fichier de quotas | [`phase-2.md`](./phase-2.md) | 1 |
| 3   | Claude : endpoint OAuth en mode dégradé | [`phase-3.md`](./phase-3.md) | 1 |
| 4   | Codex : client `codex app-server` (`account/rateLimits/read`) | [`phase-4.md`](./phase-4.md) | 1 |
| 5   | Desktop : options persistées, tray et boucle de polling distante | [`phase-5.md`](./phase-5.md) | 2, 3, 4 |
| 6   | UI : jauges Claude, source des données et auth expirée | [`phase-6.md`](./phase-6.md) | 5 |
| 7   | Mémoire et README | [`phase-7.md`](./phase-7.md) | 5, 6 |
| 8   | Exe et tests manuels Windows | [`phase-8.md`](./phase-8.md) | 7 |

## External sources

| URL | What it settled |
| --- | --- |
| https://code.claude.com/docs/en/statusline | `rate_limits.five_hour` / `seven_day` : `used_percentage` (0 à 100) et `resets_at` (secondes epoch). Présent seulement pour Pro, Max, Ultra, après la 1re réponse ; chaque fenêtre peut manquer et disparaît une fois `resets_at` passé. Config `statusLine: { type: "command", command }`. Windows : Git Bash ou PowerShell, chemins en `/`, `powershell -NoProfile -File C:/…/x.ps1`. Debounce 300 ms. |
| https://learn.chatgpt.com/docs/app-server | stdio = JSONL, JSON-RPC 2.0 sans champ `jsonrpc`. Un `initialize` (`clientInfo` : `name`, `title`, `version`) par connexion, puis la notification `initialized`. `account/rateLimits/read` : `usedPercent`, `windowDurationMins`, `resetsAt` (secondes). Requêtes expirées après ~10 s. |
| https://github.com/openai/codex/blob/main/codex-rs/app-server-protocol/src/protocol/v2/account.rs | `GetAccountRateLimitsResponse { rateLimits: RateLimitSnapshot, rateLimitsByLimitId?, … }`, `RateLimitSnapshot { primary?, secondary?, planType?, rateLimitReachedType?, … }`, `RateLimitWindow { usedPercent: i32, windowDurationMins?: i64, resetsAt?: i64 }`. |
| https://github.com/openai/codex/blob/main/codex-rs/app-server/src/request_processors/account_processor.rs | Sans auth : erreur `-32600` « codex account authentication required to read rate limits » ; échec backend : `-32603` « failed to fetch codex rate limits: … ». |
| https://github.com/sirmalloc/ccstatusline/blob/66e9a08f86b1b4a8c7684cb35401e3ad3e3e4e8d/src/utils/usage-fetch.ts | Token : `claudeAiOauth.accessToken` de `<config Claude>/.credentials.json`. `GET https://api.anthropic.com/api/oauth/usage`, `Authorization: Bearer`, `anthropic-beta: oauth-2025-04-20`. Réponse `five_hour` / `seven_day` (`utilization`, `resets_at` ISO) ou `limits[]` (`kind`, `percent`). 429 avec `Retry-After`. |
| https://docs.rs/ureq/3.4.0/ureq/ | Client HTTP bloquant, rustls + ring par défaut, timeouts par requête, `http_status_as_error(false)`, `max_redirects(0)`, limite de taille du corps. |

## Impact

| Category | Verdict and evidence | Handled in |
| --- | --- | --- |
| runtime | **Observé (doc) :** champs statusline et protocole app-server ci-dessus. **Non vérifié :** forme réelle de la réponse OAuth (aucun appel autorisé) ; parsing défensif et fixtures synthétiques. | phases 2, 3, 4 |
| runtime | **Non vérifié :** `wsl.exe -d <distro> -- codex app-server` trouve `codex` dans le `PATH`. Ici `codex` est dans `~/.local/bin`, ajouté par `~/.profile` : le lancement passe par un shell de connexion (`sh -lc 'exec codex app-server'`). | phase 4, test manuel phase 8 |
| runtime | **Non vérifié :** `ureq` (rustls + ring) cross-compile avec `cargo-xwin`. Vérifié dès la phase 3 par `cargo xwin check`. | phase 3 |
| architecture | Nouveau port `UsageFetcher` et cas d'usage `PolledUsage` dans `application`, planificateur pur `PollScheduler`, combinateur `FreshestUsage`. Un seul appel réseau, dans `adapter-claude`. Un processus de plus (`codex app-server`) dans `adapter-codex`. Sens hexagonal inchangé, vérifié par `architecture-tests`. | phases 1, 3, 4 |
| regression | Moyen. `UsageSnapshot` gagne un champ `origin` (tous les constructeurs), `UsageReport` gagne `source` et `authExpired`, `usage_loop.rs`, `tray.rs` et `ProviderCard` changent. Couverts par les tests `domain`, `application`, adapters et Vitest. | phases 1, 5, 6 |
| security | Le token OAuth est relu à chaque appel, jamais stocké, ni journalisé, ni envoyé à JS. Aucune lecture de credentials dans les tests. | phase 3 |
| documentation | `architecture.md` (OAuth, réseau, processus), `desktop.md`, `design.md`, `codebase-map.md`, `project-brief.md`, `README.md` (installation de la statusline, risque OAuth). | phase 7 |

## Rules

| Rule | Why it applies |
| --- | --- |
| ADR 0002 | Contrat de la feature : priorités des sources, option OAuth désactivée par défaut, 13 min minimum, 1 min minimum pour Codex. |
| `architecture.md` : parsing défensif, `unknown` en cas de doute | Fichier statusline, réponse OAuth, réponse app-server. |
| `architecture.md` : une distro arrêtée n'est jamais touchée | Lancement WSL de `codex app-server` et lecture des fichiers statusline via `CachedLister`. |
| `testing.md` : fixtures anonymisées, logique testable sur l'hôte | Planificateur pur dans `application`, fixtures synthétiques. |
| `coding-assertions.md` | Gates avant chaque commit et avant la fin. |
| ADR 0001 | Aucun outil Windows lancé par l'agent ; l'humain teste l'exe. |
| `vcs.md` | Un commit par phase validée. |

## Decisions

| Decision | Why |
| --- | --- |
| Client HTTP `ureq` 3 (bloquant, rustls + ring) plutôt que `reqwest` | Le polling tourne déjà dans `spawn_blocking`. `ureq` n'a pas besoin de runtime async, et rustls/ring évite OpenSSL. `reqwest` 0.13 prend `aws-lc-rs` par défaut, qui demande cmake et nasm pour cross-compiler. |
| Planificateur `PollScheduler` pur dans `application`, paramétré par une `PollPolicy` (Codex : plancher 1 min ; OAuth : plancher 13 min) | Une seule logique testée pour les 6 règles, réutilisée par les 2 sondes. |
| Priorité d'affichage : une donnée exacte (%) l'emporte sur une estimation, puis la plus récente gagne (`FreshestUsage`) | Codex : app-server ou session, la plus fraîche. Claude : statusline ou OAuth, puis estimation en tokens. |
| OAuth Claude seulement si l'option est cochée et que la statusline n'a rien écrit depuis 15 min (`STATUSLINE_FRESH_FOR`) | Traduction de « absente ou périmée » de l'ADR. |
| Option Codex app-server activée par défaut ; option OAuth désactivée par défaut ; fichier `<app_config_dir>/options.json` | App-server est officiel (ADR 0002) ; OAuth est non autorisé. |
| `codex app-server` lancé à chaque sondage puis arrêté, jamais gardé en vie | Pas d'état périmé ni de processus orphelin ; au plus 1 lancement par minute. |
