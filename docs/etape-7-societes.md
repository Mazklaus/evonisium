# Étape 7, volet sociétés et histoire

Feuille de route (document Vision, « Périmètre et porte de l'étape 7 ») : une crate `evo-society` à côté d'`evo-culture` (sociétés et institutions, économie de 20 à 30 biens par cellule, catalogue des techniques jusqu'à un équivalent du XXIe siècle avec ses voies de remplacement, sciences, guerre et diplomatie, cycles séculaires, épidémies agrégées, flux vers la planète, fin de la première version), le format de l'état des sociétés publié, et des tests historiques sur peuples d'essai qui forment la porte de l'étape. Le tout sur bancs d'essai, sans toucher `evo-life`, `evo-sim`, `evo-planet` ni `evo-genetics`.

**Verdict : la porte est tenue sur banc d'essai.** Restent à faire avec l'intégration au moteur : la mesure de la mémoire sur une partie complète, la vitesse sur la machine cible, et le branchement des flux humains sur le vrai climat d'`evo-planet`.

| Critère de la porte | Résultat (`cargo test --release -p evo-society -- --ignored porte_cinq_terres` et `tests/histoire.rs`) |
|---|---|
| Terre, ≥ 5 graines : ≥ 2 foyers agricoles indépendants | 4, 5, 4, 5, 4 foyers (graines 1 à 5) |
| Premiers États 3 à 10 ka après l'agriculture | 8 214, 8 575, 8 906, 6 593, 7 517 ans |
| Au moins un effondrement | 273, 211, 217, 357, 283 |
| Plateau malthusien avant la sortie industrielle | revenu par tête × 1,33, 1,34, 1,23, 1,21, 1,18 pendant que la population est multipliée par 5,4, 3,6, 3,6, 9,5, 4,6 ; sorties en 1781, 1342, 1001, −668, −1378 |
| Un monde de la vague 1 suit un autre chemin technique | désert : 134 techniques en 12 000 ans, sans charbon ni pétrole, énergie du soleil et du vent ; monde océan : voie aquatique |
| Les flux humains changent climat et sols, bilan carbone fermé | forçage 0,21 à 0,39 W/m² en 2000 (pas climatique de 10 ans atteint sur chaque graine), sols et érosion suivis ; bilan à 3·10⁻¹⁴ près |
| Deux espèces intelligentes se rencontrent et commercent ou se battent | test `deux_especes_se_rencontrent` |
| Fin de la v1 racontée, la simulation continue | Terre de 1750 : fin de la v1 en 2208, la simulation poursuit |
| Rejeu identique au bit | empreintes égales sur deux parties (test `le_rejeu_est_identique_au_bit_pres`) |
| Mémoire sous 5 Go | état publié de 1,9 Mo sur banc ; partie complète à mesurer dans le moteur |
| Mode historique ≥ 100 ans/s avec ≥ 300 sociétés | 180 ans/s avec 465 sociétés (conteneur à 4 cœurs) |

## Comment c'est fait

**Le monde des sociétés** (`world.rs`). Une grille de cellules (terres, mers, liens), chacune avec au plus un peuplement : espèce, peuple, société, effectif, techniques connues, plante et bête domestiquées, part urbaine, immunités. Un pas vaut un an (mode historique, règle de Vision) ; l'écologie des cellules suit au même pas sur banc d'essai et passera au pas de dix ans dans le moteur. Les calculs par cellule (économie, démographie, migrations) se font en parallèle sur l'état d'avant le pas, puis s'appliquent dans un ordre fixe : le rejeu ne dépend ni du nombre de cœurs, ni de la lecture de l'état.

**Économie** (`economy.rs`, `goods.rs`). Vingt-huit biens : grain, tubercules, bétail, produits animaux, poisson, bois, charbon de bois, pierre, poterie, textile, cuir, sel, cuivre, étain, bronze, fer, outils, armes, bâtiments, charbon, pétrole, électricité, machines, engrais, luxe, livres, médicaments. Chaque année, le travail va par tranches au secteur qui nourrit le mieux à la marge (cueillette, pêche, champs, troupeaux), puis aux demandes dans un ordre fixe (combustible des moteurs, chauffage, outils, machines, bâtiments, vêtements…), chaque bien fabriqué par la recette la moins coûteuse que la cellule connaît, entrées d'abord, limitée par le travail et par les gisements. Les champs couvrent une part des terres arables qui dépend de l'outil (houe 4 %, araire 12 %, charrue lourde 30 %, tracteur tout) plus les plaines d'inondation ; le rendement suit la plante domestiquée, le sol, le climat, l'irrigation et les engrais. Les sols s'usent et se refont. Le commerce porte les surplus vers les voisins qui manquent, avec des pertes en route ; le tribut va du vassal à la capitale du suzerain. Une mine de charbon extrait aussi pour ses voisins, qui s'en chauffent puis en nourrissent leurs machines.

**Démographie.** Naissances et morts de base, plus la faim ; la mortalité urbaine monte avec les maladies de foule. La transition démographique vient de l'école et de la santé (Caldwell, Lee), pas de la seule richesse, et s'arrête près du renouvellement.

**Domestication.** Une plante ou une bête sauvage devient cultivée là où sa cultivabilité (ou sa docilité) et l'accord avec le climat le permettent : la sélection fait monter sa qualité, comme une pression de sélection ; le génome de l'espèce sauvage n'est jamais écrit (règle de Vision). Pas d'agriculture sous un climat glaciaire. Une plante ou une bête du voisin peut s'emprunter si elle convient au climat.

**Techniques** (`catalog.rs`, `data/techniques.txt`). 203 nœuds, des outils de pierre à l'internet, avec prérequis (et groupes « au moins un de »), difficulté au sens de Henrich, exigences du corps (mains, milieu, feu, ciel visible), ressources, niveau de science requis, effets et recettes. Les voies de remplacement sont préfixées `aq_` (aquatique), `po_` (pauvre en oxygène), `st_` (souterraine) et `sm_` (sans mains) : une espèce marine sans feu passe par les coquilles, l'algoculture et l'électrochimie. L'invention suit Kremer : son rythme croît avec la population qui échange vraiment des idées (densité, villes, partenaires), avec l'écriture et la méthode expérimentale, et chaque marche de la chaîne des prérequis coûte, d'autant moins que la société a déjà gravi de marches. Les techniques se diffusent entre voisins et partenaires, et se perdent dans trop peu de praticiens (cas tasmanien).

**Sociétés** (`society.rs`). Six dimensions (centralisation, inégalités, coercition, participation, administration, cohésion) qui tendent vers des cibles tirées de la taille, des stocks, des menaces, de la circonscription et des techniques ; sept institutions (conseil, souverain, tribunal, temple, armée, marché, école) ; une étiquette d'affichage (bandes, tribu, chefferie, État, empire, État moderne) qui sert de repère et non d'échelle. Les cycles séculaires suivent Turchin et Goldstone : la pression démographique, la surproduction d'élites et la dette font monter une tension ; au-delà, crise, guerre civile, scissions, et parfois effondrement (une société d'au moins État qui perd son rang, les deux tiers de son territoire ou 40 % de sa population).

**Guerre et diplomatie.** Paix, commerce, alliance, tribut, guerre. Une guerre se décide sur le rapport des forces, l'ambition, la faim et les élites ; une bataille tue, et prend une cellule si la société peut encore la tenir (sinon c'est un raid). La taille tenable croît de la chefferie (2 cellules) à l'État moderne.

**Sciences** (`science.rs`). Pour huit domaines (mécanique, chimie, climat, géologie, biologie, hérédité, maladies, astronomie), chaque société estime une vraie grandeur du monde (la pesanteur, l'âge de la Terre…) avec une précision bornée par l'œil nu tant qu'elle n'a pas l'instrument ; les anomalies s'accumulent, le modèle dominant résiste (d'autant plus que le temple est fort) puis bascule. Certaines techniques demandent une précision minimale.

**Épidémies** (`disease.rs`). Une maladie naît des villes qui côtoient le bétail ; elle a un R₀, une létalité et une immunité durable ou non. Les vagues suivent Kermack et McKendrick ; une maladie à immunité durable devient endémique au-dessus d'une population critique et finit maladie d'enfance ; une peste qui revient tue moins que la première fois. Sur un sol vierge, toute la létalité.

**Notables.** Chefs, prophètes, conquérants, savants et inventeurs, tirés du hasard de l'état simulé quelle que soit la caméra, 5 000 vivants au plus ; ils pèsent sur leur société (un chef habile tient mieux, un savant double l'avancée de son domaine).

**Flux vers la planète.** Chaque cellule rend chaque année du CO₂ (feu, défrichement, charbon, pétrole, calcination), du méthane (ruminants, rizières), des aérosols, l'usage des sols, l'irrigation et l'érosion. Le carbone est compté de bout en bout (forêts, sols, gisements, stocks, air) : le bilan reste fermé à 10⁻¹⁴ près. Le forçage en tire une valeur en W/m² ; au-dessus de 0,1 W/m², le pas climatique passe à dix ans (règle d'architecture).

**Fin de la première version.** Quand une civilisation de tête, reliée par un réseau mondial et industrielle, a une science moderne dans les huit domaines, la chronique le raconte ; la simulation continue, sans les techniques spéculatives.

**Bancs d'essai** (`bench.rs`). Une Terre à 5° (72 × 36), de −15 000 à aujourd'hui, avec ses déserts, montagnes, fleuves, gisements et 22 espèces sauvages domesticables ; un monde désertique sans charbon ni pétrole ; un monde océan peuplé de céphalopodes ; un monde à deux espèces intelligentes ; une Terre de 1750 pour la fin de la v1 et les flux.

## État des sociétés publié

`evo_society::state::SocietyState` (`state.rs`, version 1, `FORMAT_VERSION`), tiré de l'état à chaque pas par `SocietyState::of`, lu seul, en identifiants stables, sérialisable (aller-retour vérifié) :

- `societies` : nom, parent, espèce, peuple, fondation, capitale, cellules, étiquette et nom de l'étiquette, rang le plus haut, dimensions, agrégats (population, PIB, rations, part urbaine, alphabétisation, énergie moderne, tension…), institutions, religion, crise, guerre, chef, sciences (précision, paradigme, estimation et vraie valeur), techniques connues ;
- `cells` : peuplement (société, espèce, effectif, part urbaine, ville), plante et bête, rations, PIB et puissance par tête, biens produits, consommés et en stock, prix, champs, cheptel, sol, forêt, flux, morts de l'année, maladies qui circulent, techniques ;
- `cities`, `routes` (commerce de l'année, par mer ou non), `relations`, `notables` (vivants et derniers morts, avec leurs faits), `diseases` ;
- `species` : population, sociétés, sortie industrielle, fin de la v1, espèces rencontrées ;
- `planet` : flux de l'année et cumulés, forçage, pas climatique, bilan carbone ;
- `events` : la chronique de l'année (invention, foyer agricole, étiquette, crise, guerre, conquête, épidémie, ville, paradigme, premier contact, sortie industrielle, fin de la v1…) ;
- `fingerprint` : empreinte de l'état simulé, pour les tests de rejeu.

## Simplifications et limites

- **Peuples d'essai.** Les espèces des bancs ont des lectures cognitives écrites à la main (`evo_culture::cognition::presets`), et la Terre est une carte à 5° dessinée par boîtes. Les vraies viendront du moteur.
- **Constantes d'ordre de grandeur.** Taux d'invention, coût des marches, rendements, travail aux champs, létalités : calés pour que les cinq Terres tombent dans les fourchettes de la porte, pas mesurés. Les dates varient beaucoup d'une graine à l'autre (écriture entre −5 000 et −800, sortie industrielle entre −1 400 et 1 800) : l'histoire est un tirage, et la porte vérifie des formes, pas un calendrier.
- **Industrie faible.** Les sociétés malthusiennes laissent peu de bras libres : après la vapeur, la part fossile de l'énergie monte lentement, et la Terre de 1750 ne dépasse guère 0,03 W/m² de forçage en 2000 (le forçage des cinq Terres vient surtout des sols et du méthane). C'est le premier chantier de calage avec le moteur.
- **Charbon local.** Le charbon ne va qu'aux cellules voisines d'une mine ; pas de réseau de rail ni de marché mondial des biens.
- **Science sans contenu.** Une précision par domaine, pas de théories nommées.
- **Domestication sans génétique.** La qualité d'une plante cultivée monte avec le temps ; le moteur la reliera à la vraie sélection des populations.
