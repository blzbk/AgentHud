# Tests manuels sous Windows — adapter Claude, états des agents, HUD multi-LLM

- Date : 2026-11-05
- Build testé : commits `6d24d7c` à `066a1fb`, `agenthud.exe` en release, cross-compilé
- Testé par l'humain (ADR 0001), verdict global : validé (« test, valide »)

| Check | Résultat |
| --- | --- |
| Cartes Codex et Claude, sans coupure en bas de l'overlay | ✅ (captures du 2026-11-05) |
| Usage Claude en tokens sur 5 h estimé, avec reset `~` et mention « estimé » | ✅ |
| Badge d'agent qui suit l'activité (`working` / `waiting` / `idle`) | ✅ |
| Codex : une conversation ouverte est visible avant son 1er message (lock) | ✅ |
| Claude : seules les sessions en cours sont listées (marqueurs `sessions/*.json`) | ✅ validé globalement après la phase 7 |
| Provider fermé : `offline` | ✅ validé globalement |

Points relevés pendant le test, puis corrigés en phase 7 :
- des sessions Claude terminées restaient listées ;
- Codex apparaissait `offline` alors qu'il était ouvert.
