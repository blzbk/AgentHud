# Desktop

## Framework

- Tauri, avec l'UI dans la WebView2 de Windows. Le process principal est en Rust (`apps/desktop/src-tauri`).
- Fenêtre overlay transparente, `alwaysOnTop`, sans décoration, de taille fixe 320 × 440 : le HUD prend la hauteur de son contenu et le reste de la fenêtre reste transparent. Le plafond de 4 agents par provider garantit que rien n'est coupé. Elle est click-through via `set_ignore_cursor_events`, et un raccourci clavier la rend interactive.
- En mode interactif, l'en-tête porte `data-tauri-drag-region="deep"` : on déplace l'overlay par son titre. Cela demande la permission `core:window:allow-start-dragging`.
- La position est sauvegardée dans `<app_config_dir>/window.json` (500 ms après le dernier déplacement). Au lancement, elle est ramenée sur un écran existant (`overlay::fit_on_screens`). Un fichier absent ou illisible laisse la position par défaut.
- Icône dans la zone de notification (`tray.rs`), menu « Mode interactif » / « Mode click-through », « Masquer » / « Afficher », les 2 options de quotas et « Quitter ». Les libellés suivent l'état, que le changement vienne du menu ou du raccourci.
- Une seule instance. Un 2e lancement réaffiche l'overlay masqué.

## Native access

- Lecture seule des dossiers `~/.claude` et `~/.codex`, côté Windows natif ou WSL (`\\wsl.localhost\...`, `\\wsl$\...`).
- Tout accès aux fichiers passe par Rust, jamais par les plugins fs côté JS.
- Réseau : un seul appel HTTPS, vers `api.anthropic.com/api/oauth/usage`, seulement si l'option OAuth est cochée (ADR 0002). Aucun autre accès réseau.
- Lecture du token de Claude Code (`<.claude>/.credentials.json`) seulement pour cet appel, à chaque appel, jamais stocké.
- L'heure du dernier appel OAuth est gardée dans `<app_config_dir>/oauth-last-poll` : un redémarrage n'appelle jamais avant le délai minimal (13 min).
- Processus lancés par l'app, tous sans console :
  - `wsl.exe --list --running --quiet`, au plus une fois toutes les 30 s (liste en cache), pour lister les distros WSL déjà démarrées. Une distro arrêtée n'est jamais lue, car accéder à `\\wsl.localhost\<distro>` la démarrerait.
  - `codex app-server` (option, au plus une fois par minute) : `codex.exe` ou `codex.cmd` du `PATH`, sinon `wsl.exe -d <distro> --exec sh -lc 'exec codex app-server'` pour une distro déjà démarrée. Arrêté après chaque échange.

## Options

- Fichier `<app_config_dir>/options.json` : `codexAppServer` (défaut `true`) et `claudeOauth` (défaut `false`). Illisible ou absent : valeurs par défaut.
- Cases du tray : « Quotas Codex via app-server » et « ⚠ Quotas Claude via OAuth (non autorisé par Anthropic) ». Le changement s'applique tout de suite et est enregistré.

## Statusline Claude

- Scripts fournis : `scripts/statusline/agenthud-statusline.sh` (WSL, Linux, Git Bash) et `agenthud-statusline.ps1` (Windows PowerShell). Installation manuelle dans `settings.json` de Claude Code, voir le README. L'app ne modifie jamais ces réglages.

## Build and release

- Cible : Windows uniquement, un seul `.exe`.
- En dev, l'exe est cross-compilé depuis WSL : `pnpm tauri build --runner cargo-xwin --target x86_64-pc-windows-msvc`. Voir l'ADR 0001.
- Publication sur GitHub Releases via GitHub Actions (`windows-latest`).
- Pas de signature en v1 : Windows affiche l'avertissement SmartScreen. SignPath (gratuit pour l'OSS) est prévu plus tard.
