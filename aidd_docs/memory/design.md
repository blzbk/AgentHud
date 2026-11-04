# Design

## System

- Pas de librairie de design : une interface minimale en surimpression, pensée pour être lue d'un coup d'œil.
- Styles en CSS modules, avec des tokens en variables CSS.

## Tokens

- Couleurs, espacements et typographie définis dans `apps/desktop/src/shared/theme.css`.

## Components

- Une carte par provider : jauges `5h` et `weekly`, heure de reset et âge de la donnée.
- Un badge d'état par agent : `waiting`, `working`, `idle`, `offline`.
- Toute valeur inconnue s'affiche `unknown`, jamais vide ni à 0 % par défaut.

## Accessibility

- Contraste lisible sur n'importe quel fond, car l'overlay est transparent. Prévoir un fond semi-opaque derrière le texte.
- Chaque état est porté par un libellé, en plus de la couleur.
