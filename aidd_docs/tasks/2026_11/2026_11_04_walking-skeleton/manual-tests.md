# Tests manuels sous Windows — walking skeleton

- Date : 2026-11-04
- Build testé : commit `14b2303`, `agenthud.exe` en release, cross-compilé
- Testé par l'humain (ADR 0001 : l'agent ne lance aucun programme Windows)

| Check | Résultat |
| --- | --- |
| Fenêtre transparente, au premier plan, sans bordure, absente de la barre des tâches | ✅ |
| Clics transmis à travers l'overlay au démarrage | ✅ |
| `Ctrl+Alt+H` bascule en « interactif », un second appui revient en click-through | ✅ (❌ avant `14b2303` : une 2e instance avait pris le raccourci) |
| Un 2e lancement n'ouvre pas de 2e fenêtre | ✅ |
| Aucun processus restant à la fermeture | ✅ |
| RAM au repos sous 100 Mo | ✅ 91,6 Mo, voir `ram-measurement.md` |

Les changements suivants n'ont pas été retestés sous Windows : crate `overlay` et lecture du mode au chargement de l'UI. Ils sont couverts par les tests de l'hôte, `cargo test -p overlay` et Vitest.
