# Tests manuels sous Windows — tray et déplacement

- Date : 2026-11-05
- Build testé : commit `40afded`, `agenthud.exe` en release, cross-compilé
- Testé par l'humain (ADR 0001), verdict global : « tout ok »

| # | Check | Résultat |
| --- | --- | --- |
| 1 | Click-through : un clic sur le titre traverse, la fenêtre ne bouge pas | ✅ |
| 2 | Interactif (`Ctrl+Alt+H`) : le glisser du titre déplace la fenêtre | ✅ |
| 3 | La position est retrouvée après « Quitter » puis un relancement | ✅ |
| 4 | Menu du tray : « Mode interactif », « Masquer », « Quitter » | ✅ |
| 5 | Le libellé du mode suit le menu et `Ctrl+Alt+H` | ✅ |
| 6 | « Masquer » puis « Afficher » | ✅ |
| 7 | Overlay masqué : un 2e lancement le réaffiche, avec une seule icône | ✅ |
| 8 | « Quitter » : plus d'icône ni de processus | ✅ |

Non testés : écran débranché (bornage couvert par les tests de `crates/overlay`) et `window.json` corrompu (couvert par la journalisation, sans test Windows).

Gate de build complet (installateur NSIS) : ✅ après `sudo apt install pkg-config libayatana-appindicator3-dev` dans WSL.
