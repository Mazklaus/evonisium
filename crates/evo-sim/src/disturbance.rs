//! Perturbations du monde multicellulaire (étape 4, document Fonctionnalités,
//! « Interventions ») : impact météoritique, isolement d'un groupe, poussée
//! climatique. Elles passent par la file d'ordres et l'influence comme les
//! autres interventions, et agissent sur l'environnement par les mêmes
//! grandeurs que les phénomènes naturels, jamais sur les gènes.
//!
//! - L'**impact** tue une part de la biomasse dans son rayon de dévastation
//!   (sa matière retourne à l'eau, comme toute mort), libère le CO₂ des
//!   roches carbonatées touchées et refroidit la planète le temps que les
//!   poussières retombent.
//! - La **barrière** (bras de mer ou chaîne de montagnes locale) est un arc
//!   de grand cercle que les migrations ne franchissent plus pendant sa
//!   durée : c'est l'isolement géographique qui précède la spéciation.
//! - La **poussée climatique** est une anomalie de température et de pluie
//!   sur une région, pendant une durée donnée.
//!
//! Simplification signalée : le climat du moteur est à l'équilibre à chaque
//! pas. Une anomalie plus courte que le pas est donc appliquée au prorata de
//! sa durée (un hiver d'impact de 10 ans pèse 1/1000 d'un pas de 10 ka).

use evo_core::math::Det;
use evo_planet::{BioGrid, CellEnvironment};

/// Vitesse d'arrivée typique d'un astéroïde, m·s⁻¹.
const IMPACT_SPEED: f64 = 20_000.0;
/// Masse volumique d'un astéroïde rocheux, kg·m⁻³.
const IMPACTOR_DENSITY: f64 = 3000.0;
/// Énergie de l'impact de Chicxulub (bolide de 10 km), J.
pub const CHICXULUB_J: f64 = 3.1e23;
/// Rayon de dévastation de Chicxulub, m : incendies, souffle et retombées
/// tuent l'essentiel de la vie exposée à environ 1 500 km.
const CHICXULUB_KILL_RADIUS_M: f64 = 1.5e6;
/// CO₂ libéré par Chicxulub (cible carbonatée), mol : environ 425 Gt.
const CHICXULUB_CO2_MOL: f64 = 1e16;
/// Refroidissement global de Chicxulub, K, et durée de l'hiver d'impact.
const CHICXULUB_COOLING_K: f64 = 10.0;
const IMPACT_WINTER_YEARS: f64 = 10.0;

/// Énergie cinétique d'un bolide de `diameter_km`, J.
pub fn impact_energy(diameter_km: f64) -> f64 {
    let d = diameter_km.max(0.0) * 1e3;
    let mass = IMPACTOR_DENSITY * std::f64::consts::PI / 6.0 * d * d * d;
    0.5 * mass * IMPACT_SPEED * IMPACT_SPEED
}

/// Rayon de dévastation, m : il croît comme la racine cubique de
/// l'énergie (portée d'un souffle).
pub fn kill_radius_m(diameter_km: f64) -> f64 {
    CHICXULUB_KILL_RADIUS_M * (impact_energy(diameter_km) / CHICXULUB_J).dpowf(1.0 / 3.0)
}

/// CO₂ libéré, mol, pour une part `carbonate` (0 à 1) de roche carbonatée
/// dans la cible.
pub fn impact_co2(diameter_km: f64, carbonate: f64) -> f64 {
    CHICXULUB_CO2_MOL * impact_energy(diameter_km) / CHICXULUB_J * carbonate.clamp(0.0, 1.0)
}

/// Refroidissement global de l'hiver d'impact, K (poussières et soufre ;
/// plafonné, l'atmosphère ne s'assombrit pas au-delà du noir).
pub fn impact_cooling_k(diameter_km: f64) -> f64 {
    (CHICXULUB_COOLING_K * (impact_energy(diameter_km) / CHICXULUB_J).dpowf(0.5)).min(30.0)
}

/// Durée de l'hiver d'impact, années.
pub fn impact_winter_years(diameter_km: f64) -> f64 {
    IMPACT_WINTER_YEARS * (impact_energy(diameter_km) / CHICXULUB_J).dpowf(0.25).max(0.1)
}

/// Part de la biomasse tuée à la distance `r` du point d'impact, pour un
/// rayon de dévastation `radius` (même unité) : totale au centre, nulle au
/// bord, décroissance en 1 − (r/R)².
pub fn kill_fraction(r: f64, radius: f64) -> f64 {
    if radius <= 0.0 || r >= radius {
        return 0.0;
    }
    let x = r / radius;
    (1.0 - x * x).clamp(0.0, 1.0)
}

/// Distance angulaire entre deux points de la sphère unité, rad.
pub fn angle(a: [f64; 3], b: [f64; 3]) -> f64 {
    (a[0] * b[0] + a[1] * b[1] + a[2] * b[2]).clamp(-1.0, 1.0).dacos()
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn normalise(a: [f64; 3]) -> [f64; 3] {
    let n = dot(a, a).sqrt();
    if n == 0.0 {
        return a;
    }
    [a[0] / n, a[1] / n, a[2] / n]
}

/// Barrière temporaire : un arc de grand cercle centré sur `center`,
/// orienté selon `azimuth_rad` (0 : nord-sud, π/2 : est-ouest), long de
/// `half_angle` de part et d'autre du centre.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Barrier {
    pub center: [f64; 3],
    /// Normale du plan du grand cercle qui porte l'arc.
    pub normal: [f64; 3],
    pub half_angle: f64,
    pub until_years: f64,
    /// Bras de mer (vrai) ou chaîne de montagnes (faux), pour l'affichage.
    pub sea: bool,
    /// Ordre qui l'a créée.
    pub order: u64,
}

impl Barrier {
    pub fn new(center: [f64; 3], azimuth_rad: f64, half_angle: f64, until_years: f64, sea: bool, order: u64) -> Self {
        let c = normalise(center);
        // Repère local : est et nord en `c`.
        let up = if c[2].abs() > 0.999 { [1.0, 0.0, 0.0] } else { [0.0, 0.0, 1.0] };
        let east = normalise(cross(up, c));
        let north = cross(c, east);
        // Direction de l'arc dans le plan tangent ; la normale du grand
        // cercle est perpendiculaire à `c` et à cette direction.
        let (s, co) = azimuth_rad.dsin_cos();
        let dir = [co * north[0] + s * east[0], co * north[1] + s * east[1], co * north[2] + s * east[2]];
        Self { center: c, normal: normalise(cross(c, dir)), half_angle, until_years, sea, order }
    }

    /// Vrai si le segment de `a` à `b` (voisins sur la grille) traverse
    /// l'arc.
    pub fn blocks(&self, a: [f64; 3], b: [f64; 3]) -> bool {
        let (sa, sb) = (dot(a, self.normal), dot(b, self.normal));
        if (sa > 0.0) == (sb > 0.0) {
            return false;
        }
        // Point de passage sur le grand cercle, entre a et b.
        let t = sa / (sa - sb);
        let p = normalise([a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1]), a[2] + t * (b[2] - a[2])]);
        angle(p, self.center) <= self.half_angle
    }

    /// Points de l'arc (pour le dessin), du premier bout au second.
    pub fn polyline(&self, n: usize) -> Vec<[f64; 3]> {
        let along = cross(self.normal, self.center);
        (0..=n.max(1))
            .map(|i| {
                let a = -self.half_angle + 2.0 * self.half_angle * i as f64 / n.max(1) as f64;
                let (s, c) = a.dsin_cos();
                normalise([c * self.center[0] + s * along[0], c * self.center[1] + s * along[1], c * self.center[2] + s * along[2]])
            })
            .collect()
    }
}

/// Anomalie climatique : sur une calotte (`radius` rad, ou toute la planète
/// si `global`), entre deux dates.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ClimateAnomaly {
    pub center: [f64; 3],
    pub radius: f64,
    pub global: bool,
    pub delta_k: f64,
    /// Facteur multiplicatif de la pluie (1 : inchangée).
    pub rain_factor: f64,
    pub from_years: f64,
    pub until_years: f64,
    pub order: u64,
}

impl ClimateAnomaly {
    /// Part du pas `[t0, t1)` couverte par l'anomalie.
    pub fn overlap(&self, t0: f64, t1: f64) -> f64 {
        let span = (t1 - t0).max(1e-9);
        ((t1.min(self.until_years) - t0.max(self.from_years)).max(0.0) / span).clamp(0.0, 1.0)
    }

    /// Poids de l'anomalie en un point : 1 au centre, adouci vers le bord
    /// (le dernier quart du rayon).
    pub fn weight_at(&self, p: [f64; 3]) -> f64 {
        if self.global {
            return 1.0;
        }
        let a = angle(p, self.center);
        if a >= self.radius {
            0.0
        } else if a <= 0.75 * self.radius {
            1.0
        } else {
            (self.radius - a) / (0.25 * self.radius)
        }
    }
}

/// Perturbations actives : partie de l'état simulé (sauvegardée, rejouée).
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Disturbances {
    pub barriers: Vec<Barrier>,
    pub anomalies: Vec<ClimateAnomaly>,
    /// Biomasse tuée par les impacts depuis le début, mol C.
    pub killed_biomass: f64,
}

impl Disturbances {
    pub fn is_empty(&self) -> bool {
        self.barriers.is_empty() && self.anomalies.is_empty()
    }

    /// Retire ce qui a expiré à la date `years`.
    pub fn expire(&mut self, years: f64) {
        self.barriers.retain(|b| b.until_years > years);
        self.anomalies.retain(|a| a.until_years > years);
    }

    /// Vrai si une barrière active sépare deux cellules voisines.
    pub fn blocked(&self, a: [f64; 3], b: [f64; 3]) -> bool {
        self.barriers.iter().any(|w| w.blocks(a, b))
    }

    /// Applique les anomalies du pas `[t0, t1)` aux conditions des cellules
    /// du vivant (après leur agrégation depuis la planète).
    pub fn apply_to(&self, bio: &mut BioGrid, t0: f64, t1: f64) {
        if self.anomalies.is_empty() {
            return;
        }
        let centers = &bio.grid.centers;
        for (c, env) in bio.env.iter_mut().enumerate() {
            for a in &self.anomalies {
                let w = a.overlap(t0, t1) * a.weight_at(centers[c]);
                if w > 0.0 {
                    shift(env, a.delta_k * w, 1.0 + (a.rain_factor - 1.0) * w);
                }
            }
        }
    }
}

fn shift(env: &mut CellEnvironment, delta_k: f64, rain: f64) {
    env.temperature_k = (env.temperature_k + delta_k).max(150.0);
    env.rain_mm_yr = (env.rain_mm_yr * rain.max(0.0)).max(0.0);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chicxulub_scale_is_recovered() {
        let e = impact_energy(10.0);
        assert!((e / CHICXULUB_J - 1.0).abs() < 0.05, "{e:e}");
        assert!((kill_radius_m(10.0) / 1.5e6 - 1.0).abs() < 0.05);
        // Un bolide de 1 km : dix fois moins loin.
        assert!((kill_radius_m(1.0) / 1.5e5 - 1.0).abs() < 0.05);
        assert_eq!(kill_fraction(0.0, 1.0), 1.0);
        assert_eq!(kill_fraction(1.0, 1.0), 0.0);
    }

    #[test]
    fn a_north_south_barrier_blocks_east_west_moves_only() {
        let c = [1.0, 0.0, 0.0];
        let b = Barrier::new(c, 0.0, 0.2, 1e6, true, 0);
        let west = normalise([1.0, -0.05, 0.0]);
        let east = normalise([1.0, 0.05, 0.0]);
        let north = normalise([1.0, 0.0, 0.05]);
        let far_north = normalise([1.0, -0.05, 0.5]);
        let far_north_e = normalise([1.0, 0.05, 0.5]);
        assert!(b.blocks(west, east));
        assert!(!b.blocks(c, north));
        // Au-delà du bout de l'arc, on passe.
        assert!(!b.blocks(far_north, far_north_e));
        let line = b.polyline(8);
        assert_eq!(line.len(), 9);
        assert!(line.iter().all(|p| dot(*p, b.normal).abs() < 1e-9));
    }

    #[test]
    fn anomalies_weigh_by_overlap_and_distance() {
        let a = ClimateAnomaly {
            center: [0.0, 0.0, 1.0],
            radius: 0.5,
            global: false,
            delta_k: -4.0,
            rain_factor: 0.5,
            from_years: 100.0,
            until_years: 200.0,
            order: 0,
        };
        assert_eq!(a.overlap(0.0, 100.0), 0.0);
        assert!((a.overlap(150.0, 250.0) - 0.5).abs() < 1e-12);
        assert_eq!(a.weight_at([0.0, 0.0, 1.0]), 1.0);
        assert_eq!(a.weight_at([1.0, 0.0, 0.0]), 0.0);
        let mut d = Disturbances { anomalies: vec![a], ..Default::default() };
        d.expire(199.0);
        assert_eq!(d.anomalies.len(), 1);
        d.expire(200.0);
        assert!(d.is_empty());
    }
}
