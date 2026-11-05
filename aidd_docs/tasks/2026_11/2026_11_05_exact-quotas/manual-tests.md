# Tests manuels sous Windows — quotas exacts

- Date : 2026-11-05
- Build testé : commits `87e870b` à `ca3a8c3`, `agenthud.exe` en release, cross-compilé
- Testé par l'humain (ADR 0001), verdict global : validé (« test, valide »)

| Check | Résultat |
| --- | --- |
| Statusline WSL (script chaîné à `ccstatusline`) : `~/.claude/agenthud/rate-limits.json` écrit, carte Claude avec jauges `statusline` | ✅ validé globalement |
| Carte Codex en `app-server`, sans console qui clignote ni processus `codex` restant | ✅ validé globalement |
| Option « Quotas Codex via app-server » : la décocher repasse en `session`, et le choix survit au redémarrage | ✅ validé globalement |
| Option OAuth désactivée par défaut, libellée comme dangereuse | ✅ validé globalement |

Non testés explicitement :
- la statusline côté Windows natif (PowerShell) ;
- le mode OAuth réel (appel au plus toutes les 13 min, à vos risques, ADR 0002) ;
- « auth expirée » sur Codex.

Le délai de 13 min entre 2 appels OAuth est maintenant conservé entre 2 lancements (`ca3a8c3`), ce qui est couvert par les tests du planificateur.
