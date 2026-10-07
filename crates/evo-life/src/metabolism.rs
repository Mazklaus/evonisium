//! Catalogue des voies métaboliques (« recettes » du document Organismes).
//!
//! Une voie consomme des entrées, produit des sorties et rend de l'énergie.
//! Les enthalpies libres sont des ordres de grandeur réels dans les conditions
//! de l'eau de mer ; elles ne dépendent pas encore des concentrations.
//!
//! [Simplification] Sept voies, une seule monnaie énergétique (kJ), pas de
//! dépendance thermodynamique aux concentrations ; azote et phosphore ne sont
//! pas limitants à l'étape 1.

use evo_genetics::ReactionId;
use evo_planet::WaterPool;

/// Source d'énergie d'une voie.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EnergySource {
    /// Réaction chimique : énergie par mole du substrat limitant.
    Chemical { substrate: WaterPool, half_saturation: f64, dg_kj: f64 },
    /// Lumière, captée par un pigment.
    Light,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Reaction {
    pub id: ReactionId,
    pub name: &'static str,
    pub energy: EnergySource,
    /// Second substrat obligatoire (accepteur d'électrons, réducteur) et sa
    /// constante de demi-saturation, mol·m⁻³.
    pub cosubstrate: Option<(WaterPool, f64)>,
    /// Entrées et sorties par unité de réaction (mole de substrat limitant,
    /// ou mole de carbone fixé pour la photosynthèse).
    pub inputs: &'static [(WaterPool, f64)],
    pub outputs: &'static [(WaterPool, f64)],
    /// Le carbone de la biomasse vient de la matière organique.
    pub heterotrophic: bool,
    /// Voie d'anaérobie stricte, bloquée par l'oxygène.
    pub oxygen_sensitive: bool,
}

pub const METHANOGENESIS: ReactionId = 0;
pub const FERMENTATION: ReactionId = 1;
pub const AEROBIC_RESPIRATION: ReactionId = 2;
pub const SULFATE_REDUCTION: ReactionId = 3;
pub const METHANOTROPHY: ReactionId = 4;
pub const ANOXYGENIC_PHOTOSYNTHESIS: ReactionId = 5;
pub const OXYGENIC_PHOTOSYNTHESIS: ReactionId = 6;

pub const REACTION_COUNT: usize = 7;

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
        heterotrophic: false,
        oxygen_sensitive: false,
    },
    // 2 CO₂ + H₂S + 2 H₂O → 2 CH₂O + SO₄²⁻ (par carbone fixé)
    Reaction {
        id: ANOXYGENIC_PHOTOSYNTHESIS,
        name: "photosynthèse anoxygénique",
        energy: EnergySource::Light,
        cosubstrate: Some((H2s, 1e-3)),
        inputs: &[(H2s, 0.5)],
        outputs: &[(Sulfate, 0.5)],
        heterotrophic: false,
        oxygen_sensitive: true,
    },
    // CO₂ + H₂O → CH₂O + O₂ (par carbone fixé)
    Reaction {
        id: OXYGENIC_PHOTOSYNTHESIS,
        name: "photosynthèse oxygénique",
        energy: EnergySource::Light,
        cosubstrate: None,
        inputs: &[],
        outputs: &[(O2, 1.0)],
        heterotrophic: false,
        oxygen_sensitive: false,
    },
];

/// Noms des voies présentes dans une signature.
pub fn signature_names(signature: u32) -> Vec<&'static str> {
    REACTIONS.iter().filter(|r| signature & (1 << r.id) != 0).map(|r| r.name).collect()
}

/// Libellé court d'une guilde, pour les rapports.
pub fn guild_label(signature: u32) -> String {
    let names = signature_names(signature);
    if names.is_empty() {
        "aucune voie".into()
    } else {
        names.join(" + ")
    }
}
