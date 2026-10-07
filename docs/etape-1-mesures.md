# Mesures des budgets de calcul — étape 1

Rapport produit par `evonisium bench`. Chaque mesure est une moyenne sur 20 pas après 30 pas de mise en route, avec des cellules minimales déposées dans toutes les cellules océaniques (charge maximale de l'étape 1).

Machine de mesure : Intel(R) Xeon(R) Processor @ 2.10GHz, 4 fils, 15.7 Go de mémoire. La machine cible du document Vision a 8 coeurs et 16 Go.

## Débit génétique (un fil)

| Opération | Par seconde |
|---|---|
| Évaluation complète d'un mutant (mutation, phénotype, taux de croissance) | 5.85e6 |
| Probabilité de fixation de Kimura | 4.28e7 |
| Estimation sur 8 coeurs (évaluations complètes) | 4.68e7 |

## Monde microbien complet

| Grille | Cellules | Cellules océaniques | Populations | Génomes distincts | Génération de la planète | Pas complet | dont écologie | dont évolution | dont migration | Évaluations génétiques par pas | Évaluations par seconde |
|---|---|---|---|---|---|---|---|---|---|---|---|
| niveau 6 | 40962 | 29083 | 58166 | 32952 | 0.41 s | 208 ms | 88 ms | 70 ms | 49 ms | 504144 | 2.42e6 |
| niveau 7 | 163842 | 116327 | 232647 | 125556 | 0.89 s | 871 ms | 344 ms | 280 ms | 242 ms | 2008377 | 2.31e6 |

## Vitesse du temps

Le document Vision vise environ 1 million d'années par seconde dans le monde microbien. La vitesse dépend du pas planétaire choisi : un pas plus long coûte le même calcul mais l'évolution y est plus grossière (une substitution au plus par population et par pas).

| Grille | Pas mesuré | Vitesse avec ce pas | Pas nécessaire pour 1 Ma/s ici | Même chose estimée sur 8 coeurs |
|---|---|---|---|---|
| niveau 6 | 10.0 ka | 48.1 ka par seconde | 207.9 ka | 104.0 ka |
| niveau 7 | 10.0 ka | 11.5 ka par seconde | 870.7 ka | 435.3 ka |

## Mémoire

| Grille | Monde physique | Populations | Génomes et phénotypes | Octets par génome distinct |
|---|---|---|---|---|
| niveau 6 | 8.9 Mo | 16.0 Mo | 33.3 Mo | 1061 |
| niveau 7 | 35.8 Mo | 64.1 Mo | 117.3 Mo | 980 |

Pic de mémoire résidente du processus pendant toutes les mesures : 519 Mo.

Extrapolation : 20 000 espèces × 50 génotypes de ce format occuperaient 0.91 Go. Les génomes de l'étape 1 sont ceux de cellules minimales (quelques gènes) ; un animal complexe en aura des milliers, d'où le stockage en différences par rapport au génome de référence prévu par le document Vision.
