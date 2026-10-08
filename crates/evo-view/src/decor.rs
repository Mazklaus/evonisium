//! Décor du milieu de vie d'une espèce, palier 1 (architecture, « Direction
//! artistique retenue et décor de milieu ») : le milieu physique dessiné
//! derrière la planche de l'espèce, relief, eau, ciel coloré par l'étoile et
//! tapis microbiens colorés par leurs pigments.
//!
//! Le décor ne demande aucune donnée simulée nouvelle : il lit le milieu
//! type de l'espèce (cellule où elle abonde, conditions moyennes sur son
//! aire, espèces dominantes qui partagent son milieu). Il est déterministe
//! (graine, espèce, date arrondie) et coûte moins de 200 ms en 1024 × 512.
//!
//! Le milieu type vient du service moteur `Query::SpeciesHabitat` ; pour une
//! espèce éteinte, le moteur le décrit à la date de son apogée.

use crate::frame::Frame;
use crate::palette::blackbody;
use evo_morph::canvas::{fbm, mix64, Canvas, DrawRng, INK, OCHRE, PAPER, WATER};

/// Milieu type d'une espèce, tel que le décor le lit.
#[derive(Clone, Debug, PartialEq)]
pub struct Habitat {
    pub species: u32,
    /// Cellule (du vivant) où l'espèce est la plus abondante.
    pub cell: usize,
    pub is_ocean: bool,
    /// Profondeur d'eau moyenne (m, 0 à terre) et altitude moyenne des terres.
    pub depth_m: f32,
    pub height_m: f32,
    pub temperature_k: f32,
    pub ice_cover: f32,
    pub light_w_m2: f32,
    pub vent: bool,
    /// Couleur de l'espèce (pigment) et des trois espèces de son milieu.
    pub pigment_rgb: Option<[u8; 3]>,
    pub neighbours: Vec<(u32, Option<[u8; 3]>)>,
    /// Densité du tapis : part de l'espèce dans la biomasse de son milieu, 0 à 1.
    pub density: f32,
    pub star_temperature_k: f64,
    /// Fractions de l'atmosphère qui teintent le ciel.
    pub o2_mixing: f64,
    pub ch4_ppb: f64,
    pub years: f64,
}

/// Profondeur prêtée à un lac (m) : le moteur ne publie pas encore la
/// profondeur des eaux douces. [Simplification]
const LAKE_DEPTH_M: f32 = 20.0;

impl Habitat {
    /// Décor à partir du milieu type publié par le moteur ; l'état publié
    /// fournit les pigments des espèces, la glace et l'atmosphère.
    pub fn from_engine(h: &evo_engine::Habitat, frame: &Frame) -> Habitat {
        let pigment = |id: u32| frame.species(id).and_then(|s| s.pigment_rgb);
        let cell = h.peak_bio_cell as usize;
        let ice_cover = frame.cells().get(cell).map(|c| c.ice_cover).unwrap_or(0.0);
        let is_ocean = h.sea_share + h.fresh_share >= 0.5;
        let depth_m = if !is_ocean {
            0.0
        } else if h.sea_share >= h.fresh_share {
            (-h.elevation_m).max(1.0) as f32
        } else {
            LAKE_DEPTH_M
        };
        let shared: f64 = h.companions.iter().map(|c| c.2).sum();
        Habitat {
            species: h.species,
            cell,
            is_ocean,
            depth_m,
            height_m: h.elevation_m.max(0.0) as f32,
            temperature_k: h.temperature_k as f32,
            ice_cover,
            light_w_m2: h.light_w_m2 as f32,
            vent: h.vent_share >= 0.5,
            pigment_rgb: pigment(h.species),
            neighbours: h.companions.iter().take(3).map(|c| (c.0, pigment(c.0))).collect(),
            density: (1.0 - shared).clamp(0.2, 1.0) as f32,
            star_temperature_k: h.star_temperature_k,
            o2_mixing: frame.state.globals.o2_mixing,
            ch4_ppb: frame.state.globals.ch4_ppb,
            years: h.years,
        }
    }

    /// Graine du décor : il ne change que si l'espèce change de milieu.
    pub fn decor_seed(&self, game_seed: u64) -> u64 {
        let depth_band = (self.depth_m / 250.0) as u64;
        mix64(game_seed ^ mix64(self.species as u64) ^ (self.cell as u64) << 20 ^ depth_band << 48)
    }
}

/// Couleur du ciel : la lumière de l'étoile, voilée d'orangé par une brume
/// de méthane, bleuie par la diffusion d'une atmosphère oxygénée.
pub fn sky_colour(h: &Habitat) -> [f32; 3] {
    let star = blackbody(h.star_temperature_k);
    let haze = ((h.ch4_ppb / 5000.0).sqrt() as f32).clamp(0.0, 1.0);
    let blue = ((h.o2_mixing / 0.02).sqrt() as f32).clamp(0.0, 1.0);
    let base = [star[0] * 0.92, star[1] * 0.9, star[2] * 0.86];
    let orange = [0.93, 0.68, 0.42];
    let azure = [0.62, 0.74, 0.86];
    let mut c = base;
    for k in 0..3 {
        c[k] = c[k] + (orange[k] - c[k]) * haze * 0.6;
        c[k] = c[k] + (azure[k] - c[k]) * blue * 0.7;
        // Sur papier, le ciel reste un lavis clair.
        c[k] = PAPER[k] + (c[k] - PAPER[k]) * 0.55;
    }
    c
}

fn tint(c: [u8; 3]) -> [f32; 3] {
    [c[0] as f32 / 255.0, c[1] as f32 / 255.0, c[2] as f32 / 255.0]
}

/// Peint le décor.
pub fn paint(h: &Habitat, seed: u64, width: usize, height: usize) -> Canvas {
    let mut cv = Canvas::paper(width, height, seed);
    let (w, hh) = (width as f32, height as f32);
    let sky = sky_colour(h);
    let star = blackbody(h.star_temperature_k);
    let mut rng = DrawRng::new(seed);
    let horizon = if h.is_ocean { hh * 0.24 } else { hh * 0.55 };
    // Ciel : lavis dégradé, plus dense vers le haut.
    for y in 0..(horizon as usize) {
        let k = 0.55 - 0.35 * (y as f32 / horizon);
        for x in 0..width {
            let g = fbm(seed ^ 0x5C1, x as f32 / 90.0, y as f32 / 30.0);
            cv.wash(x, y, sky, k + 0.15 * (g - 0.5));
        }
    }
    // Disque de l'étoile, au lavis de sa couleur.
    let (sx, sy, sr) = (w * rng.range(0.65, 0.85), horizon * 0.35, hh * 0.05);
    for y in 0..(horizon as usize) {
        for x in 0..width {
            let d = (x as f32 - sx).hypot(y as f32 - sy);
            if d < sr * 3.0 {
                let a = if d < sr { 0.5 } else { 0.25 * (1.0 - (d - sr) / (sr * 2.0)) };
                cv.wash(x, y, [star[0], star[1] * 0.97, star[2] * 0.9], a.max(0.0));
            }
        }
    }
    if h.is_ocean {
        paint_sea(&mut cv, h, seed, horizon, star);
    } else {
        paint_land(&mut cv, h, seed, horizon);
    }
    cv.double_frame(8.0);
    cv
}

fn paint_sea(cv: &mut Canvas, h: &Habitat, seed: u64, surface: f32, star: [f32; 3]) {
    let (width, height) = (cv.width, cv.height);
    let (w, hh) = (width as f32, height as f32);
    // Le fond est plus bas pour un milieu profond (échelle compressée).
    let depth_frac = (h.depth_m.max(1.0).log10() / 4.0).clamp(0.15, 1.0);
    let floor_base = surface + (hh - surface) * (0.45 + 0.4 * depth_frac);
    let floor = |x: f32| floor_base - 40.0 * (fbm(seed ^ 0xF100, x / 160.0, 0.5) - 0.5) - 12.0 * (fbm(seed ^ 0xF101, x / 30.0, 0.5) - 0.5);
    // Lumière qui descend dans l'eau : la couche claire s'amincit avec la
    // profondeur ; sous l'étoile rouge, l'eau paraît plus sombre.
    let attenuation = (h.depth_m / 80.0).clamp(0.0, 3.0);
    let water = [WATER[0] * (0.85 + 0.15 * star[0]), WATER[1], WATER[2] * (0.85 + 0.15 * star[2])];
    for x in 0..width {
        let fy = floor(x as f32);
        for y in (surface as usize)..height {
            let yf = y as f32;
            if yf < fy {
                let t = (yf - surface) / (fy - surface).max(1.0);
                // Superposition de passes : une par tranche de profondeur.
                let passes = 1.0 + (t * (1.0 + attenuation) * 2.0).floor();
                let g = fbm(seed ^ 0xA0, x as f32 / 70.0, yf / 50.0);
                cv.wash(x, y, water, (0.22 * passes + 0.1 * (g - 0.5)).min(0.95));
            } else {
                // Sédiment : ocre pâle, hachuré en profondeur.
                let g = fbm(seed ^ 0x5ED, x as f32 / 20.0, yf / 20.0);
                cv.wash(x, y, OCHRE, 0.35 + 0.2 * g);
                if ((x as f32 - yf) / 5.0).rem_euclid(1.0) < 0.18 && yf > fy + 8.0 {
                    cv.set(x, y, INK, 0.25);
                }
            }
        }
    }
    // Surface de l'eau : un trait d'encre ondulé.
    let mut last = (0.0, surface);
    for x in (0..width).step_by(4) {
        let y = surface + 2.5 * (x as f32 / 23.0).sin() + 1.5 * (fbm(seed ^ 0x5A, x as f32 / 40.0, 0.0) - 0.5);
        cv.line(last.0, last.1, x as f32, y, 1.4, INK, 0.9);
        last = (x as f32, y);
    }
    // Profil du fond à l'encre.
    let mut last = (0.0, floor(0.0));
    for x in (0..width).step_by(3) {
        let y = floor(x as f32);
        cv.line(last.0, last.1, x as f32, y, 1.8, INK, 1.0);
        last = (x as f32, y);
    }
    let mut rng = DrawRng::new(seed ^ 0x57A0);
    let colour = h.pigment_rgb.map(tint).unwrap_or([OCHRE[0] * 1.05, OCHRE[1] * 1.05, OCHRE[2] * 1.05]);
    let lit_floor = h.light_w_m2 > 1.0 && h.depth_m < 150.0;
    if lit_floor || h.pigment_rgb.is_some() {
        // Tapis microbiens et stromatolites sur un fond éclairé.
        let domes = 5 + (h.density * 10.0) as usize;
        for _ in 0..domes {
            let cx = rng.range(0.05, 0.95) * w;
            let rad = rng.range(18.0, 46.0) * (0.6 + h.density);
            stromatolite(cv, cx, floor(cx), rad, colour, seed ^ cx.to_bits() as u64);
        }
    }
    if h.vent {
        // Cheminée hydrothermale et son panache.
        let cx = rng.range(0.15, 0.4) * w;
        let base = floor(cx);
        let top = base - hh * 0.18;
        for k in 0..3 {
            let off = (k as f32 - 1.0) * 6.0;
            cv.line(cx + off - 8.0, base, cx + off - 2.0, top + 10.0 * k as f32, 2.0, INK, 1.0);
            cv.line(cx + off + 8.0, base, cx + off + 2.0, top + 10.0 * k as f32, 2.0, INK, 1.0);
        }
        for i in 0..40 {
            let t = i as f32 / 40.0;
            let px = cx + 30.0 * t * (t * 9.0).sin();
            let py = top - t * hh * 0.25;
            let r = 6.0 + 26.0 * t;
            disc_wash(cv, px, py, r, [0.55, 0.52, 0.5], 0.06 * (1.0 - t));
        }
    }
    // Congénères au loin : plancton esquissé en lavis sans contour.
    let plankton = 40 + (h.density * 120.0) as usize;
    for _ in 0..plankton {
        let x = rng.range(0.02, 0.98) * w;
        let y = rng.range(surface + 10.0, floor_base - 20.0);
        if h.depth_m > 200.0 && y > surface + (floor_base - surface) * 0.5 && h.pigment_rgb.is_some() {
            continue;
        }
        disc_wash(cv, x, y, rng.range(1.5, 4.0), colour, 0.35);
    }
    for (i, (_, c)) in h.neighbours.iter().enumerate() {
        let c = c.map(tint).unwrap_or(PAPER);
        for _ in 0..20 {
            let x = rng.range(0.02, 0.98) * w;
            let y = rng.range(surface + 10.0, floor_base - 10.0);
            disc_wash(cv, x, y, 1.5 + i as f32 * 0.5, c, 0.25);
        }
    }
}

fn paint_land(cv: &mut Canvas, h: &Habitat, seed: u64, horizon: f32) {
    let (width, height) = (cv.width, cv.height);
    let hh = height as f32;
    let relief = (h.height_m / 3000.0).clamp(0.1, 1.0) * hh * 0.25;
    let ground = |x: f32| horizon - relief * fbm(seed ^ 0x6A0, x / 220.0, 0.3) + 20.0 * (fbm(seed ^ 0x6A1, x / 40.0, 0.7) - 0.5);
    let icy = h.ice_cover > 0.5;
    for x in 0..width {
        let gy = ground(x as f32);
        for y in (gy.max(0.0) as usize)..height {
            let g = fbm(seed ^ 0x6A2, x as f32 / 25.0, y as f32 / 25.0);
            let c = if icy { [0.93, 0.95, 0.97] } else { OCHRE };
            cv.wash(x, y, c, 0.35 + 0.25 * g);
            // Hachures d'ombre sur les pentes.
            let slope = ground(x as f32 + 2.0) - gy;
            if slope > 0.6 && ((x as f32 + y as f32) / 5.0).rem_euclid(1.0) < 0.2 {
                cv.set(x, y, INK, 0.3);
            }
        }
    }
    let mut last = (0.0, ground(0.0));
    for x in (0..width).step_by(3) {
        let y = ground(x as f32);
        cv.line(last.0, last.1, x as f32, y, 1.8, INK, 1.0);
        last = (x as f32, y);
    }
    if let Some(c) = h.pigment_rgb.map(tint) {
        let mut rng = DrawRng::new(seed ^ 0x1A7);
        for _ in 0..(6 + (h.density * 12.0) as usize) {
            let cx = rng.range(0.05, 0.95) * width as f32;
            stromatolite(cv, cx, ground(cx), rng.range(10.0, 26.0), c, seed ^ cx.to_bits() as u64);
        }
    }
}

fn disc_wash(cv: &mut Canvas, cx: f32, cy: f32, r: f32, c: [f32; 3], alpha: f32) {
    let (x0, x1) = ((cx - r).max(0.0) as usize, ((cx + r) as usize).min(cv.width.saturating_sub(1)));
    let (y0, y1) = ((cy - r).max(0.0) as usize, ((cy + r) as usize).min(cv.height.saturating_sub(1)));
    for y in y0..=y1 {
        for x in x0..=x1 {
            let d = (x as f32 + 0.5 - cx).hypot(y as f32 + 0.5 - cy);
            if d < r {
                cv.wash(x, y, c, alpha * (1.0 - (d / r).powi(2) * 0.5));
            }
        }
    }
}

/// Silhouettes des espèces voisines (architecture, « décor de milieu avec
/// les silhouettes des espèces voisines ») : chaque voisine, de profil, au
/// lavis de son pigment cerné d'encre, posée au premier plan à droite de la
/// planche. Les tailles gardent leurs rapports vrais ; la plus petite est
/// agrandie jusqu'à rester lisible, comme les figures grossies d'une planche
/// de naturaliste.
pub fn paint_neighbours(cv: &mut Canvas, plans: &[(evo_morph::body::BodyPlan, [f32; 3])], seed: u64) {
    if plans.is_empty() {
        return;
    }
    let (w, hh) = (cv.width as f32, cv.height as f32);
    let extents: Vec<f32> = plans.iter().map(|(p, _)| evo_morph::body::extent_m(p)).collect();
    let largest = extents.iter().cloned().fold(0.0f32, f32::max).max(1e-12);
    let mut rng = DrawRng::new(seed ^ 0x51_4C48);
    let slot = (w * 0.55) / plans.len() as f32;
    for (i, ((plan, colour), e)) in plans.iter().zip(&extents).enumerate() {
        let box_h = (hh * 0.28 * (e / largest)).max(hh * 0.1);
        let box_w = (box_h * 1.6).min(slot * 0.95);
        let (bw, bh) = (box_w.max(8.0) as usize, box_h.max(8.0) as usize);
        let (mask, _) = evo_morph::body::silhouette(plan, bw, bh);
        let x0 = w * 0.42 + slot * i as f32 + (slot - bw as f32) * rng.range(0.2, 0.8);
        let y0 = hh * 0.9 - bh as f32 - rng.range(0.0, hh * 0.05);
        // Ombre portée : un lavis d'encre très léger sous la figure.
        disc_wash(cv, x0 + bw as f32 / 2.0, y0 + bh as f32 * 0.98, bw as f32 * 0.35, INK, 0.08);
        for y in 0..bh {
            for x in 0..bw {
                let m = mask[y * bw + x];
                if m <= 0.0 {
                    continue;
                }
                let (px, py) = ((x0 + x as f32) as isize, (y0 + y as f32) as isize);
                if px < 0 || py < 0 || px as usize >= cv.width || py as usize >= cv.height {
                    continue;
                }
                let (px, py) = (px as usize, py as usize);
                cv.wash(px, py, *colour, 0.55 * m);
                // Contour : là où le masque passe de dedans à dehors.
                let edge = (0..4).any(|k| {
                    let (dx, dy) = [(1isize, 0isize), (-1, 0), (0, 1), (0, -1)][k];
                    let (nx, ny) = (x as isize + dx, y as isize + dy);
                    nx < 0 || ny < 0 || nx as usize >= bw || ny as usize >= bh || mask[ny as usize * bw + nx as usize] < 0.5
                });
                if edge && m >= 0.5 {
                    cv.set(px, py, INK, 0.75);
                }
            }
        }
    }
}

/// Dôme de stromatolite : couches de tapis microbien en lavis, contour et
/// lamines à l'encre.
fn stromatolite(cv: &mut Canvas, cx: f32, base: f32, r: f32, colour: [f32; 3], seed: u64) {
    let hgt = r * 0.8;
    let (x0, x1) = ((cx - r).max(0.0) as usize, ((cx + r) as usize).min(cv.width.saturating_sub(1)));
    for x in x0..=x1 {
        let u = (x as f32 - cx) / r;
        let top = base - hgt * (1.0 - u * u).max(0.0).sqrt();
        for y in (top.max(0.0) as usize)..=((base + 2.0) as usize).min(cv.height.saturating_sub(1)) {
            let v = (base - y as f32) / hgt.max(1.0);
            let lam = ((v * 7.0 + 0.4 * fbm(seed, x as f32 / 9.0, 0.0)).fract() < 0.15) as u8 as f32;
            cv.wash(x, y, colour, 0.55 + 0.2 * v);
            if lam > 0.0 {
                cv.set(x, y, [colour[0] * 0.5, colour[1] * 0.5, colour[2] * 0.5], 0.5);
            }
        }
    }
    let mut last = (cx - r, base);
    for i in 1..=24 {
        let a = std::f32::consts::PI * (1.0 - i as f32 / 24.0);
        let p = (cx + r * a.cos(), base - hgt * a.sin());
        cv.line(last.0, last.1, p.0, p.1, 1.5, INK, 1.0);
        last = p;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> evo_engine::Habitat {
        evo_engine::Habitat {
            species: 3,
            name: "méthanogène".into(),
            living: true,
            years: 1e6,
            peak_bio_cell: 4,
            elevation_m: -2500.0,
            temperature_k: 300.0,
            seasonal_amplitude_k: 2.0,
            light_w_m2: 0.0,
            ph: 7.0,
            salinity: 35.0,
            oxygen: 0.0,
            sea_share: 1.0,
            fresh_share: 0.0,
            vent_share: 0.8,
            abs_latitude_rad: 0.3,
            star_temperature_k: 5772.0,
            planet_temperature_k: 288.0,
            pressure_pa: 1e5,
            companions: vec![(5, "fermentatrice".into(), 0.3)],
            region_at_peak: None,
        }
    }

    #[test]
    fn decor_is_deterministic_and_within_budget() {
        let f = crate::frame::tests::sample_frame(5);
        let h = Habitat::from_engine(&sample(), &f);
        assert!(h.is_ocean && h.vent && h.depth_m == 2500.0);
        assert!((h.density - 0.7).abs() < 1e-6 && h.neighbours.len() == 1);
        let seed = h.decor_seed(5);
        let t = std::time::Instant::now();
        let a = paint(&h, seed, 1024, 512);
        let ms = t.elapsed().as_millis();
        assert_eq!(a.checksum(), paint(&h, seed, 1024, 512).checksum());
        // Budget de l'architecture : 200 ms (mesuré en profil de test optimisé).
        assert!(ms < 400, "{ms} ms");
        // Silhouettes des voisines : elles marquent la planche, au même prix.
        let mut b = paint(&h, seed, 1024, 512);
        let plans: Vec<_> = (0..3).map(|k| (evo_morph::body::random_plan(k), [0.4, 0.6, 0.3])).collect();
        let t = std::time::Instant::now();
        paint_neighbours(&mut b, &plans, seed);
        let ms = t.elapsed().as_millis();
        assert_ne!(a.checksum(), b.checksum());
        assert!(ms < 400, "{ms} ms");
    }

    #[test]
    fn lakes_and_land_get_their_own_scene() {
        let f = crate::frame::tests::sample_frame(5);
        let lake = evo_engine::Habitat { sea_share: 0.0, fresh_share: 0.9, elevation_m: 300.0, ..sample() };
        let h = Habitat::from_engine(&lake, &f);
        assert!(h.is_ocean && h.depth_m == LAKE_DEPTH_M && h.height_m == 300.0);
        let land = evo_engine::Habitat { sea_share: 0.0, fresh_share: 0.1, ..lake };
        assert!(!Habitat::from_engine(&land, &f).is_ocean);
    }

    #[test]
    fn methane_haze_warms_the_sky_and_oxygen_cools_it() {
        let base = Habitat {
            species: 0,
            cell: 0,
            is_ocean: true,
            depth_m: 50.0,
            height_m: 0.0,
            temperature_k: 290.0,
            ice_cover: 0.0,
            light_w_m2: 50.0,
            vent: false,
            pigment_rgb: None,
            neighbours: vec![],
            density: 0.5,
            star_temperature_k: 5772.0,
            o2_mixing: 1e-7,
            ch4_ppb: 1.0,
            years: 0.0,
        };
        let hazy = Habitat { ch4_ppb: 5000.0, ..base.clone() };
        let oxic = Habitat { o2_mixing: 0.2, ..base.clone() };
        let (a, b, c) = (sky_colour(&base), sky_colour(&hazy), sky_colour(&oxic));
        assert!(b[0] - b[2] > a[0] - a[2]);
        assert!(c[2] - c[0] > a[2] - a[0]);
    }
}
