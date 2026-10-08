# Étape 2 : planète vivante

Feuille de route (document Vision) : « Tectonique, climat et cycles chimiques couplés à la vie microbienne par le registre de flux ». Porte : « l'oxygène s'accumule par la photosynthèse, sans script, sur les six mondes de la vague 1 ». Le coordinateur y a ajouté : une partie se rejoue à l'identique depuis sa graine et le registre de ses ordres.

## La porte

Rapport complet produit par `evonisium porte` : [etape-2-porte.md](etape-2-porte.md). Données (historiques échantillonnés, événements, ordres) dans `docs/etape-2-porte/`.

**Verdict : porte franchie sur cinq mondes sur six.** Le monde désertique produit de l'oxygène par photosynthèse et en accumule par moments (jusqu'à 1,7·10⁻⁴), sans le tenir 50 millions d'années.

| Monde | Photosynthèse oxygénique | O₂ > 10⁻⁴ | O₂ final | Accélérateur | Électrons corrigés | Verdict |
|---|---|---|---|---|---|---|
| Terre (Archéen) | 18,8 Ma | 57 Ma | 2,1·10⁻⁴ | jamais | 5,3 % | franchie |
| Monde océan | 25,8 Ma | 131 Ma | 0,55 | jamais | 3,3 % | franchie |
| Monde désertique | 186 Ma | 354 Ma (passager) | 1,4·10⁻⁵ | jamais | 17,5 % | non franchie |
| Super-Terre | 20,2 Ma | 81 Ma | 0,30 | jamais | 7,6 % | franchie |
| Petite planète | 16,2 Ma | 68 Ma | 1,2·10⁻² | jamais | 13,8 % | franchie |
| Monde sans lune | 19,0 Ma | 53 Ma | 3,1·10⁻² | jamais | 3,9 % | franchie |

Carbone et phosphore sont conservés à 10⁻⁸ près au pire, et chaque partie se rejoue à l'identique. L'accélérateur n'a jamais été nécessaire : exaptation, duplication, transfert et tunnel suffisent.

Ce qu'il faut savoir avant de s'appuyer sur ces résultats :

- **Monde désertique.** Ses mers, au fond des bassins, passent de 40 % à 5–15 % de la surface quand la croûte océanique vieillit et s'enfonce. La production y est cinquante fois plus faible que sur Terre, alors que l'hydrogène volcanique est à l'échelle de la planète : le méthane l'emporte. Pistes pour l'étape 3 : la vie des terres et des lacs (tapis microbiens), absente pour l'instant, et une calibration du dégazage réduit selon le monde.
- **Niveaux d'oxygène.** Sur la Terre, l'O₂ franchit le seuil vers 57 Ma mais oscille ; il ne le tient 50 Ma qu'au bout de 1,25 Ga. Sur les mondes sans terres ou presque (monde océan, super-Terre), rien ne freine l'oxygène hors de la respiration profonde : il monte à 30 et 55 %, avec un CO₂ de plusieurs bars. C'est cohérent avec le modèle, pas avec un monde réel qui aurait d'autres puits (oxydation des fonds océaniques, incendies au-delà de 25 %).
- **Électrons corrigés.** L'extrapolation des flux de surface sur tout le pas perd des électrons ; la correction déplace de 3 à 17 % de la production d'O₂. C'est la principale approximation numérique de cette étape : des pas plus courts ou une écologie résolue plus longtemps la réduiraient.


Un monde franchit la porte quand quatre conditions sont réunies :

1. la fraction d'O₂ de l'air dépasse 10⁻⁴ (environ 0,05 % du niveau actuel de la Terre, au-dessus de la limite de 10⁻⁵ PAL qui marque la fin de l'Archéen dans les isotopes du soufre) et s'y maintient 50 millions d'années ;
2. cet oxygène vient de la photosynthèse oxygénique apparue par évolution : c'est la seule source d'O₂ libre du moteur, et le budget de l'oxygène du rapport le prouve processus par processus ;
3. carbone et phosphore sont conservés à 10⁻⁶ près sur toute la partie (registre de flux) ;
4. la partie se rejoue à l'identique (même empreinte de l'état complet) depuis sa graine et ses ordres, y compris un changement de vitesse et deux interventions en cours de partie.

Aucune règle ne dit « à telle date, l'oxygène apparaît ». Les dates du rapport sortent de la sélection, de la chimie et du climat.

## Ce qui a été construit

| Crate | Ajouts de l'étape 2 |
|---|---|
| `evo-planet` | **Tectonique** : plaques rigides tournant autour de leur pôle, dorsales, subduction avec arcs volcaniques, collisions continentales, subsidence thermique de la croûte océanique, isostasie, érosion, réorganisation périodique des plaques. **Climat** à l'équilibre de type Budyko-Sellers par cellule : effet de serre du CO₂ (avec élargissement par la pression) et du CH₄ (saturé au-delà de 10⁴ ppb, voile organique quand CH₄/CO₂ dépasse 0,1), rétroaction glace-albédo avec mémoire (boules de neige et sorties de boule de neige), écran d'ozone, obliquité qui dérive sur un monde sans lune, étoile qui s'éclaircit. **Cycles géochimiques en boîtes** (atmosphère, océan profond, sédiments) : dégazage, altération des silicates, enfouissement du carbone organique et des carbonates, phosphore, fer, sulfure, chimie atmosphérique rapide (titrage H₂-O₂, oxydation du méthane, échappement de l'hydrogène). Six mondes de la vague 1 en préréglages. |
| `evo-genetics` | Duplication suivie de divergence entre domaines apparentés, transfert horizontal de gène, tunnel stochastique (probabilité de Weissman et coll. 2009 et simulation Wright-Fisher de référence), journal des modifications de génome avec leur cause. |
| `evo-life` | Pigments à spectre d'absorption évalués sous le spectre de l'étoile filtré par l'eau (couleur affichable). Chemin vers la photosynthèse en quatre étapes : pigment protecteur, phototrophie simple (cyclique ou rhodopsine), photosynthèse anoxygénique, photosynthèse oxygénique. Un pigment seul protège (bénéfice propre). Stœchiométrie du réducteur des autotrophes (2 H₂ par carbone fixé chez les méthanogènes, ½ CH₄ chez les méthanotrophes). Pompe biologique : 15 % de la nécromasse coule avec son phosphore. |
| `evo-core` | Événements enrichis : cause liée, origine (moteur, joueur, accélérateur), score d'intérêt. |
| `evo-sim` | **File d'ordres** datée et enregistrée, appliquée entre deux pas ; les commandes du temps (durée du pas, pause, reprise) y passent. **Rejeu** depuis la graine et le registre. **Historique échantillonné** des grandeurs globales. **État publié** daté et immuable à chaque pas, le précédent conservé. **Accélérateur** de dernier recours, journalisé. Commande `evonisium porte`. |

## Comment l'oxygène apparaît

Le verrou de l'étape 1 était le suivant : la photosynthèse demande un pigment et une enzyme à la fois, et chacun seul coûte. Le document Génétique (« Franchir les innovations à plusieurs composants ») propose une série de mécanismes, implantés dans cet ordre de préférence :

- **Exaptation.** La cellule minimale porte des cytochromes, qui absorbent vers 420 nm comme les vrais. Un pigment protecteur contre les UV a un bénéfice propre ; il devient ensuite l'antenne d'une phototrophie.
- **Rendements continus.** Une phototrophie faible rapporte un peu d'énergie, qui couvre d'abord l'entretien ; le gain croît avec l'efficacité du pigment sous la lumière de l'étoile.
- **Duplication puis divergence.** Une enzyme dupliquée peut dériver vers une enzyme apparentée (table de parenté des domaines) pendant que l'original garde sa fonction.
- **Transfert horizontal.** Un gène peut passer d'une population à une autre de la même cellule.
- **Tunnel stochastique.** Réservé aux doubles mutants qui gagnent une fonction (une réaction, une étape du chemin, une rhodopsine) : ailleurs, la seconde mutation seule, bien plus fréquente, l'emporte. Un premier mutant qui ne se fixe pas peut porter un second mutant avantageux avant de disparaître. La probabilité vient de l'équation du processus de branchement (Weissman et coll., 2009), résolue par Newton, et elle est vérifiée contre une simulation Wright-Fisher explicite :

| N | δ (coût du premier mutant) | μ₂ | s (avantage du double mutant) | Théorie | Wright-Fisher (80 000 répétitions) |
|---|---|---|---|---|---|
| 10 000 | 0,01 | 10⁻³ | 0,1 | 0,01089 | 0,01022 ± 0,00037 |
| 10 000 | 0 | 2·10⁻⁴ | 0,05 | 0,00596 | 0,00553 ± 0,00027 |
| 5 000 | 0,05 | 10⁻² | 0,2 | 0,03931 | 0,03835 ± 0,00069 |

L'approximation du branchement surestime un peu (de 2 à 7 %) : elle néglige la dérive du double mutant aux fréquences intermédiaires. L'écart est connu dans la littérature et sans conséquence ici.

- **Accélérateur, en dernier recours.** Si le chemin vers la photosynthèse ne progresse plus pendant 300 millions d'années, les mutations innovantes et les transferts sont multipliés par 100 jusqu'au franchissement suivant. Chaque fixation est tirée deux fois, sans et avec l'accélérateur : celle qui n'a lieu qu'avec lui porte la cause « Accélérateur » dans le journal des génomes et l'origine « accélérateur » dans les événements. Le rapport de la porte compte ses pas actifs et ses fixations.

Une fois la photosynthèse oxygénique apparue, l'oxygène ne s'accumule pas tout de suite. Il est d'abord consommé sur place (respiration, fer, sulfure), puis dans l'air par l'hydrogène et le méthane volcaniques et biologiques. Il ne monte que lorsque l'enfouissement du carbone organique (pompe biologique, sédiments) dépasse ces puits, comme le décrivent les modèles de la grande oxydation. Deux rétroactions le plafonnent ensuite : l'oxydation des roches exposées croît comme la racine carrée de la fraction d'O₂ (loi des modèles de type COPSE, environ 7·10¹² mol d'O₂ par an aux niveaux actuels sur des terres comme les nôtres), et un océan profond oxygéné enfouit moins de carbone organique.

## Ce que la mise au point a corrigé

Les premières parties longues ont révélé quatre défauts, corrigés avant la porte :

- **Fuite d'électrons.** Les flux de surface mesurés pendant l'écologie rapide sont prolongés sur tout le pas comme des taux d'équilibre. La fermeture du carbone et du phosphore ne garantissait pas celle des électrons : du méthane apparaissait sans source réductrice et consommait tout l'oxygène. Une couche à l'équilibre n'exporte désormais que le pouvoir réducteur de ses sources hydrothermales (la vie ne fait que le déplacer) ; l'écart corrigé est compté et publié par la porte.
- **Oscillation d'un pas à l'autre.** Avec des pas de 200 000 ans, le couplage explicite entre l'atmosphère et la vie faisait alterner méthane abondant et méthane nul. Les flux appliqués sont la moyenne de ceux du pas et du pas précédent (schéma amorti, même équilibre).
- **Génomes enflés.** Une cellule gorgée de lumière croissait au plafond quel que soit son génome : rien ne freinait les copies inutiles (300 gènes après 100 Ma). Le taux de croissance maximal baisse désormais de 0,2 % par gène (temps de réplication).
- **Monde désertique.** Trois corrections physiques, sans effet sur les autres mondes ou presque : le gradient thermique se mesure depuis l'altitude moyenne de la surface et non depuis des mers enfouies au fond des bassins (toutes les terres gelaient) ; une dorsale émergée dégaze dans l'air, seule la part immergée nourrit la mer en réducteurs (avec un plancher de 10 % pour la circulation hors axe, et des sources sur au moins 5 % des mers) ; l'altération n'use que les terres que la pluie venue des mers peut arroser (au plus une fois l'aire des mers).
- **Mémoire.** Une partie de 1,5 Ga fonde des millions de lignées éphémères ; le registre gardait chaque génome fondateur (14 Go). Une lignée éteinte sans fille oublie son génome et ne produit plus d'événement (elle reste comptée).

## Simplifications

Chaque module annonce les siennes dans son en-tête (`[Simplification]`). Les principales :

- **Tectonique** : nombre de plaques fixe, réorganisées par de nouveaux pôles plutôt que fragmentées ou fusionnées ; pas de mécanique des roches, de convection, de points chauds ni de réseau fluvial.
- **Climat** : pas de circulation ni de précipitations ; un seul albédo pour toute surface libre de glace ; seule la mémoire de la glace relie deux pas (climat à l'équilibre, pas de 200 000 ans dans la porte).
- **Cycles** : carbone inorganique de l'océan profond confondu avec celui de l'air ; chimie atmosphérique résumée en taux ; soufre non fermé. Les flux mesurés pendant l'écologie rapide sont extrapolés sur le pas comme des taux d'équilibre, en sous-pas adaptatifs.
- **Cycles (suite)** : correction du bilan des électrons et moyenne des flux de deux pas, décrites plus haut ; sur un monde sans terres (monde océan), seule la respiration profonde freine l'oxygène, qui monte très haut.
- **Vie** : au plus 12 populations par cellule (les plus petites rendent leur biomasse à l'eau) ; une seule couche d'eau de surface par cellule ; pas de tapis microbiens ni de sédiments vivants.
- **Porte** : grille de niveau 4 (2 562 cellules) au lieu de 6 (40 962), pour que les six mondes tiennent en un temps raisonnable sur plusieurs milliards d'années. Le moteur est identique ; seule la résolution change.

## Ce qui attend l'étape 3

SQLite (sauvegardes, historiques par région et par espèce), la zone d'intérêt de la caméra (canal d'observation), le client Godot (`client/`, `evo-godot`) et le double tampon de l'état publié partagé entre fils.

## Mesures

Rapport complet : [etape-2-mesures.md](etape-2-mesures.md) (machine de mesure à 4 fils, estimations pour les 6 cœurs laissés à la simulation).

Le pas complet à 40 962 cellules, à pleine charge, coûte 3,1 s contre 208 ms à l'étape 1 : la planète elle-même ne compte presque pas (15 ms), ce sont l'écologie couplée à la chimie (1 s) et l'évolution avec tunnel et transfert (1,3 s) sur 307 000 populations. Sur 6 cœurs, 1 Ma/s demanderait des pas de 2 Ma : le budget du document Vision n'est pas tenu à cette résolution. La porte a donc été passée sur la grille de 2 562 cellules (environ 0,1 à 1 Ma/s). Pistes pour l'étape 3 : moins de candidats mutants par pas quand rien ne change, écologie rapide plus courte dans les cellules à l'équilibre, plafond de populations par cellule plus bas.

La mémoire tient : 0,7 Go pour 40 962 cellules, et les très longues parties sont bornées (registre des lignées allégé, journal plafonné).

