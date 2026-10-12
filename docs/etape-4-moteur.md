# Étape 4 : moteur microbien, calibrage et vitesse

Périmètre (document d'architecture, « Périmètre consolidé de l'étape 4 ») : bilan d'électrons exact sans correction, régulation de l'O₂ par l'enfouissement du carbone limité par le phosphore, tunnel analytique et innovations tirées selon une loi de Poisson, test de saturation de la référence à 100 ka, chronologie calée sur la Terre à un facteur 3 près, cache des phénotypes, pas adaptatif et 1 Ma/s sur monde mûr.

Mesures de la suite (PR #10 puis PR #14) : porte au niveau 6 sur les six mondes, graine 2026, avec un tour d'évolution par 300 ka de temps simulé (décision de l'utilisateur du 9 octobre). Le détail par monde est dans [etape-4-porte-n6.md](etape-4-porte-n6.md).

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

**Génomes bornés par leur entretien (PR #14).** Certaines populations accumulaient jusqu'à 1 300 gènes, pour l'essentiel des centaines de régulateurs sans effet (diagnostic `EVO_DEBUG_GENOMES`). L'entretien du génome passe de 200 à 2 000 kJ par gène et par mole de carbone, toujours divisé par le volume de la cellule (les grandes cellules portent plus facilement un grand génome, Lane et Martin, 2010). Au niveau 4 : 16 à 38 gènes en moyenne au lieu de 60 à 130, les plus longs vers 300. Un coût de réplication par gène cinq fois plus fort, essayé à côté, ne changeait presque rien : la croissance de ces populations n'est pas limitée par leur taux maximal.

**Accélérateur de la photosynthèse à 800 Ma (PR #14).** Sa patience passe de 600 à 800 Ma sans progrès : les mondes aidés font leur photosynthèse oxygénique vers 820 à 870 Ma, dans la plage terrestre. Le rapport de la porte compte l'étape oxygénique pour l'accélérateur s'il agissait quand elle est apparue (il multiplie aussi des tirages qui ne portent pas sa marque ; la PR #10 les comptait pour le moteur).

## La porte au niveau 6

Code de la PR #14. Maintien de l'O₂ au-dessus de 10⁻⁴ pendant 50 Ma, graine 2026, 4 fils.

| Monde | Photosynthèse oxygénique | O₂ > 10⁻⁴ | O₂ en fin de maintien | Température | Vitesse de la partie | Verdict |
|---|---|---|---|---|---|---|
| Terre (Archéen) | 866 Ma (accélérateur) | 885 Ma | 55 % PAL | 281 K | 394 ka/s | franchie |
| Monde océan | 121 Ma (moteur) | 1,24 Ga | 35 % PAL | 324 K | 409 ka/s | franchie |
| Monde désertique | 862 Ma (accélérateur) | 1,88 Ga | 125 % PAL | 273 K | 422 ka/s | franchie |
| Super-Terre | 818 Ma (accélérateur) | 851 Ma | 15 % PAL | 283 K | 291 ka/s | franchie |
| Petite planète | 839 Ma (accélérateur) | 1,88 Ga | 99 % PAL | 269 K | 357 ka/s | franchie |
| Monde sans lune | 826 Ma (accélérateur) | 847 Ma | 60 % PAL | 280 K | 327 ka/s | franchie |

Carbone, phosphore et électrons conservés à 10⁻¹⁰ près ; rejeu identique sur les six. Au niveau 4 (même code), la porte passe aussi sur les six mondes : photosynthèse oxygénique entre 824 et 872 Ma sur cinq mondes (accélérateur), à 168 Ma seule sur le monde sans lune.

Avec le code de la PR #10 (entretien du génome à 200, accélérateur à 600 Ma), la porte passait aussi sur les six mondes, photosynthèse oxygénique entre 583 et 634 Ma, à 188 Ma sur la Terre.

## Chronologie

Repères terrestres, comptés depuis l'apparition de la vie (environ 3,8 Ga) : photosynthèse oxygénique vers +800 à +1 100 Ma (fenêtre à un facteur 3 : 300 Ma à 2,7 Ga), grande oxydation vers +1,4 Ga (fenêtre : 470 Ma à 4,2 Ga), O₂ du Protérozoïque entre 0,1 et 10 % PAL, niveau actuel vers +3,2 Ga seulement.

- Photosynthèse oxygénique : 818 à 866 Ma sur cinq mondes, dans la plage terrestre, toujours au réveil de l'accélérateur, qui est compté (**accélération déclarée**). Quand elle vient seule, elle peut être très précoce : 121 Ma sur le monde océan au niveau 6, 168 Ma sur le monde sans lune au niveau 4, 188 Ma sur la Terre avec le code de la PR #10. C'est structurel : une seule innovation rare, dont l'attente suit une loi exponentielle ; une fois sur six environ, elle tombe sous le tiers de sa moyenne. Il faudrait plusieurs étapes rares (photosystème II, complexe d'oxydation de l'eau) pour resserrer la date ; c'est hors de l'étape 4.
- Montée de l'O₂ : environ 1 Ga après la photosynthèse oxygénique sur le désert, la petite planète et le monde océan, comme sur Terre ; 20 à 35 Ma après sur la Terre, la super-Terre et le monde sans lune (**accélération déclarée** : les puits réducteurs ne retiennent pas assez longtemps l'O₂).
- Après la montée, l'O₂ terrestre retombe vers 1 à 4 % PAL et y reste de 1,25 à 3,5 Ga (mesure du fil « cellules complexes ») : c'est le Protérozoïque de la littérature. La seconde oxygénation, vers le niveau actuel, n'existe pas encore (étape 5, chantier Planète).
- Premières étapes (pigment, phototrophie simple, anoxygénique) : 2 à 22 Ma, bien trop tôt ; ce sont des mutations courantes, que π ne règle pas.

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

Monde mûr : Terre, niveau 6, 250 pas depuis un ensemencement de tout l'océan (`chrono --prepare 250`), pas adaptatif au maximum (300 ka, un tour d'évolution). Mesure finale de l'étape 4 sur le code de main après les PR #14 et #15 (génomes bornés, cellules complexes) : 63 000 populations.

| Fils | Durée d'un pas | dont évolution | Vitesse |
|---|---|---|---|
| 1 | 3,52 s | 2,26 s | 85 ka/s |
| 2 | 1,76 s | 1,04 s | 171 ka/s |
| 3 | 1,24 s | 0,69 s | 242 ka/s |
| 4 | 1,06 s | 0,59 s | 282 ka/s |

Estimation sur 6 cœurs : environ 380 ka/s (partie série d'environ 170 ms, planète et registres ; le reste se partage presque parfaitement). 4 Ga au curseur maximum prennent donc environ 2 h 55. **La porte de vitesse (500 ka/s sur 6 cœurs) n'est pas atteinte**, ni la cible de 1 Ma/s ; l'utilisateur a choisi un tour par 300 ka en connaissance de cause (9 octobre). Les génomes plus courts ont réduit l'évolution de 0,68 à 0,59 s par pas sur 4 fils, malgré 20 % de populations en plus ; l'écologie et la migration (29 %) pèsent maintenant davantage.

Avant les PR #14 et #15 (génomes jusqu'à 1 300 gènes, 53 000 populations) : 282 ka/s aussi sur 4 fils, dont 0,68 s d'évolution. Avec un tour par 100 ka : 126 ka/s.

Parties entières au niveau 6 (porte ci-dessus, mondes jeunes, 4 fils) : 291 à 422 ka/s.

## Limites connues

- **Génomes.** Les plus longs restent vers 300 gènes, souvent des régulateurs et des gènes d'adhésion sans effet notable ; la moyenne (16 à 38) est raisonnable.
- **Monde océan** à 324 K et 3,5 bar de CO₂ : sans terres, seul le plancher océanique altère, et il ne fait baisser le CO₂ que d'un quart en 1,4 Ga. Montée de l'O₂ trop rapide sur la Terre, la super-Terre et le monde sans lune (accélération déclarée).
- **Photosynthèse oxygénique sans aide** parfois très précoce (121 à 188 Ma selon le monde et la graine) : une seule innovation rare (voir la chronologie).
- **Pas de seconde oxygénation** : l'O₂ reste au niveau du Protérozoïque (1 à 4 % PAL) jusqu'à 3,5 Ga.
- **Électrons freinés** : 21 à 103 % de la production d'O₂ est déplacée quand une boîte vide freine un prélèvement ; tout est apparié, mais c'est le signe que le prolongement des flux sur le pas surestime les prélèvements.
- **Saturation des cellules** sur les 100 derniers pas : plusieurs pour cent au niveau 6 juste après la montée de l'O₂ (renouvellement des communautés, presque toutes en croissance), au-dessus du seuil de 2 %.
