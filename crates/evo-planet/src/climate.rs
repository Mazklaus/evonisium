//! Climat calculé à l'équilibre à chaque pas (document Planète, section 2,
//! « Seuil entre climat à l'équilibre et climat progressif ») : bilan
//! d'énergie de type Budyko-Sellers par cellule, effet de serre du CO₂ et du
//! CH₄, rétroaction glace-albédo, écran d'ozone contre les UV.
//!
//! Conditions du seuil respectées :
//! - mémoire des glaciations : le calcul part de l'état de glace du pas
//!   précédent ; il peut donc rester en boule de neige pour un CO₂ qui
//!   suffirait à une planète libre (deux équilibres) ;
//! - cycles orbitaux lissés : l'ensoleillement est la moyenne annuelle ;
//!   l'obliquité d'un monde sans lune dérive lentement d'un pas à l'autre ;
//! - les cycles chimiques globaux tournent en sous-pas (module `geochem`).
//!
//! [Simplification] Pas de circulation ni de précipitations ; un seul albédo
//! pour toute surface libre de glace (nuages compris) ; gradient thermique
//! fixe sur les reliefs ; mers gelées à partir de la même température seuil
//! que les terres.

use crate::grid::GeodesicGrid;
use crate::params::PlanetParams;
use evo_core::math::Det;

/// Polynôme de Legendre d'ordre 2.
#[inline]
fn p2(x: f64) -> f64 {
    0.5 * (3.0 * x * x - 1.0)
}

/// Ensoleillement moyen annuel au sommet de l'atmosphère, W·m⁻², selon la
/// latitude (approximation de North, 1975, avec s₂ déduit de l'obliquité ;
/// au-delà de 54° d'obliquité, les pôles reçoivent plus que l'équateur).
pub fn annual_insolation(flux: f64, obliquity: f64, latitude: f64) -> f64 {
    let s2 = -0.625 * p2(obliquity.dcos());
    (flux / 4.0 * (1.0 + s2 * p2(latitude.dsin()))).max(0.0)
}

/// Part de l'énergie d'un corps noir de température `t` émise entre `a_nm`
/// et `b_nm`.
pub fn planck_fraction(t: f64, a_nm: f64, b_nm: f64) -> f64 {
    const C2: f64 = 1.438_777e7; // nm·K
    let b = |nm: f64| nm.powi(-5) / (C2 / (nm * t)).dexp_m1();
    let integrate = |lo: f64, hi: f64| {
        let n = 2000;
        let h = (hi - lo) / n as f64;
        (0..n).map(|i| b(lo + (i as f64 + 0.5) * h)).sum::<f64>() * h
    };
    integrate(a_nm, b_nm) / integrate(50.0, 1.0e5)
}

/// État du climat, gardé d'un pas à l'autre.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ClimateState {
    /// Cellules englacées.
    pub ice: Vec<bool>,
    pub obliquity_rad: f64,
    pub luminosity_w: f64,
    /// Température moyenne de surface (niveau de la mer), K.
    pub mean_temperature_k: f64,
    pub ice_fraction: f64,
    /// Forçage de l'effet de serre (CO₂ + CH₄), W·m⁻².
    pub greenhouse_w_m2: f64,
    /// Facteur de transmission des UV par l'ozone (1 : pas d'ozone).
    pub uv_transmission: f64,
    /// Parts de l'énergie stellaire dans les UV nocifs (< 320 nm) et dans la
    /// lumière utilisable par des pigments (350 à 1100 nm).
    pub uv_share: f64,
    pub light_share: f64,
}

/// Composition de l'atmosphère utile au climat.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Greenhouse {
    pub co2_pa: f64,
    pub ch4_ppb: f64,
    pub o2_mixing: f64,
}

/// Sorties du climat par cellule.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CellClimate {
    pub temperature_k: f64,
    pub insolation_w_m2: f64,
    pub absorbed_w_m2: f64,
    pub ice: bool,
}

impl ClimateState {
    pub fn new(params: &PlanetParams, cells: usize) -> Self {
        Self {
            ice: vec![false; cells],
            obliquity_rad: params.obliquity_rad,
            luminosity_w: params.star_luminosity_w,
            mean_temperature_k: f64::NAN,
            ice_fraction: 0.0,
            greenhouse_w_m2: 0.0,
            uv_transmission: 1.0,
            uv_share: planck_fraction(params.star_temperature_k, 100.0, 320.0),
            light_share: planck_fraction(params.star_temperature_k, 350.0, 1100.0),
        }
    }

    /// Forçage radiatif de l'effet de serre, W·m⁻².
    pub fn greenhouse_forcing(params: &PlanetParams, g: &Greenhouse) -> f64 {
        let co2 = params.co2_forcing * (g.co2_pa.max(1e-3) / params.co2_reference_pa).dln()
            + params.co2_broadening_w_m2 * (g.co2_pa.max(0.0) / 1e5).sqrt();
        // Au-delà d'un rapport CH₄/CO₂ critique, la brume organique plafonne
        // l'effet du méthane.
        let co2_ppb = g.co2_pa / 101_325.0 * 1e9;
        let ch4_ppb = g.ch4_ppb.min(params.haze_ch4_co2_ratio * co2_ppb).max(0.0);
        // La loi en racine carrée ne vaut qu'aux faibles teneurs ; au-delà de
        // 10 ppm, le forçage ne croît plus que comme le logarithme (bandes
        // saturées), raccordé avec la même pente.
        const CH4_SATURATION_PPB: f64 = 1e4;
        let sqrt_part = |x: f64| params.ch4_forcing * (x.sqrt() - params.ch4_reference_ppb.sqrt());
        let ch4 = if ch4_ppb <= CH4_SATURATION_PPB {
            sqrt_part(ch4_ppb)
        } else {
            let slope = 0.5 * params.ch4_forcing * CH4_SATURATION_PPB.sqrt();
            sqrt_part(CH4_SATURATION_PPB) + slope * (ch4_ppb / CH4_SATURATION_PPB).dln()
        };
        co2 + ch4
    }

    /// Transmission des UV par l'ozone formé à partir de l'O₂.
    pub fn uv_transmission(params: &PlanetParams, o2_mixing: f64) -> f64 {
        1.0 / (1.0 + params.ozone_strength * (o2_mixing.max(0.0) / params.ozone_reference_mixing).sqrt())
    }

    /// Calcule le climat d'équilibre pour la composition `g`, en partant de
    /// l'état de glace courant. `elevation_above_sea` vaut 0 pour les
    /// cellules océaniques.
    pub fn solve(
        &mut self,
        params: &PlanetParams,
        grid: &GeodesicGrid,
        is_ocean: &[bool],
        elevation_above_sea: &[f64],
        years: f64,
        g: &Greenhouse,
    ) -> Vec<CellClimate> {
        let n = grid.len();
        self.luminosity_w = params.luminosity_at(years);
        let flux = self.luminosity_w / (4.0 * std::f64::consts::PI * params.orbit_m * params.orbit_m);
        self.greenhouse_w_m2 = Self::greenhouse_forcing(params, g);
        self.uv_transmission = Self::uv_transmission(params, g.o2_mixing);
        let a = params.olr_a - self.greenhouse_w_m2;
        let (b, c) = (params.olr_b, params.heat_transport);
        let total_area: f64 = grid.unit_areas.iter().sum();
        let q: Vec<f64> = (0..n).map(|i| annual_insolation(flux, self.obliquity_rad, grid.latitude(i))).collect();
        let lapse = params.lapse_rate();
        let mut out = vec![CellClimate::default(); n];
        // Le bilan d'énergie donne la température de la surface moyenne, pas
        // celle du niveau de la mer : le gradient thermique s'applique par
        // rapport à l'altitude moyenne de la surface (océans comptés au
        // niveau de la mer). Sur Terre, l'écart est de quelques centaines de
        // mètres ; sur un monde désertique, dont les mers dorment au fond des
        // bassins, toutes les terres seraient sinon des hauts plateaux gelés.
        let reference_m = (0..n).map(|i| elevation_above_sea[i].max(0.0) * grid.unit_areas[i]).sum::<f64>() / total_area;
        // Itération du point fixe glace ↔ température, depuis l'état précédent.
        for _ in 0..60 {
            let mean_absorbed: f64 = (0..n)
                .map(|i| q[i] * (1.0 - if self.ice[i] { params.albedo_ice } else { params.albedo }) * grid.unit_areas[i])
                .sum::<f64>()
                / total_area;
            // Températures en °C dans l'ajustement de l'OLR.
            let t_mean = (mean_absorbed - a) / b;
            let mut changed = false;
            for i in 0..n {
                let albedo = if self.ice[i] { params.albedo_ice } else { params.albedo };
                let absorbed = q[i] * (1.0 - albedo);
                let t_sea = 273.15 + (absorbed - a + c * t_mean) / (b + c);
                let t_local = t_sea - if is_ocean[i] { 0.0 } else { lapse * (elevation_above_sea[i].max(0.0) - reference_m) };
                let ice = t_local < params.ice_temperature_k;
                if ice != self.ice[i] {
                    changed = true;
                    self.ice[i] = ice;
                }
                out[i] = CellClimate {
                    // L'eau de mer gèle vers 271 K ; dessous, c'est la glace qui refroidit.
                    temperature_k: if is_ocean[i] { t_local.max(271.2) } else { t_local },
                    insolation_w_m2: q[i],
                    absorbed_w_m2: absorbed,
                    ice,
                };
            }
            self.mean_temperature_k = 273.15 + t_mean;
            if !changed {
                break;
            }
        }
        self.ice_fraction = (0..n).filter(|&i| self.ice[i]).map(|i| grid.unit_areas[i]).sum::<f64>() / total_area;
        out
    }

    /// Dérive chaotique de l'obliquité d'un monde sans lune massive.
    pub fn drift_obliquity(&mut self, params: &PlanetParams, dt_years: f64, gaussian: f64) {
        if params.obliquity_chaos_rad_per_sqrt_myr <= 0.0 {
            return;
        }
        let mut o = self.obliquity_rad + gaussian * params.obliquity_chaos_rad_per_sqrt_myr * (dt_years / 1e6).sqrt();
        let max = params.obliquity_max_rad;
        // Réflexion aux bornes.
        if o < 0.0 {
            o = -o;
        }
        if o > max {
            o = 2.0 * max - o;
        }
        self.obliquity_rad = o.clamp(0.0, max);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(params: &PlanetParams, state: &mut ClimateState, grid: &GeodesicGrid, co2_pa: f64) -> f64 {
        let n = grid.len();
        let g = Greenhouse { co2_pa, ch4_ppb: 0.0, o2_mixing: 0.0 };
        state.solve(params, grid, &vec![true; n], &vec![0.0; n], 0.0, &g);
        state.mean_temperature_k
    }

    #[test]
    fn planck_shares_of_a_sunlike_star() {
        let uv = planck_fraction(5772.0, 100.0, 320.0);
        assert!((0.01..0.05).contains(&uv), "{uv}");
        let light = planck_fraction(5772.0, 350.0, 1100.0);
        assert!((0.6..0.85).contains(&light), "{light}");
        assert!(planck_fraction(3200.0, 350.0, 1100.0) < light);
    }

    #[test]
    fn more_co2_is_warmer_and_snowball_has_memory() {
        let params = PlanetParams::earth_archean();
        let grid = GeodesicGrid::new(3);
        let mut s = ClimateState::new(&params, grid.len());
        let warm = run(&params, &mut s, &grid, 1e4);
        assert!((275.0..300.0).contains(&warm), "{warm}");
        assert!(s.ice_fraction < 0.3);
        let cold = run(&params, &mut s, &grid, 100.0);
        assert!(cold < warm);
        assert!(s.ice_fraction > 0.95, "boule de neige attendue, glace {}", s.ice_fraction);
        // Hystérésis : en revenant au CO₂ de départ, la planète reste gelée.
        let back = run(&params, &mut s, &grid, 1e4);
        assert!(s.ice_fraction > 0.95 && back < warm, "glace {}", s.ice_fraction);
        // Il faut bien plus de CO₂ pour en sortir.
        run(&params, &mut s, &grid, 2e5);
        assert!(s.ice_fraction < 0.5, "glace {}", s.ice_fraction);
    }

    #[test]
    fn ozone_cuts_uv_and_high_obliquity_warms_poles() {
        let params = PlanetParams::earth_archean();
        assert_eq!(ClimateState::uv_transmission(&params, 0.0), 1.0);
        assert!(ClimateState::uv_transmission(&params, 0.2) < 0.05);
        let f = 1361.0;
        assert!(annual_insolation(f, 0.4, 0.0) > annual_insolation(f, 0.4, 1.5));
        assert!(annual_insolation(f, 1.4, 0.0) < annual_insolation(f, 1.4, 1.5));
    }
}
