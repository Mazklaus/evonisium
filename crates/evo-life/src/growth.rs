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
    /// de son affinité.
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
    /// Part de la nécromasse qui forme des particules et coule hors de la
    /// couche de surface (pompe biologique : 10 à 20 % de la production dans
    /// les océans actuels).
    pub sinking_share: f64,
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
            genome_cost_kj: 200.0,
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
        }
    }
}

/// Nombre maximal de proies ou de prédateurs décrits dans une cellule (le
/// plafond de populations par cellule).
pub const MAX_TROPHIC: usize = 8;

/// Liste courte de couples (taille vue par un prédateur, valeur) : les
/// proies d'une cellule (valeur : carbone, mol·m⁻³) ou ses prédateurs
/// (valeur : mortalité, an⁻¹, d'une proie qu'ils avalent sans difficulté).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Trophic {
    pub len: u8,
    pub items: [(f64, f64); MAX_TROPHIC],
}

impl Trophic {
    pub fn push(&mut self, size: f64, value: f64) {
        if (self.len as usize) < MAX_TROPHIC && value > 0.0 {
            self.items[self.len as usize] = (size, value);
            self.len += 1;
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &(f64, f64)> {
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
}

impl Conditions {
    /// Conditions sans proie ni prédateur.
    pub fn new(temperature_k: f64, uv_w_m2: f64, light_kj: f64) -> Self {
        Self { temperature_k, uv_w_m2, light_kj, prey: Trophic::default(), predators: Trophic::default() }
    }

    /// Carbone des proies qu'un prédateur de taille `size` peut englober,
    /// mol·m⁻³.
    pub fn edible_prey(&self, size: f64, physio: &Physiology) -> f64 {
        self.prey.iter().map(|&(s, c)| c * edibility(size, s, physio)).sum()
    }

    /// Mortalité par prédation d'une proie de taille `size`, an⁻¹, bornée
    /// (la pression vient de la dernière évaluation des prédateurs : quand
    /// leurs proies s'effondrent, elle peut devenir démesurée).
    pub fn predation(&self, size: f64, physio: &Physiology) -> f64 {
        let m: f64 = self.predators.iter().map(|&(s, pressure)| pressure * edibility(s, size, physio)).sum();
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
        std::f64::consts::LN_2 / self.birth.max(physio.background_mortality)
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
    let prey = cond.edible_prey(p.cell_size, physio);
    let b = budget(p, caps, cond.light_kj, cond.uv_w_m2, prey, chem, physio);
    finish(p, &b, cond, chem, physio)
}

/// Corps d'un multicellulaire : chaque zone a son type cellulaire et ses
/// conditions (les substances dissoutes et la lumière baissent avec la
/// profondeur, les ultraviolets n'atteignent que la surface, les proies ne
/// sont prises que par la surface) ; les bilans des zones s'ajoutent,
/// pondérés par leur part des cellules.
fn body_rates(p: &Phenotype, cond: &Conditions, chem: &WaterChemistry, physio: &Physiology) -> GrowthRates {
    let body = p.body.as_ref().expect("corps");
    let mut total = Budget::default();
    let prey = cond.edible_prey(p.cell_size, physio);
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
            if surface { prey } else { 0.0 },
            &zone_chem,
            physio,
        );
        total.add_scaled(&b, z.weight);
    }
    finish(p, &total, cond, chem, physio)
}

/// Bilan d'énergie d'une cellule exposée à `light_kj`, `uv` et à des proies
/// englobables de concentration `prey` (molC·m⁻³).
#[allow(clippy::too_many_arguments)]
fn budget(p: &Phenotype, caps: &Capacities, light_kj: f64, uv: f64, prey: f64, chem: &WaterChemistry, physio: &Physiology) -> Budget {
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
        let ingest = physio.max_ingestion * p.engulfment * monod(prey, physio.prey_half);
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
    let mut out = GrowthRates { reaction: b.reaction, reaction_energy: b.reaction_energy, ..Default::default() };
    let het = b.het_doc + b.het_prey;
    let carbon_energy = het + b.auto;
    let het_share = if carbon_energy > 0.0 { het / carbon_energy } else { 0.0 };
    let cost = het_share * physio.heterotroph_biomass_kj + (1.0 - het_share) * physio.autotroph_biomass_kj;
    // L'énergie d'appoint (phototrophie simple, rhodopsine) ne fournit pas
    // d'électrons : elle paie l'entretien, pas la fabrication de biomasse à
    // partir du CO₂. Seules les voies qui apportent du carbone font croître.
    let surplus = carbon_energy - (b.maintenance - b.supplement).max(0.0);
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
        (surplus / cost).clamp(0.0, max_growth) * monod(chem[WaterPool::Po4 as usize], physio.phosphate_half)
    } else {
        0.0
    };
    // Un déficit d'énergie consomme la biomasse : mortalité de famine.
    let starvation = (-surplus).max(0.0) / cost;
    let predation = cond.predation(p.body_size, physio);
    let mortality = physio.background_mortality + starvation + b.oxygen_stress + b.uv + predation;

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
    out.r = birth - mortality;
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
