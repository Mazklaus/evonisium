//! Phénotype dérivé : ce que le génome donne à l'organisme.
//!
//! Calculé une fois par génome puis partagé par toutes les populations qui le
//! portent (« anatomie mise en cache » de l'échelle commune).
//!
//! Chaque pièce des voies lumineuses a une fonction seule (exaptation,
//! document Organismes, règle 1) : le pigment protège des ultraviolets, le
//! cytochrome et les centres réactionnels améliorent le rendement des voies
//! chimiques (chaîne de transport d'électrons), la rhodopsine pompe des
//! protons à la lumière, le complexe à manganèse détruit les espèces réactives
//! de l'oxygène.
//!
//! Depuis l'étape 4, le phénotype porte aussi l'organisation de la cellule et
//! du corps (document Organismes, « La cellule », « Endosymbiose et
//! eucaryotes », « Colonies et multicellularité ») :
//!
//! - le cytosquelette agrandit la cellule. Une grande cellule englobe des
//!   proies plus petites qu'elle, mais ses échanges avec l'eau passent par une
//!   surface qui croît moins vite que son volume : ses voies liées à la
//!   membrane (tout ce qui puise dans l'eau, la photosynthèse de l'hôte, la
//!   respiration de ce qu'elle mange) rendent moins par mole de biomasse ;
//! - un organite (partenaire englouti gardé, transmis aux cellules filles)
//!   apporte ses voies d'énergie à l'intérieur de la cellule, sans cette
//!   limite de surface : c'est ce qui libère la taille et le génome des
//!   eucaryotes ;
//! - l'adhésion fait des agrégats, puis, au-delà d'un seuil, des colonies
//!   clonales dont le développement découpe le corps en zones ;
//! - les régulateurs commandent des blocs de gènes selon le morphogène lu
//!   dans chaque zone : des zones qui expriment des gènes différents sont des
//!   types cellulaires différents.
//!
//! [Simplification] La taille, l'adhésion, le signal et la méiose se lisent
//! sur tous les gènes fonctionnels de l'organisme, sans régulation ; seuls les
//! gènes métaboliques, de lumière et de défense dépendent des régulateurs.

use crate::growth::Physiology;
use crate::metabolism::{EnergySource, ANOXYGENIC_CENTRES, FERMENTATION, REACTIONS, REACTION_COUNT};
use evo_core::math::Det;
use evo_genetics::{ChangedElement, DomainFamily, Gene, Genome, ReactionId, REGULATOR_SENSE_NM};
use std::sync::Arc;

/// Bits d'organisation ajoutés à la signature au-dessus des voies : un
/// eucaryote ou un multicellulaire forme une guilde (et une espèce affichée)
/// distincte des microbes qui ont les mêmes voies.
pub const PHAGOTROPH: u32 = 1 << 16;
pub const EUKARYOTE: u32 = 1 << 17;
pub const PLASTID: u32 = 1 << 18;
pub const MULTICELLULAR: u32 = 1 << 19;
/// Bits des voies métaboliques dans une signature.
pub const PATHWAY_MASK: u32 = (1 << REACTION_COUNT) - 1;

/// Une enzyme exprimée, telle que la lit la physiologie (y compris les
/// centres réactionnels des voies lumineuses).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Enzyme {
    pub reaction: ReactionId,
    pub efficiency: f64,
    pub affinity: f64,
    pub t_opt_k: f64,
    pub t_width_k: f64,
    /// Portée par un organite (à l'intérieur de la cellule, sans limite de
    /// surface).
    pub internal: bool,
}

/// Forme d'une colonie, tirée de la variation cachée des domaines
/// d'adhésion (pic d'absorption, sans effet sur la lumière).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Shape {
    /// Boule de cellules : des couches internes, à l'abri et à l'ombre.
    Sphere,
    /// Filament : toutes les cellules sont en surface.
    Filament,
    /// Lame à deux faces : une face éclairée, une face à l'ombre.
    Sheet,
}

/// Zone de développement d'un corps : une couche de cellules à une
/// profondeur donnée, de la surface (zone 0) vers le centre.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Zone {
    /// Part des cellules du corps.
    pub weight: f64,
    /// Profondeur, en diamètres de cellule.
    pub depth_cells: f64,
    /// Part des substances dissoutes (dont l'O₂) qui arrive à cette
    /// profondeur, et part de la lumière.
    pub access: f64,
    pub light: f64,
    /// Morphogène lu dans la zone.
    pub morphogen: f64,
    /// Type cellulaire de la zone (indice dans [`Body::types`]).
    pub cell_type: u8,
}

/// Corps d'un multicellulaire : le résultat du développement.
#[derive(Clone, Debug, PartialEq)]
pub struct Body {
    /// Nombre de cellules.
    pub cells: f64,
    pub shape: Shape,
    /// Colonie clonale (les cellules filles restent attachées).
    pub clonal: bool,
    pub zones: Vec<Zone>,
    /// Types cellulaires distincts (phénotypes de cellule, sans corps).
    pub types: Vec<Arc<Phenotype>>,
    /// Gènes exprimés par chaque type (masque sur les gènes du génome).
    pub expression: Vec<Vec<bool>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Phenotype {
    pub enzymes: Vec<Enzyme>,
    /// Quantité de pigment (somme des domaines, plafonnée à 1) : protection
    /// contre les ultraviolets.
    pub pigment: f64,
    /// Qualité de capture de la lumière par les pigments sous l'étoile de la
    /// partie (quantité × accord du spectre d'absorption), de 0 à 1.
    pub light_capture: f64,
    /// Pic d'absorption moyen des pigments, nm (couleur affichée).
    pub pigment_nm: Option<f64>,
    /// Qualité de la chaîne de transport d'électrons (cytochromes, et à
    /// moitié les centres réactionnels), de 0 à 1.
    pub electron_transport: f64,
    /// Rhodopsine : quantité × accord du spectre, de 0 à 1.
    pub rhodopsin: f64,
    /// Complexe à manganèse qui oxyde l'eau, de 0 à 1.
    pub water_oxidation: f64,
    /// Défense contre l'oxygène (plafonnée à 1).
    pub oxygen_defense: f64,
    /// Coût d'entretien, kJ par mole de carbone de biomasse et par an.
    pub maintenance_kj: f64,
    /// Nombre de gènes, fonctionnels ou non, organites compris (temps de
    /// réplication).
    pub gene_count: u32,
    /// Voies réellement utilisables (bits) et bits d'organisation : c'est la
    /// guilde métabolique.
    pub signature: u32,
    /// Pigment couplé à la chaîne de transport d'électrons : un peu d'énergie
    /// tirée de la lumière, sans carbone fixé (phototrophie simple).
    pub cyclic_phototrophy: bool,
    /// Capte la lumière d'une façon ou d'une autre.
    pub phototroph: bool,
    /// Taille linéaire de la cellule, relative à une bactérie (1 µm).
    pub cell_size: f64,
    /// Taille vue par un prédateur : cellule, agrégat ou colonie.
    pub body_size: f64,
    /// Capacité d'englober des proies (cytosquelette), de 0 à 1.
    pub engulfment: f64,
    /// Adhésion, signal, recombinase de méiose (sommes des domaines).
    pub adhesion: f64,
    pub signalling: f64,
    pub meiosis: f64,
    /// Organites, dont plastes (organites photosynthétiques).
    pub organelles: u8,
    pub plastids: u8,
    /// Reproduction sexuée (eucaryote doté d'une recombinase de méiose).
    pub sexual: bool,
    /// Gènes exprimés par la cellule (ou, pour un corps, par la surface).
    pub expressed_genes: u32,
    /// Corps d'un multicellulaire ; `None` pour une cellule seule.
    pub body: Option<Arc<Body>>,
}

/// Facteur thermique d'une enzyme : gaussienne autour de l'optimum, dont le
/// pic baisse quand la plage s'élargit (compromis généraliste-spécialiste).
#[inline]
pub fn thermal_factor(t: f64, t_opt: f64, width: f64, physio: &Physiology) -> f64 {
    let peak = 2.0 * physio.thermal_reference_width_k / (physio.thermal_reference_width_k + width);
    let x = (t - t_opt) / width;
    // Au-delà de 6 largeurs, le facteur est sous 10⁻¹⁵ : zéro, sans calcul.
    if x * x > 36.0 {
        return 0.0;
    }
    peak * (-x * x).dexp()
}

/// Capacités des voies d'un phénotype à une température : elles ne
/// dépendent que de la température, constante pendant l'écologie d'un pas,
/// et se calculent donc une fois par population et par pas.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Capacities {
    /// Capacités vues de l'eau : réduites par le rapport surface sur volume
    /// pour les voies de la membrane de l'hôte.
    pub cap: [f64; REACTION_COUNT],
    pub affinity: [f64; REACTION_COUNT],
    /// Meilleur centre réactionnel anoxygénique (second photosystème).
    pub partner: f64,
    /// Digestion des proies englouties : fermentation (dans le cytoplasme,
    /// sans limite de surface) et respiration (membrane de l'hôte ou
    /// organite).
    pub digest_fermentation: f64,
    pub digest_respiration: f64,
}

/// Gènes exprimés selon le morphogène `m` : un gène constitutif l'est
/// toujours ; un gène d'un bloc régulé l'est si son régulateur est actif.
/// Sans système de signal (`m` = `None`), les régulateurs sont actifs.
pub fn expression_mask(genome: &Genome, regulated_by: &[Option<u16>], m: Option<f64>) -> Vec<bool> {
    genome
        .genes
        .iter()
        .zip(regulated_by)
        .map(|(g, reg)| {
            g.functional
                && match (reg, m) {
                    (None, _) | (Some(_), None) => true,
                    (Some(r), Some(m)) => regulator_active(&genome.genes[*r as usize], m),
                }
        })
        .collect()
}

/// Un régulateur activateur (pic sous [`REGULATOR_SENSE_NM`]) est actif
/// au-dessus de son seuil, un régulateur inverse au-dessous.
pub fn regulator_active(regulator: &Gene, m: f64) -> bool {
    let threshold = regulator.domain.affinity;
    if regulator.domain.absorption_nm < REGULATOR_SENSE_NM {
        m >= threshold
    } else {
        m < threshold
    }
}

/// Traits d'organisation lus sur tout le génome (voir la simplification du
/// module).
#[derive(Clone, Copy, Debug, Default)]
struct Organisation {
    cytoskeleton: f64,
    adhesion: f64,
    adhesion_nm: f64,
    signalling: f64,
    signalling_reach: f64,
    meiosis: f64,
}

fn organisation(genome: &Genome) -> Organisation {
    let mut o = Organisation::default();
    for g in genome.functional_genes() {
        let d = g.domain;
        match d.family {
            DomainFamily::Cytoskeleton => o.cytoskeleton += d.efficiency,
            DomainFamily::Adhesion => {
                o.adhesion += d.efficiency;
                o.adhesion_nm += d.efficiency * d.absorption_nm;
            }
            DomainFamily::Signalling => {
                o.signalling += d.efficiency;
                o.signalling_reach += d.efficiency * d.affinity;
            }
            DomainFamily::Meiosis => o.meiosis += d.efficiency,
            _ => {}
        }
    }
    if o.adhesion > 0.0 {
        o.adhesion_nm /= o.adhesion;
    }
    if o.signalling > 0.0 {
        o.signalling_reach /= o.signalling;
    }
    o
}

/// Échelle des sommes en virgule fixe : l'addition d'entiers est exacte et
/// associative, si bien qu'une somme mise à jour gène par gène (construction
/// incrémentale) est identique au bit près à la somme recalculée.
const FIXED: f64 = (1u64 << 40) as f64;

#[inline]
fn fx(x: f64) -> i128 {
    (x * FIXED) as i128
}

#[inline]
fn fl(v: i128) -> f64 {
    v as f64 / FIXED
}

/// Sommes d'une cellule sur ses gènes exprimés (organites compris).
#[derive(Clone, Debug, Default)]
pub struct CellSums {
    reaction_eff: [i128; REACTION_COUNT],
    reaction_aff: [i128; REACTION_COUNT],
    pigment: i128,
    pigment_capture: i128,
    pigment_nm: i128,
    cytochrome: i128,
    rhodopsin: i128,
    rhodopsin_capture: i128,
    wox: i128,
    defense: i128,
    repair: i128,
    /// Structure, adhésion, signal, régulation, méiose (somme des carrés).
    cellular: i128,
    expressed: i64,
}

impl CellSums {
    /// Ajoute (`sign` = 1) ou retire (`sign` = −1) la part d'un gène exprimé.
    fn add(&mut self, gene: &Gene, membrane: f64, physio: &Physiology, sign: i128) {
        let d = gene.domain;
        self.expressed += sign as i64;
        match d.family {
            DomainFamily::Catalytic(r) if (r as usize) < REACTION_COUNT => {
                self.reaction_eff[r as usize] += sign * fx(d.efficiency);
                self.reaction_aff[r as usize] += sign * fx(d.efficiency * d.affinity);
            }
            DomainFamily::Catalytic(_) => {}
            DomainFamily::Pigment => {
                self.pigment += sign * fx(d.efficiency);
                self.pigment_capture += sign * fx(d.efficiency * physio.spectrum.match_at(d.absorption_nm));
                self.pigment_nm += sign * fx(d.efficiency * d.absorption_nm);
            }
            DomainFamily::Cytochrome => self.cytochrome += sign * fx(d.efficiency),
            DomainFamily::Rhodopsin => {
                self.rhodopsin += sign * fx(d.efficiency);
                // Pompe de la membrane de l'hôte : elle rend moins dans une
                // grande cellule.
                self.rhodopsin_capture += sign * fx(d.efficiency * membrane * physio.spectrum.match_at(d.absorption_nm));
            }
            DomainFamily::WaterOxidation => self.wox += sign * fx(d.efficiency),
            DomainFamily::OxidativeDefense => self.defense += sign * fx(d.efficiency),
            // La réparation agit sur le taux de mutation ; ici seul son coût compte.
            DomainFamily::Repair => self.repair += sign * fx(d.efficiency),
            f if f.is_cellular() => self.cellular += sign * fx(d.efficiency * d.efficiency),
            _ => {}
        }
    }

    fn of(genome: &Genome, mask: Option<&[bool]>, cell_size: f64, physio: &Physiology) -> Self {
        let mut sums = Self::default();
        let membrane = 1.0 / cell_size;
        for (gene, _) in expressed(genome, mask) {
            sums.add(gene, membrane, physio, 1);
        }
        sums
    }
}

/// Gènes exprimés d'une cellule, ceux de l'hôte selon `mask` puis ceux de ses
/// organites (vrai : porté par un organite).
fn expressed<'a>(genome: &'a Genome, mask: Option<&'a [bool]>) -> impl Iterator<Item = (&'a Gene, bool)> + 'a {
    let host = genome.genes.iter().enumerate().filter(move |(i, g)| g.functional && mask.is_none_or(|m| m[*i])).map(|(_, g)| (g, false));
    host.chain(genome.organelle_genes().map(|g| (g, true)))
}

/// Ce qui ne dépend que de l'organisation du génome : taille de la cellule,
/// développement (zones, jeux de gènes exprimés) et sommes de chaque type
/// cellulaire. Une mutation ponctuelle d'un gène métabolique, de lumière ou
/// de défense ne change que les sommes des types qui l'expriment : le
/// phénotype du mutant se construit à partir de celui du résident
/// (« construction incrémentale », document d'architecture, étape 4).
#[derive(Clone, Debug)]
pub struct Basis {
    org: Organisation,
    cell_size: f64,
    /// Colonie : forme, nombre de cellules et zones (dont le type de
    /// chacune).
    colony: Option<(Shape, f64, Vec<Zone>)>,
    /// Jeu de gènes exprimés de chaque type (`None` : tous).
    masks: Vec<Option<Vec<bool>>>,
    sums: Vec<CellSums>,
}

impl Basis {
    pub fn new(genome: &Genome, physio: &Physiology) -> Self {
        let org = organisation(genome);
        let cell_size = (1.0 + physio.cytoskeleton_size * org.cytoskeleton).min(physio.max_cell_size);
        let regulated_by = genome.regulated_by();
        let has_regulators = regulated_by.iter().any(Option::is_some);
        let signal = (org.signalling > 0.0).then_some(org.signalling.min(1.0));
        let (colony, masks) = if org.adhesion >= physio.clonal_adhesion {
            let (shape, cells, zones, masks) = develop(genome, &regulated_by, &org, signal, physio);
            (Some((shape, cells, zones)), masks.into_iter().map(Some).collect())
        } else {
            // Cellule seule : le morphogène est son propre signal.
            let mask = has_regulators.then(|| expression_mask(genome, &regulated_by, signal));
            (None, vec![mask])
        };
        let sums = masks.iter().map(|m| CellSums::of(genome, m.as_deref(), cell_size, physio)).collect();
        Self { org, cell_size, colony, masks, sums }
    }

    /// Base du génome `genome`, issu de `resident` (de base `self`) par le
    /// changement `element` ; `None` quand le changement touche
    /// l'organisation ou fusionne deux types cellulaires (il faut alors tout
    /// reconstruire). Couvre les mutations ponctuelles et les pertes de
    /// fonction, les duplications, insertions et délétions d'un gène qui
    /// n'est pas de structure ni de régulation, et les gènes des organites.
    pub fn derive(&self, resident: &Genome, genome: &Genome, element: ChangedElement, physio: &Physiology) -> Option<Self> {
        if genome.organelles.len() != resident.organelles.len() {
            return None;
        }
        let plain = |g: &Gene| !g.domain.family.is_cellular();
        let mut basis = self.clone();
        let membrane = 1.0 / basis.cell_size;
        match element {
            ChangedElement::Gene { index, .. } => {
                let i = index as usize;
                if genome.genes.len() != resident.genes.len() {
                    return None;
                }
                let (old, new) = (&resident.genes[i], &genome.genes[i]);
                if !plain(old) || !plain(new) {
                    return None;
                }
                for (mask, sums) in basis.masks.iter_mut().zip(basis.sums.iter_mut()) {
                    let active = mask.as_deref().is_none_or(|m| active_at(&resident.genes, m, i));
                    if old.functional && active {
                        sums.add(old, membrane, physio, -1);
                    }
                    if new.functional && active {
                        sums.add(new, membrane, physio, 1);
                    }
                    if let Some(m) = mask {
                        m[i] = new.functional && active;
                    }
                }
            }
            ChangedElement::Removed { index, .. } => {
                let i = index as usize;
                if genome.genes.len() + 1 != resident.genes.len() || !plain(&resident.genes[i]) {
                    return None;
                }
                let old = &resident.genes[i];
                for (mask, sums) in basis.masks.iter_mut().zip(basis.sums.iter_mut()) {
                    if old.functional && mask.as_deref().is_none_or(|m| m[i]) {
                        sums.add(old, membrane, physio, -1);
                    }
                    if let Some(m) = mask {
                        m.remove(i);
                    }
                }
            }
            ChangedElement::Inserted { index, .. } => {
                let j = index as usize;
                if genome.genes.len() != resident.genes.len() + 1 || !plain(&genome.genes[j]) {
                    return None;
                }
                // Le gène inséré rejoint le bloc du gène qui le précède.
                let new = &genome.genes[j];
                for (mask, sums) in basis.masks.iter_mut().zip(basis.sums.iter_mut()) {
                    let active = j == 0 || mask.as_deref().is_none_or(|m| active_at(&resident.genes, m, j - 1));
                    if new.functional && active {
                        sums.add(new, membrane, physio, 1);
                    }
                    if let Some(m) = mask {
                        m.insert(j, new.functional && active);
                    }
                }
            }
            ChangedElement::OrganelleGene { organelle, index } => {
                let (o, i) = (organelle as usize, index as usize);
                let (Some(old_o), Some(new_o)) = (resident.organelles.get(o), genome.organelles.get(o)) else { return None };
                if old_o.genes.len() != new_o.genes.len() || i >= old_o.genes.len() {
                    return None;
                }
                let (old, new) = (&old_o.genes[i], &new_o.genes[i]);
                if !plain(old) || !plain(new) {
                    return None;
                }
                for sums in basis.sums.iter_mut() {
                    if old.functional {
                        sums.add(old, membrane, physio, -1);
                    }
                    if new.functional {
                        sums.add(new, membrane, physio, 1);
                    }
                }
            }
            _ => return None,
        }
        // Deux types devenus identiques n'en font plus qu'un : la
        // reconstruction complète les fusionne.
        let masks = &basis.masks;
        if (1..masks.len()).any(|k| masks[..k].contains(&masks[k])) {
            return None;
        }
        Some(basis)
    }
}

/// Activité, dans un type cellulaire de jeu `mask`, du bloc qui contient le
/// gène `i` : celle du dernier régulateur fonctionnel qui le précède (un
/// régulateur se commande lui-même), ou vrai pour un gène constitutif.
fn active_at(genes: &[Gene], mask: &[bool], i: usize) -> bool {
    (0..=i).rev().find(|&k| genes[k].functional && genes[k].domain.family == DomainFamily::Regulator).is_none_or(|k| mask[k])
}

/// Développement d'une colonie clonale : nombre de cellules, forme, zones,
/// morphogène, puis un jeu de gènes exprimés par type cellulaire.
fn develop(
    genome: &Genome,
    regulated_by: &[Option<u16>],
    org: &Organisation,
    signal: Option<f64>,
    physio: &Physiology,
) -> (Shape, f64, Vec<Zone>, Vec<Vec<bool>>) {
    let doublings = 1.0 + physio.colony_doublings * (org.adhesion - physio.clonal_adhesion).min(1.0);
    let cells = doublings.dexp2().min(physio.max_colony_cells);
    let shape = if org.adhesion_nm < 500.0 {
        Shape::Filament
    } else if org.adhesion_nm > 800.0 {
        Shape::Sheet
    } else {
        Shape::Sphere
    };
    // Géométrie : couches de cellules de la surface vers le centre
    // (poids, profondeur).
    let mut layout: Vec<(f64, f64)> = Vec::new();
    match shape {
        Shape::Filament => layout.push((1.0, 0.0)),
        Shape::Sheet => {
            if cells >= 4.0 {
                layout.push((0.5, 0.0));
                layout.push((0.5, 1.0));
            } else {
                layout.push((1.0, 0.0));
            }
        }
        Shape::Sphere => {
            // Rayon en cellules d'une boule de `cells` cellules.
            let radius = 0.62 * cells.dcbrt();
            let zones = (radius.ceil() as usize).clamp(1, physio.max_body_zones);
            let step = radius / zones as f64;
            for k in 0..zones {
                let outer = radius - k as f64 * step;
                let inner = (outer - step).max(0.0);
                let weight = (outer * outer * outer - inner * inner * inner) / (radius * radius * radius);
                layout.push((weight, k as f64 * step));
            }
        }
    }
    // Les cellules collées puisent moins dans l'eau : le coût premier de la
    // vie en colonie.
    let contact = match shape {
        Shape::Filament => physio.contact_filament,
        Shape::Sheet => physio.contact_sheet,
        Shape::Sphere => physio.contact_sphere,
    };
    let reach = physio.morphogen_reach_cells * (0.5 + org.signalling_reach);
    let mut zones: Vec<Zone> = Vec::with_capacity(layout.len());
    let mut masks: Vec<Vec<bool>> = Vec::new();
    for &(weight, depth) in &layout {
        let light = (-depth / physio.diffusion_cells).dexp();
        let access = light * (1.0 - contact);
        // Morphogène émis par la surface, qui décroît vers l'intérieur.
        let m = signal.map(|s| s * (-depth / reach).dexp());
        let mask = expression_mask(genome, regulated_by, m);
        let cell_type = match masks.iter().position(|x| *x == mask) {
            Some(t) => t,
            None => {
                masks.push(mask);
                masks.len() - 1
            }
        };
        zones.push(Zone { weight, depth_cells: depth, access, light, morphogen: m.unwrap_or(0.0), cell_type: cell_type as u8 });
    }
    (shape, cells, zones, masks)
}

impl Phenotype {
    pub fn from_genome(genome: &Genome, physio: &Physiology) -> Self {
        Self::assemble(genome, &Basis::new(genome, physio), physio)
    }

    /// Phénotype d'un génome à partir de sa base.
    pub fn assemble(genome: &Genome, basis: &Basis, physio: &Physiology) -> Self {
        let org = &basis.org;
        let cell_size = basis.cell_size;
        let Some((shape, cells, zones)) = &basis.colony else {
            let mut p = Self::cell(genome, basis.masks[0].as_deref(), &basis.sums[0], org, cell_size, physio);
            // Agrégats : l'adhésion sous le seuil de la colonie colle les
            // cellules en amas lâches, que les prédateurs avalent moins bien.
            p.body_size = cell_size * (1.0 + physio.aggregate_size * org.adhesion);
            return p;
        };
        let types: Vec<Arc<Phenotype>> = basis
            .masks
            .iter()
            .zip(&basis.sums)
            .map(|(m, sums)| Arc::new(Self::cell(genome, m.as_deref(), sums, org, cell_size, physio)))
            .collect();
        // Le phénotype de l'organisme : celui de la surface, avec la
        // signature de tous les types, l'entretien moyen et le corps.
        let mut top = (*types[zones[0].cell_type as usize]).clone();
        let mut signature = MULTICELLULAR;
        let mut maintenance = 0.0;
        for z in zones {
            let t = &types[z.cell_type as usize];
            signature |= t.signature;
            maintenance += z.weight * t.maintenance_kj;
            top.phototroph |= t.phototroph;
        }
        top.signature = signature;
        top.maintenance_kj = maintenance;
        let colony = match shape {
            Shape::Filament => cells.sqrt(),
            _ => cells.dcbrt(),
        };
        top.body_size = cell_size * colony;
        let expression = basis.masks.iter().map(|m| m.clone().unwrap_or_default()).collect();
        top.body = Some(Arc::new(Body { cells: *cells, shape: *shape, clonal: true, zones: zones.clone(), types, expression }));
        top
    }

    /// Phénotype d'une cellule qui exprime les gènes de `mask` (tous les
    /// gènes fonctionnels si `None`) et ceux de ses organites, de sommes
    /// `sums`.
    fn cell(genome: &Genome, mask: Option<&[bool]>, sums: &CellSums, org: &Organisation, cell_size: f64, physio: &Physiology) -> Self {
        let enzymes: Vec<Enzyme> = expressed(genome, mask)
            .filter_map(|(gene, internal)| {
                let d = gene.domain;
                match d.family {
                    DomainFamily::Catalytic(r) if (r as usize) < REACTION_COUNT => Some(Enzyme {
                        reaction: r,
                        efficiency: d.efficiency,
                        affinity: d.affinity,
                        t_opt_k: d.t_opt_k,
                        t_width_k: d.t_width_k,
                        internal,
                    }),
                    _ => None,
                }
            })
            .collect();
        let reaction_eff: [f64; REACTION_COUNT] = std::array::from_fn(|r| fl(sums.reaction_eff[r]));
        let (pigment, pigment_capture, pigment_nm_sum) = (fl(sums.pigment), fl(sums.pigment_capture), fl(sums.pigment_nm));
        let (cytochrome, rhodopsin, rhodopsin_capture) = (fl(sums.cytochrome), fl(sums.rhodopsin), fl(sums.rhodopsin_capture));
        let (wox, defense, repair, cellular) = (fl(sums.wox), fl(sums.defense), fl(sums.repair), fl(sums.cellular));
        let expressed = sums.expressed as u32;
        let volume = cell_size * cell_size * cell_size;
        let gene_count = genome.genes.len() + genome.organelles.iter().map(|o| o.genes.len()).sum::<usize>();
        // Une copie du génome par cellule : son coût par mole de biomasse
        // baisse avec le volume de la cellule (énergie par gène).
        let mut maintenance = physio.base_maintenance_kj + physio.genome_cost_kj * gene_count as f64 / volume;
        maintenance += physio.expression_cost_kj * expressed as f64;
        // Totaux par voie et par famille : le coût est convexe sur le total,
        // pour qu'une duplication ne rende pas une protéine moins chère.
        // Le signal règle l'expression sur le besoin : les protéines des voies
        // coûtent moins cher (document Organismes, « Capteurs moléculaires »).
        let regulation = 1.0 - physio.signalling_saving * org.signalling.min(1.0);
        for (&e, &aff) in reaction_eff.iter().zip(&sums.reaction_aff) {
            if e > 0.0 {
                let a = fl(aff) / e;
                maintenance += regulation * physio.gene_cost_kj * (e * e + 0.25 * a * a);
            }
        }
        maintenance += physio.gene_cost_kj
            * (regulation * (pigment * pigment + wox * wox + rhodopsin * rhodopsin)
                + defense * defense
                + 0.25 * cytochrome * cytochrome
                + 0.25 * repair * repair
                + 0.25 * cellular)
            // Cytosquelette dynamique (polymérisation de l'actine, recyclage
            // des membranes) : un poste d'énergie important des cellules qui
            // changent de forme et englobent.
            + physio.cytoskeleton_cost_kj * org.cytoskeleton.min(1.5);

        let light_capture = if pigment > 0.0 { (pigment.min(1.0) * pigment_capture / pigment).min(1.0) } else { 0.0 };
        let pigment_nm = (pigment > 0.0).then(|| pigment_nm_sum / pigment);
        let centres: f64 = ANOXYGENIC_CENTRES.iter().map(|&r| reaction_eff[r as usize]).sum::<f64>()
            + reaction_eff[crate::metabolism::OXYGENIC_PHOTOSYNTHESIS as usize];
        let electron_transport = (cytochrome + 0.5 * centres).min(1.0);
        let water_oxidation = wox.min(1.0);
        let has_partner = ANOXYGENIC_CENTRES.iter().any(|&r| reaction_eff[r as usize] > 0.0);

        let mut signature = enzymes.iter().fold(0u32, |acc, e| {
            let usable = match REACTIONS[e.reaction as usize].energy {
                EnergySource::Light { partner_rc, water_oxidation: needs_wox } => {
                    light_capture > 0.0 && (!partner_rc || has_partner) && (!needs_wox || water_oxidation > 0.0)
                }
                EnergySource::Chemical { .. } => true,
            };
            if usable {
                acc | (1 << e.reaction)
            } else {
                acc
            }
        });
        let cyclic_phototrophy = light_capture > 0.0 && electron_transport > 0.0;
        let rhodopsin_q = if rhodopsin > 0.0 { (rhodopsin.min(1.0) * rhodopsin_capture / rhodopsin).min(1.0) } else { 0.0 };
        let light_signature = REACTIONS.iter().any(|r| r.is_light() && signature & (1 << r.id) != 0);
        // Englober demande une cellule plus grande que ses proies et une voie
        // pour les digérer ; on n'est classé phagotrophe (guilde à part) qu'à
        // partir de la taille où une bactérie s'avale à moitié.
        let digests = reaction_eff[FERMENTATION as usize] > 0.0 || reaction_eff[crate::metabolism::AEROBIC_RESPIRATION as usize] > 0.0;
        let engulfment = if digests && cell_size > physio.engulf_min_ratio { org.cytoskeleton.min(1.0) } else { 0.0 };
        if engulfment > 0.0 && cell_size >= 0.5 * (physio.engulf_min_ratio + physio.engulf_full_ratio) {
            signature |= PHAGOTROPH;
        }
        let organelles = genome.organelles.len() as u8;
        let plastids = genome
            .organelles
            .iter()
            .filter(|o| o.genes.iter().any(|g| g.functional && matches!(g.domain.family, DomainFamily::Pigment)))
            .count() as u8;
        if organelles > 0 {
            signature |= EUKARYOTE;
        }
        if plastids > 0 {
            signature |= PLASTID;
        }
        let phototroph = light_signature || cyclic_phototrophy || rhodopsin_q > 0.0;
        Self {
            enzymes,
            pigment: pigment.min(1.0),
            light_capture,
            pigment_nm,
            electron_transport,
            rhodopsin: rhodopsin_q,
            water_oxidation,
            oxygen_defense: (defense + 0.5 * water_oxidation).min(1.0),
            maintenance_kj: maintenance,
            gene_count: gene_count as u32,
            signature,
            cyclic_phototrophy,
            phototroph,
            cell_size,
            body_size: cell_size,
            engulfment,
            adhesion: org.adhesion,
            signalling: org.signalling,
            meiosis: org.meiosis,
            organelles,
            plastids,
            sexual: organelles > 0 && org.meiosis >= physio.sex_meiosis_threshold,
            expressed_genes: expressed,
            body: None,
        }
    }

    /// Volume de la cellule relatif à une bactérie.
    pub fn cell_volume(&self) -> f64 {
        self.cell_size * self.cell_size * self.cell_size
    }

    /// Nombre de cellules d'un individu.
    pub fn cells(&self) -> f64 {
        self.body.as_ref().map_or(1.0, |b| b.cells)
    }

    /// Nombre de types cellulaires.
    pub fn cell_types(&self) -> usize {
        self.body.as_ref().map_or(1, |b| b.types.len())
    }

    pub fn is_eukaryote(&self) -> bool {
        self.signature & EUKARYOTE != 0
    }

    pub fn is_multicellular(&self) -> bool {
        self.signature & MULTICELLULAR != 0
    }

    pub fn is_phagotroph(&self) -> bool {
        self.signature & PHAGOTROPH != 0
    }

    /// Capacités et affinités de toutes les voies à la température `t`, en
    /// un passage sur les enzymes (mêmes sommes, dans le même ordre, que
    /// [`Phenotype::capacity`]).
    pub fn capacities(&self, t: f64, physio: &Physiology) -> Capacities {
        let mut cap = [0.0; REACTION_COUNT];
        let mut aff = [0.0; REACTION_COUNT];
        let membrane = 1.0 / self.cell_size;
        let (mut ferment, mut respire) = (0.0, 0.0);
        for e in &self.enzymes {
            let c = e.efficiency * thermal_factor(t, e.t_opt_k, e.t_width_k, physio);
            let light = REACTIONS[e.reaction as usize].is_light();
            // Ce qui puise dans l'eau traverse la membrane de l'hôte ; la
            // lumière d'un plaste est captée à l'intérieur.
            let seen = if light && e.internal { c * physio.organelle_scale } else { c * membrane };
            cap[e.reaction as usize] += seen;
            aff[e.reaction as usize] += seen * e.affinity;
            if e.reaction == FERMENTATION {
                ferment += c;
            } else if e.reaction == crate::metabolism::AEROBIC_RESPIRATION {
                respire += if e.internal { c * physio.organelle_scale } else { c * membrane };
            }
        }
        let mut affinity = [1.0; REACTION_COUNT];
        for r in 0..REACTION_COUNT {
            if cap[r] > 0.0 {
                affinity[r] = aff[r] / cap[r];
            } else {
                cap[r] = 0.0;
            }
        }
        let partner = ANOXYGENIC_CENTRES.iter().map(|&r| cap[r as usize].min(1.0)).fold(0.0, f64::max);
        Capacities { cap, affinity, partner, digest_fermentation: ferment, digest_respiration: respire }
    }

    /// Capacité et affinité moyenne d'une voie à la température `t`.
    pub fn capacity(&self, reaction: ReactionId, t: f64, physio: &Physiology) -> (f64, f64) {
        let c = self.capacities(t, physio);
        (c.cap[reaction as usize], c.affinity[reaction as usize])
    }

    /// Voie principale : la voie utilisable portée par le plus d'efficacité
    /// enzymatique (à égalité, la première). C'est la guilde au sens large,
    /// celle que le plafond de populations par cellule protège ; la signature
    /// complète distingue en plus chaque combinaison de voies.
    pub fn main_pathway(&self) -> Option<ReactionId> {
        let mut eff = [0.0; REACTION_COUNT];
        for e in &self.enzymes {
            if self.signature & (1 << e.reaction) != 0 {
                eff[e.reaction as usize] += e.efficiency;
            }
        }
        let mut best: Option<(usize, f64)> = None;
        for (r, &e) in eff.iter().enumerate() {
            if e > 0.0 && best.is_none_or(|b| e > b.1) {
                best = Some((r, e));
            }
        }
        best.map(|b| b.0 as ReactionId)
    }

    /// Clé de guilde du plafond de populations : la voie principale et
    /// l'organisation (phagotrophe, eucaryote, plaste, multicellulaire).
    pub fn guild_key(&self) -> Option<u32> {
        self.main_pathway().map(|r| r as u32 | (self.signature & !PATHWAY_MASK))
    }

    /// Température optimale moyenne des enzymes, pondérée par l'efficacité.
    pub fn mean_t_opt(&self) -> Option<f64> {
        let w: f64 = self.enzymes.iter().map(|e| e.efficiency).sum();
        (w > 0.0).then(|| self.enzymes.iter().map(|e| e.efficiency * e.t_opt_k).sum::<f64>() / w)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metabolism::*;
    use evo_genetics::{Domain, Organelle};

    fn gene(family: DomainFamily, efficiency: f64, affinity: f64, nm: f64) -> Gene {
        Gene { domain: Domain { family, efficiency, affinity, t_opt_k: 300.0, t_width_k: 10.0, absorption_nm: nm }, functional: true }
    }

    #[test]
    fn a_big_cell_loses_surface_but_an_organelle_does_not() {
        use DomainFamily::*;
        let physio = Physiology::default();
        let small = Genome::new(
            vec![gene(Catalytic(AEROBIC_RESPIRATION), 1.0, 1.0, 500.0), gene(Catalytic(FERMENTATION), 1.0, 1.0, 500.0)],
            [0; 32],
        );
        let mut big = small.clone();
        big.genes.push(gene(Cytoskeleton, 1.0, 1.0, 500.0));
        let ps = Phenotype::from_genome(&small, &physio);
        let pb = Phenotype::from_genome(&big, &physio);
        assert_eq!(ps.cell_size, 1.0);
        assert!(pb.cell_size > 3.0);
        assert!(pb.is_phagotroph() && !ps.is_phagotroph());
        let cs = ps.capacities(300.0, &physio);
        let cb = pb.capacities(300.0, &physio);
        let r = AEROBIC_RESPIRATION as usize;
        assert!((cb.cap[r] * pb.cell_size - cs.cap[r]).abs() < 1e-12);
        // La fermentation des proies se fait dans le cytoplasme.
        assert_eq!(cb.digest_fermentation, cs.digest_fermentation);
        // Un organite respiratoire rend la respiration à pleine capacité.
        let mut euk = big.clone();
        euk.organelles.push(Organelle {
            genes: vec![gene(Catalytic(AEROBIC_RESPIRATION), 1.0, 1.0, 500.0)],
            origin_lineage: 1,
            acquired_years: 0.0,
        });
        let pe = Phenotype::from_genome(&euk, &physio);
        assert!(pe.is_eukaryote());
        assert!(pe.capacities(300.0, &physio).digest_respiration > 0.9);
        // Le génome coûte moins par mole de biomasse dans une grande cellule.
        assert!(pe.organelles == 1 && pe.signature & EUKARYOTE != 0);
    }

    #[test]
    fn incremental_build_matches_full_build_bit_for_bit() {
        use evo_genetics::{mutate_with_kind, MutationKind, MutationParams, MUTATION_KINDS};
        use rand::{Rng, SeedableRng};
        use DomainFamily::*;
        let physio = Physiology::default();
        let params = MutationParams::default();
        let mut rng = rand::rngs::StdRng::seed_from_u64(7);
        let mut genome = Genome::new(
            vec![
                gene(Catalytic(FERMENTATION), 0.7, 0.8, 500.0),
                gene(Signalling, 0.6, 1.0, 500.0),
                gene(Adhesion, 0.9, 1.0, 600.0),
                gene(Regulator, 1.0, 0.4, 500.0),
                gene(Pigment, 0.5, 1.0, 650.0),
                gene(Catalytic(ANOXYGENIC_PHOTOSYNTHESIS), 0.8, 1.0, 500.0),
                gene(Rhodopsin, 0.3, 1.0, 550.0),
                gene(Cytochrome, 0.4, 1.0, 500.0),
            ],
            [0; 32],
        );
        genome.organelles.push(Organelle {
            genes: vec![gene(Catalytic(AEROBIC_RESPIRATION), 0.6, 1.0, 500.0)],
            origin_lineage: 1,
            acquired_years: 0.0,
        });
        let mut incremental = 0;
        let mut resized = 0;
        for _ in 0..3000 {
            let basis = Basis::new(&genome, &physio);
            let kind =
                if rng.random::<f64>() < 0.5 { MutationKind::Point } else { MUTATION_KINDS[rng.random_range(0..MUTATION_KINDS.len())] };
            let change = mutate_with_kind(&genome, kind, &params, &mut rng);
            let full = Phenotype::from_genome(&change.genome, &physio);
            if let Some(b) = basis.derive(&genome, &change.genome, change.element, &physio) {
                incremental += 1;
                if change.genome.genes.len() != genome.genes.len() {
                    resized += 1;
                }
                assert_eq!(Phenotype::assemble(&change.genome, &b, &physio), full, "{:?}", change.element);
            }
            if change.genome.genes.len() < 40 {
                genome = change.genome;
            }
        }
        assert!(
            incremental > 1500 && resized > 300,
            "{incremental} constructions incrémentales, dont {resized} avec un gène en plus ou en moins"
        );
    }

    #[test]
    fn a_colony_develops_zones_and_regulators_make_cell_types() {
        use DomainFamily::*;
        let physio = Physiology::default();
        let mut genes = vec![
            gene(Catalytic(FERMENTATION), 1.0, 1.0, 500.0),
            gene(Signalling, 1.0, 1.0, 500.0),
            // Adhésion forte, forme en boule.
            gene(Adhesion, 1.2, 1.0, 600.0),
        ];
        let g0 = Genome::new(genes.clone(), [0; 32]);
        let p0 = Phenotype::from_genome(&g0, &physio);
        let body = p0.body.as_ref().expect("colonie");
        assert!(p0.is_multicellular());
        assert!(body.cells > 100.0 && body.zones.len() > 1, "{} cellules, {} zones", body.cells, body.zones.len());
        assert_eq!(body.types.len(), 1);
        assert!(p0.body_size > p0.cell_size * 4.0);
        // Un régulateur activateur de seuil intermédiaire n'exprime son bloc
        // (une voie lumineuse) qu'en surface.
        genes.push(gene(Regulator, 1.0, 0.5, 500.0));
        genes.push(gene(Pigment, 1.0, 1.0, 600.0));
        genes.push(gene(Catalytic(ANOXYGENIC_PHOTOSYNTHESIS), 1.0, 1.0, 500.0));
        let g1 = Genome::new(genes, [0; 32]);
        let p1 = Phenotype::from_genome(&g1, &physio);
        let body = p1.body.as_ref().unwrap();
        assert_eq!(body.types.len(), 2);
        assert_eq!(body.zones[0].cell_type, 0);
        assert!(body.types[0].phototroph && !body.types[1].phototroph);
        assert!(body.types[1].maintenance_kj < body.types[0].maintenance_kj);
    }

    #[test]
    fn without_signalling_regulators_are_on() {
        use DomainFamily::*;
        let physio = Physiology::default();
        let g = Genome::new(vec![gene(Regulator, 1.0, 0.5, 500.0), gene(Catalytic(FERMENTATION), 1.0, 1.0, 500.0)], [0; 32]);
        let p = Phenotype::from_genome(&g, &physio);
        assert_eq!(p.signature & PATHWAY_MASK, 1 << FERMENTATION);
        let mut g2 = g.clone();
        g2.genes.push(gene(Signalling, 0.2, 1.0, 500.0));
        // Signal faible sous le seuil : le bloc se tait.
        let p2 = Phenotype::from_genome(&g2, &physio);
        assert_eq!(p2.signature & PATHWAY_MASK, 0);
    }
}
