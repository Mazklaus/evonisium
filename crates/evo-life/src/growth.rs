//! Taux de croissance et coefficient de sélection (document Organismes,
//! section « Sorties calculées pour les autres chantiers ») :
//!
//! r(g, c) = (E_assimilée − E_entretien − E_activité) / E_descendant
//!           − (m_fond + m_prédation + m_maladie + m_stress)
//!
//! s = (r_mutant − r_résident) × T_génération
//!
//! Les rapports sont exprimés par mole de carbone de biomasse : E_descendant
//! est le coût de fabrication d'une mole de carbone de biomasse. Il n'y a
//! encore ni prédation, ni maladie, ni activité motrice ; le stress compte
//! l'oxygène et, depuis l'étape 2, les ultraviolets.

use crate::metabolism::{EnergySource, AEROBIC_RESPIRATION, FERMENTATION, REACTIONS, REACTION_COUNT};
use crate::phenotype::{Capacities, Phenotype};
use crate::spectrum::LightSpectrum;
use evo_planet::{WaterChemistry, WaterPool};

/// Constantes physiologiques des unicellulaires. Ce sont des ordres de
/// grandeur de microbiologie, à calibrer ; aucune n'est propre à la Terre.
/// Seul le spectre de lumière disponible dépend de la planète (étoile et
/// couche d'eau) : le monde le remplace à sa création.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Physiology {
    /// Débit maximal d'une enzyme d'efficacité 1, mol de substrat par mole
    /// de carbone de biomasse et par an.
    pub max_uptake: f64,
    /// Part de la lumière absorbée convertie en énergie chimique par une
    /// photosynthèse aux pièces parfaites (qualités toutes à 1).
    pub photo_efficiency: f64,
    /// Même chose pour la phototrophie simple (pigment couplé à la chaîne de
    /// transport d'électrons, sans fixation de carbone) et pour la rhodopsine.
    pub cyclic_efficiency: f64,
    pub rhodopsin_efficiency: f64,
    /// Gain de rendement des voies chimiques apporté par une chaîne de
    /// transport d'électrons parfaite (cytochromes).
    pub electron_transport_gain: f64,
    /// Coût de fabrication d'une mole de carbone de biomasse, kJ, à partir du
    /// CO₂ (autotrophes) ou de matière organique (hétérotrophes).
    pub autotroph_biomass_kj: f64,
    pub heterotroph_biomass_kj: f64,
    /// Entretien, kJ·molC⁻¹·an⁻¹ : base, réplication par gène présent
    /// (pseudogènes compris), expression par gène fonctionnel, et coût des
    /// protéines d'une voie, qui croît avec le carré de son activité totale et
    /// de son affinité. L'entretien du génome, divisé par le volume de la
    /// cellule, est ce qui borne sa longueur : à 200 kJ, des centaines de
    /// régulateurs sans effet s'accumulaient (jusqu'à 1 300 gènes) ; à 2 000,
    /// la moyenne reste vers 20 à 40 gènes et les plus longs vers 300, et les
    /// grandes cellules portent plus facilement un grand génome (Lane et
    /// Martin, 2010).
    pub base_maintenance_kj: f64,
    pub genome_cost_kj: f64,
    pub expression_cost_kj: f64,
    pub gene_cost_kj: f64,
    /// Largeur thermique de référence, K.
    pub thermal_reference_width_k: f64,
    /// Mortalité de fond, an⁻¹.
    pub background_mortality: f64,
    /// Mortalité maximale par stress oxydant sans défense, an⁻¹, et
    /// concentration de demi-effet de l'O₂, mol·m⁻³.
    pub oxygen_stress_mortality: f64,
    pub oxygen_stress_half: f64,
    /// Concentration d'O₂ qui divise par deux l'activité des voies anaérobies.
    pub oxygen_inhibition_half: f64,
    /// Mortalité due aux ultraviolets, an⁻¹ par W·m⁻² d'UV en surface, et part
    /// maximale des UV qu'un pigment à pleine quantité arrête.
    pub uv_mortality: f64,
    pub pigment_uv_shield: f64,
    /// Rapport carbone sur phosphore de la biomasse (Redfield) et demi-
    /// saturation de la croissance par le phosphate, mol·m⁻³.
    pub carbon_to_phosphorus: f64,
    pub phosphate_half: f64,
    /// Part de la nécromasse d'une bactérie isolée qui forme des particules
    /// et coule hors de la couche de surface (pompe biologique : 10 à 20 % de
    /// la production dans les océans actuels).
    pub sinking_share: f64,
    /// Part qui coule au plus, pour les grands corps (rapport d'export de 0,1
    /// à 0,5 selon la taille du plancton, Laws et coll., 2000). Voir
    /// [`Physiology::sinking_share_of`].
    pub max_sinking_share: f64,
    /// Résistance à la dessiccation à partir de laquelle un organisme vit
    /// hors de l'eau (étape 5). [Simplification] Deux habitats tranchés,
    /// l'eau et la terre ferme, sans amphibies.
    pub terrestrial_tolerance: f64,
    /// Mortalité par dessiccation hors de l'eau d'un organisme sans
    /// protection, sur un sol sec, an⁻¹ ; elle baisse avec la résistance et
    /// avec l'humidité du sol.
    pub desiccation_mortality: f64,
    /// Baisse de la croissance dans l'eau due à l'enveloppe protectrice, à
    /// résistance 1 (elle freine les échanges avec l'eau).
    pub cuticle_water_penalty: f64,
    /// Part de la litière de la terre ferme enfouie (sédiments des deltas,
    /// tourbes), le reste étant décomposé dans le sol.
    pub land_burial_share: f64,
    /// Photosynthèse des terres selon le CO₂ de l'air (Michaelis-Menten) :
    /// point de compensation et demi-saturation au-delà, Pa.
    pub land_co2_compensation_pa: f64,
    pub land_co2_half_pa: f64,
    /// Taux de croissance maximal, an⁻¹ (doublement en une heure environ).
    pub max_growth: f64,
    /// Baisse relative du taux de croissance maximal par gène : répliquer un
    /// génome plus long prend plus de temps. Sans elle, une cellule gorgée
    /// d'énergie (phototrophe en pleine lumière) croît au plafond quel que
    /// soit son génome, et les copies inutiles s'accumulent sans frein.
    pub replication_cost_per_gene: f64,
    /// Carbone d'une cellule, mol (environ 10⁻¹³ g de carbone).
    pub carbon_per_cell: f64,
    /// Lumière disponible selon la longueur d'onde (étoile, couche d'eau).
    pub spectrum: LightSpectrum,
    /// Cellule complexe (étape 4). Taille gagnée par unité de cytosquelette
    /// (taille relative à une bactérie de 1 µm) et taille maximale.
    pub cytoskeleton_size: f64,
    pub max_cell_size: f64,
    /// Entretien du cytosquelette dynamique, kJ·molC⁻¹·an⁻¹ par unité.
    pub cytoskeleton_cost_kj: f64,
    /// Capacité d'une voie d'organite relative à la même voie sur la
    /// membrane d'une bactérie.
    pub organelle_scale: f64,
    /// Rapport de taille prédateur sur proie à partir duquel une proie peut
    /// être englobée, et à partir duquel elle l'est sans difficulté.
    pub engulf_min_ratio: f64,
    pub engulf_full_ratio: f64,
    /// Ingestion maximale d'un phagotrophe, mol de carbone de proie par mole
    /// de carbone et par an (quelques fois sa masse par jour), et
    /// concentration de proies de demi-saturation, molC·m⁻³.
    pub max_ingestion: f64,
    pub prey_half: f64,
    /// Mortalité par prédation au plus, an⁻¹.
    pub max_predation: f64,
    /// Économie relative sur le coût des protéines des voies apportée par un
    /// système de signal complet (expression réglée sur le besoin).
    pub signalling_saving: f64,
    /// Recombinase de méiose à partir de laquelle un eucaryote est sexué.
    pub sex_meiosis_threshold: f64,
    /// Adhésion d'une colonie clonale (les cellules filles restent
    /// attachées) ; en dessous, des agrégats dont la taille vue par un
    /// prédateur croît de `aggregate_size` par unité d'adhésion.
    pub clonal_adhesion: f64,
    pub aggregate_size: f64,
    /// Doublements d'une colonie par unité d'adhésion au-delà du seuil, et
    /// nombre maximal de cellules (étape 4 : colonies, corps simples).
    pub colony_doublings: f64,
    pub max_colony_cells: f64,
    /// Zones de développement d'un corps au plus.
    pub max_body_zones: usize,
    /// Profondeur, en cellules, sur laquelle les substances dissoutes et la
    /// lumière baissent d'un facteur e dans un corps.
    pub diffusion_cells: f64,
    /// Portée du morphogène émis par la surface, en cellules, pour une
    /// portée de récepteur moyenne.
    pub morphogen_reach_cells: f64,
    /// Part de la membrane d'une cellule collée à ses voisines, qui ne
    /// puise plus dans l'eau : filament, feuillet, boule.
    pub contact_filament: f64,
    pub contact_sheet: f64,
    pub contact_sphere: f64,
    /// Animaux et plantes (étape 5). Entretien, kJ·molC⁻¹·an⁻¹ par unité,
    /// des moteurs (cils, muscles), des récepteurs sensoriels, du tissu
    /// nerveux (le plus cher, comme le cerveau) et de la matrice de soutien.
    pub motor_cost_kj: f64,
    /// Entretien de l'enveloppe protectrice par unité, sans plafond : une
    /// enveloppe plus épaisse que le besoin se perd.
    pub cuticle_cost_kj: f64,
    pub sensor_cost_kj: f64,
    pub neural_cost_kj: f64,
    pub matrix_cost_kj: f64,
    /// Cellules d'un corps doté d'une matrice de soutien au plus (au-delà de
    /// `max_colony_cells`, la taille n'est plus bornée que par la viabilité).
    pub max_body_cells: f64,
    /// Longueur que porte un corps mou, dans l'eau (portée) et sur la terre
    /// ferme, m, et facteur gagné par unité de matrice de soutien.
    pub soft_length_water_m: f64,
    pub soft_length_land_m: f64,
    pub support_reach: f64,
    /// Demi-épaisseur qu'un organisme aérobie oxygène par diffusion seule, m,
    /// et facteur gagné par une circulation complète.
    pub oxygen_diffusion_m: f64,
    pub circulation_reach: f64,
    /// Moteur à partir duquel un corps porte des appendices, et matrice
    /// qu'il faut pour les attacher (levier rigide).
    pub limb_motor: f64,
    pub limb_matrix: f64,
    /// Tissu nerveux qui coordonne à moitié un corps de 2¹² cellules (le
    /// besoin croît avec le nombre de doublements).
    pub neural_half: f64,
    /// Vitesse d'un nageur de 1 mm à motricité 1, m·s⁻¹, et exposant de la
    /// longueur (Bejan et Marden, 2006 : environ L^0,5 à L^0,85 selon le
    /// milieu).
    pub speed_m_s: f64,
    pub speed_exponent: f64,
    /// Portée de détection par unité de récepteur, en longueurs de corps.
    pub detection_lengths: f64,
    /// Ouverture de la bouche relative à la taille d'un corps mobile, et part
    /// d'un organisme fixé plus grand qu'elle qu'un brouteur peut prendre.
    pub mouth_share: f64,
    pub grazing_share: f64,
    /// Poids de la défense (carapace, épines, bois) dans la fuite d'une proie.
    pub defense_weight: f64,
    /// Sélection sexuelle : avantage relatif de la parade (organe sonore)
    /// chez un sexué qui la perçoit.
    pub sexual_selection: f64,
}

impl Physiology {
    /// Facteur de la photosynthèse des terres pour `co2_pa` Pa de CO₂ : nul
    /// sous le point de compensation, saturé bien au-delà (sans les
    /// stomates ni la voie en C₄).
    pub fn land_co2_factor(&self, co2_pa: f64) -> f64 {
        let x = (co2_pa - self.land_co2_compensation_pa).max(0.0);
        x / (x + self.land_co2_half_pa)
    }

    /// Part de la nécromasse qui coule hors de la couche de surface pour un
    /// corps de taille linéaire `body_size` (relative à une bactérie). La
    /// vitesse de chute suit la loi de Stokes (carré de la taille) ; une
    /// particule coule si elle quitte la couche avant d'être reminéralisée,
    /// soit v / (v + v₀), plafonné à `max_sinking_share`. v₀ est tel qu'une
    /// bactérie isolée donne `sinking_share`. Les grandes algues et les
    /// colonies exportent donc davantage (Logan et coll., 1995 ; Lenton et
    /// coll., 2014). [Simplification] Ni agrégats ni pelotes fécales, ni
    /// lest minéral.
    pub fn sinking_share_of(&self, body_size: f64) -> f64 {
        let (base, max) = (self.sinking_share, self.max_sinking_share.max(self.sinking_share));
        if base <= 0.0 || base >= max {
            return base;
        }
        let v0 = max / base - 1.0;
        let v = body_size.max(1.0).powi(2);
        max * v / (v + v0)
    }
}

impl Default for Physiology {
    fn default() -> Self {
        Self {
            max_uptake: 2.0e4,
            photo_efficiency: 0.05,
            cyclic_efficiency: 0.01,
            rhodopsin_efficiency: 0.004,
            electron_transport_gain: 0.15,
            autotroph_biomass_kj: 2000.0,
            heterotroph_biomass_kj: 400.0,
            base_maintenance_kj: 2000.0,
            genome_cost_kj: 2000.0,
            expression_cost_kj: 300.0,
            gene_cost_kj: 2000.0,
            thermal_reference_width_k: 10.0,
            background_mortality: 20.0,
            oxygen_stress_mortality: 200.0,
            oxygen_stress_half: 1e-3,
            oxygen_inhibition_half: 1e-4,
            uv_mortality: 0.75,
            pigment_uv_shield: 0.9,
            carbon_to_phosphorus: 106.0,
            phosphate_half: 1e-4,
            sinking_share: 0.15,
            max_sinking_share: 0.5,
            terrestrial_tolerance: 0.5,
            desiccation_mortality: 200.0,
            cuticle_water_penalty: 0.2,
            land_burial_share: 0.003,
            land_co2_compensation_pa: 10.0,
            land_co2_half_pa: 30.0,
            max_growth: 6000.0,
            replication_cost_per_gene: 2e-3,
            carbon_per_cell: 1e-14,
            spectrum: LightSpectrum::default(),
            cytoskeleton_size: 6.0,
            max_cell_size: 30.0,
            cytoskeleton_cost_kj: 10000.0,
            organelle_scale: 1.0,
            engulf_min_ratio: 1.2,
            engulf_full_ratio: 2.5,
            max_ingestion: 2000.0,
            prey_half: 1e-3,
            max_predation: 1000.0,
            signalling_saving: 0.4,
            sex_meiosis_threshold: 0.1,
            clonal_adhesion: 0.5,
            aggregate_size: 2.0,
            colony_doublings: 10.0,
            max_colony_cells: 4096.0,
            max_body_zones: 4,
            diffusion_cells: 1.5,
            morphogen_reach_cells: 1.0,
            contact_filament: 0.3,
            contact_sheet: 0.5,
            contact_sphere: 0.5,
            motor_cost_kj: 6000.0,
            cuticle_cost_kj: 600.0,
            sensor_cost_kj: 2000.0,
            neural_cost_kj: 12000.0,
            matrix_cost_kj: 1500.0,
            max_body_cells: 1.7e13,
            soft_length_water_m: 2e-3,
            soft_length_land_m: 5e-4,
            support_reach: 10.0,
            oxygen_diffusion_m: 1e-3,
            circulation_reach: 300.0,
            limb_motor: 0.5,
            limb_matrix: 0.2,
            neural_half: 0.3,
            speed_m_s: 0.03,
            speed_exponent: 0.85,
            detection_lengths: 20.0,
            mouth_share: 0.5,
            grazing_share: 0.2,
            defense_weight: 3.0,
            sexual_selection: 0.05,
        }
    }
}

/// Nombre maximal de proies ou de prédateurs décrits dans une cellule (le
/// plafond de populations par cellule).
pub const MAX_TROPHIC: usize = 8;

/// Une proie ou un prédateur d'une cellule. Proie : taille vue par un
/// prédateur, carbone (mol·m⁻³), fuite, et vrai si elle est fixée et peut
/// être broutée. Prédateur : ouverture de la bouche, mortalité (an⁻¹) d'une
/// proie qu'il prend sans difficulté, attaque, et vrai s'il broute.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TrophicItem {
    pub size: f64,
    pub value: f64,
    pub guard: f64,
    pub flag: bool,
}

/// Liste courte des proies ou des prédateurs d'une cellule.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Trophic {
    pub len: u8,
    pub items: [TrophicItem; MAX_TROPHIC],
}

impl Trophic {
    pub fn push(&mut self, item: TrophicItem) {
        if (self.len as usize) < MAX_TROPHIC && item.value > 0.0 {
            self.items[self.len as usize] = item;
            self.len += 1;
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &TrophicItem> {
        self.items[..self.len as usize].iter()
    }
}

/// Part d'une proie de taille `prey` qu'un prédateur de taille `predator`
/// parvient à englober : nulle sous le rapport minimal, entière au-delà du
/// rapport de pleine capacité.
#[inline]
pub fn edibility(predator: f64, prey: f64, physio: &Physiology) -> f64 {
    let ratio = predator / prey.max(1e-9);
    ((ratio - physio.engulf_min_ratio) / (physio.engulf_full_ratio - physio.engulf_min_ratio)).clamp(0.0, 1.0)
}

/// Part d'une proie qu'un prédateur prend (étape 5) : il faut qu'elle
/// tienne dans sa bouche (ou qu'elle soit fixée et qu'il la broute), puis
/// la trouver et l'attraper. La capture vaut (1 + A) / (1 + A + E), où A est
/// l'attaque du prédateur (motricité, sens) et E la fuite de la proie
/// (motricité, sens, défense) : 1 entre microbes sans moteur ni sens.
#[inline]
pub fn capture(predator: &TrophicItem, prey: &TrophicItem, physio: &Physiology) -> f64 {
    let mut e = edibility(predator.size, prey.size, physio);
    if predator.flag && prey.flag {
        e = e.max(physio.grazing_share);
    }
    if e <= 0.0 {
        return 0.0;
    }
    e * (1.0 + predator.guard) / (1.0 + predator.guard + prey.guard)
}

impl Phenotype {
    /// Ce phénotype vu comme proie (carbone `carbon`).
    pub fn as_prey(&self, carbon: f64, physio: &Physiology) -> TrophicItem {
        let fixed = self.is_multicellular() && self.animal.motility <= 0.0;
        TrophicItem { size: self.body_size, value: carbon, guard: self.animal.escape(physio), flag: fixed }
    }

    /// Ce phénotype vu comme prédateur (pression `pressure`).
    pub fn as_predator(&self, pressure: f64) -> TrophicItem {
        let grazer = self.animal.gape > self.cell_size;
        TrophicItem { size: self.animal.gape.max(self.cell_size), value: pressure, guard: self.animal.attack(), flag: grazer }
    }
}

/// Conditions physiques d'une cellule vues par un organisme.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Conditions {
    pub temperature_k: f64,
    /// Ultraviolets en surface, W·m⁻².
    pub uv_w_m2: f64,
    /// Lumière absorbée disponible par mole de carbone de biomasse
    /// phototrophe, kJ·molC⁻¹·an⁻¹.
    pub light_kj: f64,
    /// Proies (taille, carbone dans l'eau) et prédateurs (taille, pression).
    pub prey: Trophic,
    pub predators: Trophic,
    /// Terre ferme de la cellule (étape 5) : lumière disponible par mole de
    /// carbone phototrophe terrestre, proies et prédateurs terrestres,
    /// humidité du sol (de 0 à 1), et existence d'une terre ferme.
    pub land_light_kj: f64,
    pub land_prey: Trophic,
    pub land_predators: Trophic,
    pub moisture: f64,
    pub has_land: bool,
    /// Phosphate de la solution du sol, mol·m⁻³ : celui que puisent les
    /// organismes de la terre ferme à la place de celui de l'eau.
    pub soil_po4: f64,
    /// Pression partielle du CO₂ de l'air, Pa.
    pub co2_pa: f64,
    /// Vrai une fois les conditions tournées vers la terre ferme
    /// ([`Conditions::for_habitat`]).
    pub on_land: bool,
}

impl Conditions {
    /// Conditions sans proie ni prédateur.
    pub fn new(temperature_k: f64, uv_w_m2: f64, light_kj: f64) -> Self {
        Self {
            temperature_k,
            uv_w_m2,
            light_kj,
            prey: Trophic::default(),
            predators: Trophic::default(),
            land_light_kj: 0.0,
            land_prey: Trophic::default(),
            land_predators: Trophic::default(),
            moisture: 0.0,
            has_land: false,
            soil_po4: 0.0,
            co2_pa: 40.0,
            on_land: false,
        }
    }

    /// Conditions vues par un organisme terrestre (`terrestrial`) ou
    /// aquatique : la lumière, les proies et les prédateurs de son habitat.
    pub fn for_habitat(&self, terrestrial: bool) -> Self {
        if !terrestrial {
            return *self;
        }
        Self { light_kj: self.land_light_kj, prey: self.land_prey, predators: self.land_predators, on_land: true, ..*self }
    }

    /// Carbone des proies que le prédateur `p` peut prendre, mol·m⁻³.
    pub fn edible_prey(&self, p: &Phenotype, physio: &Physiology) -> f64 {
        let me = p.as_predator(1.0);
        self.prey.iter().map(|q| q.value * capture(&me, q, physio)).sum()
    }

    /// Mortalité par prédation de la proie `p`, an⁻¹, bornée (la pression
    /// vient de la dernière évaluation des prédateurs : quand leurs proies
    /// s'effondrent, elle peut devenir démesurée).
    pub fn predation(&self, p: &Phenotype, physio: &Physiology) -> f64 {
        let me = p.as_prey(1.0, physio);
        let m: f64 = self.predators.iter().map(|q| q.value * capture(q, &me, physio)).sum();
        m.min(physio.max_predation)
    }
}

/// Résultat de l'évaluation d'un phénotype dans un milieu.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GrowthRates {
    /// Voies chimiques : mol de substrat limitant par mole de carbone de
    /// biomasse et par an. Voies lumineuses : énergie fournie,
    /// kJ·molC⁻¹·an⁻¹ (leur stoichiométrie suit le carbone qu'elles fixent).
    pub reaction: [f64; REACTION_COUNT],
    /// Énergie fournie par chaque voie, kJ·molC⁻¹·an⁻¹.
    pub reaction_energy: [f64; REACTION_COUNT],
    /// Énergie assimilée, kJ·molC⁻¹·an⁻¹, toutes sources.
    pub energy_kj: f64,
    /// Énergie des voies qui fixent le CO₂ (chimio- et photoautotrophes).
    pub autotroph_energy_kj: f64,
    /// Énergie d'appoint sans carbone (phototrophie simple, rhodopsine).
    pub supplement_kj: f64,
    /// Part de l'énergie à carbone tirée de voies hétérotrophes (matière
    /// organique dissoute et proies).
    pub heterotroph_share: f64,
    /// Parts du carbone de croissance venues de la matière organique
    /// dissoute et des proies englouties.
    pub doc_share: f64,
    pub prey_share: f64,
    /// Proies digérées pour l'énergie, mol de carbone par mole de carbone et
    /// par an, et part digérée par respiration aérobie (le reste fermente).
    pub prey_uptake: f64,
    pub prey_aerobic_share: f64,
    /// Énergie tirée des proies, kJ·molC⁻¹·an⁻¹.
    pub prey_energy_kj: f64,
    /// Coût de fabrication d'une mole de carbone de biomasse, kJ.
    pub biomass_cost_kj: f64,
    /// Taux de naissance permis par le surplus d'énergie, an⁻¹.
    pub birth: f64,
    /// Mortalité totale, an⁻¹, dont la part due aux ultraviolets et celle
    /// due aux prédateurs.
    pub mortality: f64,
    pub uv_mortality: f64,
    pub predation: f64,
    /// Taux de croissance net r, an⁻¹.
    pub r: f64,
}

impl GrowthRates {
    /// Durée d'une génération, an.
    pub fn generation_time(&self, physio: &Physiology) -> f64 {
        // Plancher : la mortalité (celle de fond d'une cellule au plus), qui
        // baisse avec l'horloge métabolique d'un grand corps.
        let floor = if self.mortality > 0.0 { self.mortality.min(physio.background_mortality) } else { physio.background_mortality };
        std::f64::consts::LN_2 / self.birth.max(floor)
    }

    /// Part de la croissance dont le carbone est fixé par la voie autotrophe
    /// `reaction` (au prorata de l'énergie qu'elle fournit).
    pub fn fixation_share(&self, reaction: usize) -> f64 {
        if self.autotroph_energy_kj > 0.0 {
            (1.0 - self.heterotroph_share) * self.reaction_energy[reaction] / self.autotroph_energy_kj
        } else {
            0.0
        }
    }

    /// Carbone de proie consommé, mol par mole de carbone et par an
    /// (digestion et croissance).
    pub fn prey_demand(&self) -> f64 {
        self.prey_uptake + self.birth * self.prey_share
    }
}

#[inline]
fn monod(c: f64, k: f64) -> f64 {
    let c = c.max(0.0);
    c / (k + c)
}

/// Horloge métabolique d'un corps : les flux par unité de masse (entretien,
/// ingestion, mortalité de fond) baissent comme la masse à la puissance
/// −1/4 (Kleiber ; Brown et coll., 2004). Comptée sur le nombre de cellules,
/// pour qu'une cellule seule garde les taux microbiens.
#[inline]
fn metabolic_clock(p: &Phenotype) -> f64 {
    if p.body.is_some() {
        1.0 / p.cells().max(1.0).sqrt().sqrt()
    } else {
        1.0
    }
}

/// Recherche des proies : demi-saturation de l'ingestion (mol·m⁻³). Un
/// prédateur qui nage et perçoit explore un volume π (portée)² × vitesse par
/// unité de temps (réponse fonctionnelle de Holling) ; quand ce volume
/// rapporté à son carbone dépasse celui d'une cellule qui filtre, la
/// demi-saturation baisse d'autant.
#[inline]
fn prey_half_saturation(p: &Phenotype, clock: f64, physio: &Physiology) -> f64 {
    let a = &p.animal;
    let reach = a.detection_m + 0.5 * a.length_m;
    let carbon = physio.carbon_per_cell * p.cell_volume() * p.cells();
    if a.speed_m_s <= 0.0 || carbon <= 0.0 {
        return physio.prey_half;
    }
    let clearance = std::f64::consts::PI * reach * reach * a.speed_m_s * evo_core::units::SECONDS_PER_YEAR / carbon;
    physio.prey_half.min(physio.max_ingestion * clock / clearance)
}

/// Bilan d'énergie d'une cellule (ou d'une zone d'un corps), avant les
/// naissances et les morts.
#[derive(Clone, Copy, Debug, Default)]
struct Budget {
    reaction: [f64; REACTION_COUNT],
    reaction_energy: [f64; REACTION_COUNT],
    het_doc: f64,
    het_prey: f64,
    auto: f64,
    supplement: f64,
    prey_uptake: f64,
    prey_aerobic: f64,
    maintenance: f64,
    oxygen_stress: f64,
    uv: f64,
}

impl Budget {
    fn add_scaled(&mut self, o: &Budget, w: f64) {
        for r in 0..REACTION_COUNT {
            self.reaction[r] += w * o.reaction[r];
            self.reaction_energy[r] += w * o.reaction_energy[r];
        }
        self.het_doc += w * o.het_doc;
        self.het_prey += w * o.het_prey;
        self.auto += w * o.auto;
        self.supplement += w * o.supplement;
        self.prey_uptake += w * o.prey_uptake;
        self.prey_aerobic += w * o.prey_aerobic;
        self.maintenance += w * o.maintenance;
        self.oxygen_stress += w * o.oxygen_stress;
        self.uv += w * o.uv;
    }
}

/// Évalue r(g, c) pour un phénotype dans une cellule.
pub fn growth_rates(p: &Phenotype, cond: &Conditions, chem: &WaterChemistry, physio: &Physiology) -> GrowthRates {
    if p.body.is_some() {
        return body_rates(p, cond, chem, physio);
    }
    growth_rates_with(p, &p.capacities(cond.temperature_k, physio), cond, chem, physio)
}

/// [`growth_rates`] avec les capacités déjà calculées à la température de
/// `cond` (pour un corps, elles sont recalculées par type cellulaire).
pub fn growth_rates_with(p: &Phenotype, caps: &Capacities, cond: &Conditions, chem: &WaterChemistry, physio: &Physiology) -> GrowthRates {
    if p.body.is_some() {
        return body_rates(p, cond, chem, physio);
    }
    let cond = &cond.for_habitat(p.is_terrestrial());
    let prey = cond.edible_prey(p, physio);
    let forage = Forage { ingestion: physio.max_ingestion, half: prey_half_saturation(p, 1.0, physio) };
    let b = budget(p, caps, cond.light_kj, cond.uv_w_m2, prey, forage, chem, physio);
    finish(p, &b, cond, chem, physio)
}

/// Corps d'un multicellulaire : chaque zone a son type cellulaire et ses
/// conditions (les substances dissoutes et la lumière baissent avec la
/// profondeur, les ultraviolets n'atteignent que la surface, les proies ne
/// sont prises que par la surface, sauf circulation) ; les bilans des zones s'ajoutent,
/// pondérés par leur part des cellules.
fn body_rates(p: &Phenotype, cond: &Conditions, chem: &WaterChemistry, physio: &Physiology) -> GrowthRates {
    let body = p.body.as_ref().expect("corps");
    let cond = &cond.for_habitat(p.is_terrestrial());
    let mut total = Budget::default();
    let prey = cond.edible_prey(p, physio);
    let clock = metabolic_clock(p);
    let forage = Forage { ingestion: physio.max_ingestion * clock, half: prey_half_saturation(p, clock, physio) };
    let caps: Vec<Capacities> = body.types.iter().map(|t| t.capacities(cond.temperature_k, physio)).collect();
    for (k, z) in body.zones.iter().enumerate() {
        let t = &body.types[z.cell_type as usize];
        let zone_chem: WaterChemistry = std::array::from_fn(|i| chem[i] * z.access);
        let surface = k == 0;
        let b = budget(
            t,
            &caps[z.cell_type as usize],
            cond.light_kj * z.light,
            if surface { cond.uv_w_m2 } else { 0.0 },
            // Un tube digestif et une circulation portent la nourriture à
            // tout le corps.
            if surface { prey } else { prey * p.animal.circulation },
            forage,
            &zone_chem,
            physio,
        );
        total.add_scaled(&b, z.weight);
    }
    finish(p, &total, cond, chem, physio)
}

/// Ingestion maximale (molC·molC⁻¹·an⁻¹) et demi-saturation (mol·m⁻³) d'un
/// mangeur de proies.
#[derive(Clone, Copy, Debug)]
struct Forage {
    ingestion: f64,
    half: f64,
}

/// Bilan d'énergie d'une cellule exposée à `light_kj`, `uv` et à des proies
/// englobables de concentration `prey` (molC·m⁻³).
#[allow(clippy::too_many_arguments)]
fn budget(
    p: &Phenotype,
    caps: &Capacities,
    light_kj: f64,
    uv: f64,
    prey: f64,
    forage: Forage,
    chem: &WaterChemistry,
    physio: &Physiology,
) -> Budget {
    let o2 = chem[WaterPool::O2 as usize].max(0.0);
    let anaerobic_factor = physio.oxygen_inhibition_half / (physio.oxygen_inhibition_half + o2);
    let chemical_gain = 1.0 + physio.electron_transport_gain * p.electron_transport;
    let mut out = Budget::default();

    // Second centre réactionnel (photosystème I) pour la voie oxygénique.
    let partner = caps.partner;

    for reaction in REACTIONS.iter() {
        if p.signature & (1 << reaction.id) == 0 {
            continue;
        }
        let (cap, affinity) = (caps.cap[reaction.id as usize], caps.affinity[reaction.id as usize]);
        if cap <= 0.0 {
            continue;
        }
        let mut limitation = match reaction.cosubstrate {
            Some((pool, k)) => monod(chem[pool as usize], k),
            None => 1.0,
        };
        if reaction.oxygen_sensitive {
            limitation *= anaerobic_factor;
        }
        let (value, e) = match reaction.energy {
            EnergySource::Chemical { substrate, half_saturation, dg_kj } => {
                let k = half_saturation / affinity.max(1e-6);
                let q = physio.max_uptake * cap * monod(chem[substrate as usize], k) * limitation;
                (q, q * dg_kj * chemical_gain)
            }
            EnergySource::Light { partner_rc, water_oxidation } => {
                // Rendement = R_max × produit des qualités des pièces × disponibilité du donneur.
                let mut quality = p.light_capture * cap.min(1.0);
                if partner_rc {
                    quality *= partner;
                }
                if water_oxidation {
                    quality *= p.water_oxidation;
                }
                let e = light_kj * physio.photo_efficiency * quality * limitation;
                (e, e)
            }
        };
        out.reaction[reaction.id as usize] = value;
        out.reaction_energy[reaction.id as usize] = e;
        if reaction.heterotrophic {
            out.het_doc += e;
        } else {
            out.auto += e;
        }
    }
    // Phagotrophie : les proies englouties sont digérées par fermentation
    // (dans le cytoplasme) ou par respiration aérobie (sur la membrane de
    // l'hôte ou dans un organite), selon ce qui rend le plus.
    if p.engulfment > 0.0 && prey > 0.0 {
        let ingest = forage.ingestion * p.engulfment * monod(prey, forage.half);
        let ferm = &REACTIONS[FERMENTATION as usize];
        let resp = &REACTIONS[AEROBIC_RESPIRATION as usize];
        let dg = |r: &crate::metabolism::Reaction| match r.energy {
            EnergySource::Chemical { dg_kj, .. } => dg_kj,
            EnergySource::Light { .. } => 0.0,
        };
        let q_f = ingest.min(physio.max_uptake * caps.digest_fermentation);
        let e_f = q_f * dg(ferm);
        let q_r = ingest.min(physio.max_uptake * caps.digest_respiration) * monod(o2, resp.cosubstrate.map_or(1e-3, |c| c.1));
        let e_r = q_r * dg(resp) * chemical_gain;
        if e_r > e_f {
            out.prey_uptake = q_r;
            out.prey_aerobic = q_r;
            out.het_prey = e_r;
        } else {
            out.prey_uptake = q_f;
            out.het_prey = e_f;
        }
    }
    let mut supplement = 0.0;
    if p.cyclic_phototrophy {
        supplement += light_kj * physio.cyclic_efficiency * p.light_capture * p.electron_transport;
    }
    supplement += light_kj * physio.rhodopsin_efficiency * p.rhodopsin;
    out.supplement = supplement;
    out.maintenance = p.maintenance_kj;
    out.oxygen_stress = physio.oxygen_stress_mortality * o2 / (o2 + physio.oxygen_stress_half) * (1.0 - p.oxygen_defense);
    out.uv = physio.uv_mortality * uv.max(0.0) * (1.0 - (physio.pigment_uv_shield * p.pigment).min(0.95));
    out
}

/// Naissances, morts et r à partir du bilan d'énergie de l'organisme.
fn finish(p: &Phenotype, b: &Budget, cond: &Conditions, chem: &WaterChemistry, physio: &Physiology) -> GrowthRates {
    // Une forme dormante (organisme terrestre sans terre ferme) ne prélève
    // rien, ne produit rien et ne mange pas.
    if cond.on_land && !cond.has_land {
        return GrowthRates { r: -physio.desiccation_mortality, ..Default::default() };
    }
    let mut out = GrowthRates { reaction: b.reaction, reaction_energy: b.reaction_energy, ..Default::default() };
    let het = b.het_doc + b.het_prey;
    let carbon_energy = het + b.auto;
    let het_share = if carbon_energy > 0.0 { het / carbon_energy } else { 0.0 };
    let cost = het_share * physio.heterotroph_biomass_kj + (1.0 - het_share) * physio.autotroph_biomass_kj;
    // L'énergie d'appoint (phototrophie simple, rhodopsine) ne fournit pas
    // d'électrons : elle paie l'entretien, pas la fabrication de biomasse à
    // partir du CO₂. Seules les voies qui apportent du carbone font croître.
    let clock = metabolic_clock(p);
    let surplus = carbon_energy - (clock * b.maintenance - b.supplement).max(0.0);
    // Sans voie qui apporte du carbone, l'énergie d'appoint ne fait que
    // réduire la famine.
    let birth = if carbon_energy > 0.0 {
        let mut max_growth = physio.max_growth / (1.0 + physio.replication_cost_per_gene * p.gene_count as f64);
        // Un individu plus massif se reproduit plus lentement (loi en
        // puissance un quart de la masse) ; une bactérie garde le plafond.
        let mass = p.cell_volume() * p.cells();
        if mass > 1.0 {
            max_growth /= mass.sqrt().sqrt();
        }
        let po4 = if cond.on_land { cond.soil_po4 } else { chem[WaterPool::Po4 as usize] };
        (surplus / cost).clamp(0.0, max_growth) * monod(po4, physio.phosphate_half)
    } else {
        0.0
    };
    // Un déficit d'énergie consomme la biomasse : mortalité de famine.
    let starvation = (-surplus).max(0.0) / cost;
    let predation = cond.predation(p, physio);
    // Hors de l'eau : dessiccation, qui baisse avec la résistance et avec
    // l'humidité du sol ; pas de terre ferme, pas de vie terrestre. Dans
    // l'eau, l'enveloppe protectrice freine les échanges.
    let dormant = cond.on_land && !cond.has_land;
    let (birth, desiccation) = if cond.on_land {
        if cond.has_land {
            // La vie terrestre ne croît que mouillée : une part du temps qui
            // suit l'humidité du sol.
            let wet = cond.moisture.clamp(0.0, 1.0);
            let co2 = if p.phototroph { physio.land_co2_factor(cond.co2_pa) } else { 1.0 };
            (birth * wet * co2, physio.desiccation_mortality * (1.0 - p.desiccation_tolerance) * (1.0 - 0.5 * wet))
        } else {
            // Plus de terre ferme libre (glace) : spores et formes de
            // dormance attendent son retour, sans croître ni mourir.
            (0.0, 0.0)
        }
    } else {
        (birth * (1.0 - physio.cuticle_water_penalty * p.desiccation_tolerance), 0.0)
    };
    let mortality = if dormant { 0.0 } else { clock * physio.background_mortality + starvation + b.oxygen_stress + b.uv + predation + desiccation };

    out.energy_kj = carbon_energy + b.supplement;
    out.autotroph_energy_kj = b.auto;
    out.supplement_kj = b.supplement;
    out.heterotroph_share = het_share;
    if carbon_energy > 0.0 {
        out.doc_share = b.het_doc / carbon_energy;
        out.prey_share = b.het_prey / carbon_energy;
    }
    out.prey_uptake = b.prey_uptake;
    out.prey_energy_kj = b.het_prey;
    out.prey_aerobic_share = if b.prey_uptake > 0.0 { b.prey_aerobic / b.prey_uptake } else { 0.0 };
    out.biomass_cost_kj = cost;
    out.birth = birth;
    out.mortality = mortality;
    out.uv_mortality = b.uv;
    out.predation = predation;
    // Une forme dormante tient, mais ne peut ni envahir ni l'emporter : sa
    // valeur sélective est celle d'un organisme terrestre sans terre.
    out.r = if dormant { -physio.desiccation_mortality } else { birth - mortality };
    out
}

/// s = (r_mutant − r_résident) × T_génération du résident.
pub fn selection_coefficient(mutant: &GrowthRates, resident: &GrowthRates, physio: &Physiology) -> f64 {
    (mutant.r - resident.r) * resident.generation_time(physio)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metabolism::*;
    use evo_genetics::{Domain, DomainFamily, Gene, Genome};
    use evo_planet::WATER_POOL_COUNT;

    fn gene(family: DomainFamily, efficiency: f64) -> Gene {
        Gene {
            domain: Domain { family, efficiency, affinity: 1.0, t_opt_k: 300.0, t_width_k: 10.0, absorption_nm: 500.0 },
            functional: true,
        }
    }

    fn genome(genes: &[(DomainFamily, f64)]) -> Genome {
        Genome::new(genes.iter().map(|&(f, e)| gene(f, e)).collect(), [0; 32])
    }

    #[test]
    fn capture_weighs_attack_against_escape_and_grazers_take_fixed_prey() {
        let physio = Physiology::default();
        let item = |size, guard, flag| TrophicItem { size, value: 1.0, guard, flag };
        // Microbes sans moteur ni sens : la seule taille compte.
        assert_eq!(capture(&item(3.0, 0.0, false), &item(1.0, 0.0, false), &physio), 1.0);
        // Une proie qui fuit est moins prise ; un chasseur rapide la rattrape.
        let slow = capture(&item(3.0, 0.0, false), &item(1.0, 2.0, false), &physio);
        let fast = capture(&item(3.0, 2.0, false), &item(1.0, 2.0, false), &physio);
        assert!(slow < 0.5 && fast > slow);
        // Un brouteur prend une part d'un organisme fixé plus grand que lui.
        assert_eq!(capture(&item(3.0, 0.0, true), &item(100.0, 0.0, true), &physio), physio.grazing_share);
        assert_eq!(capture(&item(3.0, 0.0, false), &item(100.0, 0.0, true), &physio), 0.0);
    }

    fn chem(h2: f64, o2: f64) -> WaterChemistry {
        let mut c = [0.0; WATER_POOL_COUNT];
        c[WaterPool::Dic as usize] = 8.0;
        c[WaterPool::H2 as usize] = h2;
        c[WaterPool::O2 as usize] = o2;
        c[WaterPool::Po4 as usize] = 1e-3;
        c[WaterPool::H2s as usize] = 1e-2;
        c[WaterPool::Fe2 as usize] = 1e-2;
        c[WaterPool::Mn2 as usize] = 1e-3;
        c
    }

    fn at(t: f64, uv: f64, light: f64) -> Conditions {
        Conditions::new(t, uv, light)
    }

    #[test]
    fn methanogen_grows_with_hydrogen_and_dies_without() {
        let physio = Physiology::default();
        let p = Phenotype::from_genome(&genome(&[(DomainFamily::Catalytic(METHANOGENESIS), 1.0)]), &physio);
        assert!(growth_rates(&p, &at(300.0, 0.0, 0.0), &chem(8e-4, 0.0), &physio).r > 0.0);
        assert!(growth_rates(&p, &at(300.0, 0.0, 0.0), &chem(0.0, 0.0), &physio).r < 0.0);
        // Trop froid pour ses enzymes.
        assert!(growth_rates(&p, &at(260.0, 0.0, 0.0), &chem(8e-4, 0.0), &physio).r < 0.0);
        // L'oxygène bloque la méthanogenèse et tue l'anaérobie sans défense.
        assert!(growth_rates(&p, &at(300.0, 0.0, 0.0), &chem(8e-4, 0.2), &physio).r < 0.0);
        // Sans phosphate, pas de croissance.
        let mut c = chem(8e-4, 0.0);
        c[WaterPool::Po4 as usize] = 0.0;
        assert_eq!(growth_rates(&p, &at(300.0, 0.0, 0.0), &c, &physio).birth, 0.0);
    }

    #[test]
    fn photosynthesis_needs_a_pigment() {
        let physio = Physiology::default();
        let without = Phenotype::from_genome(&genome(&[(DomainFamily::Catalytic(ANOXYGENIC_PHOTOSYNTHESIS), 1.0)]), &physio);
        let with = Phenotype::from_genome(
            &genome(&[(DomainFamily::Catalytic(ANOXYGENIC_PHOTOSYNTHESIS), 1.0), (DomainFamily::Pigment, 1.0)]),
            &physio,
        );
        assert_eq!(without.signature, 0);
        assert!(with.phototroph);
        // Lumière d'une eau de surface peu peuplée : quelques 10⁷ kJ par mole
        // de carbone phototrophe et par an.
        assert!(growth_rates(&with, &at(300.0, 0.0, 1e7), &chem(0.0, 0.0), &physio).r > 0.0);
    }

    /// Règle 1 du document Organismes : chaque pièce du chemin vers la
    /// photosynthèse apporte un bénéfice seule, et l'étape suivante en ajoute.
    #[test]
    fn every_step_towards_photosynthesis_pays() {
        use DomainFamily::*;
        let physio = Physiology::default();
        let c = chem(8e-4, 0.0);
        let cond = at(300.0, 30.0, 5e4);
        let base = [(Catalytic(METHANOGENESIS), 1.0), (Cytochrome, 0.5)];
        let r = |extra: &[(DomainFamily, f64)]| {
            let mut genes = base.to_vec();
            genes.extend_from_slice(extra);
            let p = Phenotype::from_genome(&genome(&genes), &physio);
            (growth_rates(&p, &cond, &c, &physio).r, photosynthesis_stage(&p))
        };
        let (r0, s0) = r(&[]);
        let (r1, s1) = r(&[(Pigment, 0.2)]);
        assert_eq!((s0, s1), (0, 2), "le pigment couplé au cytochrome donne la phototrophie simple");
        assert!(r1 > r0, "pigment : {r1} contre {r0}");
        let (r2, s2) = r(&[(Pigment, 0.2), (Catalytic(ANOXYGENIC_PHOTOSYNTHESIS), 0.2)]);
        assert_eq!(s2, 3);
        assert!(r2 > r1, "anoxygénique : {r2} contre {r1}");
        let (r3, s3) = r(&[
            (Pigment, 0.4),
            (Catalytic(ANOXYGENIC_PHOTOSYNTHESIS), 0.4),
            (Catalytic(OXYGENIC_PHOTOSYNTHESIS), 0.2),
            (WaterOxidation, 0.2),
        ]);
        assert_eq!(s3, 4);
        assert!(r3.is_finite());
        // Le pigment seul protège des UV même sans chaîne de transport.
        let bare = Phenotype::from_genome(&genome(&[(Catalytic(METHANOGENESIS), 1.0)]), &physio);
        let shielded = Phenotype::from_genome(&genome(&[(Catalytic(METHANOGENESIS), 1.0), (Pigment, 0.2)]), &physio);
        let g0 = growth_rates(&bare, &cond, &c, &physio);
        let g1 = growth_rates(&shielded, &cond, &c, &physio);
        assert!(g1.uv_mortality < g0.uv_mortality);
        assert!(g1.r > g0.r, "protection UV : {} contre {}", g1.r, g0.r);
    }

    #[test]
    fn better_adapted_mutant_has_positive_selection_coefficient() {
        let physio = Physiology::default();
        let resident = Phenotype::from_genome(&genome(&[(DomainFamily::Catalytic(METHANOGENESIS), 1.0)]), &physio);
        let mut g = genome(&[(DomainFamily::Catalytic(METHANOGENESIS), 1.0)]);
        g.genes[0].domain.t_opt_k = 290.0;
        let mutant = Phenotype::from_genome(&g, &physio);
        let c = chem(8e-4, 0.0);
        let r_res = growth_rates(&resident, &at(288.0, 0.0, 0.0), &c, &physio);
        let r_mut = growth_rates(&mutant, &at(288.0, 0.0, 0.0), &c, &physio);
        assert!(selection_coefficient(&r_mut, &r_res, &physio) > 0.0);
        assert!(selection_coefficient(&r_res, &r_mut, &physio) < 0.0);
    }
}
