# Chantier vitesse

Feuille de route : document d'architecture, « Chantier vitesse ». Le chantier
vient de l'étude de vitesse du 2026-10-12. Il se mène en trois vagues avant
l'intégration de l'étape 6 :

1. des optimisations pures ;
2. un recalage ;
3. une grille fine du vivant limitée aux terres et aux côtes.

Les cibles visent le PC du joueur, au niveau 4 :

| Période | Cible | Plancher |
|---|---|---|
| Jusqu'aux premiers animaux | 10 Ma/s | 5 Ma/s |
| Ère animale | 1 Ma/s | 0,5 Ma/s |

## Vague 1 : optimisations sans changer l'histoire (PR #40)

Elle apporte trois optimisations :

- un cache des bases phénotypiques ;
- une reconstruction incrémentale après une mutation structurelle ;
- une virgule fixe sur 64 bits.

Un pas demande 1,33 fois moins d'instructions. Les sauvegardes restent identiques octet pour octet.

## Vague 2 : recalage

### Ce qui change

- **Tour d'évolution d'un million d'années** au lieu de 300 ka, et **10 sous-pas d'écologie** au lieu de 30.
- **Garde-fou des guildes.** La clé de guilde du plafond de populations par cellule change :
  - **Avant :** elle combinait la voie principale et les bits d'organisation. Il y avait alors autant de guildes que de populations : 40 par cellule à 1,4 Ga, et le plafond ne tenait plus.
  - **Après :** elle combine la source d'énergie de la voie principale et l'organisation (phagotrophe, eucaryote, plaste, multicellulaire, terre ferme). La source d'énergie a quatre valeurs : chimie sans oxygène, respiration aérobie, lumière anoxygénique, photosynthèse oxygénique.
  - **Pourquoi pas l'organisation seule :** essayée, elle faisait évincer les premières oxygéniques et les premiers phagotrophes, et la complexité prenait plus d'un milliard d'années de retard.
- **Rétention des endosymbiotes** : 1e-40 par proie englobée, au lieu de 1e-38. Avec la nouvelle clé, hôtes et proies cohabitent mieux, et l'eucaryote serait sinon venu dès l'oxygénation.
- **Famine plafonnée à 20 /an** (dormance). Les cellules affamées ralentissent leur entretien au lieu de se consumer.
  - **Avant :** le monde océan perdait toute sa vie dans des cycles d'expansion et d'effondrement, où les méthanogènes vidaient l'H₂ et la famine tuait jusqu'à 2 000 /an.
  - **Après :** il garde sa vie, ses eucaryotes et ses multicellulaires.
- **Vie des terres sous la glace.** Une terre prise sous la glace comptait comme une cellule sans terre : une glaciation globale effaçait toute la vie des terres. Seules les cellules sans terre émergée perdent maintenant leurs formes terrestres. Les autres attendent le dégel en dormance.
- **Feux.** Au-delà de 21 % d'O₂ dans l'air, la production de la végétation des terres est divisée par 1 + 20 × excès². L'excès vaut (O₂ − 21 %) / 14 %, et le terme est atténué sur un sol humide.
  - Cela fait ×2,6 à 25 % et ×21 à 35 %.
  - [Simplification] C'est le frein de l'O₂ des modèles de type COPSE (Lenton et Watson).
  - Sans ce frein, l'O₂ montait de 50 à 86 % sur le désert et la petite planète, et à 66 % sur la Terre après les plantes.
  - Une mortalité par le feu brûlait toute la végétation d'un coup à 35 % d'O₂ : sa décomposition vidait l'air de son O₂, puis une glaciation totale suivait.
- Sauvegardes au format 12.

### Résultats

Toutes les parties vont jusqu'à 3,5 Ga, au niveau 4 (vie au niveau 3, puis à la résolution de la planète quand les plantes gagnent les terres), avec un pas de 100 ka.

La graine 2026 de la porte de l'étape 5 garde la Terre sans O₂ jusqu'à 3,2 Ga. Ce ne sont pas les puits d'O₂ : l'oxygénique apparaît à 0,9 Ga puis sa lignée s'éteint. À 1,2 Ga, aucune population n'a l'oxygénique pour voie principale, et la vie ne libère plus d'O₂. Or l'accélérateur de la photosynthèse ne regarde que la meilleure étape jamais atteinte : il ne se rallume pas. C'est une panne du modèle, à corriger pendant l'intégration de l'étape 6. Vision accepte cette histoire comme possible. Comme pour les portes précédentes, les graines sont essayées dans l'ordre.

Sur quatre autres graines (7, 42, 99, 414299), la Terre se comporte de façon régulière :

- l'oxygénique arrive entre 0,82 et 0,94 Ga ;
- l'O₂ se tient entre 16 et 43 % de 1,0 à 2,5 Ga.

Le tableau donne les résultats avec la graine 7. ⚑ : eucaryote venu de l'accélérateur de la complexité.

| Monde | Oxygénique | Eucaryote | Plante de terre | Système nerveux | Animal mobile | O₂ de 1,2 à 3,0 Ga |
|---|---|---|---|---|---|---|
| Terre | 0,82 Ga | 2,16 Ga ⚑ | 2,78 Ga | 2,86 Ga | 2,98 Ga | 17 à 25 % |
| Super-Terre | 0,88 Ga | 2,30 Ga ⚑ | 2,45 Ga | 3,04 Ga | 3,27 Ga | 14 à 23 % |
| Sans lune | 0,82 Ga | 2,17 Ga ⚑ | non | non | non | 15 à 24 % |
| Désert | 1,00 Ga | 2,27 Ga ⚑ | 2,79 Ga | 2,76 Ga | 3,15 Ga | 23 à 32 % |
| Petite planète | 0,86 Ga | 2,77 Ga ⚑ | non | non | non | 24 à 27 % |
| Océan | 1,25 Ga | 1,41 Ga | — | — | — | O₂ nul après 2 Ga, vie gardée (333 K à 3,5 Ga) |

- **Conditions de Vision pour la vague 2 :**
  - Aucun monde ne perd toute sa vie.
  - Le désert et la petite planète ont des eucaryotes viables, et le désert a des animaux. Sur la petite planète, les eucaryotes tiennent de leur arrivée à 2,77 Ga jusqu'à 3,5 Ga, entre 0,6 et 3,2 % de la biomasse.
  - L'O₂ reste sous 35 % sur les petits mondes.
- **Bilans** de carbone, de phosphore et d'électrons à 10⁻¹⁰ près sur toutes les parties.
- **Rejeu identique** : même sauvegarde octet pour octet à 0,3 Ga sur 1 et 4 cœurs.
- **Animaux.** Ils arrivent plus tard qu'à la porte de l'étape 5 (2,98 Ga au lieu de 2,51 sur la Terre), toujours après les plantes de terre. Sur la graine 42, la Terre a ses plantes à 2,93 Ga mais pas d'animaux avant 3,5 Ga.

### Vitesse (Terre, graine 7)

Mesures prises sur la machine de travail, lecture de la sauvegarde comprise. Le PC du joueur est estimé à 2,7 fois cette machine.

| Moment | Avant (main, 1 cœur) | Vague 2, 1 cœur | Vague 2, 4 cœurs | PC du joueur estimé |
|---|---|---|---|---|
| Ère microbienne, 1,5 à 2,3 Ga | 155 s par 100 Ma | 60 s par 100 Ma (×2,6) | 6,3 Ma/s | ≈ 17 Ma/s |
| Ère animale, 3,2 Ga | | 0,11 Ma/s | 0,41 Ma/s | ≈ 1,1 Ma/s |

À l'ère animale, la porte de l'étape 5 allait à 260 ka/s sur 4 cœurs ; la vague 2 va à 405 ka/s, soit ×1,6.

Les deux cibles sont atteintes au niveau 4.

### Reste à faire

- **Populations par cellule.** Il y en a 9,7 en moyenne à 2,0 Ga et 19 à 3,2 Ga. Les guildes de l'ère complexe (organisation × source d'énergie) sont nombreuses.
  - Règle acceptée par Vision : la dernière population de chaque guilde est exemptée, et les autres sont évincées par biomasse au-delà de 8 populations dans la cellule.
  - Une moyenne jusqu'à 20 par cellule est tolérée tant que les planchers de vitesse et les 5 Go tiennent.
- **Fin des parties.** Vers 3,2 Ga, la Terre retombe parfois dans le cycle limite déjà noté à l'étape 5 : l'O₂ s'effondre, le CO₂ bondit, puis vient une glaciation.
- **Sans lune et petite planète** n'ont pas d'animaux sur la graine 7.
- La vague 3 (grille fine du vivant limitée aux terres et aux côtes) passe en réserve : les cibles sont atteintes avec de la marge. Elle reviendra si une mesure chez le joueur tombe sous un plancher, ou si les étapes 6 et 7 coûtent trop.
