# Mesures des budgets de calcul : étape 3

Machine de mesure : Intel Xeon à 2,1 GHz, 4 fils, 15,7 Go. La machine cible du document Vision a 8 cœurs, dont 2 réservés à l'affichage : la simulation en a 6. L'estimation sur 6 cœurs suit la loi d'Amdahl, poste par poste : chaque poste est mesuré sur 1 fil et sur 4 fils, ce qui donne sa part parallèle, puis sa durée sur 6 fils en est déduite.

**En bref.** Sur un monde mûr, le plancher de 250 ka/s sur 6 cœurs est tenu au pas de 200 ka, avec deux tours d'évolution par pas (estimation : 285 ka/s ; 212 ka/s mesurés sur 4 fils). Il ne l'est pas au pas de 100 ka (estimation : 200 ka/s). La cible de 1 Ma/s reste pour l'étape 4.

## Monde mûr (chiffre de référence)

Terre au niveau 6 (40 962 cellules, vie au niveau 5), ensemencée dans tout l'océan, après 250 pas de 100 ka (25 Ma) : 69 000 populations. Moyenne sur 5 pas repris d'un même point de sauvegarde.

Pour reproduire :

```
evonisium chrono --save mur.evo --prepare 250
RAYON_NUM_THREADS=1 evonisium chrono --save mur.evo --steps 5 --step-years 200000
RAYON_NUM_THREADS=4 evonisium chrono --save mur.evo --steps 5 --step-years 200000
```

### Pas de 200 ka, deux tours d'évolution (réglage retenu pour la porte)

Un pas de plus de 100 ka enchaîne un tour « apparition puis fixation » par tranche de 100 ka. Sans cela, il ne donne pas la même évolution qu'un pas de 100 ka (voir [etape-3-equivalence.md](etape-3-equivalence.md)).

| Poste | 1 fil | 4 fils (mesuré) | Part parallèle | 6 fils (estimé) |
|---|---|---|---|---|
| planète (tectonique, climat, hydrologie, boîtes) | 17 ms | 15 ms | 16 % | 15 ms |
| écologie (30 sous-pas par cellule du vivant) | 349 ms | 98 ms | 96 % | 70 ms |
| évolution (deux tours, par dème) | 2 435 ms | 718 ms | 94 % | 527 ms |
| migration | 239 ms | 70 ms | 94 % | 51 ms |
| registres | 39 ms | 44 ms | 0 % | 39 ms |
| **pas complet** | 3 079 ms | 945 ms | | **702 ms** |

Vitesse : **212 ka/s mesurés sur 4 fils, 285 ka/s estimés sur 6 cœurs.**

### Pas de 100 ka

| Poste | 1 fil | 4 fils (mesuré) | Part parallèle | 6 fils (estimé) |
|---|---|---|---|---|
| planète | 17 ms | 19 ms | 0 % | 17 ms |
| écologie | 353 ms | 97 ms | 97 % | 69 ms |
| évolution | 1 193 ms | 412 ms | 87 % | 325 ms |
| migration | 228 ms | 68 ms | 94 % | 50 ms |
| registres | 39 ms | 40 ms | 0 % | 39 ms |
| **pas complet** | 1 830 ms | 636 ms | | **500 ms** |

Vitesse : 157 ka/s mesurés sur 4 fils, 200 ka/s estimés sur 6 cœurs.

### Pas de 200 ka, un seul tour (pour comparaison, non retenu)

| Poste | 1 fil | 4 fils (mesuré) | Part parallèle | 6 fils (estimé) |
|---|---|---|---|---|
| planète | 19 ms | 18 ms | 7 % | 18 ms |
| écologie | 345 ms | 89 ms | 99 % | 61 ms |
| évolution | 1 309 ms | 372 ms | 95 % | 268 ms |
| migration | 218 ms | 66 ms | 93 % | 49 ms |
| registres | 35 ms | 37 ms | 0 % | 35 ms |
| **pas complet** | 1 926 ms | 582 ms | | **430 ms** |

Vitesse : 344 ka/s sur 4 fils, 465 ka/s estimés sur 6 cœurs. Ce réglage divise par deux les substitutions et retarde l'oxygène de 24 Ma en moyenne.

### Où passe le temps

L'évolution prend les trois quarts du pas : 860 000 mutants construits et jugés par tour (génome, phénotype, taux de croissance, coefficient de sélection). C'est le poste que l'étape 4 doit réduire pour viser 1 Ma/s. La feuille de route y prévoit la construction incrémentale des phénotypes et un cache des phénotypes par génotype. Les registres (lignées éteintes, innovations, historique) ne sont pas parallèles mais ne pèsent que 40 ms.

## Monde jeune (chiffre de la première mesure, pour comparaison)

Rapport de `evonisium bench --levels 6` : même planète, 20 pas de mise en route seulement (65 000 populations, mais des génomes de quelques gènes). Le chiffre de 250 ka/s annoncé le 2026-10-08 à midi venait de ce cas.

| Poste | 1 fil | 4 fils (mesuré) | Part parallèle | 6 fils (estimé) |
|---|---|---|---|---|
| planète | 21 ms | 16 ms | 32 % | 15 ms |
| écologie | 275 ms | 67 ms | 100 % | 46 ms |
| évolution | 636 ms | 155 ms | 100 % | 106 ms |
| migration | 152 ms | 40 ms | 98 % | 28 ms |
| registres | 27 ms | 30 ms | 0 % | 27 ms |
| **pas complet** | 1 111 ms | 308 ms | | **222 ms** |

Au pas de 100 ka : 324 ka/s sur 4 fils, 451 ka/s estimés sur 6 cœurs. Sur un monde mûr, l'évolution coûte deux à trois fois plus cher : les génomes ont grandi (duplications, transferts), et chaque mutant coûte d'autant plus à construire et à juger.

## Plafond de populations par cellule

Monde mûr (même préparation, 250 pas de 100 ka), 20 pas de 200 ka à deux tours d'évolution, 4 fils. « Saturées » : cellules peuplées qui dépassaient le plafond avant éviction. « Établie évincée » : l'éviction a retiré une population de plus que la biomasse d'un fondateur (100 mol C), c'est-à-dire qui avait grandi depuis son arrivée.

```
evonisium chrono --save mur.evo --steps 20 --step-years 200000 --cap 12
```

| Plafond | Populations | Cellules saturées | Établie évincée | Pas complet | dont évolution |
|---|---|---|---|---|---|
| 8 (réglage actuel) | 69 000 | 51,9 % | 0,1 % | 937 ms | 692 ms |
| 12 | 97 000 | 34,2 % | 0,0 % | 1 182 ms | 884 ms |
| 16 | 129 000 | 18,8 % | 0,0 % | 1 395 ms | 1 045 ms |

Les populations n'avaient pas fini de croître au bout de 20 pas avec les plafonds 12 et 16 : leur part de cellules saturées remonterait un peu.

## Points de sauvegarde

Même monde mûr : 121 à 133 Mo sur disque. Écriture en 4 s (25 s avant correction : bincode écrivait champ par champ dans le compresseur, sans tampon), lecture en 4 à 7 s. Avant compression : lignées 232 Mo (735 000 fiches, dont 100 000 génomes fondateurs), journal des génomes 172 Mo, événements 40 Mo.
