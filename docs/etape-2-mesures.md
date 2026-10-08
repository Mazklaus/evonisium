# Mesures des budgets de calcul — étape 2

Rapport produit par `evonisium bench`. Chaque mesure est une moyenne sur 20 pas après 20 pas de mise en route, avec des cellules minimales déposées dans toutes les cellules océaniques (charge maximale). Le pas comprend désormais la planète vivante : tectonique (un pas sur dix à 100 000 ans), climat d'équilibre, boîtes chimiques globales, tunnel stochastique et transfert horizontal.

Machine de mesure : Intel(R) Xeon(R) Processor @ 2.80GHz, 4 fils, 15.7 Go de mémoire. La machine cible du document Vision a 8 coeurs et 16 Go, dont 2 réservés à l'affichage : la simulation en a 6.

## Débit génétique (un fil)

| Opération | Par seconde |
|---|---|
| Évaluation complète d'un mutant (mutation, phénotype, taux de croissance) | 2.73e6 |
| Probabilité de fixation de Kimura | 2.91e7 |
| Estimation sur 6 coeurs (évaluations complètes) | 1.64e7 |

## Monde microbien complet

| Grille | Cellules | Cellules océaniques | Populations | Génomes distincts | Génération de la planète | Pas complet | dont planète | dont écologie | dont évolution | dont migration | Évaluations génétiques par pas | Évaluations par seconde |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| niveau 6 | 40962 | 28791 | 307363 | 147036 | 1.89 s | 3096 ms | 15 ms | 1028 ms | 1297 ms | 661 ms | 3333401 | 1.08e6 |
| niveau 7 | 163842 | 117084 | 1089025 | 503776 | 8.32 s | 13017 ms | 66 ms | 3974 ms | 5594 ms | 2959 ms | 12707421 | 9.76e5 |

## Vitesse du temps

Le document Vision vise environ 1 million d'années par seconde dans le monde microbien. La vitesse dépend du pas planétaire choisi : un pas plus long coûte le même calcul mais l'évolution y est plus grossière (une substitution au plus par population et par pas).

| Grille | Pas mesuré | Vitesse avec ce pas | Pas nécessaire pour 1 Ma/s ici | Même chose estimée sur 6 coeurs |
|---|---|---|---|---|
| niveau 6 | 100.0 ka | 32.3 ka par seconde | 3.10 Ma | 2.06 Ma |
| niveau 7 | 100.0 ka | 7.7 ka par seconde | 13.02 Ma | 8.68 Ma |

## Mémoire

| Grille | Monde physique | Populations | Génomes et phénotypes | Octets par génome distinct |
|---|---|---|---|---|
| niveau 6 | 13.5 Mo | 192.2 Mo | 497.4 Mo | 3547 |
| niveau 7 | 53.9 Mo | 699.5 Mo | 1710.6 Mo | 3561 |

Pic de mémoire résidente du processus pendant toutes les mesures : 7187 Mo.

Extrapolation : 20 000 espèces × 50 génotypes de ce format occuperaient 3.32 Go. Les génomes de ces mesures sont ceux de microbes (quelques gènes) ; un animal complexe en aura des milliers, d'où le stockage en différences par rapport au génome de référence prévu par le document Vision.
