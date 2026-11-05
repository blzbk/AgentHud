# Desktop

## Framework

- Tauri, avec l'UI dans la WebView2 de Windows. Le process principal est en Rust (`apps/desktop/src-tauri`).
- Fenêtre overlay transparente, `alwaysOnTop`, sans décoration, de taille fixe 320 × 440 : le HUD prend la hauteur de son contenu et le reste de la fenêtre reste transparent. Le plafond de 4 agents par provider garantit que rien n'est coupé. Elle est click-through via `set_ignore_cursor_events`, et un raccourci clavier la rend interactive.
- En mode interactif, l'en-tête porte `data-tauri-drag-region="deep"` : on déplace l'overlay par son titre. Cela demande la permission `core:window:allow-start-dragging`.
- La position est sauvegardée dans `<app_config_dir>/window.json` (500 ms après le dernier déplacement). Au lancement, elle est ramenée sur un écran existant (`overlay::fit_on_screens`). Un fichier absent ou illisible laisse la position par défaut.
- Icône dans la zone de notification (`tray.rs`), menu « Mode interactif » / « Mode click-through », « Masquer » / « Afficher » et « Quitter ». Les libellés suivent l'état, que le changement vienne du menu ou du raccourci.
- Une seule instance. Un 2e lancement réaffiche l'overlay masqué.

## Native access

- Lecture seule des dossiers `~/.claude` et `~/.codex`, côté Windows natif ou WSL (`\\wsl.localhost\...`, `\\wsl$\...`).
- Tout accès aux fichiers passe par Rust, jamais par les plugins fs côté JS.
- Aucun accès réseau en v1.
- Seul processus lancé par l'app : `wsl.exe --list --running --quiet`, au plus une fois toutes les 30 s (liste en cache), sans console, pour lister les distros WSL déjà démarrées. Une distro arrêtée n'est jamais lue, car accéder à `\\wsl.localhost\<distro>` la démarrerait.

## Build and release

- Cible : Windows uniquement, un seul `.exe`.
- En dev, l'exe est cross-compilé depuis WSL : `pnpm tauri build --runner cargo-xwin --target x86_64-pc-windows-msvc`. Voir l'ADR 0001.
- Publication sur GitHub Releases via GitHub Actions (`windows-latest`).
- Pas de signature en v1 : Windows affiche l'avertissement SmartScreen. SignPath (gratuit pour l'OSS) est prévu plus tard.
