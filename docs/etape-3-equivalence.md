# Équivalence des pas de 100 ka et de 200 ka (étape 3)

Le document d'architecture (« Correction sur monde mûr ») accepte la porte de l'étape 3 au pas de 200 ka, à condition qu'un test montre que ce pas donne la même évolution qu'un pas de 100 ka. Il demande au moins 3 graines et trois grandeurs : la date de montée de l'O₂, les substitutions par million d'années et le nombre de guildes.

**Résultat.** Avec un seul tour « apparition puis fixation » par pas, le pas de 200 ka n'est pas équivalent. Presque chaque génotype fixe un changement à chaque pas (94 % au pas de 100 ka), si bien que le nombre de substitutions est plafonné par le nombre de pas. À 200 ka, il est divisé par deux (18 900 par Ma contre 37 700), les substitutions avantageuses aussi, et l'oxygène monte en moyenne 24 Ma plus tard. Le moteur enchaîne donc désormais un tour par tranche de 100 ka du pas (`EvolutionParams::round_years`, 100 ka par défaut). Avec ces deux tours, les six grandeurs comparées sont équivalentes aux fluctuations près entre graines.

Ce que cela coûte : l'évolution est calculée deux fois par pas de 200 ka. Les vitesses qui en résultent sont dans [etape-3-mesures.md](etape-3-mesures.md).

Pour reproduire (niveau 4, chaque partie en une à deux minutes) :

```
evonisium equivalence --level 4 --years 150e6 --seeds 2026,7,42,1,2 --save-results DOSSIER --out rapport.md
evonisium equivalence --level 4 --years 150e6 --seeds 2026,7,42,1,2 --round-years 0 ...   # un seul tour par pas
```

Le test est fait au niveau 4 (vie au niveau 3), quatre fois moins de cellules qu'à la porte, pour tenir dix parties de 150 Ma. Le pas agit sur l'évolution de chaque génotype, pas sur la résolution. Les tests unitaires `a_long_step_runs_one_evolution_round_per_slice` et `the_population_cap_never_evicts_the_last_of_a_guild` vérifient le découpage en tours et le plafond.

## Deux tours par pas de 200 ka (réglage retenu)

Rapport produit par `evonisium equivalence`. Monde « terre », grille de niveau 4 (vie un niveau en dessous), graines [2026, 7, 42, 1, 2], 150.00 Ma simulés par partie, chaque partie jouée aux pas de 100.0 ka et 200.0 ka, l'évolution enchaînant un tour « apparition puis fixation » par tranche de 100.0 ka du pas. Les deux pas partent de la même graine, mais le tirage des mutations dépend du numéro du pas : les parties divergent dans le détail, et l'équivalence se juge sur la moyenne des graines. Critère : l'écart des moyennes entre pas ne dépasse pas deux fois l'écart type entre graines. « Guildes » : voies principales (la voie utilisable qui porte le plus d'efficacité enzymatique) ; les combinaisons de voies sont données pour information.

| Graine | Pas | O₂ > 10⁻⁶ | O₂ > 1e-4 | Photosynthèse oxygénique | Substitutions par Ma | dont avantageuses | Guildes (planète) | Guildes par cellule | Combinaisons de voies | Populations | O₂ final | Calcul |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 2026 | 100.0 ka | 49.10 Ma | 49.70 Ma | 15.80 Ma | 38436 | 28632 | 7 | 4.20 | 133 | 4226 | 6.5e-2 | 123 s |
| 7 | 100.0 ka | 50.90 Ma | 51.10 Ma | 30.60 Ma | 37926 | 28786 | 8 | 4.14 | 115 | 3593 | 2.8e-1 | 89 s |
| 42 | 100.0 ka | 26.30 Ma | 26.70 Ma | 15.60 Ma | 36551 | 24083 | 9 | 3.89 | 105 | 4746 | 2.1e-2 | 60 s |
| 1 | 100.0 ka | 69.30 Ma | 70.40 Ma | 34.70 Ma | 39887 | 29336 | 6 | 4.18 | 115 | 3972 | 1.1e-1 | 72 s |
| 2 | 100.0 ka | 50.80 Ma | 51.50 Ma | 29.00 Ma | 35940 | 25390 | 7 | 3.62 | 97 | 3190 | 1.5e-1 | 62 s |
| 2026 | 200.0 ka | 56.20 Ma | 56.60 Ma | 26.40 Ma | 38272 | 27928 | 7 | 3.78 | 109 | 4037 | 4.3e-2 | 65 s |
| 7 | 200.0 ka | 43.20 Ma | 43.60 Ma | 3.60 Ma | 39446 | 29192 | 9 | 4.24 | 146 | 3971 | 1.1e-1 | 62 s |
| 42 | 200.0 ka | 68.60 Ma | 69.00 Ma | 33.40 Ma | 40901 | 30392 | 7 | 4.19 | 126 | 4678 | 6.6e-2 | 73 s |
| 1 | 200.0 ka | 48.00 Ma | 49.00 Ma | 25.40 Ma | 41569 | 30174 | 7 | 4.41 | 137 | 4020 | 4.6e-2 | 58 s |
| 2 | 200.0 ka | 103.20 Ma | 103.80 Ma | 2.40 Ma | 42803 | 31287 | 7 | 4.25 | 117 | 4603 | 6.1e-2 | 83 s |

### Comparaison

| Grandeur | Pas de 100.0 ka (moyenne ± écart type) | Pas de 200.0 ka (moyenne ± écart type) | Écart des moyennes | Verdict |
|---|---|---|---|---|
| Montée de l'O₂ au-dessus du seuil | 49.88 Ma ± 15.51 Ma | 64.40 Ma ± 24.00 Ma | 14.52 Ma | équivalent |
| Photosynthèse oxygénique | 25.14 Ma ± 8.86 Ma | 18.24 Ma ± 14.26 Ma | 6.90 Ma | équivalent |
| Substitutions par Ma | 37748.17 ± 1563.88 | 40598.23 ± 1777.37 | 2850.06 | équivalent |
| Substitutions avantageuses par Ma | 27245.32 ± 2351.07 | 29794.36 ± 1282.15 | 2549.04 | équivalent |
| Guildes (planète) | 7.40 ± 1.14 | 7.40 ± 0.89 | 0.00 | équivalent |
| Guildes par cellule | 4.00 ± 0.25 | 4.17 ± 0.23 | 0.17 | équivalent |

**Verdict : les deux pas donnent la même évolution aux fluctuations près**

## Un seul tour par pas (avant correction)

Rapport produit par `evonisium equivalence`. Monde « terre », grille de niveau 4 (vie un niveau en dessous), graines [2026, 7, 42, 1, 2], 150.00 Ma simulés par partie, chaque partie jouée aux pas de 100.0 ka et 200.0 ka, avec un seul tour « apparition puis fixation » par pas. Les deux pas partent de la même graine, mais le tirage des mutations dépend du numéro du pas : les parties divergent dans le détail, et l'équivalence se juge sur la moyenne des graines. Critère : l'écart des moyennes entre pas ne dépasse pas deux fois l'écart type entre graines. « Guildes » : voies principales (la voie utilisable qui porte le plus d'efficacité enzymatique) ; les combinaisons de voies sont données pour information.

| Graine | Pas | O₂ > 10⁻⁶ | O₂ > 1e-4 | Photosynthèse oxygénique | Substitutions par Ma | dont avantageuses | Guildes (planète) | Guildes par cellule | Combinaisons de voies | Populations | O₂ final | Calcul |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 2026 | 100.0 ka | 49.10 Ma | 49.70 Ma | 15.80 Ma | 38436 | 28632 | 7 | 4.20 | 133 | 4226 | 6.5e-2 | 123 s |
| 7 | 100.0 ka | 50.90 Ma | 51.10 Ma | 30.60 Ma | 37926 | 28786 | 8 | 4.14 | 115 | 3593 | 2.8e-1 | 89 s |
| 42 | 100.0 ka | 26.30 Ma | 26.70 Ma | 15.60 Ma | 36551 | 24083 | 9 | 3.89 | 105 | 4746 | 2.1e-2 | 60 s |
| 1 | 100.0 ka | 69.30 Ma | 70.40 Ma | 34.70 Ma | 39887 | 29336 | 6 | 4.18 | 115 | 3972 | 1.1e-1 | 72 s |
| 2 | 100.0 ka | 50.80 Ma | 51.50 Ma | 29.00 Ma | 35940 | 25390 | 7 | 3.62 | 97 | 3190 | 1.5e-1 | 62 s |
| 2026 | 200.0 ka | 50.40 Ma | 53.20 Ma | 20.60 Ma | 18428 | 13795 | 8 | 3.88 | 91 | 3963 | 9.9e-2 | 40 s |
| 7 | 200.0 ka | 45.20 Ma | 47.60 Ma | 22.80 Ma | 17917 | 13891 | 7 | 4.10 | 95 | 3712 | 8.5e-2 | 35 s |
| 42 | 200.0 ka | 72.20 Ma | 73.00 Ma | 25.00 Ma | 18780 | 13922 | 7 | 3.85 | 85 | 4248 | 2.5e-1 | 36 s |
| 1 | 200.0 ka | 78.20 Ma | 79.20 Ma | 44.00 Ma | 19691 | 14749 | 7 | 4.27 | 89 | 4273 | 5.9e-2 | 36 s |
| 2 | 200.0 ka | 114.80 Ma | 116.60 Ma | 87.20 Ma | 19713 | 15280 | 7 | 4.34 | 68 | 4616 | 2.7e-2 | 37 s |

### Comparaison

| Grandeur | Pas de 100.0 ka (moyenne ± écart type) | Pas de 200.0 ka (moyenne ± écart type) | Écart des moyennes | Verdict |
|---|---|---|---|---|
| Montée de l'O₂ au-dessus du seuil | 49.88 Ma ± 15.51 Ma | 73.92 Ma ± 27.26 Ma | 24.04 Ma | équivalent |
| Photosynthèse oxygénique | 25.14 Ma ± 8.86 Ma | 39.92 Ma ± 28.02 Ma | 14.78 Ma | équivalent |
| Substitutions par Ma | 37748.17 ± 1563.88 | 18905.66 ± 788.78 | 18842.51 | écart |
| Substitutions avantageuses par Ma | 27245.32 ± 2351.07 | 14327.55 ± 656.39 | 12917.77 | écart |
| Guildes (planète) | 7.40 ± 1.14 | 7.20 ± 0.45 | 0.20 | équivalent |
| Guildes par cellule | 4.00 ± 0.25 | 4.09 ± 0.22 | 0.09 | équivalent |

**Verdict : écart sur au moins une grandeur (voir le tableau)**
