//! Tectonique des plaques (document Planète, section 4) : plaques rigides sur
//! la sphère, chacune tournant autour de son pôle, et règles aux frontières.
//!
//! La croûte est portée par des parcelles (une par cellule au départ) qui se
//! déplacent continûment avec leur plaque ; à chaque pas tectonique, chaque
//! cellule de la grille reprend la parcelle qui s'y trouve. Une cellule vide
//! entre deux plaques qui s'écartent reçoit de la croûte océanique neuve
//! (dorsale) ; une cellule où arrivent deux plaques est une frontière
//! convergente : la croûte océanique la plus dense plonge (subduction, arc
//! volcanique qui épaissit la plaque chevauchante), deux continents se
//! soudent en épaississant la croûte (chaîne de collision). L'âge de la
//! croûte océanique fixe sa profondeur (subsidence thermique), l'épaisseur des
//! continents leur altitude (isostasie), et l'érosion use les reliefs.
//!
//! [Simplification] Plaques rigides en nombre fixe, réorganisées
//! périodiquement (nouveaux pôles) au lieu de se fragmenter ou de fusionner ;
//! pas de mécanique des roches ni de convection ; points chauds, failles
//! transformantes et réseau fluvial absents ; la croûte d'arc assez épaisse
//! devient continentale (croissance des continents).

use crate::grid::{GeodesicGrid, Vec3};
use crate::params::PlanetParams;
use evo_core::rng::{rng_for, Stream};
use rand::Rng;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Parcel {
    /// Position sur la sphère unité.
    pub pos: Vec3,
    pub plate: u16,
    pub continental: bool,
    pub thickness_km: f64,
    /// Âge de la croûte, Ma.
    pub age_myr: f64,
    /// Cellule où se trouve la parcelle.
    pub cell: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Plate {
    /// Pôle de rotation (unitaire) et vitesse angulaire, rad·Ma⁻¹.
    pub pole: Vec3,
    pub omega: f64,
}

/// Bilan d'un pas tectonique, en aire sur la sphère unité.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TectonicActivity {
    pub new_crust_area: f64,
    pub subducted_area: f64,
    pub collision_cells: u32,
    pub years: f64,
}

impl TectonicActivity {
    /// Croûte créée par Ma, aire unité.
    pub fn spreading_rate(&self) -> f64 {
        if self.years > 0.0 {
            self.new_crust_area / (self.years / 1e6)
        } else {
            0.0
        }
    }
}

#[derive(Clone, Debug)]
pub struct Tectonics {
    pub plates: Vec<Plate>,
    pub parcels: Vec<Parcel>,
    /// Parcelle portée par chaque cellule.
    pub cell_parcel: Vec<u32>,
    pub last: TectonicActivity,
    /// Taux d'expansion de référence (mesuré après la mise en route) : le
    /// dégazage et l'hydrothermalisme lui sont proportionnels.
    pub reference_spreading: f64,
    pub last_reorganisation_years: f64,
    pub steps: u64,
}

#[inline]
fn dot(a: Vec3, b: Vec3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

#[inline]
fn cross(a: Vec3, b: Vec3) -> Vec3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

#[inline]
fn normalize(v: Vec3) -> Vec3 {
    let n = dot(v, v).sqrt();
    [v[0] / n, v[1] / n, v[2] / n]
}

/// Rotation de `v` d'un angle `theta` autour de l'axe unitaire `k` (Rodrigues).
fn rotate(v: Vec3, k: Vec3, theta: f64) -> Vec3 {
    let (s, c) = theta.sin_cos();
    let kv = cross(k, v);
    let d = dot(k, v) * (1.0 - c);
    normalize([v[0] * c + kv[0] * s + k[0] * d, v[1] * c + kv[1] * s + k[1] * d, v[2] * c + kv[2] * s + k[2] * d])
}

fn random_unit(rng: &mut impl Rng) -> Vec3 {
    let z: f64 = rng.random_range(-1.0..1.0);
    let phi: f64 = rng.random_range(0.0..std::f64::consts::TAU);
    let r = (1.0 - z * z).sqrt();
    [r * phi.cos(), r * phi.sin(), z]
}

/// Cellule la plus proche d'un point, par descente depuis une cellule de
/// départ voisine (les parcelles bougent de moins d'une cellule par pas).
pub fn locate(grid: &GeodesicGrid, start: usize, p: Vec3) -> usize {
    let mut c = start;
    let mut best = dot(grid.centers[c], p);
    loop {
        let mut moved = false;
        for nb in grid.neighbours_of(c) {
            let d = dot(grid.centers[nb], p);
            if d > best {
                best = d;
                c = nb;
                moved = true;
            }
        }
        if !moved {
            return c;
        }
    }
}

/// Champ lisse aléatoire sur la sphère (somme de bosses gaussiennes).
fn smooth_field(grid: &GeodesicGrid, rng: &mut impl Rng, bumps: usize) -> Vec<f64> {
    let b: Vec<(Vec3, f64, f64)> =
        (0..bumps).map(|_| (random_unit(rng), rng.random_range(0.15..0.6), rng.random_range(-1.0..1.0))).collect();
    grid.centers
        .iter()
        .map(|&c| {
            b.iter()
                .map(|&(p, w, h)| {
                    let ang = dot(c, p).clamp(-1.0, 1.0).acos();
                    h * (-(ang / w).powi(2)).exp()
                })
                .sum()
        })
        .collect()
}

impl Tectonics {
    fn random_plates(params: &PlanetParams, rng: &mut impl Rng, count: usize, heat: f64) -> Vec<Plate> {
        let base = params.plate_speed_cm_per_yr * 1e-2 * 1e6 / params.radius_m * heat.sqrt();
        (0..count)
            .map(|_| {
                let sign = if rng.random::<bool>() { 1.0 } else { -1.0 };
                Plate { pole: random_unit(rng), omega: sign * base * rng.random_range(0.4..1.6) }
            })
            .collect()
    }

    /// Plaques de Voronoï irrégulières, continents d'un champ lisse aléatoire.
    pub fn new(grid: &GeodesicGrid, params: &PlanetParams, seed: u64) -> Self {
        let mut rng = rng_for(seed, Stream::Tectonics, &[0]);
        let n = params.plate_count.max(2) as usize;
        let seeds: Vec<Vec3> = (0..n).map(|_| random_unit(&mut rng)).collect();
        let weights: Vec<f64> = (0..n).map(|_| rng.random_range(0.8..1.25)).collect();
        let noise = smooth_field(grid, &mut rng, 48);
        let continent_field = smooth_field(grid, &mut rng, 96);
        let mut sorted = continent_field.clone();
        sorted.sort_by(f64::total_cmp);
        let k = ((1.0 - params.continental_fraction) * sorted.len() as f64) as usize;
        let threshold = sorted[k.min(sorted.len() - 1)];

        let parcels: Vec<Parcel> = (0..grid.len())
            .map(|c| {
                let x = grid.centers[c];
                let plate = (0..n)
                    .max_by(|&a, &b| {
                        let da = dot(x, seeds[a]) * weights[a] + 0.15 * noise[c];
                        let db = dot(x, seeds[b]) * weights[b] + 0.15 * noise[c];
                        da.total_cmp(&db)
                    })
                    .unwrap_or(0) as u16;
                let continental = continent_field[c] >= threshold;
                let thickness_km = if continental {
                    params.continental_crust_km * (1.0 + 0.1 * (continent_field[c] - threshold).min(1.0))
                } else {
                    params.oceanic_crust_km
                };
                Parcel { pos: x, plate, continental, thickness_km, age_myr: rng.random_range(0.0..150.0), cell: c as u32 }
            })
            .collect();
        let plates = Self::random_plates(params, &mut rng, n, 1.0);
        Self {
            plates,
            parcels,
            cell_parcel: (0..grid.len() as u32).collect(),
            last: TectonicActivity::default(),
            reference_spreading: 0.0,
            last_reorganisation_years: 0.0,
            steps: 0,
        }
    }

    pub fn parcel_of(&self, cell: usize) -> &Parcel {
        &self.parcels[self.cell_parcel[cell] as usize]
    }

    /// Vitesse d'un point sur une plaque, rad·Ma⁻¹ (vecteur tangent).
    fn velocity(&self, plate: u16, x: Vec3) -> Vec3 {
        let p = self.plates[plate as usize];
        let v = cross(p.pole, x);
        [v[0] * p.omega, v[1] * p.omega, v[2] * p.omega]
    }

    /// Avance la tectonique de `dt_years`. `heat` est la chaleur interne
    /// relative (elle règle la vitesse des plaques). Renvoie vrai si les
    /// plaques ont été réorganisées.
    pub fn step(&mut self, grid: &GeodesicGrid, params: &PlanetParams, years: f64, dt_years: f64, heat: f64, seed: u64) -> bool {
        let dt_myr = dt_years / 1e6;
        let mut rng = rng_for(seed, Stream::Tectonics, &[1, self.steps]);
        self.steps += 1;
        let mut reorganised = false;
        if years - self.last_reorganisation_years >= params.plate_reorganisation_myr * 1e6 {
            self.plates = Self::random_plates(params, &mut rng, self.plates.len(), heat);
            self.last_reorganisation_years = years;
            reorganised = true;
        }

        // 1. Déplacement des parcelles.
        for p in self.parcels.iter_mut() {
            let plate = self.plates[p.plate as usize];
            p.pos = rotate(p.pos, plate.pole, plate.omega * dt_myr);
            p.cell = locate(grid, p.cell as usize, p.pos) as u32;
            p.age_myr += dt_myr;
        }

        // 2. Parcelles par cellule.
        let n = grid.len();
        let mut order: Vec<u32> = (0..self.parcels.len() as u32).collect();
        order.sort_by_key(|&i| (self.parcels[i as usize].cell, i));
        let mut ranges = vec![(0usize, 0usize); n];
        let mut i = 0;
        while i < order.len() {
            let c = self.parcels[order[i] as usize].cell as usize;
            let mut j = i;
            while j < order.len() && self.parcels[order[j] as usize].cell as usize == c {
                j += 1;
            }
            ranges[c] = (i, j);
            i = j;
        }

        let mut activity = TectonicActivity { years: dt_years, ..Default::default() };
        let mut keep: Vec<Option<Parcel>> = vec![None; n];
        let thicken_to_continent = 2.5 * params.oceanic_crust_km;
        for c in 0..n {
            let (a, b) = ranges[c];
            if a == b {
                continue;
            }
            let group: Vec<Parcel> = order[a..b].iter().map(|&k| self.parcels[k as usize]).collect();
            // Survivant : continent d'abord (il ne plonge pas), puis la croûte
            // océanique la plus jeune (la moins dense) ; à égalité, la plus
            // proche du centre de la cellule.
            let rank = |p: &Parcel| (p.continental, -p.age_myr, dot(p.pos, grid.centers[c]));
            let mut survivor = group[0];
            for p in &group[1..] {
                if rank(p).partial_cmp(&rank(&survivor)) == Some(std::cmp::Ordering::Greater) {
                    survivor = *p;
                }
            }
            for p in &group {
                if p.plate == survivor.plate {
                    continue;
                }
                if p.continental && survivor.continental {
                    // Collision continentale : la croûte s'épaissit.
                    survivor.thickness_km = (survivor.thickness_km + 0.5 * p.thickness_km).min(2.3 * params.continental_crust_km);
                    activity.collision_cells += 1;
                } else {
                    // Subduction : arc volcanique sur la plaque chevauchante.
                    activity.subducted_area += grid.unit_areas[c];
                    survivor.thickness_km += 0.1 * params.oceanic_crust_km;
                }
            }
            if !survivor.continental && survivor.thickness_km >= thicken_to_continent {
                survivor.continental = true;
            }
            survivor.cell = c as u32;
            keep[c] = Some(survivor);
        }

        // 3. Cellules vides : dorsale si les plaques voisines s'écartent,
        //    sinon trou de discrétisation comblé par la parcelle voisine.
        for _pass in 0..4 {
            let snapshot = keep.clone();
            let mut remaining = 0;
            for c in 0..n {
                if snapshot[c].is_some() {
                    continue;
                }
                let neigh: Vec<(usize, Parcel)> = grid.neighbours_of(c).filter_map(|nb| snapshot[nb].map(|p| (nb, p))).collect();
                if neigh.is_empty() {
                    remaining += 1;
                    continue;
                }
                let mut diverging = false;
                for (i, &(ca, pa)) in neigh.iter().enumerate() {
                    for &(cb, pb) in &neigh[i + 1..] {
                        if pa.plate != pb.plate {
                            let va = self.velocity(pa.plate, grid.centers[ca]);
                            let vb = self.velocity(pb.plate, grid.centers[cb]);
                            let d = [
                                grid.centers[cb][0] - grid.centers[ca][0],
                                grid.centers[cb][1] - grid.centers[ca][1],
                                grid.centers[cb][2] - grid.centers[ca][2],
                            ];
                            let rel = [vb[0] - va[0], vb[1] - va[1], vb[2] - va[2]];
                            if dot(rel, d) > 0.0 {
                                diverging = true;
                            }
                        }
                    }
                }
                // Plaque la plus représentée autour (plus petit numéro à égalité).
                let mut counts: Vec<(u16, usize)> = Vec::new();
                for &(_, p) in &neigh {
                    match counts.iter_mut().find(|x| x.0 == p.plate) {
                        Some(x) => x.1 += 1,
                        None => counts.push((p.plate, 1)),
                    }
                }
                counts.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
                let plate = counts[0].0;
                let parcel = if diverging {
                    activity.new_crust_area += grid.unit_areas[c];
                    Parcel {
                        pos: grid.centers[c],
                        plate,
                        continental: false,
                        thickness_km: params.oceanic_crust_km,
                        age_myr: 0.0,
                        cell: c as u32,
                    }
                } else {
                    let src = neigh.iter().find(|x| x.1.plate == plate).map(|x| x.1).unwrap_or(neigh[0].1);
                    Parcel { pos: grid.centers[c], cell: c as u32, ..src }
                };
                keep[c] = Some(parcel);
            }
            if remaining == 0 {
                break;
            }
        }

        // 4. Érosion des reliefs épaissis, nouvelles parcelles.
        let relax = 1.0 - (-dt_myr / params.erosion_myr).exp();
        self.parcels = keep
            .into_iter()
            .enumerate()
            .map(|(c, p)| {
                let mut p = p.unwrap_or(Parcel {
                    pos: grid.centers[c],
                    plate: 0,
                    continental: false,
                    thickness_km: params.oceanic_crust_km,
                    age_myr: 0.0,
                    cell: c as u32,
                });
                let reference = if p.continental { params.continental_crust_km } else { params.oceanic_crust_km };
                if p.thickness_km > reference && !(p.continental && p.thickness_km < reference) {
                    p.thickness_km += (reference - p.thickness_km) * relax * if p.continental { 1.0 } else { 0.2 };
                }
                p
            })
            .collect();
        self.cell_parcel = (0..n as u32).collect();
        self.last = activity;
        reorganised
    }

    /// Altitude du dessus de la croûte par rapport au niveau de référence
    /// (dorsale à `ridge_depth_km` dessous), m : isostasie pour les
    /// continents, subsidence thermique pour la croûte océanique.
    pub fn elevation_m(&self, params: &PlanetParams, cell: usize) -> f64 {
        let p = self.parcel_of(cell);
        let km = if p.continental {
            // Flottabilité de la croûte continentale par rapport à une croûte
            // océanique jeune, à l'équilibre isostatique.
            let buoyancy = |h: f64, rho: f64| h * (1.0 - rho / params.mantle_density);
            -params.ridge_depth_km + buoyancy(p.thickness_km, params.continental_crust_density)
                - buoyancy(params.oceanic_crust_km, params.oceanic_crust_density)
        } else {
            let extra = (p.thickness_km - params.oceanic_crust_km).max(0.0) * (1.0 - params.oceanic_crust_density / params.mantle_density);
            -params.ridge_depth_km - params.subsidence_km_per_sqrt_myr * p.age_myr.min(150.0).sqrt() + extra
        };
        km * 1000.0
    }

    /// Part de la surface en croûte continentale.
    pub fn continental_share(&self, grid: &GeodesicGrid) -> f64 {
        let total: f64 = grid.unit_areas.iter().sum();
        (0..grid.len()).filter(|&c| self.parcel_of(c).continental).map(|c| grid.unit_areas[c]).sum::<f64>() / total
    }

    pub fn memory_bytes(&self) -> usize {
        self.parcels.capacity() * std::mem::size_of::<Parcel>() + self.cell_parcel.capacity() * 4
    }
}

/// Niveau de la mer (même référence que les altitudes) qui loge un volume
/// d'eau donné, m, par dichotomie.
pub fn sea_level(elevations: &[f64], areas_m2: &[f64], water_volume_m3: f64) -> f64 {
    let volume_at = |s: f64| elevations.iter().zip(areas_m2).map(|(&e, &a)| (s - e).max(0.0) * a).sum::<f64>();
    let lo0 = elevations.iter().cloned().fold(f64::MAX, f64::min);
    let (mut lo, mut hi) = (lo0, lo0 + 1.0);
    while volume_at(hi) < water_volume_m3 {
        hi = lo0 + 2.0 * (hi - lo0);
    }
    for _ in 0..80 {
        let mid = 0.5 * (lo + hi);
        if volume_at(mid) < water_volume_m3 {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    0.5 * (lo + hi)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plates_move_create_and_destroy_crust() {
        let grid = GeodesicGrid::new(4);
        let params = PlanetParams::earth_archean();
        let mut t = Tectonics::new(&grid, &params, 3);
        let continents0 = t.continental_share(&grid);
        assert!((continents0 - params.continental_fraction).abs() < 0.05, "{continents0}");
        let (mut created, mut subducted) = (0.0, 0.0);
        for i in 0..50 {
            t.step(&grid, &params, i as f64 * 1e6, 1e6, 1.0, 3);
            created += t.last.new_crust_area;
            subducted += t.last.subducted_area;
            assert_eq!(t.parcels.len(), grid.len());
        }
        assert!(created > 0.1 && subducted > 0.1, "créé {created}, subduit {subducted}");
        // De la croûte jeune existe aux dorsales, et les continents n'ont pas disparu.
        assert!((0..grid.len()).any(|c| t.parcel_of(c).age_myr < 2.0));
        let continents = t.continental_share(&grid);
        assert!(continents > 0.5 * continents0 && continents < 2.0 * continents0, "{continents0} → {continents}");
    }

    #[test]
    fn sea_level_holds_the_water() {
        let e = [-4000.0, -2000.0, 500.0];
        let a = [1.0, 1.0, 1.0];
        let s = sea_level(&e, &a, 3000.0);
        assert!((s - (-1500.0)).abs() < 1e-6, "{s}");
    }

    #[test]
    fn rotation_preserves_norm_and_moves() {
        let v = normalize([1.0, 0.2, 0.3]);
        let w = rotate(v, [0.0, 0.0, 1.0], 0.1);
        assert!((dot(w, w) - 1.0).abs() < 1e-12);
        assert!((dot(v, w) - 1.0).abs() > 1e-4);
        let grid = GeodesicGrid::new(3);
        let target = grid.centers[17];
        assert_eq!(locate(&grid, 0, target), 17);
    }
}
