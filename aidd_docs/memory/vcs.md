# VCS

## Setup

- Main branch: `main`
- Platform: `github` (`blzbk/AgentHud`, via `gh`)
- Tickets: GitHub Issues (`gh issue view <n>`)
- Hooks: aucun pour l'instant. Les gates sont dans `coding-assertions.md`.

## Branches

- Format: `type/description-courte` (par exemple `feat/codex-adapter`)
- Types in use: `feat`, `fix`, `refactor`, `test`, `docs`, `chore`

## Commits

- Convention: Conventional Commits
- Format: `type(scope): description`
- Scopes: `domain`, `application`, `adapter-claude`, `adapter-codex`, `adapter-fs`, `desktop`, `ui`
- Rules: impératif, en minuscules, 72 caractères au maximum

## Commit Strategy

AI should auto commit: `after phase`
