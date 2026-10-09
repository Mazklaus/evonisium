# Étape 4 : cellules complexes (moteur)

Feuille de route : document d'architecture, « Périmètre consolidé de l'étape 4 », volet moteur des cellules complexes. Ce volet fait apparaître sans script la phagotrophie, l'endosymbiose et la cellule eucaryote, le sexe, les colonies clonales et leurs types cellulaires, publie le plan de construction des corps, construit les phénotypes de façon incrémentale et fait passer le vivant au niveau de la planète quand des multicellulaires complexes gagnent les terres. Il est posé sur le calibrage de l'étape 4 (PR #8) et contient l'adaptateur du client (PR #7).

## Ce que le génome peut désormais faire

**Familles de domaines.** Cinq familles s'ajoutent aux voies, pigments et défenses : cytosquelette, adhésion, signal, régulateur, recombinase de méiose. Elles naissent par duplication suivie de divergence depuis des familles parentes (table de parenté du document Génétique : fermentation vers cytosquelette et signal, rhodopsine vers signal et adhésion, réparation vers régulateur et méiose…). Une copie qui diverge vers l'une de ces familles est une mutation courante : elle n'est pas soumise à la probabilité d'innovation des voies nouvelles (1e-13), parce que des homologues existent chez les procaryotes (FtsZ et MreB pour l'actine et la tubuline, systèmes à deux composants, recombinases). Avec cette probabilité, aucune complexité n'apparaissait en 3 Ga.

**La cellule.** Le cytosquelette agrandit la cellule (taille 1 + 6 × cytosquelette, au plus 30 fois une bactérie). Tout ce qui passe par la membrane de l'hôte (prélèvements dans l'eau, photosynthèse de l'hôte, respiration de l'hôte) rend 1/taille par mole de biomasse ; le génome coûte moins par mole dans une grande cellule ; le cytosquelette dynamique coûte 10 000 kJ·molC⁻¹·an⁻¹ par unité. Une cellule assez grande et qui sait digérer (fermentation ou respiration) englobe des proies plus petites qu'elle : la prédation est explicite dans l'écologie (carbone et phosphore conservés, une proie ne perd pas plus de la moitié de sa biomasse par sous-pas, la digestion aérobie consomme de l'O₂).

**Endosymbiose.** Un phagotrophe garde rarement une proie englobée. Le génome candidat est celui de l'hôte augmenté d'un organite qui porte les gènes d'énergie de la proie (voies, pigments, chaîne de transport, défense). Le nombre de rétentions attendues pendant un tour est l'offre de mutants : effectif réel du génotype × proies englobées par cellule et par an × probabilité de rétention (1e-38 par proie englobée, [Simplification] calée sur la chronologie) × durée. L'organite se fixe s'il rend l'hôte plus apte : une mitochondrie respire sans la limite de surface, un plaste photosynthétise à l'intérieur. Au plus deux organites, et pas deux fois la même fonction.

**Sexe.** Un eucaryote doté d'une recombinase de méiose (≥ 0,1) est sexué : les mutations ponctuelles avantageuses qui se fixeraient au même tour se réunissent dans un même génome au lieu de se concurrencer ([Simplification] sexe isogame et facultatif, sans coût direct).

**Colonies et types cellulaires.** Au-delà d'une adhésion de 0,5, les cellules filles restent attachées : 2 à 4 096 cellules en filament, feuillet ou boule selon la variation cachée des domaines d'adhésion. Le corps se découpe en zones de la surface vers le centre ; les substances dissoutes et la lumière baissent avec la profondeur, les cellules collées puisent 30 à 50 % moins dans l'eau, les ultraviolets et les proies n'atteignent que la surface, et un prédateur avale moins bien un gros corps. Le signal émet un morphogène depuis la surface ; chaque régulateur commande le bloc de gènes qui le suit, actif au-dessus (ou au-dessous) d'un seuil de morphogène. Deux zones qui expriment des gènes différents sont deux types cellulaires. La duplication d'un module (régulateur et son bloc) est une classe de mutation à part.

**Plan de construction.** `evo_life::BodyPlan` (crates/evo-life/src/body.rs, version 1) suit les 11 champs du document Génétique : modules (parent, attache, axe, dimensions, symétrie, articulations, matériau, revêtement, pigments, couleur structurale, motif de Turing, fonction) et types cellulaires. Il est publié par espèce dans `SpeciesView.body_plan`, avec `SpeciesView.organisation` (taille de cellule, cellules du corps, types, organites, plastes, phagotrophe, eucaryote, multicellulaire, sexué). Le client le lit par un adaptateur vers evo-morph (PR #7).

**Passage au niveau 6.** Quand un eucaryote multicellulaire à deux types cellulaires vit dans une cellule non océanique, la grille du vivant passe à la résolution de la planète (`World::refine_life_grid`) : chimie copiée, populations partagées au prorata du volume d'eau, arrondis portés au bilan. Les grilles sont emboîtées : les anciens numéros de cellule restent valides.

## Construction incrémentale des phénotypes

Les sommes du phénotype d'une cellule (voies, pigments, chaîne de transport, coûts) sont tenues en virgule fixe (entiers de 128 bits) : leur addition est exacte et associative. La base du résident (`phenotype::Basis` : organisation, développement, jeux de gènes exprimés, sommes par type cellulaire) est calculée une fois par génotype et par tour ; un mutant ponctuel d'un gène métabolique, de lumière ou de défense ne change que les sommes des types qui l'expriment, et son phénotype est identique au bit près à une reconstruction complète (test sur 3 000 mutations, colonie à deux types avec organite). Un marqueur ou un gène inactif réutilise le phénotype du résident. Les capacités du calibrage (`RawCapacities`) tiennent compte de la taille de la cellule, des organites et de la digestion.

## Résultats

Niveau 4, pas de 200 ka (allongé jusqu'à trois fois aux périodes calmes), 3 Ga au plus, commande `evonisium complexite`. Référence terrestre comptée depuis l'apparition de la vie (~3,8 Ga) : photosynthèse oxygénique ~0,9 Ga, Grande Oxydation ~1,4 Ga, eucaryotes ~2,0 Ga, multicellulaires différenciés ~2,75 Ga ; un facteur 3 donne 0,67 à 6 Ga pour les eucaryotes et 0,92 à 8 Ga pour les multicellulaires.

| Monde | Graine | Phagotrophie | Colonie | Eucaryote | Multicellulaire eucaryote à 2 types | Hors de l'eau |
|---|---|---|---|---|---|---|
| Super-Terre | 2026 | 15,6 Ma | 35,4 Ma | 584 Ma | 624 Ma | non (2,5 Ga) |
| Sans lune | 2026 | tôt | 40,2 Ma | 2,17 Ga | 2,18 Ga | non (3 Ga) |
| Terre | 2026, 7, 42, 3 | 13 à 24 Ma | 36 à 880 Ma | aucune en 3 Ga | aucune | non |

Le monde sans lune a tourné avant la correction des proies (voir plus bas) ; les autres après.

## Limites connues

- **Terre.** Aucun eucaryote en 3 Ga sur quatre graines. Après la photosynthèse oxygénique, l'O₂ retombe vers zéro et y reste ; le rapport du calibrage montre la Terre à 229 K sous 100 % de glace à 392 Ma. Le fil calibrage traite la sortie de la boule de neige (refuges chauds sous la glace, CO₂ volcanique) ; la date des eucaryotes terrestres sera à remesurer après.
- **Vallée de taille.** Un petit cytosquelette (taille 1,3 à 2,2) est désavantagé (s ≈ −0,15 sur un monde mûr) : la membrane perd plus que ce que les proies rapportent. Seules les grandes cellules (taille 3,4 et plus) envahissent, et seulement dans les cellules riches en proies. C'est une vallée réelle (l'eucaryogenèse est un événement unique), mais elle rend la phagotrophie durable rare.
- **Dates.** Super-Terre fait ses eucaryotes un peu trop tôt (584 Ma pour une fenêtre qui commence à 667 Ma), sans lune dans la fenêtre. Multicellulaires eucaryotes à deux types : quelques dizaines de Ma après les eucaryotes, soit une transition plus rapide que sur Terre. Le plaste précède la mitochondrie (premier organite dans un monde encore anoxique).
- **Sortie des eaux.** Aucun eucaryote multicellulaire complexe n'a encore gagné les terres en 3 Ga : le passage au niveau 6 est testé (bilans, poursuite de la partie) mais n'a pas eu lieu dans une partie longue.
- **Vitesse.** Ère microbienne : 1,0 à 2,0 Ma/s sur 2 fils au niveau 4. Ère des colonies (90 % de la biomasse en colonies sur super-Terre) : 150 à 360 ka/s sur 2 fils, partagés avec d'autres parties. La mesure finale au niveau 6 sur 6 cœurs revient au fil calibrage (décision de Vision).
- **Sauvegardes.** Génome, phénotype, configuration et statistiques ont changé : le format de sauvegarde est à monter à la fusion.

## Corrections faites en route

- `OriginFixation::any_fixes` rendait une fixation certaine dès que la probabilité de fixation d'une copie atteignait 1, même pour 10⁻¹² copie attendue : c'est ce qui rendait l'endosymbiose systématique. Elle vaut désormais 1 − e^(−copies).
- `CellContext::conditions_of` ne listait les proies que si un phagotrophe vivait déjà dans la cellule : aucun mutant phagotrophe ne pouvait s'installer ailleurs.
