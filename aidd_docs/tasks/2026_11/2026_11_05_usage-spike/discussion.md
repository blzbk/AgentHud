# Spike #3 — éléments de discussion et recherche

Date : 2026-11-05. Issue de référence : https://github.com/blzbk/AgentHud/issues/3.

Ce document conserve les éléments substantiels de l'échange, les corrections et le prototype proposé ; ce n'est pas une transcription verbatim ni un verdict de conformité. L'issue #3 reste l'autorité du backlog. Aucun changement de son périmètre ou de son statut n'est implicite : elle porte initialement sur Codex/OpenAI, tandis que la discussion s'est étendue à Claude.

## Besoin et contraintes exprimés

- Travail de recherche indépendant de la fonctionnalité Claude et agents déjà en cours.
- Connaître les quotas 5 h et hebdomadaires, les usages et les états des sessions.
- Cas Claude : Claude Code depuis WSL et application Claude Desktop depuis Windows.
- Couverture Desktop souhaitée : le maximum possible (Code, Chat, Cowork), avec Code comme minimum acceptable.
- Priorité initialement exprimée : ne pas enfreindre les conditions de Codex ou Claude ; étudier documentation et bonnes pratiques avant un prototype connecté.
- Après examen de ccstatusline, demande explicite de reproduire uniquement la récupération des quotas par `/api/oauth/usage`, avec un polling raisonnable.
- Cette demande technique n'établit pas une autorisation d'Anthropic et ne résout pas la tension avec l'exigence de conformité. Aucun appel authentifié n'a été effectué.
- Dernière demande : enregistrer l'ensemble des éléments de cet échange comme matière du spike ; pas implémenter l'adapter.

## Codex/OpenAI : premières pistes documentaires

Méthode : lecture de l'issue, de la mémoire pertinente et des pages officielles. Aucun essai de requête authentifiée.

| Piste | Éléments recueillis | Limites |
| --- | --- | --- |
| `codex app-server` | `account/rateLimits/read` expose les fenêtres, pourcentages et resets ; `account/usage/read` documente des résumés de tokens et des buckets journaliers | Disponibilité sur la version installée et le compte non testée ; coûts monétaires non établis par ces réponses |
| Authentification app-server | La documentation consultée permet la continuation des applications locales/open source existantes, recommande Sign in with ChatGPT et exclut cette authentification pour les services commerciaux/hébergés | Ne pas transformer cette mention en autorisation générale pour tout nouveau produit ou toute réutilisation de token |
| API d'organisation Usage/Costs | Rapports d'usage et de coûts de l'organisation API | Aucune preuve recueillie qu'elle expose les quotas 5 h/hebdomadaires de l'abonnement ChatGPT |
| Lecture directe de `~/.codex/auth.json` | Autorisation de réutilisation par AgentHud non établie | Ne pas confondre faisabilité technique et permission |

Sources :

- https://learn.chatgpt.com/docs/app-server
- https://developers.openai.com/siwc/token-sharing-open-source
- https://developers.openai.com/siwc/token-sharing-open-source/codex-app-server
- https://developers.openai.com/api/reference/resources/admin/subresources/organization/subresources/usage/methods/costs
- https://developers.openai.com/api/docs/guides/chatgpt-api-credits
- https://openai.com/policies/eu-terms-of-use/
- https://openai.com/policies/service-terms/

Observation locale : `codex --version` a retourné `codex-cli 0.166.0`. Aucun serveur lancé. Recommandation de méthode : ajouter « non établi » aux verdicts possibles, pour ne pas forcer une conclusion sans preuve.

## Claude : interfaces documentées et couverture

| Interface | Utilité établie par la documentation consultée | Ce qui reste à établir |
| --- | --- | --- |
| Statusline Claude Code | Reçoit un JSON sur stdin ; champs `rate_limits.five_hour` et `rate_limits.seven_day`, pourcentages et resets ; compteurs de contexte/tokens | Disponibilité selon version, plan et première réponse ; exécution dans Desktop non établie |
| Hooks Claude Code | Événements `SessionStart`, `UserPromptSubmit`, `PermissionRequest`, `Stop`, `StopFailure`, `SessionEnd`, notifications, sous-agents | Reconstruction fiable de l'état, interruptions, crashs et disparition des processus |
| Hooks dans Desktop Code | La documentation dit que les hooks définis dans les settings s'appliquent aussi à Desktop Code | Vérification sur la version Windows et le lieu réel d'exécution de chaque session |
| OpenTelemetry | Export officiellement documenté des tokens et des coûts estimés ; nécessite une configuration | Ne prouve pas un accès aux quotas de l'abonnement ni une couverture de Chat/Cowork |
| Claude Code Analytics API | Rapports journaliers agrégés pour l'organisation Console | Ne couvre pas les abonnements personnels ; pas un état temps réel |
| Enterprise Analytics | API officielle distincte selon offre et droits administrateur | Pas une solution établie pour ce HUD personnel |
| MCP | Claude se connecte à des outils et données externes | Aucun MCP officiel d'observation de toutes les sessions personnelles et de leurs quotas identifié |
| Agent SDK | `listSessions`, `getSessionInfo`, `getSessionMessages` pour les sessions enregistrées ; tokens/coûts estimés et événements de limitation pour ses propres exécutions | Ne fournit pas une observation temps réel des sessions WSL/Desktop déjà ouvertes ni une permission générale d'utiliser l'abonnement |

Précisions importantes :

- La statusline n'est pas un hook de cycle de vie ; c'est une commande de rendu qui reçoit un JSON. Proposition : copier seulement les métriques nécessaires dans un fichier local lu par AgentHud.
- Les hooks peuvent servir de relais local des états sans récupérer de token ; ils impliquent une configuration, contrairement à la lecture passive de la v1.
- Les quotas de l'abonnement sont partagés entre les surfaces Claude. Une mesure obtenue depuis une session Code peut renseigner le budget commun, mais doit afficher sa date ; elle ne mesure pas séparément l'activité de chaque surface.
- Ne pas confondre le pourcentage de contexte d'une conversation avec le quota de l'abonnement.
- Les transcripts et dates de modification ne prouvent pas à eux seuls qu'une session est ouverte, travaille ou attend une permission.
- Chat et Cowork : aucune interface personnelle documentée de suivi complet n'a été établie pendant cet échange.
- Correction apportée pendant l'échange : la proposition statusline n'a jamais été validée pour les quotas Desktop. Avec Desktop seul, l'export automatique des quotas reste non établi.

Sources :

- https://code.claude.com/docs/en/statusline
- https://code.claude.com/docs/en/hooks
- https://code.claude.com/docs/en/desktop
- https://code.claude.com/docs/en/monitoring-usage
- https://platform.claude.com/docs/en/manage-claude/claude-code-analytics-api
- https://platform.claude.com/docs/en/manage-claude/analytics-api
- https://support.claude.com/en/articles/11647753-how-do-usage-and-length-limits-work
- https://support.claude.com/en/articles/11175166-get-started-with-custom-connectors-using-remote-mcp
- https://code.claude.com/docs/en/agent-sdk/overview
- https://code.claude.com/docs/en/agent-sdk/sessions
- https://code.claude.com/docs/en/agent-sdk/cost-tracking
- https://code.claude.com/docs/en/agent-sdk/typescript

## Conditions d'utilisation Claude : preuves et incertitudes

Sources officielles consultées :

- https://www.anthropic.com/legal/consumer-terms — section 3 : accès automatisé interdit sauf clé API Anthropic ou permission explicite.
- https://code.claude.com/docs/en/legal-and-compliance#authentication-and-credential-use — restrictions sur la collecte, le stockage et l'intermédiation de credentials/tokens Claude.ai par les développeurs ; authentification OAuth destinée aux usages natifs prévus.
- https://code.claude.com/docs/en/agent-sdk/overview#get-started — sauf approbation préalable, pas de connexion Claude.ai ou de limites d'abonnement proposées par les développeurs tiers dans leurs produits via le SDK ; orientation vers une clé API.

Conclusion documentaire exprimée : un outil réutilisant le token Claude Code pour appeler l'endpoint peut se voir reprocher cet usage contractuellement. Aucune exception explicite pour la simple consultation personnelle des quotas n'a été trouvée. Ce n'est ni une décision juridique définitive ni une prédiction de sanction. Le faible débit, la présence de l'endpoint et son utilisation par des outils existants ne valent pas permission.

Question précise proposée à Anthropic, non envoyée : un outil local peut-il réutiliser le token de son propriétaire uniquement pour lire `/api/oauth/usage`, sans requête au modèle ?

Autre correction : « lecture locale passive » ne constitue pas en soi une preuve d'autorisation documentée pour tous les formats internes.

## Vérification du code de ccstatusline

Dépôt étudié : `sirmalloc/ccstatusline`, commit `66e9a08f86b1b4a8c7684cb35401e3ad3e3e4e8d`. Clone de recherche dans `/tmp/agenthud-ccstatusline-review` ; pas installé ni exécuté.

- [usage-prefetch.ts](https://github.com/sirmalloc/ccstatusline/blob/66e9a08f86b1b4a8c7684cb35401e3ad3e3e4e8d/src/utils/usage-prefetch.ts) : lit d'abord `rate_limits` depuis stdin. Si les widgets configurés ont les données nécessaires, pas de fetch d'usage. Champs manquants et certaines options peuvent déclencher un fetch ; donc on ne peut pas garantir l'absence d'OAuth simplement en choisissant les widgets 5 h/weekly.
- [usage-fetch.ts](https://github.com/sirmalloc/ccstatusline/blob/66e9a08f86b1b4a8c7684cb35401e3ad3e3e4e8d/src/utils/usage-fetch.ts) : lecture des credentials, requêtes et parsing.
- Lignes 237–246 : extrait `claudeAiOauth.accessToken`, ainsi que le refresh token utilisé notamment pour identifier le cache.
- Lignes 659–663 : lit `.credentials.json` dans le répertoire de configuration Claude. Sous WSL/Linux et Windows, c'est la source choisie ; `CLAUDE_CONFIG_DIR` peut remplacer le défaut `~/.claude`.
- Lignes 687–724 : sur macOS, cherche d'abord dans le Keychain (`Claude Code-credentials`, variantes de profils), puis se replie sur le fichier.
- Lignes 913–940 : définit les endpoints et les headers ; le token existant est réutilisé, sans nouveau login OAuth réalisé par ce code.
- Cache, verrouillage des requêtes et backoff limitent les appels concurrents et les répétitions ; cela ne tranche pas les permissions contractuelles.

Requête des quotas :

```http
GET https://api.anthropic.com/api/oauth/usage
Authorization: Bearer <claudeAiOauth.accessToken>
anthropic-beta: oauth-2025-04-20
```

Le widget email utilise aussi `GET https://api.anthropic.com/api/oauth/profile`, avec les mêmes headers. Il n'est pas nécessaire au besoin 5 h/weekly et est exclu de la proposition.

Il s'agit d'appels à `api.anthropic.com`, pas directement au domaine `claude.ai`.

### Structure de réponse prise en charge

Exemple fictif tiré du schéma du parseur ; pas une réponse réelle ni un contrat officiel :

```json
{
  "five_hour": { "utilization": 23.5, "resets_at": "2026-11-05T18:00:00Z" },
  "seven_day": { "utilization": 41.2, "resets_at": "2026-11-09T12:00:00Z" },
  "seven_day_sonnet": { "utilization": 15, "resets_at": "2026-11-09T12:00:00Z" },
  "seven_day_opus": null,
  "extra_usage": {
    "is_enabled": false,
    "monthly_limit": null,
    "used_credits": null,
    "utilization": null,
    "currency": null
  }
}
```

Autre forme reconnue par ccstatusline :

```json
{
  "limits": [
    { "kind": "session", "percent": 23.5, "resets_at": "2026-11-05T18:00:00Z" },
    { "kind": "weekly_all", "percent": 41.2, "resets_at": "2026-11-09T12:00:00Z" }
  ]
}
```

Champs absents/null possibles. `utilization`/`percent` représentent les pourcentages ; `resets_at` une date. Les champs plats ont priorité sur le repli `limits[]` pour 5 h et weekly, champ par champ. Les quotas concernent le compte, pas l'état d'une conversation.

## Proposition technique demandée

Demande de l'utilisateur : cloner uniquement la récupération des quotas 5 h et weekly par cet endpoint ; code rapide, simple, efficace et propre ; intensité du polling raisonnable.

Proposition, pas une fréquence approuvée par Anthropic :

- Un collecteur partagé par compte, pas un par session.
- Succès : attendre 5 minutes après la fin de l'appel.
- HTTP 429 : respecter `Retry-After` (secondes ou date HTTP), avec un minimum de 15 minutes ; ne pas plafonner une attente serveur plus longue.
- Erreur réseau, serveur ou parsing : attendre 15 minutes.
- HTTP 401/403 : suspendre les appels jusqu'à nouvelle authentification.
- Garder la dernière valeur et sa date ; erreur ou champ absent ne signifie jamais 0 %.
- Réutiliser un client HTTP ; délai de connexion 3 s, délai total 8 s ; pas de redirection ni retry automatique.
- Aucun chevauchement, aucune rafale de rattrapage après veille.

Le prototype exact proposé est conservé dans [quota-client-proposal.rs](./quota-client-proposal.rs). Dépendances proposées : `reqwest = { version = "0.13", features = ["json"] }`, `serde = { version = "1", features = ["derive"] }`, `httpdate = "1"`.

Références des signatures HTTP vérifiées : https://docs.rs/reqwest/latest/reqwest/struct.ClientBuilder.html et https://docs.rs/reqwest/latest/reqwest/struct.Response.html.

### Limites du prototype

- Non compilé, non testé ; aucun appel authentifié réalisé.
- Cœur HTTP seulement : pas de lecture de credentials, de scheduler, de cache ou de gestion de comptes implémentés.
- Gère uniquement les champs plats ; `limits[]` doit être ajouté avant intégration.
- Un JSON valide sans fenêtre peut être accepté : ne pas en déduire un quota nul ou une mesure exploitable.
- Pas encore de validation des pourcentages, des dates, de taille de réponse ni de politique détaillée par code HTTP.
- La suspension 401/403 et la conservation des valeurs sont des responsabilités du futur appelant.
- Collecteur Windows/WSL, sélection des credentials et invalidation du cache au changement de compte restent à concevoir.

## Suite à décider

1. Clarifier la permission Anthropic si la conformité explicite reste une condition impérative.
2. Définir le rattachement du volet Claude, sans réécrire silencieusement le spike Codex #3.
3. Si implémentation autorisée : adapter le prototype à l'architecture Rust, normaliser les deux formats et tester parsing, cache et backoff avec des réponses synthétiques.
4. Poursuivre séparément l'étude des états Desktop Code par hooks ; ne pas assimiler quota global, tokens de session et état temps réel.

Aucun verdict final de conformité ou de faisabilité complète ; aucune modification de l'application dans cette recherche.
