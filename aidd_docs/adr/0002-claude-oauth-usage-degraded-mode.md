# ADR 0002 — Quotas exacts : statusline Claude, OAuth Claude en mode dégradé, app-server Codex

- Statut : accepté (2026-11-05), décision de l'utilisateur prise en connaissance du risque
- Remplace en partie : la règle « Aucun appel à l'endpoint OAuth d'Anthropic » de `aidd_docs/memory/architecture.md`
- Matière : spike #3, `aidd_docs/tasks/2026_11/2026_11_05_usage-spike/`

## Contexte

- Le HUD n'affiche pour Claude que des tokens estimés, sans %. L'utilisateur veut les % exacts 5 h et hebdo pour Claude et Codex.
- La doc officielle d'Anthropic (https://code.claude.com/docs/en/legal-and-compliance) dit deux choses. Le token OAuth des plans Free, Pro, Max et Ultra est destiné à « l'usage ordinaire de Claude Code et des autres applications natives d'Anthropic ». Et « developers may not collect, store, or intermediate Claude.ai credentials or session tokens ». Anthropic peut bloquer sans préavis.
- Claude Code fournit officiellement `rate_limits.five_hour` et `rate_limits.seven_day` (%, `resets_at`) au script de sa statusline (https://code.claude.com/docs/en/statusline).
- OpenAI documente `codex app-server` et sa méthode `account/rateLimits/read`. La doc OpenAI permet aux applications locales et open source de s'appuyer sur l'auth de Codex via app-server (voir les sources du spike).

## Décision

1. **Claude, source officielle en priorité : statusline.** Un script de statusline fourni par AgentHud (Windows natif et WSL) écrit les `rate_limits` reçus de Claude Code dans un fichier JSON lu par AgentHud. L'utilisateur l'installe lui-même dans les réglages de Claude Code. AgentHud ne modifie jamais ces réglages.
2. **Claude, mode dégradé : endpoint OAuth `/api/oauth/usage`.**
   - Il est derrière une option **désactivée par défaut** et signalée comme dangereuse dans l'IHM.
   - Il n'est utilisé que si l'option est cochée **et** que la statusline ne fournit pas de donnée récente.
   - Il est interrogé au plus toutes les **13 min**, avec un recul plus long sur erreur, sur 429 ou sur token expiré.
   - Le token n'est jamais copié ni stocké par AgentHud : il est relu dans le fichier de Claude Code à chaque appel, et n'apparaît jamais dans les logs ni côté JS.
3. **Codex : `codex app-server` (`account/rateLimits/read`).**
   - C'est le client officiel de Codex, et le token n'est pas extrait.
   - Option activable dans l'IHM.
   - Polling adaptatif :
     - jamais plus d'une fois par minute ;
     - quota épuisé : polling long (5 à 10 min) et relance une fois `resets_at` passé ;
     - quota épuisé et un agent qui passe d'inactif à actif : relance plus rapide ;
     - auth expirée : fréquence réduite.
   - Les sessions JSONL restent la source passive par défaut.

## Conséquences

- **Le mode dégradé OAuth peut entraîner un blocage par Anthropic.** L'utilisateur en accepte le risque pour son propre compte. Le README et l'IHM doivent le dire explicitement avant toute diffusion open source.
- Exception à la règle « seul processus externe : `wsl.exe` » : l'app peut lancer `codex app-server` (Codex installé), et `wsl.exe -d <distro> -- codex app-server` uniquement pour une distro déjà démarrée.
- Exception à la règle « aucun accès réseau » : un seul appel HTTPS, vers `api.anthropic.com`, et uniquement en mode dégradé activé.
