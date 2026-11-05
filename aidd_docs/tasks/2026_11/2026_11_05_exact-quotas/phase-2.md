---
status: done
---

# Instruction: Claude : script de statusline et lecture du fichier de quotas

## Architecture projection

> Tree of the final files. ✅ create · ✏️ modify · ❌ delete

```txt
.
├── scripts/statusline/ ✅ agenthud-statusline.sh · agenthud-statusline.ps1
└── crates/adapter-claude/
    ├── src/ ✏️ lib.rs · ✅ statusline_source.rs
    └── tests/
        ├── ✅ claude_statusline.rs
        └── fixtures/ ✅ statusline-input.json · rate-limits.json · rate-limits-partial.json
```

## Test Scope

| Scenario | Action | Observable outcome | Actor |
| --- | --- | --- | --- |
| Setup | Dossier `.claude` temporaire (`CLAUDE_CONFIG_DIR`) | Aucun fichier de quotas | cli |
| Happy path | Le script `.sh` reçoit une entrée statusline avec `rate_limits` | `agenthud/rate-limits.json` écrit ; l'adapter lit 5 h et hebdo en %, origine `statusline` | cli |
| Edge case | Entrée sans `rate_limits` | Fichier précédent conservé | cli |
| Edge case | Commande chaînée fournie | Sa sortie est affichée, avec la même entrée | cli |
| Edge case | Fichier illisible, BOM, champs `null` ou en trop | Fenêtre absente = `None`, jamais 0 % ; pas de panique | cli |
| Teardown | Suppression du dossier temporaire | État initial | cli |

## Tasks to do

### `1)` Scripts

> Copier les quotas officiels de Claude Code dans un fichier lu par AgentHud.

1. `agenthud-statusline.sh` (POSIX sh, WSL/Linux) : lit stdin ; si `rate_limits` est présent, écrit `{"rate_limits":…}` dans `${CLAUDE_CONFIG_DIR:-$HOME/.claude}/agenthud/rate-limits.json` (fichier temporaire puis `mv`). Avec `jq` : seulement `rate_limits` ; sans `jq` : l'entrée telle quelle. Argument optionnel : commande de statusline existante, exécutée avec la même entrée, dont la sortie est affichée.
2. `agenthud-statusline.ps1` (Windows PowerShell 5+) : même contrat, `-Chain <commande>` optionnel, UTF-8 sans BOM, écriture atomique.

### `2)` Adapter

> Lire le fichier de quotas sur chaque racine.

1. `ClaudeStatuslineSource` (`UsageSource`) : lit `<racine>/agenthud/rate-limits.json` de chaque racine (Windows + distros démarrées), relu seulement si son mtime change ; `observed_at` = mtime ; garde le plus récent.
2. Parsing défensif : `rate_limits.five_hour` / `seven_day`, `used_percentage` et `resets_at` (nombre ou texte RFC 3339), BOM toléré.

## Test acceptance criteria

| Task | Acceptance criteria |
| --- | --- |
| 1 | Test d'intégration (unix) : le script `.sh` réel, lancé avec la fixture d'entrée, produit un fichier que l'adapter lit (5 h et hebdo exacts) ; sans `rate_limits`, le fichier reste ; la commande chaînée est affichée. |
| 2 | Tests sur fixtures : fenêtres partielles, `null`, texte, BOM ; origine `Statusline` ; pas de relecture si le mtime est inchangé. |
