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
use evo_genetics::{DomainFamily, Gene, Genome, ReactionId, REGULATOR_SENSE_NM};
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

impl Phenotype {
    pub fn from_genome(genome: &Genome, physio: &Physiology) -> Self {
        let org = organisation(genome);
        let cell_size = (1.0 + physio.cytoskeleton_size * org.cytoskeleton).min(physio.max_cell_size);
        let regulated_by = genome.regulated_by();
        let has_regulators = regulated_by.iter().any(Option::is_some);
        let signal = (org.signalling > 0.0).then_some(org.signalling.min(1.0));
        let clonal = org.adhesion >= physio.clonal_adhesion;
        if !clonal {
            // Cellule seule : le morphogène est son propre signal.
            let mask = if has_regulators { Some(expression_mask(genome, &regulated_by, signal)) } else { None };
            let mut p = Self::cell(genome, mask.as_deref(), &org, cell_size, physio);
            // Agrégats : l'adhésion sous le seuil de la colonie colle les
            // cellules en amas lâches, que les prédateurs avalent moins bien.
            p.body_size = cell_size * (1.0 + physio.aggregate_size * org.adhesion);
            return p;
        }
        Self::develop(genome, &regulated_by, &org, cell_size, signal, physio)
    }

    /// Développement d'une colonie clonale : nombre de cellules, forme,
    /// zones, morphogène, puis un type cellulaire par jeu de gènes exprimés.
    fn develop(
        genome: &Genome,
        regulated_by: &[Option<u16>],
        org: &Organisation,
        cell_size: f64,
        signal: Option<f64>,
        physio: &Physiology,
    ) -> Self {
        let doublings = 1.0 + physio.colony_doublings * (org.adhesion - physio.clonal_adhesion).min(1.0);
        let cells = doublings.dexp2().min(physio.max_colony_cells);
        let shape = if org.adhesion_nm < 500.0 {
            Shape::Filament
        } else if org.adhesion_nm > 800.0 {
            Shape::Sheet
        } else {
            Shape::Sphere
        };
        // Géométrie : couches de cellules de la surface vers le centre.
        let mut layout: Vec<(f64, f64, f64)> = Vec::new(); // (poids, profondeur, lumière en plus de l'accès)
        match shape {
            Shape::Filament => layout.push((1.0, 0.0, 1.0)),
            Shape::Sheet => {
                if cells >= 4.0 {
                    layout.push((0.5, 0.0, 1.0));
                    layout.push((0.5, 1.0, 1.0));
                } else {
                    layout.push((1.0, 0.0, 1.0));
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
                    layout.push((weight, k as f64 * step, 1.0));
                }
            }
        }
        let reach = physio.morphogen_reach_cells * (0.5 + org.signalling_reach);
        let mut zones: Vec<Zone> = Vec::with_capacity(layout.len());
        let mut masks: Vec<Vec<bool>> = Vec::new();
        for &(weight, depth, _) in &layout {
            let access = (-depth / physio.diffusion_cells).dexp();
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
            zones.push(Zone { weight, depth_cells: depth, access, light: access, morphogen: m.unwrap_or(0.0), cell_type: cell_type as u8 });
        }
        let types: Vec<Arc<Phenotype>> = masks.iter().map(|m| Arc::new(Self::cell(genome, Some(m), org, cell_size, physio))).collect();
        // Le phénotype de l'organisme : celui de la surface, avec la
        // signature de tous les types, l'entretien moyen et le corps.
        let mut top = (*types[zones[0].cell_type as usize]).clone();
        let mut signature = MULTICELLULAR;
        let mut maintenance = 0.0;
        for z in &zones {
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
        top.body = Some(Arc::new(Body { cells, shape, clonal: true, zones, types, expression: masks }));
        top
    }

    /// Phénotype d'une cellule qui exprime les gènes de `mask` (tous les
    /// gènes fonctionnels si `None`) et ceux de ses organites.
    fn cell(genome: &Genome, mask: Option<&[bool]>, org: &Organisation, cell_size: f64, physio: &Physiology) -> Self {
        let mut enzymes = Vec::with_capacity(genome.genes.len());
        // Totaux par voie et par famille : le coût est convexe sur le total,
        // pour qu'une duplication ne rende pas une protéine moins chère.
        let mut reaction_eff = [0.0; REACTION_COUNT];
        let mut reaction_aff = [0.0; REACTION_COUNT];
        let (mut pigment, mut pigment_capture, mut pigment_nm_sum) = (0.0, 0.0, 0.0);
        let (mut cytochrome, mut rhodopsin, mut rhodopsin_capture, mut wox, mut defense, mut repair) = (0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
        // Structure, adhésion, signal, régulation, méiose.
        let mut cellular = 0.0;
        let volume = cell_size * cell_size * cell_size;
        let gene_count = genome.genes.len() + genome.organelles.iter().map(|o| o.genes.len()).sum::<usize>();
        // Une copie du génome par cellule : son coût par mole de biomasse
        // baisse avec le volume de la cellule (énergie par gène).
        let mut maintenance = physio.base_maintenance_kj + physio.genome_cost_kj * gene_count as f64 / volume;
        let membrane = 1.0 / cell_size;
        let mut expressed = 0u32;
        let host = genome.genes.iter().enumerate().filter(|(i, g)| g.functional && mask.is_none_or(|m| m[*i])).map(|(_, g)| (g, false));
        let inside = genome.organelle_genes().map(|g| (g, true));
        for (gene, internal) in host.chain(inside) {
            let d = gene.domain;
            expressed += 1;
            maintenance += physio.expression_cost_kj;
            match d.family {
                DomainFamily::Catalytic(r) if (r as usize) < REACTION_COUNT => {
                    reaction_eff[r as usize] += d.efficiency;
                    reaction_aff[r as usize] += d.efficiency * d.affinity;
                    enzymes.push(Enzyme {
                        reaction: r,
                        efficiency: d.efficiency,
                        affinity: d.affinity,
                        t_opt_k: d.t_opt_k,
                        t_width_k: d.t_width_k,
                        internal,
                    });
                }
                DomainFamily::Catalytic(_) => {}
                DomainFamily::Pigment => {
                    pigment += d.efficiency;
                    pigment_capture += d.efficiency * physio.spectrum.match_at(d.absorption_nm);
                    pigment_nm_sum += d.efficiency * d.absorption_nm;
                }
                DomainFamily::Cytochrome => cytochrome += d.efficiency,
                DomainFamily::Rhodopsin => {
                    rhodopsin += d.efficiency;
                    // Pompe de la membrane de l'hôte : elle rend moins dans une
                    // grande cellule.
                    rhodopsin_capture += d.efficiency * membrane * physio.spectrum.match_at(d.absorption_nm);
                }
                DomainFamily::WaterOxidation => wox += d.efficiency,
                DomainFamily::OxidativeDefense => defense += d.efficiency,
                // La réparation agit sur le taux de mutation ; ici seul son coût compte.
                DomainFamily::Repair => repair += d.efficiency,
                f if f.is_cellular() => cellular += d.efficiency * d.efficiency,
                _ => {}
            }
        }
        // Le signal règle l'expression sur le besoin : les protéines des voies
        // coûtent moins cher (document Organismes, « Capteurs moléculaires »).
        let regulation = 1.0 - physio.signalling_saving * org.signalling.min(1.0);
        for r in 0..REACTION_COUNT {
            let e = reaction_eff[r];
            if e > 0.0 {
                let a = reaction_aff[r] / e;
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
