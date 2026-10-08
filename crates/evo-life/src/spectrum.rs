//! Lumière disponible selon la longueur d'onde, et couleur des pigments.
//!
//! Le spectre de l'étoile est celui d'un corps noir à sa température de
//! surface, compté en photons (c'est le nombre de photons qui fait le
//! rendement d'un photosystème), puis filtré par la couche d'eau où vivent
//! les microbes. Un pigment, défini par le pic de son spectre d'absorption,
//! reçoit la lumière à ce pic : sous une étoile comme le Soleil et dans l'eau,
//! l'optimum tombe dans le visible ; sous une étoile froide, il glisse vers le
//! rouge et l'infrarouge, comme les bactériochlorophylles réelles. Le même
//! paramètre donne la couleur affichée (document Génétique, format du plan de
//! construction ; document Organismes, « les pigments s'adaptent à l'étoile »).
//!
//! [Simplification] Bande d'absorption gaussienne unique de largeur fixe,
//! absorption de l'eau pure tabulée plus un terme de matière organique
//! dissoute ; pas de diffusion ni de physique quantique de la capture.

/// Constante de Planck × vitesse de la lumière / constante de Boltzmann, nm·K.
const HC_OVER_K_NM_K: f64 = 1.438_777e7;

/// Absorption de l'eau pure, m⁻¹ (ordres de grandeur de la littérature
/// d'optique marine, Pope et Fry 1997 ; Kou et coll. 1993 pour l'infrarouge).
const WATER_ABSORPTION: [(f64, f64); 13] = [
    (300.0, 0.05),
    (400.0, 0.007),
    (450.0, 0.009),
    (500.0, 0.026),
    (550.0, 0.064),
    (600.0, 0.24),
    (650.0, 0.35),
    (700.0, 0.65),
    (750.0, 2.6),
    (800.0, 2.0),
    (900.0, 6.8),
    (1000.0, 36.0),
    (1100.0, 20.0),
];

fn water_absorption(nm: f64) -> f64 {
    let w = &WATER_ABSORPTION;
    if nm <= w[0].0 {
        return w[0].1;
    }
    for pair in w.windows(2) {
        let ((x0, y0), (x1, y1)) = (pair[0], pair[1]);
        if nm <= x1 {
            // Interpolation en logarithme : l'absorption varie sur 4 ordres.
            let t = (nm - x0) / (x1 - x0);
            return (y0.ln() * (1.0 - t) + y1.ln() * t).exp();
        }
    }
    w[w.len() - 1].1
}

/// Densité spectrale relative de photons disponible, tabulée de 300 à
/// 1100 nm, normalisée à 1 au maximum.
#[derive(Clone, Debug, PartialEq)]
pub struct LightSpectrum {
    pub start_nm: f64,
    pub step_nm: f64,
    pub relative: Vec<f64>,
    /// Pic de la densité de photons disponible, nm.
    pub peak_nm: f64,
}

impl LightSpectrum {
    /// Spectre d'une étoile de température `star_k`, moyenné sur une couche
    /// d'eau de `water_depth_m` mètres (0 : en surface, hors de l'eau).
    pub fn new(star_k: f64, water_depth_m: f64) -> Self {
        let (start_nm, step_nm) = (300.0, 5.0);
        let n = ((1100.0 - start_nm) / step_nm) as usize + 1;
        let mut relative: Vec<f64> = (0..n)
            .map(|i| {
                let nm = start_nm + step_nm * i as f64;
                // Photons par intervalle de longueur d'onde : λ⁻⁴ / (e^(hc/λkT) − 1).
                let planck = nm.powi(-4) / ((HC_OVER_K_NM_K / (nm * star_k)).exp_m1());
                let k = water_absorption(nm) + 0.05 * (-0.015 * (nm - 440.0)).exp();
                let kh = k * water_depth_m;
                let mean_transmission = if kh < 1e-9 { 1.0 } else { -(-kh).exp_m1() / kh };
                planck * mean_transmission
            })
            .collect();
        let (imax, max) = relative.iter().cloned().enumerate().fold((0, 0.0), |a, (i, v)| if v > a.1 { (i, v) } else { a });
        for v in relative.iter_mut() {
            *v /= max;
        }
        Self { start_nm, step_nm, relative, peak_nm: start_nm + step_nm * imax as f64 }
    }

    /// Lumière reçue par un pigment dont le pic d'absorption est à `nm`,
    /// relative au meilleur pic possible sous cette étoile (de 0 à 1).
    pub fn match_at(&self, nm: f64) -> f64 {
        let x = ((nm - self.start_nm) / self.step_nm).clamp(0.0, (self.relative.len() - 1) as f64);
        let i = x.floor() as usize;
        let j = (i + 1).min(self.relative.len() - 1);
        let t = x - i as f64;
        self.relative[i] * (1.0 - t) + self.relative[j] * t
    }
}

impl Default for LightSpectrum {
    /// Étoile de type solaire, couche d'eau de 50 m (tests unitaires).
    fn default() -> Self {
        Self::new(5772.0, 50.0)
    }
}

/// Couleur, en RVB, d'un organisme portant un pigment dont le pic
/// d'absorption est à `nm`, sous une lumière blanche : chaque canal est
/// atténué selon l'absorption à sa longueur d'onde. Un pigment qui absorbe
/// dans l'infrarouge paraît presque incolore.
pub fn pigment_colour(nm: f64) -> [u8; 3] {
    const WIDTH_NM: f64 = 40.0;
    let channel = |centre: f64| {
        let x = (centre - nm) / WIDTH_NM;
        let reflected = 1.0 - 0.85 * (-0.5 * x * x).exp();
        (reflected * 255.0).round().clamp(0.0, 255.0) as u8
    };
    [channel(610.0), channel(545.0), channel(450.0)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cool_stars_push_the_best_pigment_to_the_red() {
        let sun_air = LightSpectrum::new(5772.0, 0.0);
        let dwarf_air = LightSpectrum::new(3200.0, 0.0);
        // Pic de photons d'un corps noir : λ = hc / (3,92·k·T).
        assert!((sun_air.peak_nm - 636.0).abs() < 15.0, "{}", sun_air.peak_nm);
        assert!(dwarf_air.peak_nm > 1000.0);
        // Sous l'eau, l'infrarouge disparaît et l'optimum revient vers le vert-bleu.
        let sun_water = LightSpectrum::new(5772.0, 50.0);
        assert!(sun_water.peak_nm < 600.0, "{}", sun_water.peak_nm);
        assert!(sun_water.match_at(900.0) < 0.01);
        assert!((sun_water.match_at(sun_water.peak_nm) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn absorbing_red_looks_green_and_absorbing_blue_looks_orange() {
        let red_absorber = pigment_colour(660.0);
        assert!(red_absorber[1] > red_absorber[0]);
        let blue_absorber = pigment_colour(450.0);
        assert!(blue_absorber[0] > blue_absorber[2]);
    }
}
