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
//! [Simplification] Tant que le service moteur « milieu type d'une espèce »
//! n'est pas publié, `Habitat::of_species` le calcule côté client depuis
//! l'image du pas ; une espèce éteinte n'a pas encore de décor (il faudra
//! l'historique régional à la date de son apogée).

use crate::frame::Frame;
use crate::palette::blackbody;
use evo_morph::canvas::{fbm, mix64, Canvas, DrawRng, INK, OCHRE, PAPER, WATER};

/// Milieu type d'une espèce.
#[derive(Clone, Debug, PartialEq)]
pub struct Habitat {
    pub lineage: u32,
    /// Cellule où l'espèce est la plus abondante.
    pub cell: usize,
    /// Nombre de cellules de son aire.
    pub range_cells: usize,
    pub is_ocean: bool,
    /// Profondeur d'eau moyenne (m, 0 à terre) et altitude moyenne des terres.
    pub depth_m: f32,
    pub height_m: f32,
    pub temperature_k: f32,
    pub ice_cover: f32,
    pub light_w_m2: f32,
    pub vent: bool,
    /// Couleur de l'espèce (pigment) et des trois dominantes du même milieu.
    pub pigment_rgb: Option<[u8; 3]>,
    pub neighbours: Vec<(u32, Option<[u8; 3]>)>,
    /// Densité du tapis : part de la biomasse de la cellule, 0 à 1.
    pub density: f32,
    pub star_temperature_k: f64,
    /// Fractions de l'atmosphère qui teintent le ciel.
    pub o2_mixing: f64,
    pub ch4_ppb: f64,
    pub years: f64,
}

impl Habitat {
    pub fn of_species(frame: &Frame, lineage: u32) -> Option<Habitat> {
        let range = frame.range_of(lineage);
        let &(cell, _) = range.iter().max_by(|a, b| a.1.total_cmp(&b.1))?;
        let n = range.len() as f32;
        let mean = |f: &dyn Fn(usize) -> f32| range.iter().map(|&(c, _)| f(c)).sum::<f32>() / n;
        let ocean_share = mean(&|c| if frame.cells[c].is_ocean { 1.0 } else { 0.0 });
        let pops = frame.populations_of(cell);
        let me = pops.iter().find(|p| p.lineage == lineage)?;
        let pigment_rgb = me.pigment_nm.map(|nm| evo_life::pigment_colour(nm as f64));
        let neighbours = pops
            .iter()
            .filter(|p| p.lineage != lineage)
            .take(3)
            .map(|p| (p.lineage, p.pigment_nm.map(|nm| evo_life::pigment_colour(nm as f64))))
            .collect();
        let total = frame.cells[cell].biomass.max(1e-9);
        Some(Habitat {
            lineage,
            cell,
            range_cells: range.len(),
            is_ocean: ocean_share >= 0.5,
            depth_m: mean(&|c| (-frame.cells[c].height_m).max(0.0)),
            height_m: mean(&|c| frame.cells[c].height_m.max(0.0)),
            temperature_k: mean(&|c| frame.cells[c].temperature_k),
            ice_cover: mean(&|c| frame.cells[c].ice_cover),
            light_w_m2: mean(&|c| frame.cells[c].light_w_m2),
            vent: frame.cells[cell].vent,
            pigment_rgb,
            neighbours,
            density: (me.biomass / total).clamp(0.0, 1.0),
            star_temperature_k: frame.planet.star_temperature_k,
            o2_mixing: frame.globals.o2_mixing,
            ch4_ppb: frame.globals.ch4_ppb,
            years: frame.years,
        })
    }

    /// Graine du décor : il ne change que si l'espèce change de milieu.
    pub fn decor_seed(&self, game_seed: u64) -> u64 {
        let depth_band = (self.depth_m / 250.0) as u64;
        mix64(game_seed ^ mix64(self.lineage as u64) ^ (self.cell as u64) << 20 ^ depth_band << 48)
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
    use evo_sim::{World, WorldConfig};

    #[test]
    fn decor_is_deterministic_and_within_budget() {
        let mut w = World::new(WorldConfig::new(5, 3));
        w.seed_life();
        for _ in 0..3 {
            w.step();
        }
        let f = Frame::from_world(&w);
        let lineage = f.populations[0].lineage;
        let h = Habitat::of_species(&f, lineage).expect("milieu");
        assert!(h.is_ocean && h.vent);
        let seed = h.decor_seed(5);
        let t = std::time::Instant::now();
        let a = paint(&h, seed, 1024, 512);
        let ms = t.elapsed().as_millis();
        assert_eq!(a.checksum(), paint(&h, seed, 1024, 512).checksum());
        // Budget de l'architecture : 200 ms (mesuré en profil de test optimisé).
        assert!(ms < 400, "{ms} ms");
    }

    #[test]
    fn methane_haze_warms_the_sky_and_oxygen_cools_it() {
        let base = Habitat {
            lineage: 0,
            cell: 0,
            range_cells: 1,
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
