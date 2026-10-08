//! Réseau trophique d'une cellule (document Fonctionnalités, « Outils
//! d'observation » : qui mange qui ?).
//!
//! Dans le monde microbien, on ne se mange pas : on vit des produits des
//! autres. Le réseau relie donc les espèces d'une cellule par ce qui passe
//! de l'une à l'autre dans l'eau :
//!
//! - **nourrit** : la matière organique fabriquée par les autotrophes nourrit
//!   les hétérotrophes ;
//! - **échange** (syntrophie) : un produit de l'un est la ressource de
//!   l'autre (O₂ des photosynthétiques pour les aérobies, CH₄ des
//!   méthanogènes pour les méthanotrophes, sulfure des sulfato-réducteurs
//!   pour les phototrophes au soufre…) ;
//! - **compétition** : deux espèces tirent leur énergie de la même ressource
//!   (ou de la lumière).
//!
//! L'épaisseur d'un lien suit le flux d'énergie, estimé à partir de la
//! production brute de chaque population (biomasse × taux de naissance),
//! répartie entre producteurs au prorata de leur production. C'est une
//! estimation d'affichage, signalée comme telle : le moteur ne suit pas les
//! molécules d'une population à l'autre, seulement leurs concentrations.
//!
//! La prédation et le parasitisme entre multicellulaires s'ajouteront ici
//! quand le moteur publiera le régime de chaque population.

use evo_life::metabolism::{EnergySource, REACTIONS};
use evo_planet::WaterPool;
use evo_sim::observation::PopulationView;

/// Nature d'un lien.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Link {
    /// Matière organique d'un autotrophe vers un hétérotrophe.
    Feeds,
    /// Produit de l'un, ressource de l'autre.
    Exchange,
    /// Même source d'énergie.
    Competition,
}

impl Link {
    pub fn key(self) -> &'static str {
        match self {
            Link::Feeds => "nourrit",
            Link::Exchange => "echange",
            Link::Competition => "competition",
        }
    }
}

/// Une espèce du réseau (populations d'une même espèce réunies).
#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    pub species: u32,
    pub biomass: f64,
    /// Production brute, mol C·an⁻¹.
    pub production: f64,
    /// Autotrophe (fabrique sa matière organique).
    pub autotroph: bool,
    /// Niveau trophique : 0 producteurs, 1 consommateurs de leurs
    /// produits, etc.
    pub level: u32,
    pub pigment_rgb: Option<[u8; 3]>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Edge {
    pub from: usize,
    pub to: usize,
    pub link: Link,
    /// Ce qui passe (« matière organique », « O₂ », « lumière »…).
    pub what: &'static str,
    /// Flux estimé, mol C·an⁻¹ d'équivalent production.
    pub flux: f64,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct FoodWeb {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}

fn pool_label(p: WaterPool) -> &'static str {
    match p {
        WaterPool::Dic => "CO₂",
        WaterPool::Doc => "matière organique",
        WaterPool::H2 => "H₂",
        WaterPool::Ch4 => "CH₄",
        WaterPool::O2 => "O₂",
        WaterPool::Sulfate => "sulfate",
        WaterPool::H2s => "sulfure",
        WaterPool::Fe2 => "fer ferreux",
        WaterPool::Mn2 => "manganèse",
        WaterPool::FeOx => "oxydes de fer",
        WaterPool::MnOx => "oxydes de manganèse",
        WaterPool::Po4 => "phosphate",
    }
}

/// Réactions d'une espèce (bits de sa signature).
fn reactions(signature: u32) -> impl Iterator<Item = &'static evo_life::metabolism::Reaction> {
    REACTIONS.iter().filter(move |r| signature & (1 << r.id) != 0)
}

fn autotroph(signature: u32) -> bool {
    reactions(signature).any(|r| !r.heterotrophic)
}

fn produces(signature: u32, pool: WaterPool) -> bool {
    reactions(signature).any(|r| r.outputs.iter().any(|(p, _)| *p == pool))
}

fn consumes(signature: u32, pool: WaterPool) -> bool {
    reactions(signature).any(|r| r.inputs.iter().any(|(p, _)| *p == pool) || r.cosubstrate.is_some_and(|(p, _)| p == pool))
}

/// Sources d'énergie : substrat limitant de chaque voie chimique, ou la
/// lumière (`None`).
fn energy_sources(signature: u32) -> Vec<Option<WaterPool>> {
    let mut out: Vec<Option<WaterPool>> = reactions(signature)
        .map(|r| match r.energy {
            EnergySource::Chemical { substrate, .. } => Some(substrate),
            EnergySource::Light { .. } => None,
        })
        .collect();
    out.dedup();
    out
}

/// Réseau d'une cellule à partir de ses populations.
pub fn build(populations: &[PopulationView]) -> FoodWeb {
    // Espèces, dans l'ordre de première apparition (déterministe).
    let mut nodes: Vec<Node> = Vec::new();
    for p in populations {
        let production = (p.biomass.max(0.0) * p.birth_per_year.max(0.0)) as f64;
        match nodes.iter_mut().find(|n| n.species == p.species) {
            Some(n) => {
                n.biomass += p.biomass as f64;
                n.production += production;
                if n.pigment_rgb.is_none() {
                    n.pigment_rgb = p.pigment_rgb;
                }
            }
            None => nodes.push(Node {
                species: p.species,
                biomass: p.biomass as f64,
                production,
                autotroph: autotroph(p.species),
                level: 0,
                pigment_rgb: p.pigment_rgb,
            }),
        }
    }
    let mut edges = Vec::new();
    // Ce qui circule : matière organique des autotrophes, puis les produits
    // des voies.
    let flows: [(WaterPool, Link); 6] = [
        (WaterPool::Doc, Link::Feeds),
        (WaterPool::O2, Link::Exchange),
        (WaterPool::Ch4, Link::Exchange),
        (WaterPool::H2s, Link::Exchange),
        (WaterPool::Sulfate, Link::Exchange),
        (WaterPool::H2, Link::Exchange),
    ];
    for (pool, link) in flows {
        let producers: Vec<usize> = (0..nodes.len())
            .filter(|&i| if pool == WaterPool::Doc { nodes[i].autotroph } else { produces(nodes[i].species, pool) })
            .collect();
        let total: f64 = producers.iter().map(|&i| nodes[i].production).sum();
        if total <= 0.0 {
            continue;
        }
        for j in 0..nodes.len() {
            if !consumes(nodes[j].species, pool) {
                continue;
            }
            for &i in &producers {
                if i == j {
                    continue;
                }
                let flux = nodes[j].production * nodes[i].production / total;
                if flux > 0.0 {
                    edges.push(Edge { from: i, to: j, link, what: pool_label(pool), flux });
                }
            }
        }
    }
    // Compétition : même source d'énergie (une fois par paire).
    for i in 0..nodes.len() {
        let si = energy_sources(nodes[i].species);
        for j in i + 1..nodes.len() {
            let sj = energy_sources(nodes[j].species);
            if let Some(shared) = si.iter().find(|s| sj.contains(s)) {
                let flux = nodes[i].production.min(nodes[j].production);
                if flux > 0.0 {
                    let what = shared.map_or("lumière", pool_label);
                    edges.push(Edge { from: i, to: j, link: Link::Competition, what, flux });
                }
            }
        }
    }
    // Niveaux : un consommateur est un cran au-dessus du plus haut de ses
    // fournisseurs (les cycles sont coupés par la borne).
    for _ in 0..nodes.len() {
        let mut changed = false;
        for e in edges.iter().filter(|e| e.link != Link::Competition) {
            let want = (nodes[e.from].level + 1).min(nodes.len() as u32);
            if !nodes[e.to].autotroph && nodes[e.to].level < want {
                nodes[e.to].level = want;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    FoodWeb { nodes, edges }
}

#[cfg(test)]
mod tests {
    use super::*;
    use evo_life::metabolism::{AEROBIC_RESPIRATION, FERMENTATION, METHANOGENESIS, METHANOTROPHY, OXYGENIC_PHOTOSYNTHESIS};

    fn pop(species: u32, biomass: f32, birth: f32) -> PopulationView {
        PopulationView {
            lineage: 0,
            species,
            biomass,
            growth_per_year: 0.0,
            birth_per_year: birth,
            genes: 3,
            pigment_rgb: None,
            pigment_nm: None,
            phototroph: false,
            photosynthesis_stage: 0,
        }
    }

    #[test]
    fn a_microbial_mat_feeds_and_exchanges() {
        let photo = 1 << OXYGENIC_PHOTOSYNTHESIS;
        let aerobe = 1 << AEROBIC_RESPIRATION;
        let fermenter = 1 << FERMENTATION;
        let methanogen = 1 << METHANOGENESIS;
        let methanotroph = 1 << METHANOTROPHY;
        let web = build(&[
            pop(photo, 100.0, 2.0),
            pop(aerobe, 20.0, 1.0),
            pop(fermenter, 10.0, 1.0),
            pop(fermenter, 5.0, 1.0),
            pop(methanogen, 4.0, 1.0),
            pop(methanotroph, 2.0, 1.0),
        ]);
        assert_eq!(web.nodes.len(), 5, "les populations d'une espèce sont réunies");
        let idx = |s: u32| web.nodes.iter().position(|n| n.species == s).unwrap();
        let has = |a: u32, b: u32, l: Link| web.edges.iter().any(|e| e.from == idx(a) && e.to == idx(b) && e.link == l);
        assert!(has(photo, aerobe, Link::Feeds));
        assert!(has(photo, aerobe, Link::Exchange), "O₂");
        assert!(has(photo, fermenter, Link::Feeds));
        assert!(has(fermenter, methanotroph, Link::Exchange), "CH₄");
        assert!(web.edges.iter().any(|e| e.link == Link::Competition && e.what == "matière organique"));
        assert_eq!(web.nodes[idx(photo)].level, 0);
        assert!(web.nodes[idx(aerobe)].level >= 1);
        assert!(web.edges.iter().all(|e| e.flux > 0.0));
    }
}
