# Porte de l'étape 2 : l'oxygène s'accumule par la photosynthèse

Rapport produit par `evonisium porte`. Graine 2026, grille de niveau 4 (2562 cellules), pas de 200.0 ka, au plus 1.50 Ga par monde. Critère : la fraction d'O₂ de l'air dépasse 1e-4 et s'y maintient 50.00 Ma ; l'O₂ vient de la photosynthèse oxygénique apparue par évolution ; carbone et phosphore conservés ; la partie se rejoue à l'identique depuis sa graine et ses ordres.

**Verdict : porte franchie sur 5 mondes sur 6**

## Chemin vers la photosynthèse

Dates de première apparition sur la planète, depuis le dépôt de la cellule minimale.

| Monde | pigment protecteur | phototrophie simple | photosynthèse anoxygénique | photosynthèse oxygénique | Rhodopsine | Origine de l'étape oxygénique |
|---|---|---|---|---|---|---|
| Terre (Archéen) | 200.0 ka | 200.0 ka | 400.0 ka | 18.80 Ma | 200.0 ka | moteur |
| Monde océan | 200.0 ka | 200.0 ka | 400.0 ka | 25.80 Ma | 200.0 ka | moteur |
| Monde désertique | 200.0 ka | 200.0 ka | 1.80 Ma | 185.80 Ma | 1.40 Ma | moteur |
| Super-Terre | 200.0 ka | 200.0 ka | 600.0 ka | 20.20 Ma | 400.0 ka | moteur |
| Petite planète | 200.0 ka | 200.0 ka | 400.0 ka | 16.20 Ma | 200.0 ka | moteur |
| Monde sans lune | 200.0 ka | 200.0 ka | 400.0 ka | 19.00 Ma | 200.0 ka | moteur |

## Oxygène et planète

| Monde | Durée simulée | O₂ > 10⁻⁶ | O₂ > seuil | O₂ final | O₂ maximal | CO₂ final | Température | Glace | Océan | Boules de neige | Réorganisations des plaques | Verdict |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Terre (Archéen) | 1.30 Ga | 57.00 Ma | 57.40 Ma | 2.1e-4 | 2.7e-2 | 4167 Pa | 291 K | 0 % | 96 % | 0 | 8 | franchie |
| Monde océan | 183.00 Ma | 130.60 Ma | 131.20 Ma | 5.5e-1 | 5.5e-1 | 520804 Pa | 326 K | 0 % | 100 % | 0 | 1 | franchie |
| Monde désertique | 1.50 Ga | 263.40 Ma | 353.80 Ma | 1.4e-5 | 1.7e-4 | 593 Pa | 280 K | 17 % | 14 % | 0 | 9 | non franchie |
| Super-Terre | 136.00 Ma | 80.60 Ma | 80.60 Ma | 3.0e-1 | 3.1e-1 | 121157 Pa | 304 K | 2 % | 94 % | 0 | 0 | franchie |
| Petite planète | 118.20 Ma | 67.80 Ma | 68.20 Ma | 1.2e-2 | 2.3e-2 | 1807 Pa | 268 K | 38 % | 56 % | 0 | 0 | franchie |
| Monde sans lune | 1.24 Ga | 51.20 Ma | 53.20 Ma | 3.1e-2 | 3.1e-2 | 4808 Pa | 290 K | 1 % | 96 % | 0 | 8 | franchie |

## Budget de l'oxygène (cumulé sur la partie, mol d'O₂)

La photosynthèse oxygénique est la seule source. La couche de surface en reprend une partie (respiration, oxydation du fer et du sulfure sur place) ; le reste gagne l'air, où les puits globaux le consomment.

| Monde | Photosynthèse (brut) | Libéré vers l'air | Repris en surface | Respiration profonde | Gaz réduits (H₂) | Méthane | Fer et manganèse | Roches exposées | Sulfure |
|---|---|---|---|---|---|---|---|---|---|
| Terre (Archéen) | 4.97e23 | 4.93e22 | 6.26e16 | 3.11e22 | 2.20e20 | 1.40e22 | 8.33e17 | 2.16e20 | 3.85e21 |
| Monde océan | 3.65e23 | 5.60e22 | 1.42e18 | 5.37e22 | 2.12e19 | 1.19e21 | 6.10e18 | 0.00e0 | 2.50e19 |
| Monde désertique | 4.09e22 | 6.62e21 | 1.62e13 | 5.33e19 | 1.48e20 | 3.45e21 | 1.29e16 | 1.47e19 | 2.95e21 |
| Super-Terre | 4.14e23 | 3.54e22 | 6.31e17 | 3.29e22 | 8.23e19 | 1.89e21 | 9.50e18 | 6.10e19 | 2.04e20 |
| Petite planète | 4.74e22 | 6.86e21 | 8.03e13 | 5.94e21 | 6.82e18 | 8.80e20 | 1.84e16 | 2.49e19 | 1.39e19 |
| Monde sans lune | 5.19e23 | 4.80e22 | 1.01e17 | 3.23e22 | 2.20e20 | 1.40e22 | 1.20e18 | 2.36e20 | 1.30e21 |

## Aide de l'accélérateur, bilans et rejeu

« Électrons corrigés » : écart du bilan des électrons des flux de surface prolongés sur chaque pas, corrigé avant leur application, en part de la production photosynthétique d'O₂ de la partie.

| Monde | Pas avec accélérateur | Modifications fixées grâce à lui | Modifications fixées par cause | Bilan carbone | Bilan phosphore | Électrons corrigés | Rejeu identique | Calcul |
|---|---|---|---|---|---|---|---|---|
| Terre (Archéen) | 0 | 0 | mutation spontanée 130006085, transfert horizontal 9627668 | 4.4e-9 | 8.4e-12 | 5.3 % | oui | 2254 s |
| Monde océan | 0 | 0 | mutation spontanée 24968880, transfert horizontal 2050446 | 6.4e-10 | 1.9e-12 | 3.3 % | oui | 475 s |
| Monde désertique | 0 | 0 | mutation spontanée 15688577, transfert horizontal 1040105 | 1.3e-8 | 1.2e-12 | 17.5 % | oui | 1449 s |
| Super-Terre | 0 | 0 | mutation spontanée 14081383, transfert horizontal 1246557 | 3.9e-9 | 5.0e-12 | 7.6 % | oui | 662 s |
| Petite planète | 0 | 0 | mutation spontanée 6854446, transfert horizontal 555712 | 3.8e-9 | 4.9e-12 | 13.8 % | oui | 267 s |
| Monde sans lune | 0 | 0 | mutation spontanée 112209234, transfert horizontal 9734476 | 1.3e-9 | 6.0e-12 | 3.9 % | oui | 2251 s |

