# Étape 4 : moteur microbien, calibrage et vitesse

Périmètre (document d'architecture, « Périmètre consolidé de l'étape 4 ») : bilan d'électrons exact sans correction, régulation de l'O₂ par l'enfouissement du carbone limité par le phosphore, tunnel analytique et innovations tirées selon une loi de Poisson, test de saturation de la référence à 100 ka, chronologie calée sur la Terre à un facteur 3 près, cache des phénotypes, pas adaptatif et 1 Ma/s sur monde mûr.

Mesures de la suite (PR #10) : porte au niveau 6 sur les six mondes, graine 2026, avec un tour d'évolution par 300 ka de temps simulé (décision de l'utilisateur du 9 octobre). Le détail par monde est dans [etape-4-porte-n6.md](etape-4-porte-n6.md).

## Suite : ce qui a changé

**Fixations multiples.** Tous les candidats qui se fixent au cours d'un tour, chacun selon sa probabilité (une loi de Poisson quand l'offre n'est pas saturée), sont réunis gène par gène dans un même génome (`combine_changes`), jugé une fois avec un phénotype construit en entier ; la réunion est gardée si elle vaut au moins le meilleur seul et garde sa guilde. Les changements d'organites et de blocs ne se combinent pas. `--fixation-unique` revient à l'ancien comportement.

**Tours comptés sur le temps simulé.** Le moteur fait un tour par tranche de 300 ka écoulée (`EvolutionParams::round_years`, horloge `Progress::evolution_clock`) ; le reste passe au pas suivant. Le nombre de tours ne dépend donc ni de la durée du pas ni de son allongement. Format de sauvegarde 6.

**Le curseur de vitesse ne touche plus l'histoire.** Le client réglait la durée du pas d'après la vitesse (un ordre `SetStepYears` à chaque changement) ; il ne fait plus que freiner le moteur. Le pas, ses allongements, les tours d'évolution et le mode du climat (équilibre au-delà de 10 ka) ne dépendent que du temps simulé et de l'état du monde. Test `the_speed_cursor_never_changes_history` (evo-engine) : la même partie jouée d'un trait à toute vitesse, puis freinée à 4 Ma/s, relâchée, freinée à 1 Ma/s puis à 10 Ma/s avec la caméra déplacée, arrive au même état, bit pour bit. Aux vitesses lentes (1 ka/s), un pas de 100 à 300 ka met donc plusieurs minutes à s'afficher ; l'interpolation entre deux états publiés revient au client.

**Biais de délétion.** Les pseudogènes (gènes sans fonction) ont leur propre tirage de délétions, proportionnel à leur part du génome ; ils sont purgés au lieu de s'accumuler (4 % de gènes inactifs à 600 Ma).

**Tectonique stable.** Les continents couvraient puis étaient noyés, et les plaques fusionnaient jusqu'à une seule : plus de terres ni de dorsales, 6 bar de CO₂ et 340 K à 2 Ga. Corrigé : la parcelle la plus proche de chaque plaque survit, la surface continentale est tenue à sa fraction, les plaques sont redécoupées à chaque réorganisation. La Terre reste entre 286 et 290 K sur 2 Ga.

**Phosphore et oxygène.** L'enfouissement du carbone est plafonné par le phosphore profond (rapport C/P) ; l'altération du plancher océanique suit la température (énergie d'activation équivalente à 40 K) et libère du phosphore. L'O₂ ne monte plus à 360 % PAL.

**Refuges sous la glace.** Les sources hydrothermales gardent une eau à 283 K sous la banquise : la petite planète survit à sa glaciation.

**π gardé à 3·10⁻¹⁴.** Avec les tours de 300 ka, la porte au niveau 4 passe sur les six mondes sans recalage (ci-dessous).

**Points de reprise.** `evonisium porte --reprise DOSSIER` sauve le monde et l'état de la porte tous les 50 pas ; une partie interrompue reprend de là et donne le même rapport qu'une partie d'un seul tenant (vérifié). Les parties au niveau 6 durent 30 à 100 minutes chacune.

## La porte au niveau 6

Maintien de l'O₂ au-dessus de 10⁻⁴ pendant 50 Ma, graine 2026, 4 fils.

| Monde | Photosynthèse oxygénique | O₂ > 10⁻⁴ | O₂ en fin de maintien | Température | Vitesse de la partie | Verdict |
|---|---|---|---|---|---|---|
| Terre (Archéen) | 188 Ma | 1,21 Ga | 65 % PAL | 285 K | 390 ka/s | franchie |
| Monde océan | 634 Ma | 646 Ma | 11 % PAL | 324 K | 373 ka/s | franchie |
| Monde désertique | 583 Ma | 1,65 Ga | 110 % PAL | 276 K | 403 ka/s | franchie |
| Super-Terre | 616 Ma | 635 Ma | 8 % PAL | 288 K | 240 ka/s | franchie |
| Petite planète | 617 Ma | 1,64 Ga | 83 % PAL | 269 K | 315 ka/s | franchie |
| Monde sans lune | 616 Ma | 1,63 Ga | 68 % PAL | 283 K | 277 ka/s | franchie |

La photosynthèse oxygénique vient du moteur sur les six mondes (l'accélérateur aide ailleurs, compté au rapport). Carbone, phosphore et électrons conservés à 10⁻¹⁰ près ; rejeu identique sur les six.

Au niveau 4 (même code, graine 2026), la porte passe aussi sur les six mondes : photosynthèse oxygénique entre 327 et 682 Ma, O₂ au seuil entre 347 Ma et 1,76 Ga, de 8 à 144 % PAL en fin de maintien.

## Chronologie

Repères terrestres, comptés depuis l'apparition de la vie (environ 3,8 Ga) : photosynthèse oxygénique vers +800 à +1 100 Ma (fenêtre à un facteur 3 : 300 Ma à 2,7 Ga), grande oxydation vers +1,4 Ga (fenêtre : 470 Ma à 4,2 Ga).

- Photosynthèse oxygénique : 583 à 634 Ma sur cinq mondes, dans la fenêtre ; la Terre à 188 Ma (graine 2026, niveau 6), un peu tôt. Au niveau 4, la Terre l'a à 672 Ma.
- Montée de l'O₂ : sur la Terre, le désert, la petite planète et le monde sans lune, 1 Ga environ après la photosynthèse oxygénique, comme sur Terre ; les puits réducteurs tiennent maintenant. Sur le monde océan et la super-Terre, encore 12 à 19 Ma après : **accélération déclarée**, la vraie cause revient au chantier Planète (étape 5).
- Premières étapes (pigment, phototrophie simple, anoxygénique) : 2 à 16 Ma, bien trop tôt ; ce sont des mutations courantes, que π ne règle pas.

## PR #8 : ce qui était fait

**Bilan d'électrons exact.** La correction de l'étape 3 (jusqu'à 37 % sur la petite planète) est supprimée. Chaque cellule prolonge ses flux de surface sur le pas en fermant exactement carbone, phosphore et électrons (`steady_rates`) ; la croissance autotrophe est réglée voie par voie sur le réducteur réellement consommé. Quand une boîte globale vide freine un prélèvement, ce qu'il alimentait est freiné avec lui (`pair_throttled`). Écart restant : 3·10⁻¹³ à 1,2·10⁻¹¹ de la production d'O₂, arrondis seulement ; part non reprise : 0 %.

**Régulation de l'O₂.** Trois changements dans evo-planet :
- l'océan profond passe de l'oxygéné à l'anoxique quand la demande de la matière organique exportée dépasse l'apport de la ventilation (`deep_oxic`, loi de Hill), et le rapport C/P enfoui suit (250 oxique, 4 000 anoxique, Van Cappellen et Ingall, 1994) ;
- le phosphore dissous a un puits indépendant de l'oxygène, l'apatite authigène (temps de séjour 100 ka) : sans lui, il montait à mille fois la Terre sous l'océan anoxique et l'O₂ s'emballait ensuite jusqu'à 390 % PAL ;
- l'oxydation des roches réduites suit l'altération des silicates des terres (3 mol d'O₂ par mol de CO₂ à 1 PAL, exposant 0,5) au lieu de la surface des terres : l'érosion qui apporte le phosphore expose aussi le kérogène et la pyrite.

**Innovations et tunnel.** Les mutants innovants (de novo, duplication suivie de divergence) apparaissent selon une loi de Poisson de paramètre apparitions × probabilité d'innovation (π = 3·10⁻¹⁴ par défaut, recalé avec les fixations multiples) ; le tunnel de Weissman et coll. (2009) est calculé analytiquement pour chacun d'eux qui ne se fixe pas seul, sans borne d'essais.

**Éviction sous le plafond.** Classer selon la fitness d'invasion (`--eviction invasion`) a été mesuré : 47 % de cellules saturées sur les 100 derniers pas contre 1,3 % avec la biomasse, et une production primaire divisée par cinq. À l'équilibre, chaque résident a un r proche de zéro, souvent un peu négatif, et un fondateur qui arrive un r positif : la règle évince les résidents établis. Vision a gardé la biomasse ; l'option reste, désactivée.

**Cache.** Les capacités enzymatiques du résident sont calculées une fois par cellule jugée ; celles d'un mutant le sont par différence de sa liste d'enzymes (`RawCapacities`). La construction incrémentale des phénotypes appartient au fil « cellules complexes ».

**Pas adaptatif.** Un pas qui s'achève sans événement notable (intérêt ≥ 0,5) ni variation de plus de 10 % de l'O₂ allonge le suivant d'un pas demandé, jusqu'à trois fois ; le moindre événement notable le ramène au pas demandé. La décision ne dépend que de l'état simulé (rejeu identique, testé). L'évolution garde un tour par 100 ka. `--pas-fixe` le désactive pour la porte ; l'équivalence compare des pas fixes.

## PR #8 : la porte au niveau 4 (un tour par 100 ka)

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

## PR #8 : chronologie

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

Monde mûr : Terre, niveau 6, 250 pas depuis un ensemencement de tout l'océan (`chrono --prepare 250`), 53 000 populations, pas adaptatif au maximum (300 ka, un tour d'évolution). Code mesuré : cette branche réunie avec celle des cellules complexes (construction incrémentale étendue, commit d9245cd).

| Fils | Durée d'un pas | dont évolution | Vitesse |
|---|---|---|---|
| 1 | 3,44 s | 2,48 s | 87 ka/s |
| 2 | 1,85 s | 1,25 s | 162 ka/s |
| 3 | 1,32 s | 0,88 s | 227 ka/s |
| 4 | 1,07 s | 0,68 s | 282 ka/s |

Estimation sur 6 cœurs : environ 385 ka/s (partie série d'environ 130 ms, planète et registres ; le reste se partage avec une efficacité de 85 à 90 %). **La porte de vitesse (500 ka/s sur 6 cœurs) n'est pas atteinte**, ni la cible de 1 Ma/s. Avec un tour par 100 ka, la même mesure donnait 126 ka/s sur 4 fils : passer à 300 ka a plus que doublé la vitesse. L'évolution prend encore 64 % du pas (520 000 évaluations génétiques). Leviers restants : moins de candidats ponctuels par génotype, génomes plus courts (ci-dessous), registres et migration (17 %).

Parties entières au niveau 6 (porte ci-dessus, mondes jeunes, 4 fils) : 240 à 403 ka/s.

## Limites connues

- **Gonflement des génomes.** Le biais de délétion purge les pseudogènes, mais certaines populations gardent jusqu'à 1 300 gènes fonctionnels (copies de petite efficacité d'une même voie qui s'additionnent) ; toute l'évolution étant en O(gènes), c'est aussi un coût de vitesse.
- **Monde océan** à 324 K et 3,9 bar de CO₂ (sans terres, ni altération ni apport de phosphore continental) ; sa montée de l'O₂ et celle de la super-Terre restent trop rapides (accélération déclarée).
- **Terre au niveau 6** : photosynthèse oxygénique à 188 Ma, sous la fenêtre (une graine).
- **Électrons freinés** : 21 à 103 % de la production d'O₂ est déplacée quand une boîte vide freine un prélèvement ; tout est apparié, mais c'est le signe que le prolongement des flux sur le pas surestime les prélèvements.
- **Saturation des cellules** sur les 100 derniers pas : 5 à 13 % au niveau 6 juste après la montée de l'O₂ (renouvellement des communautés, presque toutes en croissance), au-dessus du seuil de 2 % ; 3,5 % sur le monde mûr.
