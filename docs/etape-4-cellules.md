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
| Monde océan | 2026 | 29,4 Ma | 76,8 Ma | 855 Ma | non | non (vie éteinte vers 2,5 Ga) |
| Monde désertique | 2026 | 27,6 Ma | 47,4 Ma | aucune en 3 Ga (accélérateur dès 1,63 Ga) | non | non |
| Terre | 7 | 20,4 Ma | 51,0 Ma | 1,50 Ga, avec l'accélérateur | non | non |
| Terre | 2026 | 13,2 Ma | 39,0 Ma | aucune en 3 Ga (accélérateur dès 1,61 Ga) | non | non |

Sans lune a tourné avant la correction des proies, océan et désert avant l'accélérateur gradué, la Terre avec. Sans accélérateur, la Terre n'a fait aucun eucaryote en 3 Ga sur quatre graines (2026, 7, 42, 3).

**Porte de l'étape 4 non franchie.** Les eucaryotes apparaissent seuls sur trois mondes de la vague 1 (super-Terre, sans lune, océan), à des dates dans la fenêtre sauf super-Terre (un peu tôt) ; les multicellulaires eucaryotes à deux types sur deux (super-Terre, sans lune). Sur Terre, l'eucaryote ne vient qu'avec l'accélérateur et sur une graine sur deux, et aucun multicellulaire eucaryote différencié n'apparaît.

## Limites connues

- **Terre.** Sans aide, aucun eucaryote en 3 Ga sur quatre graines. Ce n'est pas le climat : sur la graine 2026, la glace ne couvre tout qu'autour de 700 Ma, puis l'O₂ tient entre 0,5 et 0,7 et la température monte de 275 à 319 K entre 1 et 1,5 Ga. C'est le nombre d'hôtes : à 1,3 Ga, la sonde (`examples/sonde.rs`) trouve 34 populations phagotrophes sur 3 066 hétérotrophes, 28 partenaires candidats dont 3 avantageux, et 7×10⁻¹⁰ fixation attendue par tour, soit moins d'une chance sur cent mille par milliard d'années. La super-Terre et le monde océan, plus grands ou plus productifs, nourrissent assez de phagotrophes. D'où l'accélérateur gradué ci-dessous.
- **Vallée de taille.** Un petit cytosquelette (taille 1,3 à 2,2) est désavantagé (s ≈ −0,15 sur un monde mûr) : la membrane perd plus que ce que les proies rapportent. Seules les grandes cellules (taille 3,4 et plus) envahissent, et seulement dans les cellules riches en proies. C'est une vallée réelle (l'eucaryogenèse est un événement unique), mais elle rend la phagotrophie durable rare.
- **Persistance.** Là où l'eucaryote apparaît, il reste minoritaire et recule souvent : 7 % de la biomasse sur super-Terre à 750 Ma puis 1 à 6 % ; 1 % sur le monde océan à 1 Ga puis 0 % ; sur Terre (graine 7) il disparaît en moins de 250 Ma, avant une glaciation totale vers 2 Ga. Les phagotrophes restent sous 1 % de la biomasse sur Terre. C'est la prochaine cible : rendre le mode de vie phagotrophe et la cellule eucaryote durables, plutôt que d'accélérer encore leur apparition.
- **Climat des longues parties (au fil calibrage).** Terre graine 2026 : 364 K et O₂ en baisse à 3 Ga ; monde océan : vie éteinte vers 2,5 Ga ; Terre graine 7 : glaciation totale vers 2 Ga.
- **Dates.** Super-Terre fait ses eucaryotes un peu trop tôt (584 Ma pour une fenêtre qui commence à 667 Ma), sans lune dans la fenêtre. Multicellulaires eucaryotes à deux types : quelques dizaines de Ma après les eucaryotes, soit une transition plus rapide que sur Terre. Le plaste précède la mitochondrie (premier organite dans un monde encore anoxique).
- **Sortie des eaux.** Aucun eucaryote multicellulaire complexe n'a encore gagné les terres en 3 Ga : le passage au niveau 6 est testé (bilans, poursuite de la partie) mais n'a pas eu lieu dans une partie longue.
- **Vitesse.** Ère microbienne : 1,0 à 2,0 Ma/s sur 2 fils au niveau 4. Ère des colonies (90 % de la biomasse en colonies sur super-Terre) : 150 à 360 ka/s sur 2 fils, partagés avec d'autres parties. La mesure finale au niveau 6 sur 6 cœurs revient au fil calibrage (décision de Vision).
- **Sauvegardes.** Génome, phénotype, configuration et statistiques ont changé : le format de sauvegarde passe à 5.

## Accélérateur de la complexité

Décision de l'utilisateur (2026-10-07) : émergence assistée, accélérateurs seulement quand l'évolution stagne. Après 1 Ga sans nouvelle étape de la complexité depuis la photosynthèse oxygénique (ou depuis la dernière étape franchie), l'accélérateur multiplie par 100 les mutations innovantes et les copies divergentes vers les familles de structure. Sur les rétentions d'endosymbiotes, son effet est décuplé tous les 100 Ma tant qu'aucune étape nouvelle n'apparaît, jusqu'à 10¹² ([Simplification] accélération déclarée ; les fixations qu'il produit portent la cause « accélérateur » et l'événement de mise en marche est publié). Il s'arrête à la première étape nouvelle.

## Corrections faites en route

- `OriginFixation::any_fixes` rendait une fixation certaine dès que la probabilité de fixation d'une copie atteignait 1, même pour 10⁻¹² copie attendue : c'est ce qui rendait l'endosymbiose systématique. Elle vaut désormais 1 − e^(−copies).
- `CellContext::conditions_of` ne listait les proies que si un phagotrophe vivait déjà dans la cellule : aucun mutant phagotrophe ne pouvait s'installer ailleurs.
