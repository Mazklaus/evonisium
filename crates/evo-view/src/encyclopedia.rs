//! Encyclopédie du jeu : le tableau des simplifications assumées (règle du
//! doc d'architecture : toute simplification proposée par un chantier y
//! entre, pour que le joueur puisse la consulter). Textes copiés des docs de
//! conception le 13 octobre 2026, en français seulement.

/// Une simplification assumée.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Simplification {
    /// Chantier qui l'a posée.
    pub source: &'static str,
    pub title: &'static str,
    pub reality: &'static str,
    pub lost: &'static str,
    pub kept: &'static str,
}

const fn s(source: &'static str, title: &'static str, reality: &'static str, lost: &'static str, kept: &'static str) -> Simplification {
    Simplification { source, title, reality, lost, kept }
}

pub const SIMPLIFICATIONS: &[Simplification] = &[
    s("Architecture", "Grille d'environ 110 km (40 962 cellules)", "Milieux variant au mètre près", "Micro-habitats (une mare, une grotte)", "Gradients climatiques, barrières géographiques, isolement"),
    s("Architecture", "Saisons en moyennes", "Météo jour par jour", "Événements météo ponctuels", "Pression saisonnière (hibernation, migration)"),
    s("Architecture", "Climat à l'équilibre en avance rapide (pas de 50 000 à 100 000 ans)", "Climat qui varie en continu", "Cycles orbitaux de 23 000 à 100 000 ans, moyennés tant que le joueur accélère ; les cycles glaciaires récents ne sont plus visibles en accélération maximale", "Tendances longues (luminosité, CO₂, oxygène, dérive des continents), glaciations globales, hivers d'impact, refuges et ponts terrestres, grâce aux précautions de la section 2 du document Planète (validé le 7 octobre 2026)"),
    s("Architecture", "Génotypes représentatifs par dème", "Chaque individu compte", "Une partie de la variation rare", "Mutation, recombinaison, dérive, sélection"),
    s("Architecture", "Régime « apparition puis fixation » pour les microbes", "Plusieurs mutations en compétition simultanée", "Interférence clonale, dynamiques fines", "Taux d'adaptation et innovations métaboliques"),
    s("Architecture", "Générations agrégées par pas", "Une génération après l'autre", "Dynamiques très rapides entre deux pas", "Tendance évolutive moyenne"),
    s("Architecture", "Génome abstrait", "ADN de millions à milliards de bases", "Biochimie moléculaire réelle", "Hérédité, mutation, duplication de gènes, régulation"),
    s("Architecture", "Plafond d'environ 20 000 espèces vivantes", "Des millions d'espèces", "Biodiversité fine des insectes et microbes", "Grandes lignées, réseaux trophiques"),
    s("Architecture", "Individus détaillés seulement dans les zones actives et observées", "Comportements partout", "Comportements rares hors de la vue du joueur", "Leur effet moyen via le niveau population"),
    s("Architecture", "Accélérateurs des grandes transitions", "Des transitions rares, sur des milliards d'années", "Une part de la rareté réelle (endosymbiose, multicellularité)", "Forme et date imprévisibles ; leviers actifs seulement en cas de stagnation"),
    s("Organismes et écosystèmes", "Chimie : Sous-ensemble de la liste chimique de Planète, une vingtaine de voies paramétrées", "Des milliers de molécules et d'enzymes", "Innovations biochimiques vraiment inédites", ""),
    s("Organismes et écosystèmes", "Cellule : Liste de composants, taille et forme", "Organisation spatiale interne", "Rien de visible à l'échelle du jeu", ""),
    s("Organismes et écosystèmes", "Endosymbiose : Probabilité réglable, plus élevée que dans la réalité", "Événement unique et très improbable", "Une partie de la rareté réelle", ""),
    s("Organismes et écosystèmes", "Développement : Développement simulé sur quelques centaines de cellules, puis graphe de modules mis en cache", "Milliards de cellules et réseaux géniques complexes", "Finesse anatomique", ""),
    s("Organismes et écosystèmes", "Corps : Modules avec dimensions, masses, matériaux ; biomécanique par formules", "Physique 3D complète", "Comportements physiques subtils", ""),
    s("Organismes et écosystèmes", "Physiologie : Capacité, coût et tolérances par système", "Dynamique interne des organes", "Maladies métaboliques fines", ""),
    s("Organismes et écosystèmes", "Perception : Détection probabiliste selon portée et précision", "Signaux réels (images, sons)", "Camouflages très fins", ""),
    s("Organismes et écosystèmes", "Populations : Populations agrégées par case, individus seulement sous l'œil du joueur", "Chaque individu compte", "Variations locales très fines", ""),
    s("Organismes et écosystèmes", "Microbes : Guildes métaboliques", "Myriades de lignées avec transferts de gènes", "Diversité microbienne détaillée", ""),
    s("Organismes et écosystèmes", "Temps : Pas de temps adaptatifs et probabilités d'innovation réglables", "Milliards d'années", "Le rythme réel exact", ""),
    s("Intelligence et sociétés", "3. Intelligence et sociétés — marques [Simplification] (pas de tableau)", "", "", ""),
    s("Intelligence et sociétés", "On ne simule pas les neurones un par un. Le « cerveau » d'un organisme est un petit vecteur de paramètres et, pour les espèces complexes, un petit réseau de décision. La taille du cerveau sert de budget qui limite le nombre de comportements et la mémoire disponibles.", "", "", ""),
    s("Intelligence et sociétés", "L'« état » perçu par un individu est résumé en quelques variables (faim, danger proche, congénères visibles, objets à portée) plutôt qu'une vraie perception. On perd des comportements très fins, on gagne des millions d'individus simulables.", "", "", ""),
    s("Intelligence et sociétés", "On ne génère pas de vraie grammaire. Une langue est un vecteur de concepts disponibles et une mesure d'expressivité. Option cosmétique : générer des noms et des mots par phonétique procédurale pour donner une identité sonore à chaque peuple.", "", "", ""),
    s("Intelligence et sociétés", "Au-delà de quelques milliers d'individus suivis en détail, le groupe est simulé en agrégé : un effectif, une structure par âge, une cohésion, une hiérarchie moyenne. Seuls les personnages notables (inventeurs et passeurs, puis chefs, prophètes, conquérants et savants à l'étape 7) restent suivis un par un.", "", "", ""),
    s("Intelligence et sociétés", "Génération mixte, décidée le 7 octobre 2026. Les techniques, institutions et activités économiques forment un catalogue fini de familles paramétrées, écrit à la main, parce qu'elles sont liées aux ressources et aux lois de la planète. Les croyances, mythes, rites, arts, noms et langues sont générés librement par combinaison d'éléments (divinités, interdits, récits, motifs, sons), parce qu'une incohérence y gêne peu et que c'est là que chaque peuple gagne son identité. Leurs effets mécaniques restent bornés : une croyance générée agit seulement sur la cohésion, la coopération, les tabous de ressources et la légitimité.", "", "", ""),
    s("Intelligence et sociétés", "Les voies de remplacement sont écrites à la main comme le reste du catalogue de techniques. On ne les invente pas en cours de partie, ce qui demande d'anticiper les grands types de mondes que génère le chantier Planète.", "", "", ""),
    s("Intelligence et sociétés", "Le graphe des techniques est écrit à la main par les concepteurs, avec plusieurs centaines de nœuds, et ses effets sont des chiffres. On ne simule pas la physique d'un four ou d'un moteur. En revanche, l'ordre et le lieu d'apparition ne sont jamais fixés.", "", "", ""),
    s("Intelligence et sociétés", "Les institutions sont des modules prédéfinis (conseil, roi, tribunal, temple, armée) avec des paramètres. Les régimes variés naissent de leurs combinaisons, mais on ne simule pas la négociation politique individu par individu au-delà des personnages notables.", "", "", ""),
    s("Intelligence et sociétés", "Les échanges se font entre localités ou sociétés, pas entre individus, sauf pour les personnages notables. Une dizaine à quelques dizaines de catégories de biens au lieu de milliers de produits.", "", "", ""),
    s("Intelligence et sociétés", "Un modèle scientifique est un niveau de précision par domaine et sous-domaine, pas une théorie formalisée. Les textes et noms des découvertes sont générés pour l'affichage.", "", "", ""),
    s("Intelligence et sociétés", "Les dynamiques séculaires sont portées par une dizaine de variables agrégées par société (population, élites, dette, légitimité, cohésion, surplus, technique). Les grands événements émergent de leurs seuils, sans script historique.", "", "", ""),
    s("Intelligence et sociétés", "Dans la couche sociétés, la géographie humaine suit les cellules de la grille de la planète (environ 110 km de côté) ; chaque cellule porte au plus quelques groupes. La diplomatie et la guerre se calculent entre sociétés voisines par cellules de contact.", "", "", ""),
    s("Sociétés (étape 7)", "Peuples d'essai : Les espèces des bancs ont des lectures cognitives écrites à la main, et la Terre de banc est une carte à 5° dessinée par boîtes.", "", "", ""),
    s("Sociétés (étape 7)", "Constantes d'ordre de grandeur : Taux d'invention, coût des marches, rendements, travail aux champs, létalités : calés pour que les cinq Terres d'essai tombent dans les fourchettes de la porte, pas mesurés.", "", "", "Les dates varient beaucoup d'une graine à l'autre : l'histoire est un tirage."),
    s("Sociétés (étape 7)", "Industrie faible : Les sociétés malthusiennes laissent peu de bras libres : après la vapeur, la part fossile de l'énergie monte lentement.", "", "", ""),
    s("Sociétés (étape 7)", "Charbon local : Le charbon ne va qu'aux cellules voisines d'une mine ; pas de réseau de rail ni de marché mondial des biens.", "", "", ""),
    s("Sociétés (étape 7)", "Science sans contenu : Une précision par domaine, pas de théories nommées.", "", "", ""),
    s("Sociétés (étape 7)", "Domestication sans génétique : La qualité d'une plante cultivée monte avec le temps ; le moteur la reliera à la vraie sélection des populations.", "", "", ""),
    s(
        "Client (étape 7)",
        "Sociétés d'essai : en attendant que le moteur fasse vivre les sociétés, le client pose sur les terres de la partie des sociétés d'essai (milieu tiré des cellules, espèces sauvages de banc réparties selon le climat, gisements tirés au hasard) et déroule leur histoire à part.",
        "",
        "Rien de cette histoire ne remonte vers le monde.",
        "L'affichage des sociétés, des villes et de la comparaison.",
    ),
];
