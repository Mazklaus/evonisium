# Étape 3 : moteur

Feuille de route (document Vision, consolidée après l'étape 2) : faire tourner la planète vivante à la résolution normale, à au moins 250 ka par seconde sur 6 cœurs, avec la vie des terres et des lacs, un bilan d'électrons fermé, et les services que le client attend (canal d'observation, événements et historiques interrogeables, points de sauvegarde, influence et intervention). Porte : l'oxygène s'accumule sur les six mondes de la vague 1 (le désert peut échouer), les électrons sont conservés, la vitesse plancher est tenue, et une partie se rejoue à l'identique sur Linux, Windows et macOS, quelle que soit la caméra.

## La porte

Rapport complet produit par `evonisium porte` : [etape-3-porte.md](etape-3-porte.md). Données (historiques échantillonnés, événements) dans `docs/etape-3-porte/`.

**Verdict : porte franchie sur les six mondes**, graine 2026, niveau 6 (40 962 cellules physiques, vie au niveau 5), pas de 200 ka avec deux tours d'évolution par pas.

| Monde | Photosynthèse oxygénique | O₂ > 10⁻⁴ | O₂ final | Accélérateur | Électrons corrigés | Vitesse (4 fils) | Verdict |
|---|---|---|---|---|---|---|---|
| Terre (Archéen) | 8,4 Ma | 33,0 Ma | 0,27 (131 % PAL) | jamais | 7,2 % | 291 ka/s | franchie |
| Monde océan | 4,6 Ma | 56,8 Ma | 0,79 (377 % PAL) | jamais | 6,1 % | 199 ka/s | franchie |
| Monde désertique | 7,8 Ma | 24,8 Ma | 0,12 (58 % PAL) | jamais | 10,8 % | 271 ka/s | franchie |
| Super-Terre | 4,0 Ma | 37,2 Ma | 0,65 (311 % PAL) | jamais | 7,8 % | 182 ka/s | franchie |
| Petite planète | 3,2 Ma | 70,4 Ma | 9,6·10⁻⁴ (0,46 % PAL) | jamais | 37,2 % | 213 ka/s | franchie |
| Monde sans lune | 2,8 Ma | 31,2 Ma | 0,31 (146 % PAL) | jamais | 9,9 % | 279 ka/s | franchie |

Carbone et phosphore sont conservés à 4·10⁻¹¹ près au pire, le registre des électrons à 7·10⁻⁸ près, et chaque partie se rejoue à l'identique (caméra immobile dans une partie, mobile dans l'autre). La CI compare l'empreinte d'une même partie sur Linux, Windows et macOS (job `rejeu`). Le monde désertique, en échec à l'étape 2, franchit la porte grâce aux lacs et aux sols humides : 3 300 cellules d'eaux douces colonisées.

Vitesse : sur un monde mûr (69 000 populations), 212 ka/s mesurés sur 4 fils et 285 ka/s estimés sur 6 cœurs ([etape-3-mesures.md](etape-3-mesures.md)). Les vitesses du tableau sont celles de parties entières, plus jeunes en moyenne.

## Conditions de Vision (document d'architecture, « Correction sur monde mûr »)

- **Pas de 200 ka.** Accepté s'il donne la même évolution qu'un pas de 100 ka. Avec un seul tour « apparition puis fixation » par pas, ce n'est pas le cas : les substitutions sont divisées par deux. Le moteur enchaîne donc un tour par tranche de 100 ka du pas (`EvolutionParams::round_years`). Avec ce réglage, les six grandeurs comparées sur 5 graines au niveau 4 sont équivalentes ([etape-3-equivalence.md](etape-3-equivalence.md)). Une graine au niveau 6 (Terre, 2026, 150 Ma) tombe dans la plage du niveau 4 : O₂ au-dessus de 10⁻⁴ à 33 Ma (niveau 4 : 27 à 104 Ma), photosynthèse oxygénique à 8,4 Ma (2,4 à 35 Ma), 9 guildes (6 à 9), 4,2 guildes par cellule (3,6 à 4,4). Les substitutions par Ma ne se comparent pas d'un niveau à l'autre : elles croissent avec le nombre de populations, et une fixation vaut pour tout un dème.
- **Plafond de 8 populations par cellule du vivant.** On évince d'abord la moins abondante (à biomasse égale, la dernière arrivée), jamais la dernière d'une guilde. La guilde est la voie principale (`Phenotype::main_pathway`, neuf au plus). La signature complète des voies distingue des centaines de combinaisons et viderait le plafond de son sens : une cellule en gardait plus de cent.
- **Tunnel borné à 2 essais par génotype et par pas.** Quand la borne est atteinte, chaque essai compte pour (candidats / essais), ce qui garde le taux de franchissement sans biais. La borne est atteinte pour 82 à 86 % des génotypes candidats.

## Limites connues

- **Plafond de populations saturé.** Sur la porte, 68 à 100 % des cellules peuplées dépassent le plafond avant éviction (100 derniers pas), bien au-delà du seuil de 10 % fixé par Vision. Mais l'éviction ne retire presque que des arrivants du pas même (colons, mutants d'une combinaison de voies nouvelle), qui n'ont pas encore grandi. Sur le monde mûr de [etape-3-mesures.md](etape-3-mesures.md), 52 % des cellules sont saturées, mais seules 0,1 % voient partir une population établie, c'est-à-dire de plus que la biomasse d'un fondateur. Une voie principale nouvelle n'est jamais évincée : elle est la dernière de sa guilde. Le plafond est soumis à Vision pour décision. Le relever coûte presque linéairement en populations : à 12, 34 % de cellules saturées et 1,18 s par pas sur 4 fils ; à 16, 19 % et 1,40 s ; à 8, 0,94 s.
- **Évolution rapide.** La photosynthèse oxygénique apparaît en 3 à 8 Ma, l'oxygène s'accumule en 25 à 70 Ma ; sur Terre, il a fallu des centaines de millions d'années. Le tunnel sans biais et les deux tours par pas l'accélèrent encore par rapport à l'étape 2 (16 à 26 Ma). L'accélérateur n'est jamais nécessaire.
- **Référence de 100 ka elle-même saturée.** Au pas de 100 ka, 94 % des génotypes fixent déjà un changement par tour : le nombre de substitutions est plafonné par le nombre de tours. Vision fera vérifier à l'étape 4 qu'un tour par 50 ka ne change pas l'évolution, et, si besoin, tirera plusieurs fixations par tour selon une loi de Poisson.
- **Niveaux d'oxygène au-dessus de la littérature.** La littérature place l'O₂ du Protérozoïque entre 0,1 et 10 % du niveau actuel (Lyons et coll., 2014 ; Planavsky et coll., 2014). Ici, l'O₂ final va de 0,5 % (petite planète) à 380 % (monde océan). Les puits ajoutés à l'étape 3 (oxydation du plancher océanique, des roches exposées, du sulfure) restent loin de la respiration profonde. Il manque encore le rétrocontrôle par le phosphore et les incendies au-delà de 25 %.
- **Électrons corrigés.** Prolonger les flux de surface sur tout le pas perd des électrons. La correction, inscrite au registre, déplace de 6 à 11 % de la production d'O₂, et 37 % sur la petite planète, dont la production est faible face au dégazage.
- **Format des sauvegardes.** bincode n'est pas auto-descriptif : tout changement d'un champ sauvegardé rend les anciennes sauvegardes illisibles. La version du format (2) les refuse avec un message clair. Un format versionné champ par champ viendra quand les parties devront survivre aux mises à jour.
- **Vitesse.** Le plancher n'est tenu qu'au pas de 200 ka, et sur l'estimation à 6 cœurs. Au pas de 100 ka, l'estimation est de 200 ka/s. L'évolution fait les trois quarts du pas ; la cible de 1 Ma/s est reportée à l'étape 4 (phénotypes incrémentaux, cache par génotype).
