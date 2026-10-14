//! Le milieu d'une cellule, tel que les sociétés l'exploitent : climat,
//! productivité, sols, forêts, gisements et espèces sauvages.
//!
//! Dans la partie, ces grandeurs viennent de Planète (climat, gisements
//! issus du carbone enfoui et de la géologie) et des populations du moteur
//! (espèces sauvages). Sur banc d'essai, `bench` les écrit à la main.

use crate::goods::Good;
use evo_core::math::Det;
use evo_culture::catalog::Milieu;
use serde::{Deserialize, Serialize};

/// Gisements d'une cellule, en tonnes (métal contenu pour les minerais,
/// carbone pour le charbon et le pétrole).
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Deposits {
    pub coal_c: f64,
    pub oil_c: f64,
    pub copper: f64,
    pub tin: f64,
    pub iron: f64,
    pub uranium: f64,
    /// Sel exploitable (salines, gemme), t par an.
    pub salt: f64,
}

/// Une espèce sauvage, décrite par son phénotype.
///
/// [Simplification] Le moteur fournira ces traits depuis le phénotype de
/// chaque population (taille, régime, sociabilité, reproduction,
/// tempérament) ; la domesticabilité en découle par [`WildSpecies::tameness`]
/// et [`WildSpecies::cultivability`], jamais d'une liste d'espèces.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WildSpecies {
    pub id: u32,
    pub name: String,
    pub kind: WildKind,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum WildKind {
    Plant(PlantTraits),
    Animal(AnimalTraits),
}

/// Ce qui fait une plante cultivable (Diamond, 1997 ; Harlan, 1992).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlantTraits {
    /// Organe récolté : graine, racine ou tubercule, fruit.
    pub organ: PlantOrgan,
    /// Masse d'une graine, mg (céréales sauvages : 10 à 50 mg).
    pub seed_mg: f64,
    /// Plante annuelle (tout son effort va dans la graine).
    pub annual: bool,
    /// Autofécondation : les variétés choisies ne se mélangent pas aux
    /// sauvages.
    pub selfing: bool,
    /// Rendement sauvage, t par hectare.
    pub wild_yield_t_ha: f64,
    /// Climat où elle pousse : température moyenne, °C, et pluie, mm.
    pub temp_c: f64,
    pub rain_mm: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PlantOrgan {
    Seed,
    Root,
    Fruit,
}

/// Les six critères de Diamond pour un grand animal domestique.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct AnimalTraits {
    pub mass_kg: f64,
    /// Part de plantes dans le régime, de 0 (carnivore) à 1 (herbivore).
    pub herbivory: f64,
    /// Années jusqu'à la maturité.
    pub maturity_years: f64,
    /// Se reproduit en captivité.
    pub captive_breeding: bool,
    /// Agressivité, de 0 à 1.
    pub aggression: f64,
    /// Tendance à la panique, de 0 à 1.
    pub panic: f64,
    /// Vit en troupeau hiérarchisé, de 0 (solitaire, territorial) à 1.
    pub herd: f64,
    /// Donne du lait en quantité (une espèce qui allaite longtemps).
    pub milk: bool,
}

impl WildSpecies {
    /// Domesticabilité d'un animal, de 0 à 1.
    pub fn tameness(&self) -> f64 {
        let WildKind::Animal(a) = &self.kind else { return 0.0 };
        if !a.captive_breeding || a.mass_kg < 1.0 {
            return 0.0;
        }
        let diet = a.herbivory.clamp(0.0, 1.0);
        let growth = (4.0 / a.maturity_years.max(0.5)).min(1.0);
        let temper = (1.0 - a.aggression).clamp(0.0, 1.0) * (1.0 - a.panic).clamp(0.0, 1.0);
        let herd = a.herd.clamp(0.0, 1.0);
        (diet * growth * temper * herd).sqrt()
    }

    /// Cultivabilité d'une plante, de 0 à 1.
    pub fn cultivability(&self) -> f64 {
        let WildKind::Plant(p) = &self.kind else { return 0.0 };
        let base = match p.organ {
            PlantOrgan::Seed => {
                let seed = (p.seed_mg / 30.0).min(1.0);
                seed * if p.annual { 1.0 } else { 0.4 } * if p.selfing { 1.0 } else { 0.6 }
            }
            PlantOrgan::Root => 0.7,
            PlantOrgan::Fruit => 0.45,
        };
        base * (p.wild_yield_t_ha / 0.5).min(1.0).sqrt()
    }

    /// Bien que donne l'espèce une fois domestiquée.
    pub fn good(&self) -> Good {
        match &self.kind {
            WildKind::Plant(p) => match p.organ {
                PlantOrgan::Seed => Good::Grain,
                _ => Good::Roots,
            },
            WildKind::Animal(_) => Good::Livestock,
        }
    }

    /// Climat convenable pour une plante (1 au mieux).
    pub fn suits(&self, temp_c: f64, rain_mm: f64) -> f64 {
        match &self.kind {
            WildKind::Plant(p) => {
                let dt = (temp_c - p.temp_c) / 8.0;
                let dr = (rain_mm.max(50.0) / p.rain_mm.max(50.0)).dln() / 0.9;
                (-(dt * dt + dr * dr)).dexp()
            }
            WildKind::Animal(a) => {
                // Les grands animaux supportent mal les tropiques humides
                // (maladies, mouches) et le froid extrême.
                let t = (temp_c - 12.0) / 15.0;
                let size = (a.mass_kg / 300.0).min(1.0);
                (-(t * t) * (0.5 + size)).dexp()
            }
        }
    }
}

/// Le milieu d'une cellule.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Land {
    pub area_km2: f64,
    pub lat_deg: f64,
    pub lon_deg: f64,
    pub milieu: Milieu,
    /// Cellule d'océan (non habitable par une espèce terrestre).
    pub ocean: bool,
    pub coast: bool,
    pub temp_c: f64,
    pub rain_mm: f64,
    /// Variabilité du climat d'une année à l'autre, de 0 (stable) à 1 (climat
    /// glaciaire, oscillations rapides) : l'agriculture demande un climat
    /// stable (Richerson, Boyd et Bettinger, 2001).
    pub variability: f64,
    /// Relief, de 0 (plaine) à 1 (haute montagne).
    pub relief: f64,
    /// Rivière : part des terres irrigables, de 0 à 1.
    pub river: f64,
    /// Part des terres labourables et des pâtures.
    pub arable: f64,
    pub pasture: f64,
    /// Carbone des forêts au départ, t C par km².
    pub forest_c_km2: f64,
    pub deposits: Deposits,
    /// Vent et soleil exploitables, de 0 à 1.
    pub wind: f64,
    pub sun: f64,
    /// Sources chaudes (géothermie).
    pub geothermal: f64,
    /// Oxygène de l'air, %.
    pub o2_percent: f64,
    /// Ciel visible (astronomie et calendriers).
    pub sky: bool,
    /// Espèces sauvages de la cellule.
    pub wild: Vec<u32>,
}

impl Land {
    /// Production primaire nette, kg C par m² et par an, par le modèle de
    /// Miami (Lieth, 1975) : la plus faible des limites par la chaleur et
    /// par la pluie.
    pub fn npp(&self) -> f64 {
        if self.ocean {
            // Production marine, plus forte près des côtes (Field et al.,
            // 1998 : environ 0,14 kg C par m² et par an en moyenne).
            return if self.coast { 0.25 } else { 0.12 };
        }
        let t = 3000.0 / (1.0 + (1.315 - 0.119 * self.temp_c).dexp());
        let p = 3000.0 * (1.0 - (-0.000664 * self.rain_mm).dexp());
        // g de matière sèche par m² → kg C (45 % de carbone).
        t.min(p) * 0.45 / 1000.0
    }

    /// Le feu brûle-t-il ici ?
    pub fn fire(&self) -> bool {
        self.milieu != Milieu::Water && self.o2_percent >= evo_culture::catalog::FIRE_O2_PERCENT
    }
}
