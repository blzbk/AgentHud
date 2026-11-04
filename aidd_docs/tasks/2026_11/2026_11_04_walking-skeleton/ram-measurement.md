# Mesure de la RAM — walking skeleton

- Date : 2026-11-04
- Build : `agenthud.exe` 0.1.0, release, cross-compilé depuis WSL (`cargo-xwin`), commit `14b2303`
- WebView2 : 156.0.4314.58, lu dans le registre avant l'ADR 0001 et non reconfirmé à la mesure
- Méthode : Gestionnaire des tâches Windows, onglet Détails, colonne mémoire, après 60 s au repos, une seule instance
- Mesure réalisée par l'humain

| Processus | RAM |
| --- | --- |
| `agenthud.exe` | 14,0 Mo |
| `msedgewebview2.exe` (GPU) | 45,2 Mo |
| `msedgewebview2.exe` (renderer ou browser) | 23,6 Mo |
| `msedgewebview2.exe` (network) | 6,0 Mo |
| `msedgewebview2.exe` (storage) | 2,8 Mo |
| **Total** | **91,6 Mo** |

## Verdict

✅ Total sous la cible de 100 Mo, avec 8,4 Mo de marge. La marge est faible : le processus GPU pèse à lui seul près de la moitié du total.

Pistes si les features suivantes font dépasser la cible :
- régler `MemoryUsageTargetLevel=Low` au repos ;
- désactiver l'accélération GPU de la WebView (à mesurer).
