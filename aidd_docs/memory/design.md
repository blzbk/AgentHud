# Design

## System

- Pas de librairie de design : une interface minimale en surimpression, pensée pour être lue d'un coup d'œil.
- Styles en CSS modules, avec des tokens en variables CSS.

## Tokens

- Couleurs, espacements et typographie définis dans `apps/desktop/src/shared/theme.css`.

## Components

- Une carte par provider, dans l'ordre Codex puis Claude : âge de la donnée, une ligne par fenêtre d'usage, puis la liste des agents.
  - Codex : jauges `5h` et `hebdo` en %, avec l'heure de reset.
  - Claude : `5h` en tokens (`1,2 M tokens`, `850 k tokens`), reset préfixé de `~` et mention « estimé ». Pas de ligne hebdo.
- Un badge d'état par agent, avec son libellé : `working`, `waiting`, `idle` ou `unknown`, suivi du projet ou d'un identifiant court. Au plus 4 agents par provider, puis `+N`. Un provider sans session active affiche un seul badge `offline`.
- Toute valeur inconnue s'affiche `unknown`, jamais vide ni à 0 % par défaut.

## Accessibility

- Contraste lisible sur n'importe quel fond, car l'overlay est transparent. Prévoir un fond semi-opaque derrière le texte.
- Chaque état est porté par un libellé, en plus de la couleur.
