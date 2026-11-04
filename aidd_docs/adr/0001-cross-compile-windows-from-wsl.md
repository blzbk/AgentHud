# ADR 0001 — Cross-compiler l'exe Windows depuis WSL

- Statut : accepté (2026-11-04)
- Révision : à revoir si l'exe cross-compilé se comporte différemment d'un build natif

## Contexte

- La cible est un `.exe` Windows (Tauri 2, cible `x86_64-pc-windows-msvc`).
- Le repo et le développement sont dans WSL2 (ext4).
- La sandbox de l'agent IA ne doit pas être contournée. Lancer `cmd.exe`, `powershell.exe` ou un outil Windows depuis la session exécute du code hors de la sandbox WSL.
- Un outil Windows qui lit des fichiers WSL (ou l'inverse) passe par le protocole 9P. Un build Rust ou un `pnpm install` devient alors 5 à 20 fois plus lent. De plus, `cmd.exe` refuse un chemin `\\wsl$` comme dossier courant.

## Décision

- Le développement, les tests et le build se font **dans WSL**, avec des outils Linux uniquement.
- L'exe Windows est **cross-compilé** avec `cargo-xwin` :
  `pnpm tauri build --runner cargo-xwin --target x86_64-pc-windows-msvc`
- L'exe sort dans `target/x86_64-pc-windows-msvc/release/`, et l'installateur NSIS dans `.../bundle/nsis/`.
- **L'humain** lance l'exe côté Windows : tests manuels et mesure de la RAM. L'agent ne lance aucun processus Windows.

## Options écartées

- **Repo sur `C:` et build Windows natif lancé par l'agent :** contourne la sandbox.
- **Build Windows natif sur le repo WSL :** lenteur de 9P, et `cmd.exe` incompatible avec un chemin UNC.
- **GitHub Actions `windows-latest` comme seul build :** boucle de 5 à 10 min par essai, trop lente pour mettre au point l'overlay. On le garde pour les releases.

## Conséquences

- Il faut installer, avec `sudo` (fait par l'humain) : `nsis`, `lld`, `llvm`, `clang`, la cible `x86_64-pc-windows-msvc` et `cargo-xwin`.
- `cargo-xwin` télécharge le Windows SDK et le CRT de Microsoft. Cela implique d'accepter la licence Microsoft.
- La documentation Tauri juge ce chemin « moins testé » qu'un build natif. Seul NSIS est disponible, sans MSI. La signature demande un outil externe.
- Les releases officielles restent construites nativement par GitHub Actions `windows-latest`.
