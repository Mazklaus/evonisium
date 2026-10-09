# Porte de l'étape 3 : l'oxygène s'accumule sur les six mondes à la résolution normale

Rapport produit par `evonisium porte`. Grille de niveau 6 (40962 cellules physiques, vie un niveau en dessous), pas de 200.0 ka, au plus 1.50 Ga par partie, un tour d'évolution « apparition puis fixation » par tranche de 100.0 ka du pas, graines essayées dans l'ordre [2026, 7, 42] jusqu'à la première qui franchit la porte. Critère : la fraction d'O₂ de l'air dépasse 1e-4 et s'y maintient 50.00 Ma ; l'O₂ vient de la photosynthèse oxygénique apparue par évolution ; carbone, phosphore et électrons conservés à 10⁻⁶ près ; la partie se rejoue à l'identique depuis sa graine et ses ordres, avec une caméra qui bouge dans l'une et pas dans l'autre.

**Verdict : porte franchie sur les 6 mondes**

## Chemin vers la photosynthèse

Dates de première apparition sur la planète, depuis le dépôt de la cellule minimale.

| Monde | Graine | pigment protecteur | phototrophie simple | photosynthèse anoxygénique | photosynthèse oxygénique | Rhodopsine | Origine de l'étape oxygénique |
|---|---|---|---|---|---|---|---|
| Terre (Archéen) | 2026 | 200.0 ka | 200.0 ka | 400.0 ka | 8.40 Ma | 200.0 ka | moteur |
| Monde océan | 2026 | 200.0 ka | 200.0 ka | 200.0 ka | 4.60 Ma | 200.0 ka | moteur |
| Monde désertique | 2026 | 200.0 ka | 200.0 ka | 200.0 ka | 7.80 Ma | 200.0 ka | moteur |
| Super-Terre | 2026 | 200.0 ka | 200.0 ka | 200.0 ka | 4.00 Ma | 200.0 ka | moteur |
| Petite planète | 2026 | 200.0 ka | 200.0 ka | 400.0 ka | 3.20 Ma | 200.0 ka | moteur |
| Monde sans lune | 2026 | 200.0 ka | 200.0 ka | 400.0 ka | 2.80 Ma | 200.0 ka | moteur |

## Oxygène et planète

« PAL » : niveau actuel de l'atmosphère terrestre (21 %). Pour comparaison, la littérature place l'O₂ du Protérozoïque, après la Grande Oxydation, entre 0,1 % et 10 % du niveau actuel selon les auteurs (Lyons, Reinhard et Planavsky, 2014, *Nature* 506 ; Planavsky et coll., 2014, *Science* 346), et celui de l'Archéen sous 10⁻⁵ PAL. La partie s'arrête dès que la porte est franchie : l'O₂ final est celui de la fin du maintien, pas un plateau à l'équilibre.

| Monde | Graine | Durée simulée | O₂ > 10⁻⁶ | O₂ > seuil | O₂ final | O₂ final (PAL) | O₂ maximal | CO₂ final | Température | Glace | Océan | Eaux douces colonisées | Biomasse des eaux douces | Verdict |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Terre (Archéen) | 2026 | 84.20 Ma | 32.00 Ma | 33.00 Ma | 2.7e-1 | 130.73 % | 2.9e-1 | 5836 Pa | 274 K | 29 % | 55 % | 2576 cellules | 1.7e12 mol C | franchie |
| Monde océan | 2026 | 109.60 Ma | 54.60 Ma | 56.80 Ma | 7.9e-1 | 377.15 % | 7.9e-1 | 60447 Pa | 298 K | 0 % | 100 % | 0 cellules | -0.0e0 mol C | franchie |
| Monde désertique | 2026 | 78.40 Ma | 23.80 Ma | 24.80 Ma | 1.2e-1 | 57.56 % | 1.2e-1 | 20495 Pa | 273 K | 42 % | 29 % | 3322 cellules | 6.8e12 mol C | franchie |
| Super-Terre | 2026 | 88.40 Ma | 36.40 Ma | 37.20 Ma | 6.5e-1 | 310.96 % | 6.5e-1 | 128199 Pa | 305 K | 1 % | 97 % | 49 cellules | 3.1e11 mol C | franchie |
| Petite planète | 2026 | 141.60 Ma | 66.00 Ma | 70.40 Ma | 9.6e-4 | 0.46 % | 1.4e-3 | 13835 Pa | 275 K | 44 % | 58 % | 1880 cellules | 2.2e11 mol C | franchie |
| Monde sans lune | 2026 | 81.20 Ma | 30.00 Ma | 31.20 Ma | 3.1e-1 | 145.84 % | 3.1e-1 | 5567 Pa | 274 K | 31 % | 55 % | 2387 cellules | 1.5e12 mol C | franchie |

## Budget de l'oxygène (cumulé sur la partie, mol d'O₂)

La photosynthèse oxygénique est la seule source. La couche de surface en reprend une partie (respiration, oxydation du fer et du sulfure sur place) ; le reste gagne l'air, où les puits globaux le consomment.

| Monde | Graine | Photosynthèse (brut) | Libéré vers l'air | Repris en surface | Respiration profonde | Gaz réduits (H₂) | Méthane | Fer et manganèse | Plancher océanique | Roches exposées | Sulfure |
|---|---|---|---|---|---|---|---|---|---|---|---|
| Terre (Archéen) | 2026 | 1.76e23 | 2.75e22 | 1.14e16 | 2.61e22 | 2.35e19 | 7.01e20 | 2.26e18 | 1.15e20 | 3.01e20 | 1.12e20 |
| Monde océan | 2026 | 3.56e23 | 5.27e22 | 5.61e17 | 5.04e22 | 4.96e19 | 1.11e21 | 5.53e18 | 1.13e20 | 0.00e0 | 5.63e19 |
| Monde désertique | 2026 | 8.17e22 | 1.10e22 | 4.04e16 | 1.02e22 | 5.16e18 | 6.95e20 | 9.03e16 | 1.05e19 | 1.61e20 | 3.49e18 |
| Super-Terre | 2026 | 5.39e23 | 7.75e22 | 8.83e17 | 7.46e22 | 8.14e19 | 1.30e21 | 1.26e19 | 1.11e20 | 4.19e19 | 5.65e18 |
| Petite planète | 2026 | 2.49e22 | 8.38e21 | 0.00e0 | 1.14e21 | 8.62e18 | 2.33e21 | 5.55e16 | 4.91e19 | 8.25e18 | 4.84e21 |
| Monde sans lune | 2026 | 1.66e23 | 2.53e22 | 1.26e16 | 2.35e22 | 2.23e19 | 9.19e20 | 2.23e18 | 1.07e20 | 2.56e20 | 3.71e20 |

## Aide de l'accélérateur, bilans, rejeu et vitesse

« Électrons corrigés » : écart du bilan des électrons des flux de surface prolongés sur chaque pas, corrigé avant leur application et inscrit au registre, en part de la production photosynthétique d'O₂ de la partie. « Bilan électrons » : écart du registre de flux, qui doit rester nul. La vitesse est celle de la partie entière, sur la machine de mesure.

| Monde | Graine | Pas avec accélérateur | Modifications fixées grâce à lui | Modifications fixées par cause | Bilan carbone | Bilan phosphore | Bilan électrons | Électrons corrigés | Rejeu identique | Calcul | Vitesse |
|---|---|---|---|---|---|---|---|---|---|---|---|
| Terre (Archéen) | 2026 | 0 | 0 | mutation spontanée 37062476, transfert horizontal 2499043 | 3.1e-11 | 1.6e-11 | 4.2e-8 | 7.2 % | oui | 290 s sur 4 fils | 290.7 ka par seconde |
| Monde océan | 2026 | 0 | 0 | mutation spontanée 69221746, transfert horizontal 4735206 | 2.9e-12 | 3.9e-13 | 4.9e-8 | 6.1 % | oui | 545 s sur 4 fils | 200.9 ka par seconde |
| Monde désertique | 2026 | 0 | 0 | mutation spontanée 31016294, transfert horizontal 1840430 | 2.4e-11 | 8.3e-12 | 1.9e-8 | 10.8 % | oui | 284 s sur 4 fils | 275.8 ka par seconde |
| Super-Terre | 2026 | 0 | 0 | mutation spontanée 54507676, transfert horizontal 3876106 | 9.7e-12 | 3.0e-12 | 5.8e-8 | 7.8 % | oui | 471 s sur 4 fils | 187.6 ka par seconde |
| Petite planète | 2026 | 0 | 0 | mutation spontanée 44254176, transfert horizontal 3596631 | 6.3e-12 | 2.1e-13 | 1.7e-8 | 37.2 % | oui | 662 s sur 4 fils | 213.9 ka par seconde |
| Monde sans lune | 2026 | 0 | 0 | mutation spontanée 37022040, transfert horizontal 2661627 | 3.6e-11 | 2.7e-12 | 6.8e-8 | 9.9 % | oui | 290 s sur 4 fils | 279.6 ka par seconde |

## Garde-fous du plafond de populations et du tunnel

Plafond de populations par cellule du vivant : on évince d'abord la moins abondante, jamais la dernière d'une guilde (voie principale). « Dépassent le plafond » : part des cellules peuplées qui dépassaient le plafond avant éviction, presque toujours à cause d'arrivants du pas. « Saturées » : part des cellules qui ont perdu une population établie (plus que la biomasse d'un fondateur) ; au-delà de 2 % sur les 100 derniers pas (monde mûr), la règle est à revoir. Tunnel : au plus 2 essais par génotype et par pas, chacun pondéré par (candidats / essais) quand la borne est atteinte ; « borne atteinte » : part des génotypes candidats au tunnel qui avaient plus de candidats que d'essais.

| Monde | Graine | Dépassent le plafond (partie) | Dépassent le plafond (100 derniers pas) | Saturées (partie) | Saturées (100 derniers pas) | dont population encore en croissance | Borne du tunnel atteinte | Essais du tunnel | Réussites |
|---|---|---|---|---|---|---|---|---|---|
| Terre (Archéen) | 2026 | 76.0 % | 85.0 % | 2.80 % | 0.41 % | 0.36 % | 83.3 % | 72207129 | 20998 |
| Monde océan | 2026 | 83.1 % | 100.0 % | 2.54 % | 0.75 % | 0.72 % | 83.1 % | 137727743 | 37860 |
| Monde désertique | 2026 | 85.3 % | 96.1 % | 3.10 % | 1.86 % | 1.72 % | 86.2 % | 61610312 | 14312 |
| Super-Terre | 2026 | 84.5 % | 99.9 % | 3.61 % | 1.91 % | 1.74 % | 82.8 % | 108407627 | 30908 |
| Petite planète | 2026 | 55.7 % | 68.0 % | 1.04 % | 0.78 % | 0.72 % | 82.9 % | 87901328 | 15008 |
| Monde sans lune | 2026 | 79.4 % | 85.8 % | 3.44 % | 0.38 % | 0.34 % | 82.0 % | 70943859 | 21952 |
