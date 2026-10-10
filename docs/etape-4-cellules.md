# Étape 4 : cellules complexes (moteur)

Feuille de route : document d'architecture, « Périmètre consolidé de l'étape 4 », volet moteur des cellules complexes. Ce volet fait apparaître sans script la phagotrophie, l'endosymbiose et la cellule eucaryote, le sexe, les colonies clonales et leurs types cellulaires, publie le plan de construction des corps, construit les phénotypes de façon incrémentale et fait passer le vivant au niveau de la planète quand des multicellulaires complexes gagnent les terres. Il est posé sur le calibrage de l'étape 4 (PR #8) et contient l'adaptateur du client (PR #7).

## Ce que le génome peut désormais faire

**Familles de domaines.** Cinq familles s'ajoutent aux voies, pigments et défenses : cytosquelette, adhésion, signal, régulateur, recombinase de méiose. Elles naissent par duplication suivie de divergence depuis des familles parentes (table de parenté du document Génétique : fermentation vers cytosquelette et signal, rhodopsine vers signal et adhésion, réparation vers régulateur et méiose…). Une copie qui diverge vers l'une de ces familles n'est pas soumise à la probabilité d'innovation des voies nouvelles (3e-14), parce que des homologues existent chez les procaryotes (FtsZ et MreB pour l'actine et la tubuline, systèmes à deux composants, recombinases), mais à une probabilité propre d'être fonctionnelle, `structural_probability` = 10⁻¹² par copie apparue ([Simplification] calée sur la chronologie des premières colonies et des filaments différenciés). Sans elle, les colonies venaient vers 9 Ma et deux types cellulaires vers 11 à 15 Ma.

**La cellule.** Le cytosquelette agrandit la cellule (taille 1 + 6 × cytosquelette, au plus 30 fois une bactérie). Tout ce qui passe par la membrane de l'hôte (prélèvements dans l'eau, photosynthèse de l'hôte, respiration de l'hôte) rend 1/taille par mole de biomasse ; le génome coûte moins par mole dans une grande cellule ; le cytosquelette dynamique coûte 10 000 kJ·molC⁻¹·an⁻¹ par unité. Une cellule assez grande et qui sait digérer (fermentation ou respiration) englobe des proies plus petites qu'elle : la prédation est explicite dans l'écologie (carbone et phosphore conservés, une proie ne perd pas plus de la moitié de sa biomasse par sous-pas, la digestion aérobie consomme de l'O₂).

**Endosymbiose.** Un phagotrophe garde rarement une proie englobée. Le génome candidat est celui de l'hôte augmenté d'un organite qui porte les gènes d'énergie de la proie (voies, pigments, chaîne de transport, défense). Le nombre de rétentions attendues pendant un tour est l'offre de mutants : effectif réel du génotype × proies englobées par cellule et par an × probabilité de rétention (1e-38 par proie englobée, [Simplification] calée sur la chronologie) × durée. L'organite se fixe s'il rend l'hôte plus apte : une mitochondrie respire sans la limite de surface, un plaste photosynthétise à l'intérieur. Au plus deux organites, et pas deux fois la même fonction.

**Sexe.** Un eucaryote doté d'une recombinase de méiose (≥ 0,1) est sexué : les mutations ponctuelles avantageuses qui se fixeraient au même tour se réunissent dans un même génome au lieu de se concurrencer ([Simplification] sexe isogame et facultatif, sans coût direct).

**Colonies et types cellulaires.** Au-delà d'une adhésion de 0,5, les cellules filles restent attachées : 2 à 4 096 cellules en filament, feuillet ou boule selon la variation cachée des domaines d'adhésion. Le corps se découpe en zones de la surface vers le centre ; les substances dissoutes et la lumière baissent avec la profondeur, les cellules collées puisent 30 à 50 % moins dans l'eau, les ultraviolets et les proies n'atteignent que la surface, et un prédateur avale moins bien un gros corps. Le signal émet un morphogène depuis la surface ; chaque régulateur commande le bloc de gènes qui le suit, actif au-dessus (ou au-dessous) d'un seuil de morphogène. Deux zones qui expriment des gènes différents sont deux types cellulaires. La duplication d'un module (régulateur et son bloc) est une classe de mutation à part.

**Plan de construction.** `evo_life::BodyPlan` (crates/evo-life/src/body.rs, version 1) suit les 11 champs du document Génétique : modules (parent, attache, axe, dimensions, symétrie, articulations, matériau, revêtement, pigments, couleur structurale, motif de Turing, fonction) et types cellulaires. Il est publié par espèce dans `SpeciesView.body_plan`, avec `SpeciesView.organisation` (taille de cellule, cellules du corps, types, organites, plastes, phagotrophe, eucaryote, multicellulaire, sexué). Le client le lit par un adaptateur vers evo-morph (PR #7).

**Passage au niveau 6.** Quand un eucaryote multicellulaire à deux types cellulaires vit dans une cellule non océanique, la grille du vivant passe à la résolution de la planète (`World::refine_life_grid`) : chimie copiée, populations partagées au prorata du volume d'eau, arrondis portés au bilan. Les grilles sont emboîtées : les anciens numéros de cellule restent valides.

## Construction incrémentale des phénotypes

Les sommes du phénotype d'une cellule (voies, pigments, chaîne de transport, coûts) sont tenues en virgule fixe (entiers de 128 bits) : leur addition est exacte et associative. La base du résident (`phenotype::Basis` : organisation, développement, jeux de gènes exprimés, sommes par type cellulaire) est calculée une fois par génotype et par tour ; un mutant ponctuel d'un gène métabolique, de lumière ou de défense ne change que les sommes des types qui l'expriment, et son phénotype est identique au bit près à une reconstruction complète (test sur 3 000 mutations, colonie à deux types avec organite). Un marqueur ou un gène inactif réutilise le phénotype du résident. Les capacités du calibrage (`RawCapacities`) tiennent compte de la taille de la cellule, des organites et de la digestion.

## Résultats

Niveau 4 (6 quand un multicellulaire complexe gagne les terres), un tour d'évolution par 300 ka de temps simulé, graine 2026 sauf mention, 3 à 3,5 Ga, commande `evonisium complexite`. Code : calibrage de la PR #10 avec son correctif (entretien du génome à 2 000 kJ par gène, patience de l'accélérateur de la photosynthèse à 800 Ma), gènes de structure à 10⁻¹². Référence terrestre comptée depuis l'apparition de la vie (~3,8 Ga) : photosynthèse oxygénique ~0,9 Ga, eucaryotes ~2,0 Ga, multicellulaires différenciés ~2,75 Ga ; un facteur 3 donne 0,67 à 6 Ga pour les eucaryotes et 0,92 à 8 Ga pour les multicellulaires.

| Monde | Colonie | Deux types (procaryotes) | Photosynthèse O₂ | Eucaryote | Multicellulaire eucaryote à 2 types | Hors de l'eau |
|---|---|---|---|---|---|---|
| Terre | 280 Ma | 406 Ma | 816 Ma | 890 Ma | 2,31 Ga, accélérateur dès 1,89 Ga | non (3 Ga) |
| Terre, graine 7 | 211 Ma | 653 Ma | 826 Ma | 1,02 Ga | 2,69 Ga, accélérateur dès 2,02 Ga | non (3,5 Ga) |
| Sans lune | 207 Ma | 709 Ma | 815 Ma | 917 Ma | **1,02 Ga, sans aide** | non (3 Ga) |
| Désertique | 57 Ma | 2,15 Ga | 862 Ma | 914 Ma | 2,31 Ga, accélérateur dès 1,91 Ga | 2,32 Ga |
| Petite planète | 55 Ma | 166 Ma | 851 Ma | 933 Ma | 2,63 Ga, accélérateur dès 1,93 Ga | 3,00 Ga |
| Super-Terre | 25 Ma | 66 Ma | 116 Ma | 894 Ma | non (3,5 Ga, accélérateur dès 1,89 Ga) | non |
| Océan | 469 Ma | 472 Ma | 457 Ma | 577 Ma | non (3 Ga, accélérateur dès 1,58 Ga) | non |

Photosynthèse oxygénique : par l'accélérateur du calibrage sur tous les mondes sauf super-Terre et océan. Eucaryotes : sans aide partout, 50 à 120 Ma après la photosynthèse oxygénique.

**Porte de l'étape 4 franchie avec accélération déclarée.** Eucaryotes puis multicellulaires eucaryotes à deux types cellulaires, sans script, sur la Terre (deux graines) et quatre mondes de la vague 1 sur cinq testés pour les eucaryotes, trois pour les multicellulaires (sans lune, désertique, petite planète). Toutes les dates sont dans la fenêtre du facteur 3. Le multicellulaire eucaryote différencié n'apparaît sans aide que sur le monde sans lune : ailleurs il vient avec l'accélérateur de la complexité, qui tourne 400 à 700 Ma avant de l'obtenir (déclaré, voir plus bas). Le vivant passe au niveau 6 sur les mondes désertique et petite planète quand ces corps gagnent les terres. Les colonies procaryotes à deux types précèdent l'eucaryote, comme les filaments de cyanobactéries à hétérocystes.

Mesures précédentes (un tour par 100 ka, avant le correctif du calibrage) : eucaryote seul sur la Terre graine 2026 à 703 Ma ; avec l'accélérateur, eucaryote à 1,66 Ga, multicellulaires à deux types à 1,70 Ga, sortie des eaux à 1,73 Ga.

## Limites connues

- **Terre.** Les eucaryotes y apparaissent désormais sans aide (890 Ma et 1,02 Ga sur deux graines), depuis que le fil calibrage a corrigé le climat et l'entretien du génome. Le passage des eucaryotes aux corps différenciés reste rare : il faut l'accélérateur sur les deux graines.
- **Vallée de taille.** Un petit cytosquelette (taille 1,3 à 2,2) est désavantagé (s ≈ −0,15 sur un monde mûr) : la membrane perd plus que ce que les proies rapportent. Seules les grandes cellules (taille 3,4 et plus) envahissent, et seulement dans les cellules riches en proies. C'est une vallée réelle (l'eucaryogenèse est un événement unique), mais elle rend la phagotrophie durable rare.
- **Persistance (corrigée).** Avec le climat et l'entretien du génome du fil calibrage, l'eucaryote tient : 15 à 23 % de la biomasse sur Terre entre 2 et 3 Ga, 35 à 41 % sur la graine 7 après 2,75 Ga, 54 à 73 % sur le monde désertique après 3 Ga. Les phagotrophes procaryotes restent sous 6 %.
- **O₂ des longues parties (au fil calibrage).** Sur Terre, l'O₂ culmine vers 0,08 juste après la photosynthèse oxygénique puis retombe vers 0,002 à 0,009 de 1,25 à 3,5 Ga ; le monde désertique garde 0,04 à 0,1. Les eucaryotes s'en accommodent, mais la Terre réelle monte au contraire vers 0,2.
- **Dates.** Eucaryotes un peu tôt par rapport à la Terre réelle (0,9 Ga au lieu de 2 Ga, facteur 2,2) et très peu après la photosynthèse oxygénique : l'O₂ dissous suffit à rendre la mitochondrie avantageuse. Le plaste précède la mitochondrie (premier organite dans un monde encore peu oxygéné).
- **Sortie des eaux.** Elle a lieu sur les mondes désertique (2,32 Ga) et petite planète (3,00 Ga), aussitôt ou peu après les premiers corps différenciés ; pas encore sur Terre en 3,5 Ga.
- **Vitesse.** Un fil par partie, quatre parties sur quatre cœurs, un tour par 300 ka : 0,55 à 1,0 Ma/s en moyenne sur 3 à 3,5 Ga au niveau 4 ; le monde désertique, passé au niveau 6 à 2,32 Ga, tombe à 547 ka/s en moyenne. La mesure finale au niveau 6 sur 6 cœurs revient au fil calibrage (décision de Vision).
- **Colonies trop précoces (corrigé).** Les copies divergentes donnaient un gène de structure ou de régulation fonctionnel à chaque tirage : colonies vers 9 Ma, deux types vers 11 à 15 Ma. Balayage de `structural_probability` sur Terre : à 10⁻¹³ les types cellulaires ne viennent plus qu'avec l'accélérateur ; à 10⁻¹² les procaryotes font deux types sans aide entre 166 et 709 Ma sur cinq mondes sur sept, et le monde sans lune ses multicellulaires eucaryotes.
- **Sauvegardes.** Génome, phénotype, configuration et statistiques ont changé : le format de sauvegarde passe à 5, puis à 7 avec la probabilité des gènes de structure (6 au fil calibrage).

## Accélérateur de la complexité

Décision de l'utilisateur (2026-10-07) : émergence assistée, accélérateurs seulement quand l'évolution stagne. Après 1 Ga sans nouvelle étape de la complexité depuis la photosynthèse oxygénique (ou depuis la dernière étape franchie), l'accélérateur multiplie par 100 les mutations innovantes et les copies divergentes vers les familles de structure. Sur les rétentions d'endosymbiotes, son effet est décuplé tous les 100 Ma tant qu'aucune étape nouvelle n'apparaît, jusqu'à 10¹² ([Simplification] accélération déclarée ; les fixations qu'il produit portent la cause « accélérateur » et l'événement de mise en marche est publié). Il s'arrête à la première étape nouvelle.

## Corrections faites en route

- `OriginFixation::any_fixes` rendait une fixation certaine dès que la probabilité de fixation d'une copie atteignait 1, même pour 10⁻¹² copie attendue : c'est ce qui rendait l'endosymbiose systématique. Elle vaut désormais 1 − e^(−copies).
- `CellContext::conditions_of` ne listait les proies que si un phagotrophe vivait déjà dans la cellule : aucun mutant phagotrophe ne pouvait s'installer ailleurs.
