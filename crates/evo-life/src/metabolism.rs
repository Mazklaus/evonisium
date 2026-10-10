//! Catalogue des voies métaboliques (« recettes » du document Organismes).
//!
//! Une voie consomme des entrées, produit des sorties et rend de l'énergie.
//! Les enthalpies libres sont des ordres de grandeur réels dans les conditions
//! de l'eau de mer ; elles ne dépendent pas encore des concentrations.
//!
//! Les voies lumineuses suivent la règle des innovations à plusieurs pièces
//! (document Organismes, « chaque étape doit payer ») : leur rendement est le
//! produit continu des qualités de leurs pièces (pigment, centres
//! réactionnels, complexe d'oxydation de l'eau) et de la disponibilité du
//! donneur d'électrons. Le centre réactionnel d'une voie est le domaine
//! `Catalytic` de cette voie ; le donneur qu'il accepte est sa spécificité.
//!
//! [Simplification] Neuf voies, une seule monnaie énergétique (kJ), pas de
//! dépendance thermodynamique aux concentrations ; l'azote n'est pas
//! limitant ; le carbone des voies lumineuses est fixé par la voie de
//! fixation héritée des chimioautotrophes, qui n'est pas un gène à part.

use crate::phenotype::Phenotype;
use evo_genetics::{DomainFamily, DomainRelation, ReactionId};
use evo_planet::WaterPool;

/// Source d'énergie d'une voie.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EnergySource {
    /// Réaction chimique : énergie par mole du substrat limitant.
    Chemical { substrate: WaterPool, half_saturation: f64, dg_kj: f64 },
    /// Lumière captée par un pigment et transmise à un centre réactionnel.
    /// `partner_rc` : la voie demande en plus un second centre réactionnel
    /// (n'importe quel centre anoxygénique, futur photosystème I) ;
    /// `water_oxidation` : elle demande le complexe qui oxyde l'eau.
    Light { partner_rc: bool, water_oxidation: bool },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Reaction {
    pub id: ReactionId,
    pub name: &'static str,
    pub energy: EnergySource,
    /// Second substrat obligatoire (accepteur ou donneur d'électrons) et sa
    /// constante de demi-saturation, mol·m⁻³.
    pub cosubstrate: Option<(WaterPool, f64)>,
    /// Entrées et sorties par unité de réaction : mole de substrat limitant
    /// pour une voie chimique, mole de carbone fixé pour une voie lumineuse.
    pub inputs: &'static [(WaterPool, f64)],
    pub outputs: &'static [(WaterPool, f64)],
    /// Pour une voie chimique autotrophe : pouvoir réducteur consommé (négatif)
    /// ou produit (positif) par mole de carbone fixée dans la biomasse. Le
    /// CO₂ ne devient matière organique qu'avec 4 électrons par carbone ; les
    /// voies lumineuses les comptent déjà dans leurs entrées.
    pub fixation: &'static [(WaterPool, f64)],
    /// Le carbone de la biomasse vient de la matière organique.
    pub heterotrophic: bool,
    /// Voie d'anaérobie stricte, bloquée par l'oxygène.
    pub oxygen_sensitive: bool,
}

impl Reaction {
    pub fn is_light(&self) -> bool {
        matches!(self.energy, EnergySource::Light { .. })
    }
}

pub const METHANOGENESIS: ReactionId = 0;
pub const FERMENTATION: ReactionId = 1;
pub const AEROBIC_RESPIRATION: ReactionId = 2;
pub const SULFATE_REDUCTION: ReactionId = 3;
pub const METHANOTROPHY: ReactionId = 4;
pub const ANOXYGENIC_PHOTOSYNTHESIS: ReactionId = 5;
pub const OXYGENIC_PHOTOSYNTHESIS: ReactionId = 6;
pub const PHOTOFERROTROPHY: ReactionId = 7;
pub const MANGANESE_PHOTOTROPHY: ReactionId = 8;

pub const REACTION_COUNT: usize = 9;

/// Voies lumineuses anoxygéniques : leurs centres servent de second centre
/// (photosystème I) à la voie oxygénique.
pub const ANOXYGENIC_CENTRES: [ReactionId; 3] = [ANOXYGENIC_PHOTOSYNTHESIS, PHOTOFERROTROPHY, MANGANESE_PHOTOTROPHY];

use WaterPool::*;

/// Le catalogue, indexé par [`ReactionId`].
pub const REACTIONS: [Reaction; REACTION_COUNT] = [
    // 4 H₂ + CO₂ → CH₄ + 2 H₂O
    Reaction {
        id: METHANOGENESIS,
        name: "méthanogenèse",
        energy: EnergySource::Chemical { substrate: H2, half_saturation: 2e-3, dg_kj: 33.0 },
        cosubstrate: None,
        inputs: &[(H2, 1.0), (Dic, 0.25)],
        outputs: &[(Ch4, 0.25)],
        // CO₂ + 2 H₂ → CH₂O + H₂O
        fixation: &[(H2, -2.0)],
        heterotrophic: false,
        oxygen_sensitive: true,
    },
    // 2 CH₂O → CH₄ + CO₂ (fermentation méthanogène globale)
    Reaction {
        id: FERMENTATION,
        name: "fermentation",
        energy: EnergySource::Chemical { substrate: Doc, half_saturation: 1e-2, dg_kj: 30.0 },
        cosubstrate: None,
        inputs: &[(Doc, 1.0)],
        outputs: &[(Ch4, 0.5), (Dic, 0.5)],
        fixation: &[],
        heterotrophic: true,
        oxygen_sensitive: false,
    },
    // CH₂O + O₂ → CO₂ + H₂O
    Reaction {
        id: AEROBIC_RESPIRATION,
        name: "respiration aérobie",
        energy: EnergySource::Chemical { substrate: Doc, half_saturation: 1e-2, dg_kj: 479.0 },
        cosubstrate: Some((O2, 1e-3)),
        inputs: &[(Doc, 1.0), (O2, 1.0)],
        outputs: &[(Dic, 1.0)],
        fixation: &[],
        heterotrophic: true,
        oxygen_sensitive: false,
    },
    // 2 CH₂O + SO₄²⁻ → 2 CO₂ + H₂S
    Reaction {
        id: SULFATE_REDUCTION,
        name: "sulfato-réduction",
        energy: EnergySource::Chemical { substrate: Doc, half_saturation: 1e-2, dg_kj: 50.0 },
        cosubstrate: Some((Sulfate, 1e-2)),
        inputs: &[(Doc, 1.0), (Sulfate, 0.5)],
        outputs: &[(Dic, 1.0), (H2s, 0.5)],
        fixation: &[],
        heterotrophic: true,
        oxygen_sensitive: true,
    },
    // CH₄ + 2 O₂ → CO₂ + 2 H₂O
    Reaction {
        id: METHANOTROPHY,
        name: "méthanotrophie",
        energy: EnergySource::Chemical { substrate: Ch4, half_saturation: 1e-3, dg_kj: 818.0 },
        cosubstrate: Some((O2, 1e-3)),
        inputs: &[(Ch4, 1.0), (O2, 2.0)],
        outputs: &[(Dic, 1.0)],
        // Le méthane fournit les électrons : ½ CH₄ → ½ CO₂ pour un carbone fixé.
        fixation: &[(Ch4, -0.5), (Dic, 0.5)],
        heterotrophic: false,
        oxygen_sensitive: false,
    },
    // 2 CO₂ + H₂S + 2 H₂O → 2 CH₂O + SO₄²⁻ (par carbone fixé)
    Reaction {
        id: ANOXYGENIC_PHOTOSYNTHESIS,
        name: "photosynthèse anoxygénique au soufre",
        energy: EnergySource::Light { partner_rc: false, water_oxidation: false },
        cosubstrate: Some((H2s, 1e-3)),
        inputs: &[(H2s, 0.5)],
        outputs: &[(Sulfate, 0.5)],
        fixation: &[],
        heterotrophic: false,
        oxygen_sensitive: true,
    },
    // CO₂ + H₂O → CH₂O + O₂ (par carbone fixé)
    Reaction {
        id: OXYGENIC_PHOTOSYNTHESIS,
        name: "photosynthèse oxygénique",
        energy: EnergySource::Light { partner_rc: true, water_oxidation: true },
        cosubstrate: None,
        inputs: &[],
        outputs: &[(O2, 1.0)],
        fixation: &[],
        heterotrophic: false,
        oxygen_sensitive: false,
    },
    // 4 Fe²⁺ + CO₂ + 11 H₂O → CH₂O + 4 Fe(OH)₃ + 8 H⁺ (par carbone fixé)
    Reaction {
        id: PHOTOFERROTROPHY,
        name: "photosynthèse anoxygénique au fer",
        energy: EnergySource::Light { partner_rc: false, water_oxidation: false },
        cosubstrate: Some((Fe2, 1e-3)),
        inputs: &[(Fe2, 4.0)],
        outputs: &[(FeOx, 4.0)],
        fixation: &[],
        heterotrophic: false,
        oxygen_sensitive: true,
    },
    // 2 Mn²⁺ + CO₂ + 3 H₂O → CH₂O + 2 MnO₂ + 4 H⁺ (par carbone fixé) : le
    // manganèse, d'abord simple donneur, sur le chemin de l'oxydation de l'eau.
    Reaction {
        id: MANGANESE_PHOTOTROPHY,
        name: "photosynthèse anoxygénique au manganèse",
        energy: EnergySource::Light { partner_rc: false, water_oxidation: false },
        cosubstrate: Some((Mn2, 1e-4)),
        inputs: &[(Mn2, 2.0)],
        outputs: &[(MnOx, 2.0)],
        fixation: &[],
        heterotrophic: false,
        oxygen_sensitive: true,
    },
];

/// Noms des voies présentes dans une signature.
pub fn signature_names(signature: u32) -> Vec<&'static str> {
    REACTIONS.iter().filter(|r| signature & (1 << r.id) != 0).map(|r| r.name).collect()
}

/// Libellé court d'une guilde, pour les rapports : ses voies, puis son
/// organisation (bits au-dessus des voies, voir `phenotype`).
pub fn guild_label(signature: u32) -> String {
    use crate::phenotype::{EUKARYOTE, MULTICELLULAR, PHAGOTROPH, PLASTID};
    let names = signature_names(signature);
    let mut label = if names.is_empty() { "aucune voie".to_string() } else { names.join(" + ") };
    let mut traits = Vec::new();
    if signature & PHAGOTROPH != 0 {
        traits.push("phagotrophe");
    }
    if signature & EUKARYOTE != 0 {
        traits.push("eucaryote");
    }
    if signature & PLASTID != 0 {
        traits.push("à plaste");
    }
    if signature & MULTICELLULAR != 0 {
        traits.push("multicellulaire");
    }
    if !traits.is_empty() {
        label = format!("{label} ({})", traits.join(", "));
    }
    label
}

/// Parentés entre familles de domaines (règle « recycler l'existant par
/// duplication » du document Génétique) : le pigment dérive des cytochromes
/// (porphyrines, chlorophylle issue de la voie de l'hème), les centres
/// réactionnels dérivent de transporteurs d'électrons et changent de donneur
/// entre eux, le centre oxygénique dérive d'un centre anoxygénique, le
/// complexe à manganèse dérive du site de liaison du manganèse.
pub fn domain_relations() -> Vec<DomainRelation> {
    use DomainFamily::*;
    let rel = |from, to, weight| DomainRelation { from, to, weight };
    let mut v = vec![
        rel(Cytochrome, Pigment, 1.0),
        rel(Pigment, Cytochrome, 0.3),
        rel(Cytochrome, Catalytic(ANOXYGENIC_PHOTOSYNTHESIS), 0.3),
        rel(Cytochrome, Catalytic(PHOTOFERROTROPHY), 0.3),
        rel(Cytochrome, Catalytic(AEROBIC_RESPIRATION), 0.3),
        rel(Catalytic(MANGANESE_PHOTOTROPHY), WaterOxidation, 1.0),
        rel(OxidativeDefense, WaterOxidation, 0.2),
        rel(WaterOxidation, OxidativeDefense, 0.5),
        // Méthanogenèse inverse (méthanotrophie anaérobie et aérobie).
        rel(Catalytic(METHANOGENESIS), Catalytic(METHANOTROPHY), 0.2),
        rel(Catalytic(FERMENTATION), Catalytic(SULFATE_REDUCTION), 0.2),
        // Cellule complexe (étape 4). Le cytosquelette dérive des kinases de
        // sucres de la fermentation (superfamille de l'actine) ; les
        // récepteurs de signal, des kinases et des photorécepteurs ; les
        // protéines d'adhésion, de protéines de membrane et du cytosquelette ;
        // les facteurs de transcription et la recombinase de méiose, des
        // protéines qui lient l'ADN pour le réparer.
        rel(Catalytic(FERMENTATION), Cytoskeleton, 0.05),
        rel(Catalytic(FERMENTATION), Signalling, 0.05),
        rel(Rhodopsin, Signalling, 0.3),
        rel(Rhodopsin, Adhesion, 0.1),
        rel(Cytoskeleton, Adhesion, 0.2),
        rel(Signalling, Adhesion, 0.1),
        rel(Repair, Regulator, 0.2),
        rel(Signalling, Regulator, 0.1),
        rel(Regulator, Regulator, 0.5),
        rel(Repair, Meiosis, 0.3),
        // Vie hors de l'eau (étape 5) : l'enveloppe protectrice dérive des
        // enzymes des sucres (polysaccharides extracellulaires) et des
        // protéines de la matrice d'adhésion.
        rel(Catalytic(FERMENTATION), Cuticle, 0.05),
        rel(Adhesion, Cuticle, 0.1),
    ];
    // Les centres anoxygéniques changent de donneur entre eux, et chacun peut
    // donner le centre oxygénique (photosystème II).
    for a in ANOXYGENIC_CENTRES {
        for b in ANOXYGENIC_CENTRES {
            if a != b {
                v.push(rel(Catalytic(a), Catalytic(b), 1.0));
            }
        }
        v.push(rel(Catalytic(a), Catalytic(OXYGENIC_PHOTOSYNTHESIS), 0.5));
    }
    v
}

/// Chemin vers la photosynthèse (documents Génétique et Organismes) : étape la
/// plus avancée atteinte par un phénotype.
pub const PHOTOSYNTHESIS_PATHWAY: &str = "photosynthèse";
pub const PHOTOSYNTHESIS_STAGES: [&str; 5] =
    ["aucune", "pigment protecteur", "phototrophie simple", "photosynthèse anoxygénique", "photosynthèse oxygénique"];
pub const RHODOPSIN_PATHWAY: &str = "rhodopsine";

pub fn photosynthesis_stage(p: &Phenotype) -> u8 {
    if p.signature & (1 << OXYGENIC_PHOTOSYNTHESIS) != 0 {
        4
    } else if ANOXYGENIC_CENTRES.iter().any(|&r| p.signature & (1 << r) != 0) {
        3
    } else if p.cyclic_phototrophy {
        2
    } else if p.pigment > 0.0 {
        1
    } else {
        0
    }
}
