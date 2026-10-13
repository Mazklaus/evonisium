//! Villes et villages dans la scène au sol (étape 7, palier 3b du rendu) :
//! une place au centre avec son temple, son palais et sa halle, des rues
//! (organiques, en étoile, ou en damier quand la société connaît
//! l'urbanisme), des maisons le long des rues, des immeubles au cœur des
//! villes denses, des ateliers, des usines en lisière à l'ère de la vapeur,
//! un rempart et ses tours, des moulins, des greniers, des champs en lanières
//! autour, et des ruines quand la ville a perdu ses habitants.
//!
//! Tout se lit dans l'état des sociétés (`SocietyState`) : la cellule, sa
//! ville, les techniques connues de la cellule (l'époque des bâtiments), la
//! société (couleur, rang) et son peuple (le style). Affichage seul : rien
//! n'en remonte au moteur.
//!
//! [Simplification] Une ville réelle compte des milliers de maisons : la
//! scène en montre au plus quelques centaines, plus nombreuses quand la
//! ville est grande. Le plan ne vient pas d'une histoire des rues.

use crate::camp::Camp;
use crate::ground::{Patch, PATCH_SIZE_M};
use crate::society::colour;
use evo_morph::building::{footprint, Era, Kind, Style};
use evo_morph::canvas::{mix64, DrawRng};
use evo_society::catalog::{catalog, Known};
use evo_society::society::Label;
use evo_society::state::SocietyState;
use std::f32::consts::{PI, TAU};

/// Plafond des bâtiments d'une scène (instances).
pub const MAX_BUILDINGS: usize = 520;

/// Ce que la scène montre d'une cellule habitée par une société.
#[derive(Clone, Debug, PartialEq)]
pub struct TownView {
    pub cell: u32,
    /// Nom de la ville, ou celui de la société pour un village.
    pub name: String,
    pub society: String,
    pub society_id: u32,
    pub label: Label,
    pub era: Era,
    /// Graine du style : le peuple de la société.
    pub culture: u64,
    pub accent: [f32; 3],
    /// Une ville (sinon un village de cultivateurs).
    pub city: bool,
    pub capital: bool,
    /// Habitants montrés, et le plus haut qu'ait connu la ville.
    pub pop: f64,
    pub peak: f64,
    pub fortified: bool,
    pub grid: bool,
    pub windmill: bool,
    pub granary: bool,
    pub market: bool,
    pub temple: bool,
    pub crop: Option<u32>,
    pub herd: bool,
}

fn knows(known: &Known, key: &str) -> bool {
    catalog().id(key).is_some_and(|t| known.has(t))
}

/// Époque des bâtiments d'après les techniques connues.
pub fn era_of(known: &Known) -> Era {
    let k = |key| knows(known, key);
    if k("acier_bessemer") && k("ciment_portland") {
        Era::Modern
    } else if k("machine_vapeur") || k("haut_fourneau") {
        Era::Industrial
    } else if k("beton_romain") || k("urbanisme") {
        Era::Classical
    } else if k("fortifications") || k("ecriture") {
        Era::Monumental
    } else if k("brique_crue") {
        Era::Brick
    } else {
        Era::Village
    }
}

/// La ville ou le village de la cellule ; `None` quand la cellule n'a ni
/// ville ni cultivateurs sédentaires.
pub fn view(state: &SocietyState, cell: u32) -> Option<TownView> {
    let c = state.cells.iter().find(|c| c.cell == cell && c.pop > 0.0)?;
    let s = state.societies.iter().find(|s| s.id == c.society)?;
    let city = c.city.and_then(|id| state.cities.iter().find(|x| x.id == id));
    if city.is_none() && !c.sedentary {
        return None;
    }
    let known = c.known.or(&s.known);
    let k = |key| knows(&known, key);
    let (name, pop, peak) = match city {
        Some(x) => (x.name.clone(), x.pop, x.peak.max(x.pop)),
        // Un village parmi ceux de la cellule.
        None => (s.name.clone(), (c.pop * 0.02).clamp(80.0, 600.0), 0.0),
    };
    Some(TownView {
        cell,
        name,
        society: s.name.clone(),
        society_id: s.id,
        label: s.label,
        era: era_of(&known),
        culture: mix64(s.people as u64 ^ 0x7065_7570),
        accent: colour(s.id),
        city: city.is_some(),
        capital: s.capital == cell,
        pop,
        peak,
        fortified: k("fortifications"),
        grid: k("urbanisme"),
        windmill: k("moulin_vent") || k("moulin_eau"),
        granary: k("grenier") || k("stockage"),
        market: k("biens_monnaie") || k("pieces") || k("metal_pese"),
        temple: city.is_some() || k("ecriture") || k("calendrier_solaire"),
        crop: c.crop,
        herd: c.herd.is_some(),
    })
}

/// Un bâtiment posé : modèle (rang dans `Town::models`), pied au sol, cap
/// (l'avant, la porte, tourné vers `heading`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Building {
    pub model: u16,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub heading: f32,
}

impl Building {
    /// Transformation d'instance (3 lignes de 4), comme celle des objets.
    pub fn rows(&self) -> [f32; 12] {
        let (sn, cs) = (-self.heading).sin_cos();
        [cs, 0.0, sn, self.x, 0.0, 1.0, 0.0, self.y, -sn, 0.0, cs, self.z]
    }
}

/// Un modèle de bâtiment : sorte et graine de variante.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Model {
    pub kind: Kind,
    pub seed: u64,
}

/// Une rue : ligne brisée et largeur.
#[derive(Clone, Debug, PartialEq)]
pub struct Street {
    pub points: Vec<(f32, f32)>,
    pub width: f32,
}

/// Un champ : rectangle de sillons.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Field {
    pub centre: (f32, f32),
    pub heading: f32,
    pub length: f32,
    pub width: f32,
    pub colour: [f32; 3],
}

/// La ville d'une scène.
#[derive(Clone, Debug, PartialEq)]
pub struct Town {
    pub view: TownView,
    pub style: Style,
    pub models: Vec<Model>,
    pub buildings: Vec<Building>,
    pub plaza: (f32, f32),
    pub plaza_half: f32,
    pub plaza_heading: f32,
    pub streets: Vec<Street>,
    pub fields: Vec<Field>,
    /// Rayon de la partie bâtie.
    pub radius: f32,
    pub seed: u64,
}

/// Grille d'occupation, mailles de 2 m, sur tout le carré.
struct Occupancy {
    n: usize,
    cells: Vec<bool>,
}

const OCC_STEP: f32 = 2.0;

impl Occupancy {
    fn new() -> Occupancy {
        let n = (PATCH_SIZE_M / OCC_STEP) as usize;
        Occupancy { n, cells: vec![false; n * n] }
    }

    fn index(&self, x: f32, z: f32) -> Option<usize> {
        let i = ((x + PATCH_SIZE_M / 2.0) / OCC_STEP).floor();
        let j = ((z + PATCH_SIZE_M / 2.0) / OCC_STEP).floor();
        if i < 0.0 || j < 0.0 || i as usize >= self.n || j as usize >= self.n {
            return None;
        }
        Some(j as usize * self.n + i as usize)
    }

    /// Mailles d'un rectangle tourné (demi-longueur selon le cap `h`,
    /// demi-largeur en travers).
    fn rect(&self, cx: f32, cz: f32, h: f32, hl: f32, hw: f32) -> Vec<Option<usize>> {
        let (s, c) = h.sin_cos();
        let r = hl.hypot(hw);
        let mut out = Vec::new();
        let mut z = cz - r;
        while z <= cz + r {
            let mut x = cx - r;
            while x <= cx + r {
                let (dx, dz) = (x - cx, z - cz);
                let (u, v) = (dx * c + dz * s, -dx * s + dz * c);
                if u.abs() <= hl && v.abs() <= hw {
                    out.push(self.index(x, z));
                }
                x += OCC_STEP;
            }
            z += OCC_STEP;
        }
        out
    }

    fn free(&self, cx: f32, cz: f32, h: f32, hl: f32, hw: f32) -> bool {
        self.rect(cx, cz, h, hl, hw).into_iter().all(|i| i.is_some_and(|i| !self.cells[i]))
    }

    fn mark(&mut self, cx: f32, cz: f32, h: f32, hl: f32, hw: f32) {
        for i in self.rect(cx, cz, h, hl, hw).into_iter().flatten() {
            self.cells[i] = true;
        }
    }

    fn mark_line(&mut self, a: (f32, f32), b: (f32, f32), width: f32) {
        let (dx, dz) = (b.0 - a.0, b.1 - a.1);
        let len = dx.hypot(dz);
        if len < 1e-3 {
            return;
        }
        let h = dz.atan2(dx);
        self.mark((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0, h, len / 2.0 + width / 2.0, width / 2.0);
    }
}

/// Le sol d'une emprise : à sec (hors de la mer) et assez plat ; rend
/// l'altitude du pied (la plus basse des coins, le bâtiment s'enfonce un
/// peu côté pente).
fn ground_of(patch: &Patch, cx: f32, cz: f32, h: f32, hl: f32, hw: f32) -> Option<f32> {
    let (s, c) = h.sin_cos();
    let mut lo = f32::INFINITY;
    let mut hi = f32::NEG_INFINITY;
    for (u, v) in [(0.0, 0.0), (hl, hw), (hl, -hw), (-hl, hw), (-hl, -hw)] {
        let (x, z) = (cx + u * c - v * s, cz + u * s + v * c);
        if x.abs() > PATCH_SIZE_M / 2.0 - 20.0 || z.abs() > PATCH_SIZE_M / 2.0 - 20.0 {
            return None;
        }
        let y = patch.height_at(x, z);
        if patch.water && y < 0.6 {
            return None;
        }
        lo = lo.min(y);
        hi = hi.max(y);
    }
    (hi - lo <= 0.3 * hl.min(hw) + 1.2).then_some(lo)
}

/// Le replat le plus doux près du centre, au sec, pour la place.
fn plaza_spot(patch: &Patch) -> (f32, f32) {
    let mut best = (0.0, 0.0);
    let mut score = f32::INFINITY;
    for j in -12..=12 {
        for i in -12..=12 {
            let (x, z) = (i as f32 * 25.0, j as f32 * 25.0);
            let h = patch.height_at(x, z);
            if patch.water && h < 2.0 {
                continue;
            }
            let mut slope = 0.0;
            for r in [10.0, 30.0] {
                slope += (patch.height_at(x + r, z) - patch.height_at(x - r, z)).abs()
                    + (patch.height_at(x, z + r) - patch.height_at(x, z - r)).abs();
            }
            let s = slope + 0.01 * x.hypot(z);
            if s < score {
                score = s;
                best = (x, z);
            }
        }
    }
    best
}

/// Bâtiments voulus pour `pop` habitants.
fn building_count(pop: f64, city: bool) -> usize {
    if city {
        (10.0 + 6.0 * (pop / 100.0).sqrt()).min(420.0) as usize
    } else {
        (8.0 + pop.sqrt() * 0.9).clamp(10.0, 40.0) as usize
    }
}

/// Une place à pourvoir le long d'une rue.
#[derive(Clone, Copy)]
struct Slot {
    x: f32,
    z: f32,
    /// Vers la rue.
    heading: f32,
    /// Distance à la place, avec un peu de hasard.
    rank: f32,
    /// Rue principale ou anneau extérieur.
    dist: f32,
}

/// Dispose la ville de `view` dans la scène.
pub fn layout(view: &TownView, patch: &Patch, seed: u64) -> Town {
    let seed = mix64(seed ^ 0x746F_776E);
    let mut rng = DrawRng::new(seed);
    let style = Style::of(view.era, view.culture, view.accent);
    let era = view.era;
    let mut occ = Occupancy::new();
    let plaza = plaza_spot(patch);
    let theta = rng.range(0.0, TAU);
    let live = building_count(view.pop, view.city);
    // Une ville déchue garde le plan de sa grandeur : le reste est en ruine.
    let ruined = view.city && view.pop < 0.5 * view.peak;
    let planned = if ruined { building_count(view.peak, true).max(live) } else { live };
    let house = footprint(Kind::House, &style);
    let dense = view.city && (view.pop > 20_000.0 || era >= Era::Industrial && view.pop > 5_000.0);
    let area = (house.0 + 4.0) * (house.1 + 3.0) * if dense { 1.6 } else { 2.2 };
    let radius = ((planned as f32 * area / PI).sqrt() * 1.4).clamp(45.0, 560.0);
    let plaza_half = if view.city { 16.0 + (view.pop as f32).log10().max(2.0) * 2.0 } else { 9.0 };
    occ.mark(plaza.0, plaza.1, theta, plaza_half, plaza_half);

    let mut models: Vec<Model> = Vec::new();
    let mut model = |kind: Kind, variant: u64| -> u16 {
        let m = Model { kind, seed: mix64(view.culture ^ variant.wrapping_mul(0x9E37) ^ kind as u64) };
        match models.iter().position(|x| *x == m) {
            Some(i) => i as u16,
            None => {
                models.push(m);
                (models.len() - 1) as u16
            }
        }
    };
    let mut buildings: Vec<Building> = Vec::new();
    let put = |occ: &mut Occupancy, buildings: &mut Vec<Building>, m: u16, kind: Kind, x: f32, z: f32, h: f32| -> bool {
        let (l, w) = footprint(kind, &style);
        let (hl, hw) = (l / 2.0 + 1.0, w / 2.0 + 1.0);
        if buildings.len() >= MAX_BUILDINGS || !occ.free(x, z, h, hl, hw) {
            return false;
        }
        let Some(y) = ground_of(patch, x, z, h, l / 2.0, w / 2.0) else { return false };
        occ.mark(x, z, h, hl, hw);
        buildings.push(Building { model: m, x, y, z, heading: h });
        true
    };

    // La place : temple, palais, halle, chacun face à la place.
    let mut around = Vec::new();
    if view.temple && (view.city || era >= Era::Brick) {
        around.push(Kind::Temple);
    }
    if view.city && (view.capital || view.label >= Label::State) {
        around.push(Kind::Palace);
    }
    if view.city && view.market {
        around.push(Kind::Market);
    }
    for (side, &kind) in around.iter().enumerate() {
        let m = model(kind, 0);
        let (l, _) = footprint(kind, &style);
        let base = theta + side as f32 * PI / 2.0;
        'try_: for step in 0..14 {
            for da in [0.0, 0.25, -0.25] {
                let a = base + da;
                let r = plaza_half + l / 2.0 + 1.5 + step as f32 * 4.0;
                let (x, z) = (plaza.0 + r * a.cos(), plaza.1 + r * a.sin());
                if put(&mut occ, &mut buildings, m, kind, x, z, a + PI) {
                    break 'try_;
                }
            }
        }
    }

    // Rues.
    let main_w = if view.city { 7.0 } else { 4.5 };
    let mut streets = Vec::new();
    if view.grid {
        // Damier : îlots de deux rangs de maisons.
        let spacing = 2.0 * house.0 + 2.0 * 3.0 + main_w;
        let k = (radius / spacing).ceil() as i32 + 1;
        let (s, c) = theta.sin_cos();
        let at = |u: f32, v: f32| (plaza.0 + u * c - v * s, plaza.1 + u * s + v * c);
        let reach = radius + spacing * 0.5;
        let base = plaza_half + main_w / 2.0;
        for i in 0..=k {
            for u in [base + i as f32 * spacing, -base - i as f32 * spacing] {
                if u.abs() > reach {
                    continue;
                }
                let half = (reach * reach - u * u).max(0.0).sqrt();
                streets.push(Street { points: vec![at(u, -half), at(u, half)], width: main_w });
                streets.push(Street { points: vec![at(-half, u), at(half, u)], width: main_w });
            }
        }
    } else {
        let spokes = if view.city { 4 + (rng.next_u64() % 3) as usize } else { 2 + (rng.next_u64() % 2) as usize };
        for k in 0..spokes {
            let mut a = theta + PI / 4.0 + TAU * k as f32 / spokes as f32 + rng.range(-0.2, 0.2);
            let mut p = (plaza.0 + plaza_half * a.cos(), plaza.1 + plaza_half * a.sin());
            let mut points = vec![p];
            let mut r = plaza_half;
            while r < radius * 1.15 {
                a += rng.range(-0.22, 0.22);
                p = (p.0 + 22.0 * a.cos(), p.1 + 22.0 * a.sin());
                r = (p.0 - plaza.0).hypot(p.1 - plaza.1);
                points.push(p);
            }
            streets.push(Street { points, width: main_w });
        }
        if view.city && planned > 60 {
            let rings: &[f32] = if planned > 220 { &[0.42, 0.8] } else { &[0.6] };
            for &f in rings {
                let n = 28;
                let jitter = rng.range(0.0, 1.0);
                let points: Vec<(f32, f32)> = (0..=n)
                    .map(|i| {
                        let a = theta + TAU * i as f32 / n as f32;
                        let r = radius * f * (1.0 + 0.06 * (3.0 * a + jitter * 6.0).sin());
                        (plaza.0 + r * a.cos(), plaza.1 + r * a.sin())
                    })
                    .collect();
                streets.push(Street { points, width: main_w - 1.5 });
            }
        }
    }
    // Les rues s'arrêtent aux monuments et à la place : coupées en
    // tronçons de 12 m au plus, sans ceux qui tombent dessus.
    let mut cut = Vec::new();
    for s in streets {
        let mut cur: Vec<(f32, f32)> = Vec::new();
        for w in s.points.windows(2) {
            let (a, b) = (w[0], w[1]);
            let n = ((b.0 - a.0).hypot(b.1 - a.1) / 12.0).ceil().max(1.0) as usize;
            for i in 0..n {
                let p = (a.0 + (b.0 - a.0) * i as f32 / n as f32, a.1 + (b.1 - a.1) * i as f32 / n as f32);
                let q = (a.0 + (b.0 - a.0) * (i + 1) as f32 / n as f32, a.1 + (b.1 - a.1) * (i + 1) as f32 / n as f32);
                let blocked =
                    [p, q, ((p.0 + q.0) / 2.0, (p.1 + q.1) / 2.0)].iter().any(|&(x, z)| occ.index(x, z).is_none_or(|k| occ.cells[k]));
                if blocked {
                    if cur.len() >= 2 {
                        cut.push(Street { points: std::mem::take(&mut cur), width: s.width });
                    }
                    cur.clear();
                } else {
                    if cur.is_empty() {
                        cur.push(p);
                    }
                    cur.push(q);
                }
            }
        }
        if cur.len() >= 2 {
            cut.push(Street { points: cur, width: s.width });
        }
    }
    let streets = cut;
    for s in &streets {
        for w in s.points.windows(2) {
            occ.mark_line(w[0], w[1], s.width);
        }
    }

    // Les places le long des rues, les plus proches de la place d'abord.
    let mut slots: Vec<Slot> = Vec::new();
    let offset = house.0 / 2.0 + 1.6;
    for s in &streets {
        for w in s.points.windows(2) {
            let (a, b) = (w[0], w[1]);
            let (dx, dz) = (b.0 - a.0, b.1 - a.1);
            let len = dx.hypot(dz);
            if len < 1.0 {
                continue;
            }
            let (ux, uz) = (dx / len, dz / len);
            let step = house.1 + rng.range(1.2, 2.6);
            let mut t = rng.range(0.0, step);
            while t < len {
                for side in [1.0f32, -1.0] {
                    let (nx, nz) = (-uz * side, ux * side);
                    let o = s.width / 2.0 + offset;
                    let (x, z) = (a.0 + ux * t + nx * o, a.1 + uz * t + nz * o);
                    let dist = (x - plaza.0).hypot(z - plaza.1);
                    if dist > radius * 1.2 {
                        continue;
                    }
                    slots.push(Slot { x, z, heading: (-nz).atan2(-nx), rank: dist * rng.range(0.85, 1.15), dist });
                }
                t += step;
            }
        }
    }
    slots.sort_by(|a, b| a.rank.total_cmp(&b.rank));

    // Usines en lisière, d'un même côté de la ville.
    if era >= Era::Industrial && view.city {
        let want = ((view.pop / 40_000.0) as usize).clamp(2, 8);
        let side = rng.range(0.0, TAU);
        let m = model(Kind::Factory, 0);
        let mut outer: Vec<&Slot> = slots.iter().filter(|s| s.dist > radius * 0.6).collect();
        outer.sort_by(|a, b| {
            let d = |s: &Slot| (((s.z - plaza.1).atan2(s.x - plaza.0) - side + 3.0 * PI) % TAU - PI).abs();
            d(a).total_cmp(&d(b))
        });
        let mut n = 0;
        for s in outer {
            let (l, _) = footprint(Kind::Factory, &style);
            let back = l / 2.0 - house.0 / 2.0;
            let (x, z) = (s.x - back * s.heading.cos(), s.z - back * s.heading.sin());
            if n < want && put(&mut occ, &mut buildings, m, Kind::Factory, x, z, s.heading) {
                n += 1;
            }
        }
    }

    // Maisons, immeubles et ateliers.
    let house_models: Vec<u16> = (0..3).map(|v| model(Kind::House, v)).collect();
    let ruin_models: Vec<u16> = (0..2).map(|v| model(Kind::Ruin, v)).collect();
    let block_models: Vec<u16> = (0..2).map(|v| model(Kind::Block, v)).collect();
    let workshop = model(Kind::Workshop, 0);
    let core = radius * if era >= Era::Industrial { 0.55 } else { 0.38 };
    let mut placed = 0usize;
    for s in &slots {
        if placed >= planned {
            break;
        }
        let ruin = ruined && placed >= live;
        let (kind, m) = if !ruin && dense && s.dist < core {
            (Kind::Block, block_models[(rng.next_u64() % 2) as usize])
        } else if !ruin && era >= Era::Brick && rng.unit() < 0.06 {
            (Kind::Workshop, workshop)
        } else if ruin {
            (Kind::Ruin, ruin_models[(rng.next_u64() % 2) as usize])
        } else {
            (Kind::House, house_models[(rng.next_u64() % 3) as usize])
        };
        let (l, _) = footprint(kind, &style);
        let back = l / 2.0 - house.0 / 2.0;
        let (x, z) = (s.x - back * s.heading.cos(), s.z - back * s.heading.sin());
        if put(&mut occ, &mut buildings, m, kind, x, z, s.heading + rng.range(-0.05, 0.05)) {
            placed += 1;
        }
    }
    // Rayon bâti, pour le rempart, les moulins et les champs.
    let built = buildings.iter().map(|b| (b.x - plaza.0).hypot(b.z - plaza.1)).fold(plaza_half + 10.0, f32::max);

    // Greniers près de la place.
    if view.granary && era < Era::Industrial {
        let m = model(Kind::Granary, 0);
        let want = if view.city { 3 } else { 1 + (rng.next_u64() % 2) as usize };
        let mut n = 0;
        for _ in 0..40 {
            if n >= want {
                break;
            }
            let a = rng.range(0.0, TAU);
            let r = rng.range(plaza_half + 8.0, (built * 0.7).max(plaza_half + 14.0));
            if put(&mut occ, &mut buildings, m, Kind::Granary, plaza.0 + r * a.cos(), plaza.1 + r * a.sin(), a + PI) {
                n += 1;
            }
        }
    }

    // Rempart : un anneau de tours reliées, ouvert où passent les rues.
    let mut wall_r = built + 14.0;
    if view.fortified && view.city && era <= Era::Classical {
        let towers = ((TAU * wall_r / 46.0).ceil() as usize).clamp(8, 72);
        let mt = model(Kind::WallTower, 0);
        let ms = model(Kind::WallSegment, 0);
        let mut street_occ = Occupancy::new();
        for s in &streets {
            for w in s.points.windows(2) {
                street_occ.mark_line(w[0], w[1], s.width + 4.0);
            }
        }
        let at = |k: f32| {
            let a = theta + TAU * k / towers as f32;
            (plaza.0 + wall_r * a.cos(), plaza.1 + wall_r * a.sin())
        };
        let (seg_l, _) = footprint(Kind::WallSegment, &style);
        for k in 0..towers {
            let (a, b) = (at(k as f32), at(k as f32 + 1.0));
            let ok = |x: f32, z: f32| {
                let y = patch.height_at(x, z);
                !(patch.water && y < 0.6) && street_occ.index(x, z).is_some_and(|i| !street_occ.cells[i])
            };
            if ok(a.0, a.1) && buildings.len() < MAX_BUILDINGS {
                buildings.push(Building { model: mt, x: a.0, y: patch.height_at(a.0, a.1) - 0.5, z: a.1, heading: 0.0 });
            }
            let (dx, dz) = (b.0 - a.0, b.1 - a.1);
            let len = dx.hypot(dz);
            let n = (len / seg_l).ceil().max(1.0) as usize;
            for i in 0..n {
                let t = (i as f32 + 0.5) / n as f32;
                let (x, z) = (a.0 + dx * t, a.1 + dz * t);
                if ok(x, z) && buildings.len() < MAX_BUILDINGS {
                    buildings.push(Building { model: ms, x, y: patch.height_at(x, z) - 0.3, z, heading: dz.atan2(dx) });
                }
            }
        }
        wall_r += 6.0;
    } else {
        wall_r = built + 6.0;
    }

    // Moulins sur les hauteurs, hors les murs.
    if view.windmill && era < Era::Modern {
        let m = model(Kind::Windmill, 0);
        let mut spots: Vec<(f32, f32, f32)> = (0..60)
            .map(|_| {
                let a = rng.range(0.0, TAU);
                let r = rng.range(wall_r + 15.0, wall_r + 120.0);
                let (x, z) = (plaza.0 + r * a.cos(), plaza.1 + r * a.sin());
                (x, z, patch.height_at(x, z))
            })
            .collect();
        spots.sort_by(|a, b| b.2.total_cmp(&a.2));
        let want = if view.city { 3 } else { 1 };
        let mut n = 0;
        for (x, z, _) in spots {
            if n < want && put(&mut occ, &mut buildings, m, Kind::Windmill, x, z, rng.range(0.0, TAU)) {
                n += 1;
            }
        }
    }

    // Champs en lanières autour.
    let mut fields = Vec::new();
    if view.crop.is_some() {
        let palette: [[f32; 3]; 5] = [[0.78, 0.66, 0.38], [0.55, 0.6, 0.32], [0.6, 0.47, 0.32], [0.83, 0.76, 0.48], [0.47, 0.55, 0.3]];
        let reach = (wall_r + 120.0 + 0.6 * radius).min(PATCH_SIZE_M / 2.0 - 60.0);
        let want = if view.city { 140 } else { 50 };
        let mut tries = 0;
        while fields.len() < want && tries < want * 12 {
            tries += 1;
            let a = rng.range(0.0, TAU);
            let r = rng.range(wall_r + 18.0, reach);
            let (x, z) = (plaza.0 + r * a.cos(), plaza.1 + r * a.sin());
            // Lanières rayonnantes, tournées vers le bourg.
            let h = a + rng.range(-0.25, 0.25) + if rng.unit() < 0.5 { 0.0 } else { PI / 2.0 };
            let (l, w) = (rng.range(40.0, 90.0), rng.range(18.0, 40.0));
            if !occ.free(x, z, h, l / 2.0 + 2.0, w / 2.0 + 2.0) || ground_of(patch, x, z, h, l / 2.0, w / 2.0).is_none() {
                continue;
            }
            occ.mark(x, z, h, l / 2.0 + 2.0, w / 2.0 + 2.0);
            let c = palette[(rng.next_u64() % palette.len() as u64) as usize];
            fields.push(Field { centre: (x, z), heading: h, length: l, width: w, colour: c });
        }
    }

    Town { view: view.clone(), style, models, buildings, plaza, plaza_half, plaza_heading: theta, streets, fields, radius: built, seed }
}

impl Town {
    /// Le « campement » des gens de la ville : la place, où l'on échange.
    pub fn folk_camp(&self) -> Camp {
        let folk = if self.view.city { (20.0 + (self.view.pop as f32).log10() * 8.0).clamp(24.0, 70.0) } else { 14.0 };
        let folk = if self.view.city && self.view.pop < 0.5 * self.view.peak { folk * 0.4 } else { folk };
        Camp {
            people: self.view.name.clone(),
            centre: self.plaza,
            radius: self.plaza_half - 1.0,
            fixed: Vec::new(),
            tools: Vec::new(),
            ornament: false,
            spear: None,
            shares: [0.0, 0.08, 0.3, 0.0],
            folk: folk as usize,
            seed: self.seed,
        }
    }

    /// Nombre de bâtiments de chaque sorte, dans l'ordre de `Kind::ALL`.
    pub fn counts(&self) -> [usize; 12] {
        let mut out = [0; 12];
        for b in &self.buildings {
            let k = self.models[b.model as usize].kind;
            out[Kind::ALL.iter().position(|x| *x == k).unwrap_or(0)] += 1;
        }
        out
    }

    /// Sol de la ville : rues, place et champs posés sur le relief, un peu
    /// au-dessus pour ne pas se mêler au terrain. Sommets, normales,
    /// couleurs, triangles (sens direct vu d'en haut).
    pub fn ground_mesh(&self, patch: &Patch) -> TownGround {
        let mut g = TownGround::default();
        let era = self.view.era;
        let street = match era {
            Era::Village | Era::Brick => [0.64, 0.55, 0.42],
            Era::Monumental | Era::Classical => [0.7, 0.67, 0.6],
            Era::Industrial => [0.52, 0.5, 0.47],
            Era::Modern => [0.42, 0.42, 0.41],
        };
        let paved = match era {
            Era::Village => [0.6, 0.52, 0.4],
            _ => [0.76, 0.73, 0.66],
        };
        for s in &self.streets {
            for w in s.points.windows(2) {
                g.ribbon(patch, w[0], w[1], s.width, street);
            }
        }
        let p = self.plaza;
        let d = self.plaza_half * std::f32::consts::SQRT_2;
        let h = self.plaza_heading;
        g.rect(patch, p, h + PI / 4.0, d, d, paved, 1);
        for f in &self.fields {
            g.rect(patch, f.centre, f.heading, f.length / 2.0, f.width / 2.0, f.colour, 7);
        }
        g
    }
}

/// Le sol de la ville, prêt pour un maillage.
#[derive(Clone, Debug, Default)]
pub struct TownGround {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub colours: Vec<[f32; 3]>,
    pub indices: Vec<u32>,
}

const LIFT: f32 = 0.35;

fn normal_at(patch: &Patch, x: f32, z: f32) -> [f32; 3] {
    let e = 4.0;
    let dx = (patch.height_at(x + e, z) - patch.height_at(x - e, z)) / (2.0 * e);
    let dz = (patch.height_at(x, z + e) - patch.height_at(x, z - e)) / (2.0 * e);
    let k = 1.0 / (1.0 + dx * dx + dz * dz).sqrt();
    [-dx * k, k, -dz * k]
}

impl TownGround {
    fn vertex(&mut self, patch: &Patch, x: f32, z: f32, c: [f32; 3]) -> u32 {
        self.positions.push([x, patch.height_at(x, z) + LIFT, z]);
        self.normals.push(normal_at(patch, x, z));
        self.colours.push(c);
        (self.positions.len() - 1) as u32
    }

    /// Un quadrilatère a b c d (dans l'ordre du tour), tourné vers le haut.
    fn quad(&mut self, a: u32, b: u32, c: u32, d: u32) {
        let p = |i: u32| self.positions[i as usize];
        let (pa, pb, pc) = (p(a), p(b), p(c));
        let up = (pb[2] - pa[2]) * (pc[0] - pa[0]) - (pb[0] - pa[0]) * (pc[2] - pa[2]);
        if up >= 0.0 {
            self.indices.extend_from_slice(&[a, b, c, a, c, d]);
        } else {
            self.indices.extend_from_slice(&[a, c, b, a, d, c]);
        }
    }

    fn ribbon(&mut self, patch: &Patch, a: (f32, f32), b: (f32, f32), width: f32, c: [f32; 3]) {
        let (dx, dz) = (b.0 - a.0, b.1 - a.1);
        let len = dx.hypot(dz);
        if len < 0.5 {
            return;
        }
        let (nx, nz) = (-dz / len * width / 2.0, dx / len * width / 2.0);
        let n = (len / 6.0).ceil() as usize;
        let mut prev: Option<(u32, u32)> = None;
        for i in 0..=n {
            let t = i as f32 / n as f32;
            let (x, z) = (a.0 + dx * t, a.1 + dz * t);
            let l = self.vertex(patch, x + nx, z + nz, c);
            let r = self.vertex(patch, x - nx, z - nz, c);
            if let Some((pl, pr)) = prev {
                self.quad(pl, pr, r, l);
            }
            prev = Some((l, r));
        }
    }

    /// Rectangle drapé ; `strips` lanières alternées en travers (sillons).
    #[allow(clippy::too_many_arguments)]
    fn rect(&mut self, patch: &Patch, centre: (f32, f32), h: f32, hl: f32, hw: f32, c: [f32; 3], strips: usize) {
        let (s, co) = h.sin_cos();
        let at = |u: f32, v: f32| (centre.0 + u * co - v * s, centre.1 + u * s + v * co);
        let nu = ((2.0 * hl / 8.0).ceil() as usize).max(1);
        let nv = strips.max(1);
        for j in 0..nv {
            let k = if j % 2 == 0 { 1.0 } else { 0.88 };
            let col = [c[0] * k, c[1] * k, c[2] * k];
            let (v0, v1) = (-hw + 2.0 * hw * j as f32 / nv as f32, -hw + 2.0 * hw * (j + 1) as f32 / nv as f32);
            let mut prev: Option<(u32, u32)> = None;
            for i in 0..=nu {
                let u = -hl + 2.0 * hl * i as f32 / nu as f32;
                let (x0, z0) = at(u, v0);
                let (x1, z1) = at(u, v1);
                let a = self.vertex(patch, x0, z0, col);
                let b = self.vertex(patch, x1, z1, col);
                if let Some((pa, pb)) = prev {
                    self.quad(pa, a, b, pb);
                }
                prev = Some((a, b));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ground::Site;

    fn patch() -> Patch {
        Patch::new(
            Site {
                cell: 3,
                elevation_m: 200.0,
                is_ocean: false,
                temperature_k: 290.0,
                ice_cover: 0.0,
                roughness_m: 60.0,
                life_rgb: None,
                biomass_per_m2: 0.3,
                sky: [0.8, 0.85, 0.9],
            },
            5,
        )
    }

    fn town(era: Era, pop: f64, peak: f64) -> TownView {
        TownView {
            cell: 3,
            name: "Ura".into(),
            society: "Ki".into(),
            society_id: 2,
            label: Label::State,
            era,
            culture: 77,
            accent: [0.6, 0.2, 0.2],
            city: true,
            capital: true,
            pop,
            peak,
            fortified: true,
            grid: false,
            windmill: true,
            granary: true,
            market: true,
            temple: true,
            crop: Some(4),
            herd: false,
        }
    }

    fn count(t: &Town, k: Kind) -> usize {
        t.counts()[Kind::ALL.iter().position(|x| *x == k).unwrap()]
    }

    #[test]
    fn a_city_has_its_plaza_streets_houses_walls_and_fields() {
        let p = patch();
        let t = layout(&town(Era::Monumental, 20_000.0, 20_000.0), &p, 3);
        assert_eq!(count(&t, Kind::Temple), 1);
        assert_eq!(count(&t, Kind::Palace), 1);
        assert_eq!(count(&t, Kind::Market), 1);
        assert!(count(&t, Kind::House) > 70, "{:?}", t.counts());
        assert!(count(&t, Kind::WallTower) >= 8);
        assert!(count(&t, Kind::WallSegment) > 20);
        assert!(count(&t, Kind::Granary) >= 1);
        assert_eq!(count(&t, Kind::Ruin), 0);
        assert!(t.fields.len() > 40, "{}", t.fields.len());
        assert!(t.buildings.len() <= MAX_BUILDINGS);
        assert!(t.models.len() < 24, "{}", t.models.len());
        assert_eq!(t, layout(&town(Era::Monumental, 20_000.0, 20_000.0), &p, 3));
        // En damier aussi, le temple et le palais trouvent leur place.
        let grid = layout(&TownView { grid: true, ..town(Era::Classical, 60_000.0, 60_000.0) }, &p, 4);
        assert_eq!((count(&grid, Kind::Temple), count(&grid, Kind::Palace)), (1, 1), "{:?}", grid.counts());
        let g = t.ground_mesh(&p);
        assert!(!g.indices.is_empty() && g.indices.len().is_multiple_of(3));
        // Toutes les faces du sol regardent en haut.
        for tri in g.indices.chunks(3) {
            let [a, b, c] = [tri[0], tri[1], tri[2]].map(|i| g.positions[i as usize]);
            let e1 = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
            let e2 = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
            assert!(e1[2] * e2[0] - e1[0] * e2[2] >= -1e-3);
        }
    }

    #[test]
    fn buildings_do_not_overlap() {
        let p = patch();
        let t = layout(&town(Era::Classical, 60_000.0, 60_000.0), &p, 8);
        let solid: Vec<&Building> =
            t.buildings.iter().filter(|b| !matches!(t.models[b.model as usize].kind, Kind::WallSegment | Kind::WallTower)).collect();
        for (i, a) in solid.iter().enumerate() {
            for b in &solid[i + 1..] {
                assert!((a.x - b.x).hypot(a.z - b.z) > 3.0, "{a:?} {b:?}");
            }
        }
    }

    #[test]
    fn a_fallen_city_keeps_its_plan_in_ruins_and_modern_towns_lose_their_walls() {
        let p = patch();
        let fallen = layout(&town(Era::Monumental, 2_000.0, 40_000.0), &p, 3);
        assert!(count(&fallen, Kind::Ruin) > count(&fallen, Kind::House), "{:?}", fallen.counts());
        let modern = layout(&town(Era::Modern, 400_000.0, 400_000.0), &p, 3);
        assert_eq!(count(&modern, Kind::WallTower), 0);
        assert!(count(&modern, Kind::Factory) >= 2, "{:?}", modern.counts());
        assert!(count(&modern, Kind::Block) > 20, "{:?}", modern.counts());
    }

    #[test]
    fn a_village_is_small() {
        let p = patch();
        let v = TownView {
            city: false,
            capital: false,
            fortified: false,
            market: false,
            pop: 300.0,
            peak: 0.0,
            ..town(Era::Village, 0.0, 0.0)
        };
        let t = layout(&v, &p, 1);
        assert_eq!(count(&t, Kind::Palace) + count(&t, Kind::Market) + count(&t, Kind::WallTower), 0);
        let houses = count(&t, Kind::House);
        assert!((8..=40).contains(&houses), "{:?}", t.counts());
        assert!(t.folk_camp().folk >= 10);
    }

    #[test]
    fn the_era_follows_the_techniques() {
        let mut k = Known::default();
        assert_eq!(era_of(&k), Era::Village);
        for (key, era) in [
            ("brique_crue", Era::Brick),
            ("ecriture", Era::Monumental),
            ("urbanisme", Era::Classical),
            ("machine_vapeur", Era::Industrial),
            ("acier_bessemer", Era::Industrial),
            ("ciment_portland", Era::Modern),
        ] {
            k.set(catalog().id(key).unwrap());
            assert_eq!(era_of(&k), era, "{key}");
        }
    }
}
