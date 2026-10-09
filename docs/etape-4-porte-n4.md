# Porte de l'oxygène au niveau 4 avec le calibrage de l'étape 4

Rapport produit par `evonisium porte`. Grille de niveau 4 (2562 cellules physiques, vie un niveau en dessous), pas de 200.0 ka, au plus 2.00 Ga par partie, un tour d'évolution « apparition puis fixation » par tranche de 100.0 ka du pas, graines essayées dans l'ordre [2026] jusqu'à la première qui franchit la porte. Critère : la fraction d'O₂ de l'air dépasse 1e-4 et s'y maintient 50.00 Ma ; l'O₂ vient de la photosynthèse oxygénique apparue par évolution ; carbone, phosphore et électrons conservés à 10⁻⁶ près ; la partie se rejoue à l'identique depuis sa graine et ses ordres, avec une caméra qui bouge dans l'une et pas dans l'autre.

**Verdict : porte non franchie : 5 mondes sur 6 (voir le détail)**

## Chemin vers la photosynthèse

Dates de première apparition sur la planète, depuis le dépôt de la cellule minimale.

| Monde | Graine | pigment protecteur | phototrophie simple | photosynthèse anoxygénique | photosynthèse oxygénique | Rhodopsine | Origine de l'étape oxygénique |
|---|---|---|---|---|---|---|---|---|---|
| Terre (Archéen) | 2026 | 1.20 Ma | 1.20 Ma | 7.20 Ma | 317.40 Ma | 6.00 Ma | moteur |
| Monde océan | 2026 | 2.40 Ma | 2.40 Ma | 7.80 Ma | 619.80 Ma | 10.20 Ma | moteur |
| Monde désertique | 2026 | 2.40 Ma | 2.40 Ma | 10.20 Ma | 615.60 Ma | 10.80 Ma | moteur |
| Super-Terre | 2026 | 2.40 Ma | 2.40 Ma | 2.60 Ma | 419.60 Ma | 6.20 Ma | moteur |
| Petite planète | 2026 | 1.80 Ma | 1.80 Ma | 11.40 Ma | 627.00 Ma | 4.20 Ma | accélérateur |
| Monde sans lune | 2026 | 1.20 Ma | 1.20 Ma | 5.40 Ma | 607.20 Ma | 5.40 Ma | moteur |

## Oxygène et planète

« PAL » : niveau actuel de l'atmosphère terrestre (21 %). Pour comparaison, la littérature place l'O₂ du Protérozoïque, après la Grande Oxydation, entre 0,1 % et 10 % du niveau actuel selon les auteurs (Lyons, Reinhard et Planavsky, 2014, *Nature* 506 ; Planavsky et coll., 2014, *Science* 346), et celui de l'Archéen sous 10⁻⁵ PAL. La partie s'arrête dès que la porte est franchie : l'O₂ final est celui de la fin du maintien, pas un plateau à l'équilibre.

| Monde | Graine | Durée simulée | O₂ > 10⁻⁶ | O₂ > seuil | O₂ final | O₂ final (PAL) | O₂ maximal | CO₂ final | Température | Glace | Océan | Eaux douces colonisées | Biomasse des eaux douces | Verdict |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Terre (Archéen) | 2026 | 392.00 Ma | 337.80 Ma | 340.00 Ma | 2.0e-1 | 97.24 % | 3.1e-1 | 830 Pa | 229 K | 100 % | 64 % | 0 cellules | -0.0e0 mol C | franchie |
| Monde océan | 2026 | 681.40 Ma | 629.40 Ma | 629.60 Ma | 2.9e-1 | 137.46 % | 2.9e-1 | 2485134 Pa | 372 K | 0 % | 100 % | 0 cellules | -0.0e0 mol C | franchie |
| Monde désertique | 2026 | 681.00 Ma | 630.20 Ma | 630.60 Ma | 1.8e-1 | 84.53 % | 2.6e-1 | 1786 Pa | 271 K | 37 % | 14 % | 363 cellules | 4.8e12 mol C | franchie |
| Super-Terre | 2026 | 493.60 Ma | 440.60 Ma | 441.20 Ma | 1.4e-1 | 64.70 % | 6.1e-1 | 48662 Pa | 300 K | 0 % | 97 % | 10 cellules | 7.9e11 mol C | franchie |
| Petite planète | 2026 | 657.20 Ma | 647.00 Ma | 647.20 Ma | 0.0e0 | 0.00 % | 1.5e-2 | 3116 Pa | 242 K | 100 % | 84 % | 0 cellules | -0.0e0 mol C | non franchie |
| Monde sans lune | 2026 | 1.67 Ga | 1.61 Ga | 1.61 Ga | 8.4e-1 | 400.69 % | 8.4e-1 | 4572 Pa | 294 K | 0 % | 100 % | 0 cellules | -0.0e0 mol C | franchie |

## Budget de l'oxygène (cumulé sur la partie, mol d'O₂)

La photosynthèse oxygénique est la seule source. La couche de surface en reprend une partie (respiration, oxydation du fer et du sulfure sur place) ; le reste gagne l'air, où les puits globaux le consomment.

| Monde | Graine | Photosynthèse (brut) | Libéré vers l'air | Repris en surface | Respiration profonde | Gaz réduits (H₂) | Méthane | Fer et manganèse | Plancher océanique | Roches exposées | Sulfure |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Terre (Archéen) | 2026 | 6.70e22 | 1.43e22 | 6.18e15 | 7.11e21 | 1.79e19 | 6.68e21 | 7.11e17 | 5.07e19 | 3.75e20 | 9.73e17 |
| Monde océan | 2026 | 3.57e23 | 7.17e22 | 7.52e17 | 6.89e22 | 1.83e19 | 1.35e21 | 1.99e18 | 4.41e19 | 0.00e0 | 2.39e17 |
| Monde désertique | 2026 | 2.08e22 | 4.44e21 | 1.70e15 | 1.08e20 | 4.16e18 | 4.12e21 | 1.03e17 | 4.87e18 | 1.72e20 | 3.82e17 |
| Super-Terre | 2026 | 4.30e23 | 6.19e22 | 4.96e17 | 4.45e22 | 6.64e19 | 1.57e22 | 1.23e19 | 8.30e19 | 1.43e21 | 1.81e18 |
| Petite planète | 2026 | 3.07e19 | 2.05e19 | 6.04e16 | 8.14e15 | 5.41e16 | 1.93e19 | 1.99e15 | 7.34e17 | 2.01e17 | 1.14e17 |
| Monde sans lune | 2026 | 1.71e23 | 3.70e22 | 3.30e17 | 3.33e22 | 1.99e18 | 2.96e21 | 3.64e17 | 1.14e19 | 0.00e0 | 1.16e17 |

## Aide de l'accélérateur, bilans, rejeu et vitesse

« Écart des électrons » : écart entre le pouvoir oxydant des flux de surface prolongés sur chaque pas et celui de leurs sources hydrothermales, en part de la production photosynthétique d'O₂ de la partie ; il n'est pas corrigé et ne vient que des arrondis. « Électrons freinés » : pouvoir oxydant déplacé quand une boîte vide freine un prélèvement des couches (ce qu'il alimentait est freiné avec lui), même unité ; « non repris » : la part qu'aucun flux de la couche n'a pu reprendre. « Bilan électrons » : écart du registre de flux, qui doit rester nul. La vitesse est celle de la partie entière, sur la machine de mesure.

| Monde | Graine | Pas avec accélérateur | Modifications fixées grâce à lui | Modifications fixées par cause | Bilan carbone | Bilan phosphore | Bilan électrons | Écart des électrons | Électrons freinés | Non repris | Rejeu identique | Calcul | Vitesse |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Terre (Archéen) | 2026 | 0 | 0 | mutation spontanée 15013394, transfert horizontal 888966 | 7.5e-12 | 3.0e-13 | 1.7e-12 | 3.1e-13 | 59.4 % | 0.00 % | oui | 135 s sur 4 fils | 2.90 Ma par seconde |
| Monde océan | 2026 | 21 | 13055 | mutation spontanée 26796889, transfert horizontal 2157881, accélérateur 13055 | 5.2e-12 | 1.1e-13 | 3.6e-11 | 9.9e-12 | 33.2 % | 0.00 % | oui | 239 s sur 4 fils | 2.85 Ma par seconde |
| Monde désertique | 2026 | 10 | 3722 | mutation spontanée 20110642, transfert horizontal 1349061, accélérateur 3722 | 1.6e-11 | 2.3e-12 | 1.5e-12 | 3.3e-13 | 86.1 % | 0.00 % | oui | 214 s sur 4 fils | 3.18 Ma par seconde |
| Super-Terre | 2026 | 0 | 0 | mutation spontanée 20769314, transfert horizontal 1305837 | 9.8e-12 | 6.3e-12 | 1.7e-11 | 2.2e-12 | 22.8 % | 0.00 % | oui | 199 s sur 4 fils | 2.48 Ma par seconde |
| Petite planète | 2026 | 27 | 3180 | mutation spontanée 14147901, transfert horizontal 739446, accélérateur 3180 | 3.3e-11 | 3.3e-14 | 1.0e-11 | 1.2e-11 | 52110.3 % | 0.00 % | oui | 156 s sur 4 fils | 4.22 Ma par seconde |
| Monde sans lune | 2026 | 4 | 3891 | mutation spontanée 55891086, transfert horizontal 3557295, accélérateur 3891 | 3.8e-12 | 3.8e-12 | 9.1e-11 | 7.8e-12 | 23.7 % | 0.00 % | oui | 444 s sur 4 fils | 3.75 Ma par seconde |

## Garde-fous du plafond de populations et du tunnel

Plafond de populations par cellule du vivant : jamais la dernière d'une guilde (voie principale) ; parmi les autres, on évince d'abord la plus basse fitness d'invasion (taux de croissance dans la communauté résidente), ou la moins abondante sous la règle de l'étape 3. « Dépassent le plafond » : part des cellules peuplées qui dépassaient le plafond avant éviction, sur toute la partie et sur ses 100 derniers pas. « Saturées » : part des cellules peuplées qui ont perdu une population établie (plus que la biomasse d'un fondateur), même découpage (au-delà de 2 % sur monde mûr, la règle est à revoir) ; « dont en croissance » : celles dont la population évincée croissait encore. Innovations : mutants innovants (de novo, duplication suivie de divergence) apparus sur la partie, tirés selon une loi de Poisson, dont ceux que l'accélérateur a ajoutés. Tunnel : essais (un par mutant innovant qui ne se fixe pas seul) et réussites, au taux de Weissman et coll. (2009) tiré selon une loi de Poisson.

| Monde | Graine | Dépassent le plafond (partie) | Dépassent le plafond (100 derniers pas) | Saturées (partie) | Saturées (100 derniers pas) | dont en croissance | Innovations apparues | dont accélérateur | Essais du tunnel | Réussites |
|---|---|---|---|---|---|---|---|---|---|---|
| Terre (Archéen) | 2026 | 37.5 % | 78.4 % | 0.55 % | 1.74 % | 1.55 % | 47075613 | 0 | 9272179 | 6766 |
| Monde océan | 2026 | 49.8 % | 98.2 % | 0.52 % | 3.40 % | 2.49 % | 193736273 | 48244061 | 24095916 | 19589 |
| Monde désertique | 2026 | 28.0 % | 92.5 % | 0.68 % | 3.68 % | 3.41 % | 31769080 | 9853006 | 9180259 | 5537 |
| Super-Terre | 2026 | 47.6 % | 97.1 % | 1.41 % | 4.73 % | 3.86 % | 248418929 | 0 | 25533037 | 23702 |
| Petite planète | 2026 | 21.1 % | 12.2 % | 0.24 % | 0.11 % | 0.07 % | 19280009 | 10360877 | 5337095 | 5443 |
| Monde sans lune | 2026 | 24.6 % | 98.0 % | 0.68 % | 11.27 % | 9.26 % | 67541024 | 4611248 | 23733111 | 21674 |
