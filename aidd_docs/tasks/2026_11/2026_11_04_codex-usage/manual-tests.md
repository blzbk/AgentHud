# Tests manuels sous Windows — adapter Codex

- Date : 2026-11-05
- Build testé : commit `5441c78`, `agenthud.exe` en release, cross-compilé
- Testé par l'humain (ADR 0001)

| Check | Résultat |
| --- | --- |
| La carte Codex s'affiche avec les jauges 5h et hebdo, l'heure de reset et l'âge | ✅ validé par l'humain (« c'est ok, c'est même top ») |
| Valeurs identiques au `/status` de Codex (à 1 % près) | ✅ inclus dans la validation globale, sans relevé chiffré |
| WSL arrêté au lancement : il reste arrêté | ⚪ non testé |
| Aucune console qui clignote toutes les 3 s | ⚪ non testé explicitement, aucun problème signalé |
| RAM au repos sous 100 Mo | ⚪ non mesuré |
| WSL démarré : il peut encore s'arrêter seul pendant que l'overlay tourne | ⚪ non testé, suivi dans une issue |

Note : un lancement « sans effet » venait d'une ancienne instance encore ouverte. La protection single-instance ignorait le 2e lancement sans rien afficher.
