# Étape 5 : animaux et écosystèmes (moteur)

Feuille de route : document d'architecture, « Périmètre de l'étape 5 », volet moteur. Ce volet fait sortir la vie de l'eau, fait apparaître sans script des animaux mobiles, des prédateurs et leurs proies, des squelettes, des plantes à tissus de soutien et des organes sonores, et tient le climat et l'oxygène de la planète pendant ces transitions. Il est posé sur l'étape 4 (PR #9 et #15).

## La terre ferme

**Enveloppe et habitat.** La famille `Cuticle` (polysaccharides des croûtes, cuticule cireuse, paroi des spores) donne la résistance à la dessiccation ; au-delà de 0,5, l'organisme vit hors de l'eau (bit `TERRESTRIAL`). [Simplification] Deux habitats tranchés, sans amphibies. Sur la terre ferme, la croissance ne se fait que mouillée (humidité du sol), la dessiccation tue ce qui n'est pas protégé, et l'enveloppe freine les échanges dans l'eau.

**Phosphore du sol.** L'altération livre son phosphore aux sols des cellules (pondéré par la terre ferme, l'humidité et la température) ; la terre ferme y puise et y rend le sien. La solution du sol est lessivée vers la couche d'eau.

**Échanges avec l'air.** La vie terrestre prend son CO₂ et rend son O₂ à l'atmosphère, pas à l'eau. Sa litière se décompose dans le sol (méthane quand il manque d'O₂) ; 0,3 % est enfoui (tourbes, deltas). Seule la litière du régime établi est prolongée sur le pas : l'excès d'une hécatombe passagère ne l'est pas (c'est ce qui provoquait un emballement de l'O₂ et du CO₂ vers 2,3 Ga).

**Feux.** Au-delà de 17 % d'O₂, les incendies brûlent la litière avant son enfouissement (rétroaction du modèle COPSE, Lenton et Watson, 2000) : elle tient l'O₂ sous 30 %.

**CO₂ des plantes.** La photosynthèse des terres et l'altération biotique suivent le CO₂ de l'air (point de compensation 10 Pa, demi-saturation 30 Pa au-delà) : sans elle, la vie terrestre affamait l'air de CO₂ et déclenchait des glaciations répétées.

**Glace et dormance.** Une terre gelée garde un refuge humide minuscule ; un organisme terrestre sans terre libre est dormant : il ne prélève rien, ne produit rien, ne meurt pas et ne peut ni envahir ni l'emporter. Une cellule sans terre ferme du tout ne garde pas de formes terrestres.

## Les animaux et les plantes

**Familles de domaines.** Quatre familles s'ajoutent, nées par duplication et divergence (table de parenté) :

| Famille | Équivalent | Parents |
|---|---|---|
| `Motor` | myosine, dynéine : cils, flagelles, muscles | cytosquelette |
| `Sensor` | opsines animales, canaux mécanosensibles, chimiorécepteurs | rhodopsine, signal |
| `Neural` | canaux et synapses des cellules nerveuses | signal, récepteur |
| `Matrix` | collagène, chitine, cellulose, lignine | adhésion, enveloppe |

Chacune coûte de l'entretien (muscles 6 000, récepteurs 2 000, tissu nerveux 12 000, matrice 1 500 kJ·molC⁻¹·an⁻¹ par unité).

**Traits** (`evo_life::AnimalTraits`, champ `animal` du phénotype) :
- motricité : moteurs × coordination, × 1,5 avec des appendices ;
- vitesse : 0,03 m·s⁻¹ × motricité × L^0,85 (L en mm, longueur de la sphère de même volume) ;
- portée de détection : 20 longueurs de corps par unité de récepteur ;
- défense : 1 − e^(−matrice) ;
- circulation : moteurs × coordination, chez un corps à tissus différents ;
- ouverture de la bouche : la moitié de la taille du corps chez un animal ;
- segments : un par régulateur ;
- paires d'appendices : à partir de 0,5 de moteur.

La coordination d'un corps est n / (n + 0,3 × doublements / 12), où n est le tissu nerveux. Un peu de signal entre cellules voisines en tient lieu chez les petits corps.

**Corps organisés.** Segments, appendices, circulation et organe sonore n'existent que chez un eucaryote multicellulaire à au moins deux types cellulaires (Lane et Martin, 2010 : l'énergie des mitochondries ; les tissus différenciés). La matrice libère la taille d'un corps massif ou d'une lame d'eucaryote au-delà des 4 096 cellules d'une colonie (jusqu'à 1,7 × 10¹³ cellules). Un filament reste une colonie.

**Prédation.** Une proie doit tenir dans la bouche du prédateur (rapport de taille, comme à l'étape 4). Un brouteur prend aussi 20 % d'un organisme fixé plus grand que lui. La capture vaut (1 + A) / (1 + A + E). A est l'attaque (motricité + sens) et E la fuite (motricité + sens + 3 × défense). Entre microbes sans moteur ni sens, elle vaut 1. Un tube digestif et une circulation portent la nourriture à tout le corps.

**Constructeur de phénotypes : viabilité.** Un mutant dont le plan n'est pas viable n'est pas retenu (`NonViable`) :
- `Detached` : des appendices sans matrice rigide (≥ 0,2) où s'attacher ;
- `Overweight` : un corps dont la sphère de même volume dépasse ce que porte son squelette, soit 3 mm dans l'eau et 1 mm sur terre pour un corps mou, × 10 par unité de matrice ;
- `Suffocating` : un corps aérobie dont la demi-épaisseur dépasse 1 mm sans circulation (× 300 avec une circulation complète).

**Organe sonore** (`evo_life::SoundOrgan`, dans la fonction du module tronc) : il naît des muscles, d'une pièce rigide et des nerfs.
- Type : frottement ou tymbale sous une carapace terrestre, larynx ou sac vocal avec un squelette interne terrestre, percussion, vessie natatoire ou clics dans l'eau.
- Taille : un dixième du corps.
- Puissance : 1 − e^(−moteurs × matrice).
- Complexité : nerfs / (1 + nerfs).
- Chœur chez un sexué, et moment de la journée, selon la variation cachée des récepteurs.
- Graine héritée, qui ne change qu'avec les gènes de l'appel.

**Sélection sexuelle.** Chez un sexué doté de récepteurs, la parade (puissance × (0,5 + complexité)) ajoute 0,05 × son gain au coefficient de sélection.

**Plan de construction, version 2.** Un animal a un tronc allongé et bilatéral. Il porte ses segments, des paires d'appendices articulées (charnière, section du muscle) attachées aux premiers segments, et son organe sonore. Une plante terrestre à matrice porte du bois, une écorce et des feuilles (un étage par régulateur). Matériaux : hydrostatique, chitine (carapace), cartilage puis os, bois.

**Étapes datées** (chemin « animaux et plantes », événements `Innovation`) : animal mobile, système nerveux, squelette, plante à tissus de soutien hors de l'eau, animal hors de l'eau, organe sonore. Sauvegardes au format 9.

## Porte : des animaux qui mangent

**Hétérotrophes.** Un animal est un eucaryote multicellulaire mobile qui englobe, sans photosynthèse. La perte du plaste passe par la perte de fonction d'un gène d'organite (comme l'apicoplaste des apicomplexes, devenu non photosynthétique).

**Horloge métabolique** (Kleiber ; West, Brown et Enquist, 1997). Un corps à réseau de distribution (circulation, ou squelette au tiers) vit plus lentement à mesure qu'il grossit. Le facteur vaut 1 / (1 + réseau × (N^¼ − 1)) pour N cellules. Il ralentit l'entretien, la mortalité de fond et l'ingestion maximale. Il est réservé aux eucaryotes à au moins deux types cellulaires : appliqué à toutes les colonies, il laissait les procaryotes coloniaux tout envahir.

**Recherche des proies** (réponse de Holling). La clairance d'un chasseur vaut π (détection + L/2)² × vitesse, rapportée à son carbone. La demi-saturation de son ingestion est le minimum entre celle des proies et ingestion maximale / clairance : un chasseur rapide et doté de sens trouve des proies rares.

**Peau d'une cellule.** Une sphère de plus de 4 096 cellules a une peau d'une cellule au contact du milieu. Ses zones intérieures sont vues à mi-épaisseur.

**Pelotes fécales** (Turner, 2015). Les restes non digérés des mangeurs multicellulaires coulent comme leurs cadavres, avec leur phosphore. Ceux des brouteurs unicellulaires restent dans la boucle microbienne : appliquées à eux, les pelotes retardaient les eucaryotes d'un milliard d'années.

**Méthane de l'air.** Les méthanotrophes des couches de surface demandaient à l'air jusqu'à 2 × 10¹⁵ mol de méthane par an, pour un stock de 2 × 10¹³ mol. Le facteur commun aux flux carbonés tombait alors vers 10⁻⁵ et coupait l'export organique et l'O₂ de la photosynthèse : l'O₂ s'effondrait après 1,5 Ga. Cette oxydation est désormais freinée seule, avec son oxydant (O₂, ou sulfate sans O₂), à bilans de carbone et d'électrons exacts.

**Endosymbioses dans l'eau.** [Simplification] L'eucaryogenèse et les plastes primaires sont aquatiques. Un hôte des terres, où les proies sont rares, ne gardait qu'une lignée condamnée : sur la Terre (graine 2026), l'eucaryote que l'accélérateur faisait naître sur la terre ferme s'éteignait, et la partie restait sans animaux.

**Extinctions de masse et radiations.** Le nombre de lignées vivantes est relevé par fenêtres de 10 Ma. Une extinction de masse (`MassExtinction`) est notée quand il tombe à 60 % ou moins du maximum des six fenêtres précédentes. Une radiation (`Radiation`) l'est quand il double au moins par rapport à leur minimum. Les deux règles ne valent qu'à partir de 50 lignées, et un épisode à la fois.

**Mémoire des longues parties.** [Simplification] À 3,5 Ga, une partie fondait 18,7 millions de lignées et inscrivait 14 millions d'événements « nouvelle lignée » : 5,5 Go en mémoire.
- Le génome fondateur d'une lignée éteinte est oublié.
- Une lignée éteinte depuis plus de 50 Ma, sans descendance gardée, sort du registre.
- Ses naissances et extinctions sortent du journal au même âge. La base SQLite de la partie garde tout le journal.
- Les identifiants restent uniques et croissants : registre et journal deviennent creux, et leurs lecteurs suivent un curseur d'identifiant (`total`, `since`).

Sauvegardes au format 11.

## Résultats

Résultats de la porte (graine 2026, niveau 4, pas de 100 ka, un tour d'évolution par 300 ka, jusqu'à 3,5 Ga), commande `evonisium complexite --bilan`. Bilans de carbone, de phosphore et d'électrons à 10⁻¹⁰ près sur toutes les parties. ⚑ : eucaryote venu de l'accélérateur de la complexité.

| Monde | Eucaryote | Multicellulaire à 2 types | Plante de terre | Système nerveux | Animal mobile | Squelette | Animal hors de l'eau | Mémoire max |
|---|---|---|---|---|---|---|---|---|
| Terre | 1,88 Ga ⚑ | 2,41 Ga | 2,46 Ga | 2,41 Ga | 2,51 Ga | 2,54 Ga | 2,54 Ga | 2,2 Go |
| Super-Terre | 2,19 Ga ⚑ | 2,25 Ga | 2,27 Ga | 2,44 Ga | 2,46 Ga | 2,54 Ga | 2,47 Ga | 5,4 Go (avant l'élagage du journal) |
| Sans lune | 1,02 Ga | 1,42 Ga | 1,42 Ga | 2,63 Ga | 2,70 Ga | 2,73 Ga | 2,77 Ga | 2,5 Go |
| Petite planète | 2,69 Ga ⚑ | 3,28 Ga | 3,29 Ga | 3,28 Ga | non | non | non | 1,2 Go |
| Désert | 2,21 Ga ⚑, éteint | non | non | non | non | non | non | 2,1 Go |
| Océan | vie éteinte | | | | | | | |

- **Plantes avant animaux** sur les trois mondes à animaux.
- **O₂** (part de l'atmosphère) : de 18 à 29 % sur la Terre de 1,2 à 3,4 Ga, 22 % sur la super-Terre à 3,5 Ga, de 5 à 35 % sans lune. Il ne s'effondre plus.
- **Prédateurs et proies.** Sur la Terre, de 2,5 à 2,8 Ga : 2 à 34 populations d'animaux, de 9 à 6 000 mol C. Leur alimentation vient pour 1 à 16 % des proies, et 13 à 1 557 populations sont chassées. Les corps vont jusqu'à 11 m et 11 m/s, avec jusqu'à 14 paires d'appendices et des organes sonores. Les animaux restent une petite part de la biomasse.
- **Extinctions de masse et radiations** : 2 ou 3 de chaque par partie.
- **Rejeu identique.** Les deux parties de la Terre, avant et après l'élagage du journal, ont la même histoire.
- **Vitesse.** À l'ère animale (Terre, 2,8 Ga), 30 Ma prennent 115 s sur 4 cœurs, lecture de la sauvegarde comprise, soit ≈ 260 ka/s ; 147 ka/s sur 2 cœurs. Porte : 100 ka/s sur 6 cœurs.

**Verdict : porte franchie avec accélération déclarée.** Des animaux mobiles, prédateurs, à squelette et sortis de l'eau apparaissent sans script sur la Terre et sur deux mondes de la vague 1 (super-Terre et sans lune), après les plantes de terre. Sur deux de ces trois mondes, l'eucaryote vient de l'accélérateur de la complexité ; ensuite, aucune transition n'est accélérée. La mémoire reste sous 5 Go.

## Reste à faire

- Désert : l'eucaryote de l'accélérateur ne vit que de matière dissoute (sa photosynthèse ne fixe pas de carbone) et s'éteint. La chimie de ses lacs est aussi extrême : 200 mol/m³ d'O₂, 5 × 10⁵ mol/m³ de sulfate.
- Océan : la vie s'éteint (déjà fragile à l'étape 4, 324 K).
- Petite planète et désert : l'O₂ monte de 50 à 86 % de l'atmosphère.
- Les animaux restent une petite part de la biomasse. La Terre finit en glaciation globale vers 3,5 Ga (cycle limite déjà noté).
- Les lectures cognitives du fil Cognition (`evo_culture::cognition::Cognition`).
- Le chantier vitesse, après cette PR.
