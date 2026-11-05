# AgentHud

Overlay Windows, toujours au premier plan et click-through, qui affiche l'usage des quotas et l'état des agents de Claude Code et de Codex.

## Quotas exacts de Claude : statusline

Claude Code donne les quotas 5 h et hebdo (`rate_limits`) à sa statusline. Le script fourni les copie dans `~/.claude/agenthud/rate-limits.json`, que AgentHud lit. Il n'envoie rien sur le réseau. AgentHud ne modifie jamais les réglages de Claude Code : ajoutez vous-même une ligne dans `settings.json`.

Disponible pour les plans Pro, Max et Ultra, après la première réponse d'une session.

**WSL ou Linux** (`~/.claude/settings.json`) :

```json
"statusLine": { "type": "command", "command": "sh /chemin/vers/AgentHud/scripts/statusline/agenthud-statusline.sh" }
```

**Windows natif** (`%USERPROFILE%\.claude\settings.json`, chemins en `/`) :

```json
"statusLine": { "type": "command", "command": "powershell -NoProfile -ExecutionPolicy Bypass -File C:/chemin/vers/AgentHud/scripts/statusline/agenthud-statusline.ps1" }
```

Pour garder une statusline existante, passez sa commande au script. Elle reçoit la même entrée, et sa sortie reste affichée :

- sh : `sh …/agenthud-statusline.sh '~/.claude/statusline.sh'`
- PowerShell : `… -File …/agenthud-statusline.ps1 -Chain \"powershell -NoProfile -File C:/Users/moi/.claude/statusline.ps1\"` (lancée via `cmd.exe`)

Le script sh utilise `jq` s'il est installé ; sinon il garde l'entrée entière dans le fichier.

## Quotas Codex : app-server

Avec la case « Quotas Codex via app-server » du tray (cochée par défaut), AgentHud lance `codex app-server`, le client officiel de Codex, et lit `account/rateLimits/read`, au plus une fois par minute. Il cherche `codex` dans le `PATH` Windows, puis dans chaque distro WSL **déjà démarrée** (shell de connexion). Sans réponse, les fichiers de session restent la source.

## ⚠ Mode dégradé : quotas Claude via OAuth

La case « ⚠ Quotas Claude via OAuth (non autorisé par Anthropic) » du tray est **décochée par défaut**.

- Cochée, AgentHud réutilise le token de connexion de Claude Code (`~/.claude/.credentials.json`) pour appeler `https://api.anthropic.com/api/oauth/usage`. **Anthropic n'autorise pas cet usage par un outil tiers et peut bloquer votre compte sans préavis.** Vous en acceptez le risque pour votre propre compte.
- L'appel n'a lieu que si la statusline n'a rien écrit depuis 15 min, au plus toutes les 13 min, avec un recul plus long en cas d'erreur, de 429 (`Retry-After` respecté) ou de token expiré (60 min, « auth expirée » affiché).
- Le token est relu à chaque appel, jamais stocké, journalisé ni transmis à l'interface.
- La carte Claude affiche la source : `statusline`, `OAuth ⚠` ou `estimation`.

Décision et justification : `aidd_docs/adr/0002-claude-oauth-usage-degraded-mode.md`.

## Options

Enregistrées dans `options.json`, dans le dossier de configuration de l'app (`%APPDATA%\dev.blzbk.agenthud\`).
