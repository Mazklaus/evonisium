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

## Résultats

Commande `evonisium complexite --suivre --bilan`, niveau 4 puis 6, un tour d'évolution par 300 ka, jusqu'à 3,5 Ga, environ 760 ka/s sur 4 cœurs (porte : 100 ka/s sur 6 cœurs). Bilans de carbone, de phosphore et d'électrons à 10⁻¹⁰ près sur toutes les parties. ⚑ : après l'accélérateur de la complexité.

| Monde | Eucaryote | Multicellulaire à 2 types | Animal mobile | Système nerveux | Squelette | Plante de terre | Animal hors de l'eau | Organe sonore |
|---|---|---|---|---|---|---|---|---|
| Terre, graine 2026 | 933 Ma | 2,32 Ga ⚑ | 2,32 Ga | 2,35 Ga | 2,37 Ga | 2,37 Ga | 2,32 Ga | 2,37 Ga |
| Terre, graine 7 | 1,93 Ga ⚑ | 3,03 Ga ⚑ | non | non | non | non | non | non |
| Sans lune | 892 Ma | 2,54 Ga ⚑ | 2,54 Ga | non | non | non | non | non |

O₂ (part de l'atmosphère) :
- Terre, graine 2026 : 13 à 25 % de 1,5 à 2,5 Ga.
- Terre, graine 7 : 23 à 25 % de 2,5 à 3 Ga.
- Sans lune : 0,2 à 8 %.

Il retombe en fin de partie sur les deux Terres : sous 0,1 % vers 3 Ga (graine 2026), 0,06 % à 3,5 Ga (graine 7).

**Verdict : porte pas encore franchie.** Les animaux, leurs nerfs, leurs squelettes, les plantes de terre et les organes sonores apparaissent sans script sur la Terre (graine 2026). Mais :
- les deux autres parties n'ont pas d'animaux à terre ;
- le premier animal naît déjà terrestre au lieu de suivre les plantes ;
- l'O₂ ne tient pas jusqu'à la fin ;
- la fin de partie oscille entre glaciations globales et épisodes chauds (cycle limite à dégazage faible, Mills et coll., 2011).

Sur une partie précédente (graine 2026, avant le correctif de l'enveloppe), la chronologie était meilleure : animaux à 1,60 Ga, squelette et plantes de terre à 2,51 Ga, O₂ de 7 à 26 % de 0,6 à 3,0 Ga.

## Reste à faire

- La porte : animaux et sortie des eaux sur deux mondes de la vague 1, plantes avant animaux, O₂ tenu jusqu'à la fin.
- Une sortie du cycle des glaciations de fin de partie.
- Les extinctions et radiations, suivies par un compteur.
- Les lectures cognitives du fil Cognition (`evo_culture::cognition::Cognition`), après la fusion.
