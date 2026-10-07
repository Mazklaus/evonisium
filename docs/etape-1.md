# Étape 1 : prototype scientifique sans affichage

Feuille de route (document Vision) : « Grille planète fixe, populations microbiennes, génétique en régime apparition puis fixation ». Porte : « tests théoriques au vert (Hardy-Weinberg, Kimura), budgets de calcul mesurés ».

## La porte

**Tests théoriques.** `cargo test --workspace` lance 32 tests, dont les tests de validation scientifique de `crates/evo-genetics/tests/validation.rs`. Chaque test a une graine fixe et un seuil d'environ 4 écarts types.

| Test | Théorie | Simulation |
|---|---|---|
| Hardy-Weinberg après une génération, départ sans hétérozygotes (p = 0,3, 100 000 individus) | 0,4200 d'hétérozygotes | 0,4206, χ² = 0,90 (seuil 10,83) |
| Hardy-Weinberg maintenu sur 10 générations, dérive de p | χ² < 10,83 à chaque génération | vérifié |
| Kimura, mutant avantageux (N = 100, s = 0,02) | 0,03994 | 0,03963 ± 0,00098 |
| Kimura, mutant avantageux (N = 500, s = 0,01) | 0,01980 | 0,01945 ± 0,00069 |
| Kimura, mutant neutre (N = 100) | 0,01000 | 0,01020 ± 0,00050 |
| Kimura, mutant désavantageux (N = 100, s = −0,005) | 0,00585 | 0,00610 ± 0,00039 |
| Kimura, sélection forte (N = 50, s = 0,1) | 0,18128 | 0,18045 ± 0,00272 |
| Kimura depuis une fréquence de 20 % | formule générale u(p₀) | vérifié |
| Apparition puis fixation : taux de substitution neutre = taux de mutation | u | 1,002·u et 1,000·u |
| Apparition puis fixation contre Wright-Fisher explicite (N = 50, N·u = 0,02, s = 0,05) | 767 substitutions | 790 (régime), 833 (explicite) |

La simulation de référence est une population de Wright-Fisher suivie génération par génération ; la formule de Kimura et le régime « apparition puis fixation » du moteur doivent la reproduire.

**Budgets de calcul.** Mesurés par `evonisium bench`, rapport complet dans [etape-1-mesures.md](etape-1-mesures.md) (machine de mesure à 4 fils ; la cible du document Vision a 8 coeurs).

| Budget | Visé (document Vision) | Mesuré, 40 962 cellules | Mesuré, 163 842 cellules |
|---|---|---|---|
| Mémoire du monde physique | 0,5 à 1 Go | 9 Mo | 36 Mo |
| Mémoire des populations | 0,5 Go | 16 Mo | 64 Mo |
| Mémoire des génomes | 2 à 3 Go | 33 Mo (environ 1 Ko par génome distinct) | 117 Mo |
| Calcul génétique | 10⁷ à 10⁸ opérations par seconde | 5,9·10⁶ évaluations complètes de mutant par seconde et par fil, soit environ 4,7·10⁷ estimées sur 8 coeurs | idem |
| Pas complet du monde microbien | — | 208 ms | 871 ms |
| Monde microbien à 1 million d'années par seconde | 1 Ma/s | possible avec un pas de 210 ka ici, environ 100 ka sur 8 coeurs | pas de 870 ka ici, environ 435 ka sur 8 coeurs |

Ce que les mesures disent :

- La mémoire est très loin des budgets ; elle ne contraint rien à cette étape.
- Le temps de calcul est le vrai budget. À 40 962 cellules, 1 Ma/s demande des pas planétaires d'environ 100 000 ans sur la machine cible, ce qui reste fin à l'échelle de l'évolution microbienne. À 163 842 cellules, il faut des pas quatre fois plus longs.
- Recommandation : garder 40 962 cellules par défaut. Le passage à 163 842 n'est pas justifié tant que les modules de l'étape 2 (tectonique, climat, cycles) n'ont pas pris leur part du budget ; il reste disponible par l'option `--level 7`.

## Ce qui a été construit

| Crate | Contenu |
|---|---|
| `evo-core` | Hasard à graine dérivé de (graine, système, identifiants stables) : même histoire quel que soit le nombre de coeurs (testé à 1 et 4 fils). Horloge maître et abonnements des systèmes. Journal d'événements. Registre de flux qui vérifie la conservation du carbone. |
| `evo-planet` | Paramètres de planète, préréglage Terre archéenne (aucune constante terrestre ailleurs). Grille géodésique (icosaèdre subdivisé, 10·4ⁿ + 2 cellules, 12 pentagones). Relief aléatoire avec 71 % d'océan, climat moyen par latitude (bilan d'énergie, effet de l'obliquité, gradient thermique déduit de la gravité). Sources hydrothermales. Liste commune des 28 pools chimiques, dont 7 simulés dans l'eau de surface. |
| `evo-genetics` | Génome à domaines paramétrés (catalyse, pigment, réparation, défense contre l'oxygène) et marqueur neutre. Six classes de mutations : ponctuelle, perte de fonction, duplication, délétion, marqueur neutre, gène de novo. Taux de mutation porté par les gènes de réparation. Kimura, Wright-Fisher, Hardy-Weinberg, régime « apparition puis fixation ». Registre des lignées. |
| `evo-life` | Sept voies métaboliques (méthanogenèse, fermentation, respiration aérobie, sulfato-réduction, méthanotrophie, photosynthèses anoxygénique et oxygénique). Phénotype dérivé du génome. Taux de croissance r(g, c) et coefficient de sélection s = (r_mutant − r_résident) × T_génération, selon les formules du document Organismes. Dynamique des populations couplée à la chimie : la nécromasse nourrit les fermenteurs. |
| `evo-sim` | Le monde : écologie, évolution, migration, registres, en parallèle par cellule. Outil `evonisium` (`run`, `bench`). |

## Ce qu'on observe

Partie de référence : `evonisium run --level 6 --steps 300` (40 962 cellules, 3 millions d'années, graine 1).

- Les cellules minimales déposées sur 311 sources hydrothermales colonisent les 29 083 cellules océaniques en 600 000 ans environ.
- Elles s'adaptent à la température locale sans qu'aucune règle ne le leur dise : l'écart moyen entre l'optimum de leurs enzymes et l'eau où elles vivent passe de 4,5 K pendant la colonisation à 0,01 K après 3 millions d'années.
- Une diversification apparaît par perte de fonction : à côté de la cellule d'origine (méthanogenèse et fermentation), une guilde spécialisée de méthanogènes purs s'installe et coexiste partout avec elle.
- Les méthanogènes divisent par 200 l'hydrogène dissous et produisent du méthane ; le carbone est conservé à 10⁻¹⁴ près.
- La biomasse totale baisse d'un facteur 10 en 3 millions d'années pendant que l'hydrogène résiduel baisse : la sélection favorise les souches qui épuisent le mieux la ressource, au prix d'un entretien plus coûteux (compromis connu entre vitesse et rendement). C'est un résultat plausible, mais il dépend de constantes physiologiques non calibrées et sera à surveiller.
- La photosynthèse n'apparaît pas. Elle demande deux innovations à la fois (un pigment et une enzyme), et chacune seule est un coût : avec les grands effectifs microbiens, la sélection l'élimine. C'est exactement le cas de « stagnation » prévu par le document Génétique pour les accélérateurs. Il faudra le traiter à l'étape 2, dont la porte demande que l'oxygène s'accumule par photosynthèse.

## Simplifications de l'étape 1

Chacune s'ajoute à celles déjà listées dans les documents de conception.

| Simplification | Ce qu'on perd | Quand on la lève |
|---|---|---|
| Planète fixe : ni tectonique, ni climat transitoire, ni cycles géochimiques | Rétroactions du vivant sur la planète | Étape 2 |
| Une seule couche simulée, l'eau de surface des océans ; terres sans vie | Fonds, sédiments, sols, tapis microbiens côtiers | Étape 2 |
| Gaz sans stock atmosphérique (ils s'échappent) ; seul le carbone est comptabilisé | Accumulation d'oxygène et de méthane dans l'air | Étape 2 |
| Azote et phosphore non limitants | Limitation de la productivité | Étape 2 |
| Écologie quasi stationnaire : quelques jours de dynamique rapide par pas planétaire | Dynamiques transitoires fines | À mesurer |
| Une substitution au plus par population et par pas (pas d'interférence clonale) | La vitesse d'adaptation dépend un peu du pas choisi | À calibrer contre le niveau « individus échantillonnés » |
| Effectif efficace plafonné à 10⁸ | Effets des effectifs réels de 10²⁰ cellules | Valeur réaliste pour les bactéries |
| Un domaine par protéine, pas de réseau de régulation, haploïde et asexué | Régulation, plasticité, recombinaison | Étape 4 (développement) |
| Migration : seuls les génotypes passent d'une cellule à l'autre, pas la biomasse (sauf à la colonisation) | Flux de biomasse entre cellules | Étape 2 |
| Lignée créée seulement à une innovation métabolique ; pas encore d'espèces par distance génétique | Arbre du vivant détaillé | Étape 3 |

Les constantes physiologiques (`Physiology` dans `evo-life`) sont des ordres de grandeur de microbiologie, pas des valeurs calibrées : la calibration fine viendra avec les prototypes de l'étape 2.
