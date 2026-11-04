# Testing

## Strategy

- **Unitaires** sur `domain` et `application` : règles métier, déduction de `AgentState`, calcul des fenêtres et des resets. Sans I/O, avec les ports mockés.
- **Adapters** testés sur des **fixtures JSONL réelles anonymisées**, une par version de CLI rencontrée. C'est la protection contre les changements de schéma.
- **UI** : Vitest sur la logique d'affichage. Pas de tests e2e en v1.

## Tools

- `cargo test` pour Rust.
- Vitest pour TS.

## Conventions

- Tests unitaires Rust dans un module `#[cfg(test)]` du fichier testé. Tests d'adapter dans `crates/adapter-*/tests/`.
- Fixtures dans `crates/adapter-*/tests/fixtures/`. Ne jamais y committer de données personnelles ni de token.
- Un bug de parsing commence par une fixture qui le reproduit.

## Run

- Commandes et ordre : voir `coding-assertions.md` (section « Before push »).
- `src-tauri` ne compile que pour Windows. Seule sa compilation de tests est vérifiée depuis WSL. Toute logique à tester va donc dans un crate compilable sur l'hôte, par exemple `crates/overlay`.
