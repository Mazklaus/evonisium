//! Colonne stratigraphique (document Fonctionnalités, « Outils
//! d'observation » : que retiendront les roches ?).
//!
//! La colonne d'un lieu empile, de la plus ancienne à la plus récente, les
//! couches que la partie y a déposées. Chaque couche couvre une tranche de
//! temps ; elle est tirée de deux historiques que le moteur garde déjà :
//!
//! - l'historique de la **région** (mer ou terre, température, oxygène des
//!   eaux, biomasse, espèce dominante) donne la roche et les fossiles ;
//! - les **sédiments globaux** (carbonates, carbone organique, oxydes de fer
//!   enfouis depuis le début) donnent la part du carbone enfoui sous forme
//!   organique, donc la signature isotopique δ¹³C des carbonates, et les
//!   époques de fer rubané.
//!
//! Les impacts du joueur laissent une couche à iridium, les glaciations une
//! tillite. Simplifications signalées : la roche est déduite de conditions
//! moyennes (pas de transport de sédiments simulé), l'épaisseur vient d'un
//! taux de sédimentation type (marin 20 m/Ma, continental 5 m/Ma), et le
//! δ¹³C suit le bilan de masse le plus simple, δ = δ_entrée + ε·f_org avec
//! δ_entrée = −5 ‰ et ε = 25 ‰ (Kump et Arthur, 1999).

use crate::query::RegionSample;

/// Sédiments enfouis à une date (cumuls depuis le début), mol.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SedimentSample {
    pub years: f64,
    pub carbonate_c: f64,
    pub organic_c: f64,
    pub iron_oxides: f64,
    /// Fraction molaire d'O₂ de l'atmosphère.
    pub o2_mixing: f64,
    /// Glace : part de la surface.
    pub ice_fraction: f64,
}

/// Roche d'une couche.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rock {
    /// Calcaire de mer chaude, construit en partie par des tapis microbiens
    /// quand la vie est là.
    Limestone,
    /// Marnes et argiles de mer froide ou profonde.
    Marl,
    /// Schistes noirs : mer sans oxygène où la matière organique s'enfouit.
    BlackShale,
    /// Fer rubané : le fer dissous des mers anoxiques précipite.
    BandedIron,
    /// Grès continental.
    Sandstone,
    /// Grès rouges : des terres oxydées, signe d'oxygène dans l'air.
    RedBeds,
    /// Tillite : dépôt glaciaire.
    Tillite,
}

impl Rock {
    pub fn key(self) -> &'static str {
        match self {
            Rock::Limestone => "calcaire",
            Rock::Marl => "marnes",
            Rock::BlackShale => "schistes-noirs",
            Rock::BandedIron => "fer-rubane",
            Rock::Sandstone => "gres",
            Rock::RedBeds => "gres-rouges",
            Rock::Tillite => "tillite",
        }
    }

    pub fn marine(self) -> bool {
        matches!(self, Rock::Limestone | Rock::Marl | Rock::BlackShale | Rock::BandedIron)
    }
}

/// Une couche de la colonne.
#[derive(Clone, Debug, PartialEq)]
pub struct Stratum {
    pub from_years: f64,
    pub to_years: f64,
    pub rock: Rock,
    /// Épaisseur estimée, m.
    pub thickness_m: f64,
    /// δ¹³C des carbonates, ‰ (NaN sans enfouissement mesurable).
    pub delta13c: f64,
    /// Fossiles : stromatolithes et traces de tapis microbiens.
    pub stromatolites: bool,
    /// Espèce dominante de la région dans la tranche (0 sans vie).
    pub dominant_species: u32,
    pub species_count: u32,
    /// Couche à iridium (impact pendant la tranche).
    pub impact: bool,
}

/// Taux de sédimentation types, m par an.
const MARINE_RATE: f64 = 20e-6;
const CONTINENTAL_RATE: f64 = 5e-6;
/// Bilan isotopique du carbone (Kump et Arthur, 1999).
const DELTA_IN: f64 = -5.0;
const EPSILON: f64 = 25.0;
/// Seuil d'O₂ atmosphérique des grès rouges (~1 % du niveau actuel).
const RED_BEDS_O2: f64 = 2e-3;
/// O₂ dissous sous lequel une mer est anoxique, mol·m⁻³.
const ANOXIC: f64 = 1e-3;

fn interpolate(s: &[SedimentSample], years: f64) -> SedimentSample {
    match s.iter().position(|x| x.years >= years) {
        None => s.last().copied().unwrap_or_default(),
        Some(0) => s[0],
        Some(i) => {
            let (a, b) = (s[i - 1], s[i]);
            let t = ((years - a.years) / (b.years - a.years).max(1e-9)).clamp(0.0, 1.0);
            let l = |x: f64, y: f64| x + t * (y - x);
            SedimentSample {
                years,
                carbonate_c: l(a.carbonate_c, b.carbonate_c),
                organic_c: l(a.organic_c, b.organic_c),
                iron_oxides: l(a.iron_oxides, b.iron_oxides),
                o2_mixing: l(a.o2_mixing, b.o2_mixing),
                ice_fraction: l(a.ice_fraction, b.ice_fraction),
            }
        }
    }
}

/// Colonne d'une région en `layers` tranches de temps égales au plus.
/// `region` : historique de la région, par date ; `sediments` : cumuls
/// globaux, par date ; `impacts` : dates des impacts touchant la région.
pub fn column(region: &[RegionSample], sediments: &[SedimentSample], impacts: &[f64], layers: usize) -> Vec<Stratum> {
    let (Some(first), Some(last)) = (region.first(), region.last()) else { return Vec::new() };
    let (t0, t1) = (first.years, last.years);
    if t1 <= t0 {
        return Vec::new();
    }
    let n = layers.clamp(1, 200);
    let span = (t1 - t0) / n as f64;
    // Rythme moyen du fer rubané sur la partie, pour juger d'une époque.
    let mut out: Vec<Stratum> = Vec::with_capacity(n);
    for k in 0..n {
        let (a, b) = (t0 + k as f64 * span, t0 + (k + 1) as f64 * span);
        let inside: Vec<&RegionSample> = region.iter().filter(|r| r.years > a && r.years <= b).collect();
        let samples: Vec<&RegionSample> = if inside.is_empty() {
            region.iter().min_by(|x, y| (x.years - b).abs().total_cmp(&(y.years - b).abs())).into_iter().collect()
        } else {
            inside
        };
        let m = samples.len() as f64;
        let ocean = samples.iter().map(|r| r.ocean_fraction).sum::<f64>() / m;
        let temp = samples.iter().map(|r| r.temperature_k).sum::<f64>() / m;
        let oxygen = samples.iter().map(|r| r.oxygen).sum::<f64>() / m;
        let biomass = samples.iter().map(|r| r.biomass).sum::<f64>() / m;
        let dominant = samples.iter().max_by(|x, y| x.biomass.total_cmp(&y.biomass)).map_or(0, |r| r.dominant_species);
        let species_count = samples.iter().map(|r| r.species_count).max().unwrap_or(0);
        let (sa, sb) = (interpolate(sediments, a), interpolate(sediments, b));
        let d_carb = (sb.carbonate_c - sa.carbonate_c).max(0.0);
        let d_org = (sb.organic_c - sa.organic_c).max(0.0);
        let d_iron = (sb.iron_oxides - sa.iron_oxides).max(0.0);
        let f_org = if d_carb + d_org > 0.0 { d_org / (d_carb + d_org) } else { f64::NAN };
        let ice = 0.5 * (sa.ice_fraction + sb.ice_fraction);
        let o2 = 0.5 * (sa.o2_mixing + sb.o2_mixing);
        let marine = ocean >= 0.5;
        let rock = if temp < 263.0 || ice > 0.6 {
            Rock::Tillite
        } else if !marine {
            if o2 >= RED_BEDS_O2 {
                Rock::RedBeds
            } else {
                Rock::Sandstone
            }
        } else if oxygen < ANOXIC && d_iron > 0.05 * (d_carb + d_org).max(1.0) {
            Rock::BandedIron
        } else if oxygen < ANOXIC && f_org > 0.3 {
            Rock::BlackShale
        } else if temp > 288.0 {
            Rock::Limestone
        } else {
            Rock::Marl
        };
        let rate = if rock.marine() { MARINE_RATE } else { CONTINENTAL_RATE };
        out.push(Stratum {
            from_years: a,
            to_years: b,
            rock,
            thickness_m: rate * (b - a),
            delta13c: if f_org.is_finite() { DELTA_IN + EPSILON * f_org } else { f64::NAN },
            stromatolites: marine && biomass > 0.0 && matches!(rock, Rock::Limestone | Rock::Marl),
            dominant_species: dominant,
            species_count,
            impact: impacts.iter().any(|&y| y > a && y <= b),
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn region(years: f64, ocean: f64, temp: f64, oxygen: f64, biomass: f64) -> RegionSample {
        RegionSample {
            years,
            region: 0,
            temperature_k: temp,
            ocean_fraction: ocean,
            oxygen,
            biomass,
            dominant_species: if biomass > 0.0 { 7 } else { 0 },
            species_count: u32::from(biomass > 0.0),
        }
    }

    #[test]
    fn rocks_follow_conditions_and_isotopes_follow_burial() {
        // Une mer chaude, d'abord sans vie ni oxygène (fer rubané), puis
        // vivante et oxygénée (calcaire à stromatolithes), puis une terre
        // émergée sous un air oxygéné (grès rouges).
        let region: Vec<RegionSample> = (0..=30)
            .map(|i| {
                let y = i as f64 * 1e6;
                match i {
                    0..=10 => region(y, 1.0, 295.0, 0.0, 0.0),
                    11..=20 => region(y, 1.0, 295.0, 0.2, 1e9),
                    _ => region(y, 0.0, 290.0, 0.0, 1e8),
                }
            })
            .collect();
        let sediments: Vec<SedimentSample> = (0..=30)
            .map(|i| {
                let y = i as f64 * 1e6;
                let k = i as f64;
                SedimentSample {
                    years: y,
                    carbonate_c: 8e18 * k,
                    organic_c: 2e18 * k,
                    iron_oxides: if i <= 10 { 1e18 * k } else { 1e19 },
                    o2_mixing: if i <= 10 { 0.0 } else { 0.05 },
                    ice_fraction: 0.0,
                }
            })
            .collect();
        let col = column(&region, &sediments, &[15.5e6], 3);
        assert_eq!(col.len(), 3);
        assert_eq!(col[0].rock, Rock::BandedIron);
        assert_eq!(col[1].rock, Rock::Limestone);
        assert!(col[1].stromatolites && col[1].impact && !col[0].impact);
        assert_eq!(col[2].rock, Rock::RedBeds);
        // f_org = 0,2 : δ¹³C = 0 ‰, comme les carbonates terrestres.
        assert!(col[1].delta13c.abs() < 1e-9, "{}", col[1].delta13c);
        assert!((col[0].thickness_m - 200.0).abs() < 1.0);
        assert!(col[2].thickness_m < col[1].thickness_m);
    }
}
