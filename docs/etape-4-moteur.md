# Étape 4 : moteur microbien, calibrage et vitesse

Périmètre (document d'architecture, « Périmètre consolidé de l'étape 4 ») : bilan d'électrons exact sans correction, régulation de l'O₂ par l'enfouissement du carbone limité par le phosphore, tunnel analytique et innovations tirées selon une loi de Poisson, test de saturation de la référence à 100 ka, chronologie calée sur la Terre à un facteur 3 près, cache des phénotypes, pas adaptatif et 1 Ma/s sur monde mûr.

Mesures au niveau 4 (2 562 cellules physiques), graine 2026, sauf mention contraire. La porte au niveau 6 n'a pas été repassée.

## Ce qui est fait

**Bilan d'électrons exact.** La correction de l'étape 3 (jusqu'à 37 % sur la petite planète) est supprimée. Chaque cellule prolonge ses flux de surface sur le pas en fermant exactement carbone, phosphore et électrons (`steady_rates`) ; la croissance autotrophe est réglée voie par voie sur le réducteur réellement consommé. Quand une boîte globale vide freine un prélèvement, ce qu'il alimentait est freiné avec lui (`pair_throttled`). Écart restant : 3·10⁻¹³ à 1,2·10⁻¹¹ de la production d'O₂, arrondis seulement ; part non reprise : 0 %.

**Régulation de l'O₂.** Trois changements dans evo-planet :
- l'océan profond passe de l'oxygéné à l'anoxique quand la demande de la matière organique exportée dépasse l'apport de la ventilation (`deep_oxic`, loi de Hill), et le rapport C/P enfoui suit (250 oxique, 4 000 anoxique, Van Cappellen et Ingall, 1994) ;
- le phosphore dissous a un puits indépendant de l'oxygène, l'apatite authigène (temps de séjour 100 ka) : sans lui, il montait à mille fois la Terre sous l'océan anoxique et l'O₂ s'emballait ensuite jusqu'à 390 % PAL ;
- l'oxydation des roches réduites suit l'altération des silicates des terres (3 mol d'O₂ par mol de CO₂ à 1 PAL, exposant 0,5) au lieu de la surface des terres : l'érosion qui apporte le phosphore expose aussi le kérogène et la pyrite.

**Innovations et tunnel.** Les mutants innovants (de novo, duplication suivie de divergence) apparaissent selon une loi de Poisson de paramètre apparitions × probabilité d'innovation (π = 10⁻¹³ par défaut) ; le tunnel de Weissman et coll. (2009) est calculé analytiquement pour chacun d'eux qui ne se fixe pas seul, sans borne d'essais.

**Éviction sous le plafond.** Classer selon la fitness d'invasion (`--eviction invasion`) a été mesuré : 47 % de cellules saturées sur les 100 derniers pas contre 1,3 % avec la biomasse, et une production primaire divisée par cinq. À l'équilibre, chaque résident a un r proche de zéro, souvent un peu négatif, et un fondateur qui arrive un r positif : la règle évince les résidents établis. Vision a gardé la biomasse ; l'option reste, désactivée.

**Cache.** Les capacités enzymatiques du résident sont calculées une fois par cellule jugée ; celles d'un mutant le sont par différence de sa liste d'enzymes (`RawCapacities`). La construction incrémentale des phénotypes appartient au fil « cellules complexes ».

**Pas adaptatif.** Un pas qui s'achève sans événement notable (intérêt ≥ 0,5) ni variation de plus de 10 % de l'O₂ allonge le suivant d'un pas demandé, jusqu'à trois fois ; le moindre événement notable le ramène au pas demandé. La décision ne dépend que de l'état simulé (rejeu identique, testé). L'évolution garde un tour par 100 ka. `--pas-fixe` le désactive pour la porte ; l'équivalence compare des pas fixes.

## La porte au niveau 4

Rapport complet : [etape-4-porte-n4.md](etape-4-porte-n4.md). Maintien de l'O₂ au-dessus de 10⁻⁴ pendant 50 Ma.

| Monde | Photosynthèse oxygénique | O₂ > 10⁻⁴ | O₂ en fin de maintien | Accélérateur | Verdict |
|---|---|---|---|---|---|
| Terre (Archéen) | 317 Ma | 340 Ma | 97 % PAL | jamais | franchie |
| Monde océan | 620 Ma | 630 Ma | 137 % PAL | 21 pas | franchie |
| Monde désertique | 616 Ma | 631 Ma | 85 % PAL | 10 pas | franchie |
| Super-Terre | 420 Ma | 441 Ma | 65 % PAL | jamais | franchie |
| Petite planète | 627 Ma | 647 Ma | vie éteinte à 657 Ma | 27 pas | non franchie |
| Monde sans lune | 607 Ma | 1,61 Ga | 401 % PAL, encore en montée | 4 pas | franchie |

Carbone, phosphore et registre des électrons conservés à 4·10⁻¹¹ près ; rejeu identique sur les six mondes. Vitesse des parties entières : 2,5 à 4,2 Ma/s sur 4 fils (mondes jeunes).

## Chronologie

Repères terrestres, comptés depuis l'apparition de la vie (environ 3,8 Ga) : photosynthèse anoxygénique vers +400 Ma, oxygénique vers +800 à +1 100 Ma (fenêtre à un facteur 3 : 300 Ma à 2,7 Ga), grande oxydation vers +1,4 Ga (fenêtre : 470 Ma à 4,2 Ga).

- Photosynthèse oxygénique : 317 à 627 Ma, dans la fenêtre sur les six mondes. Sur la Terre et la super-Terre, l'évolution seule l'atteint ; sur les quatre autres, juste après le réveil de l'accélérateur (patience portée à 600 Ma), qui est compté.
- Montée de l'O₂ : 10 à 25 Ma après la photosynthèse oxygénique au lieu de plusieurs centaines de Ma sur Terre. Terre (340 Ma) et super-Terre (441 Ma) tombent sous la fenêtre ; **accélération déclarée** : les puits réducteurs (gaz volcaniques, fer) ne retiennent pas l'O₂ assez longtemps.
- Premières étapes (pigment, phototrophie simple, anoxygénique) : 1 à 11 Ma, bien trop tôt ; ce sont des mutations courantes, que π ne règle pas.

## Saturation de la référence à 100 ka

Terre, niveau 4, pas fixe de 100 ka, graines 2026, 7 et 42, 600 Ma.

| Tours par 100 ka | Substitutions par Ma | dont avantageuses | Guildes par cellule |
|---|---|---|---|
| 1 | 35 300 ± 4 500 | 13 600 ± 2 500 | 1,90 |
| 2 | 70 000 ± 1 800 | 25 900 ± 1 700 | 1,95 |

La référence est saturée : chaque tour fixe presque toujours, et doubler les tours double les substitutions. La chronologie des innovations est calée en tours (π), indépendamment du pas ; les fixations multiples tirées selon une loi de Poisson ne sont pas faites (à trancher par Vision : un tour par 50 ka double le coût de l'évolution).

## Vitesse sur monde mûr

Monde mûr : Terre, niveau 6, 250 pas depuis un ensemencement de tout l'océan (`chrono --prepare 250`), 61 000 populations. Pas adaptatif au maximum (300 ka, trois tours d'évolution).

| Fils | Durée d'un pas | dont évolution | Vitesse |
|---|---|---|---|
| 1 | 3,64 s | 2,71 s | 82 ka/s |
| 4 | 1,31 s | 0,92 s | 229 ka/s |

Estimation sur 6 cœurs : environ 320 ka/s. **La cible de 1 Ma/s n'est pas atteinte.** L'évolution prend 70 % du pas (1,4 million d'évaluations génétiques par pas). Leviers restants : construction incrémentale des phénotypes (fil « cellules complexes », environ un tiers de l'évolution), moins de candidats ponctuels par génotype, gonflement des génomes (ci-dessous).

## Limites connues

- **Petite planète.** La montée de l'O₂ détruit le méthane, la planète gèle entière (eau à 271 K sous la glace) et toute la vie meurt en 8 Ma : les génomes gonflés (50 à 100 gènes, entretien jusqu'à 10⁵ kJ par mole de carbone et par an) ne paient plus leur entretien au froid. Deux corrections essayées puis retirées : compromis thermique au niveau de la voie (le gonflement ne vient pas de l'étalement des optimums) et Q10 de l'entretien (l'O₂ ne monte plus nulle part en 2 Ga). Il manque des refuges (sources chaudes, glace mince).
- **Gonflement des génomes.** Copies de petite efficacité d'une même voie qui s'additionnent (jusqu'à 400 gènes) ; c'est aussi un coût de vitesse, toute l'évolution étant en O(gènes).
- **Monde sans lune** à 401 % PAL à la fin du maintien, encore en montée ; **monde océan** à 25 bar de CO₂ et 372 K (sans terres, ni altération ni apport de phosphore continental).
- **Électrons freinés** : 23 à 86 % de la production d'O₂ est déplacée quand une boîte vide freine un prélèvement ; tout est apparié, mais c'est le signe que le prolongement des flux sur le pas surestime les prélèvements.
- **Saturation des cellules** sur les 100 derniers pas : 1,7 à 11 % juste après la montée de l'O₂ (renouvellement des communautés), au-dessus du seuil de 2 % ; 0 % sur le monde mûr.
