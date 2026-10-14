//! Palier 3 du rendu, seconde part (étape 7, document « Rendu du vivant »,
//! palier 3b) : les bâtiments des sociétés, tirés d'une grammaire de formes.
//! Un bâtiment est une pile de volumes simples (bloc, toit à deux pentes ou
//! en croupe, terrasse, dôme, gradins, colonnes, cheminée), percés de
//! fenêtres à l'encre. Les techniques connues fixent l'époque (matériaux,
//! hauteurs, plan) et la culture fixe le style (forme des toits, du temple,
//! couleurs), d'où des villes qui se ressemblent dans une société et
//! diffèrent d'une société à l'autre.
//!
//! Repère : celui des objets ([`crate::artefact`]) : y vers le haut, x vers
//! l'avant (la porte), en mètres, base à y = 0 et centrée. Chaque bâtiment
//! descend un peu sous le sol pour couvrir la pente.
//!
//! [Simplification] Architecture illustrative : les formes disent l'époque
//! et la culture d'un coup d'œil, elles ne sortent pas d'un calcul de
//! structure ni d'un climat.

use crate::body::Mesh;
use crate::canvas::{mix64, DrawRng};
use crate::facet::{add, cross, mul, norm, sub, Builder, P};
use std::f32::consts::TAU;

/// Époque des bâtiments, lue dans les techniques connues.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Era {
    /// Village : bois, torchis et chaume.
    Village,
    /// Brique crue : maisons à terrasse.
    Brick,
    /// Architecture monumentale : pierre de taille, remparts.
    Monumental,
    /// Béton et urbanisme : tuiles, colonnes, dômes, étages.
    Classical,
    /// Vapeur : brique cuite, immeubles, usines à cheminée.
    Industrial,
    /// Acier et ciment : immeubles de béton, tours de verre.
    Modern,
}

impl Era {
    pub const ALL: [Era; 6] = [Era::Village, Era::Brick, Era::Monumental, Era::Classical, Era::Industrial, Era::Modern];

    pub fn label(self, fr: bool) -> &'static str {
        match (self, fr) {
            (Era::Village, true) => "village de bois et de chaume",
            (Era::Brick, true) => "brique crue",
            (Era::Monumental, true) => "pierre de taille",
            (Era::Classical, true) => "tuiles, colonnes et dômes",
            (Era::Industrial, true) => "brique et vapeur",
            (Era::Modern, true) => "béton, acier et verre",
            (Era::Village, false) => "wood and thatch village",
            (Era::Brick, false) => "mudbrick",
            (Era::Monumental, false) => "dressed stone",
            (Era::Classical, false) => "tiles, columns and domes",
            (Era::Industrial, false) => "brick and steam",
            (Era::Modern, false) => "concrete, steel and glass",
        }
    }
}

/// Ce qu'on bâtit.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Kind {
    House,
    /// Maison à étages des villes denses (immeuble, tour aux temps
    /// modernes).
    Block,
    Granary,
    Temple,
    Palace,
    Market,
    WallSegment,
    WallTower,
    Workshop,
    Factory,
    Windmill,
    /// Maison abandonnée : murs sans toit.
    Ruin,
}

impl Kind {
    pub const ALL: [Kind; 12] = [
        Kind::House,
        Kind::Block,
        Kind::Granary,
        Kind::Temple,
        Kind::Palace,
        Kind::Market,
        Kind::WallSegment,
        Kind::WallTower,
        Kind::Workshop,
        Kind::Factory,
        Kind::Windmill,
        Kind::Ruin,
    ];

    pub fn label(self, fr: bool) -> &'static str {
        match (self, fr) {
            (Kind::House, true) => "maison",
            (Kind::Block, true) => "immeuble",
            (Kind::Granary, true) => "grenier",
            (Kind::Temple, true) => "temple",
            (Kind::Palace, true) => "palais",
            (Kind::Market, true) => "halle",
            (Kind::WallSegment, true) => "rempart",
            (Kind::WallTower, true) => "tour du rempart",
            (Kind::Workshop, true) => "atelier",
            (Kind::Factory, true) => "usine",
            (Kind::Windmill, true) => "moulin à vent",
            (Kind::Ruin, true) => "ruine",
            (Kind::House, false) => "house",
            (Kind::Block, false) => "apartment block",
            (Kind::Granary, false) => "granary",
            (Kind::Temple, false) => "temple",
            (Kind::Palace, false) => "palace",
            (Kind::Market, false) => "market hall",
            (Kind::WallSegment, false) => "city wall",
            (Kind::WallTower, false) => "wall tower",
            (Kind::Workshop, false) => "workshop",
            (Kind::Factory, false) => "factory",
            (Kind::Windmill, false) => "windmill",
            (Kind::Ruin, false) => "ruin",
        }
    }
}

/// Forme des toits d'une culture.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Roof {
    /// Deux pentes.
    Gable,
    /// Quatre pentes (croupe).
    Hip,
    /// Terrasse à parapet.
    Flat,
    /// Toits relevés en gradins (pagode).
    Tiered,
}

/// Forme du temple d'une culture.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TempleForm {
    /// Pyramide à degrés (ziggourat, temple méso-américain).
    Steps,
    /// Colonnade sous un toit à deux pentes.
    Colonnade,
    /// Salle sous plusieurs toits superposés.
    Pagoda,
    /// Salle carrée sous un dôme.
    Dome,
}

/// Style d'une culture à une époque : ce que la grammaire choisit une fois
/// pour toute la ville.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Style {
    pub era: Era,
    pub roof: Roof,
    pub temple: TempleForm,
    /// Maisons rondes (village seulement).
    pub round: bool,
    pub wall: [f32; 3],
    pub roof_colour: [f32; 3],
    /// Couleur de la culture (bannières, dôme, frise).
    pub accent: [f32; 3],
    /// Étages des maisons de ville.
    pub storeys: u32,
}

/// Couleurs des matériaux, sRGB, dans la palette de l'Atlas.
pub const THATCH: [f32; 3] = [0.66, 0.56, 0.33];
pub const WATTLE: [f32; 3] = [0.6, 0.49, 0.36];
pub const MUDBRICK: [f32; 3] = [0.74, 0.62, 0.45];
pub const STONE: [f32; 3] = [0.71, 0.68, 0.61];
pub const LIMESTONE: [f32; 3] = [0.84, 0.81, 0.72];
pub const TILE: [f32; 3] = [0.66, 0.34, 0.23];
pub const SLATE: [f32; 3] = [0.37, 0.39, 0.43];
pub const BRICK: [f32; 3] = [0.6, 0.31, 0.23];
pub const CONCRETE: [f32; 3] = [0.68, 0.68, 0.65];
pub const GLASS: [f32; 3] = [0.43, 0.52, 0.58];
pub const TIMBER: [f32; 3] = [0.42, 0.31, 0.21];
pub const INK: [f32; 3] = [0.2, 0.18, 0.16];
pub const COPPER: [f32; 3] = [0.42, 0.6, 0.52];
pub const GILT: [f32; 3] = [0.8, 0.63, 0.29];

fn pick<T: Copy>(rng: &mut DrawRng, list: &[T]) -> T {
    list[(rng.next_u64() % list.len() as u64) as usize]
}

impl Style {
    /// Style de la culture `culture` (graine de la société ou du peuple) à
    /// l'époque `era`, avec sa couleur. La forme des toits et du temple
    /// tient d'une époque à l'autre : une culture garde son allure.
    pub fn of(era: Era, culture: u64, accent: [f32; 3]) -> Style {
        let mut rng = DrawRng::new(mix64(culture ^ 0x7374_796C));
        let roof = pick(&mut rng, &[Roof::Gable, Roof::Gable, Roof::Hip, Roof::Flat, Roof::Tiered]);
        let temple = pick(&mut rng, &[TempleForm::Steps, TempleForm::Colonnade, TempleForm::Pagoda, TempleForm::Dome]);
        let round = rng.range(0.0, 1.0) < 0.5;
        let warm = rng.range(0.0, 1.0) < 0.5;
        let (wall, roof_colour, roof, storeys) = match era {
            Era::Village => (WATTLE, THATCH, if roof == Roof::Flat { Roof::Gable } else { roof }, 1),
            Era::Brick => (MUDBRICK, MUDBRICK, Roof::Flat, 1),
            Era::Monumental => (if warm { MUDBRICK } else { STONE }, if roof == Roof::Flat { MUDBRICK } else { THATCH }, roof, 1),
            Era::Classical => (if warm { LIMESTONE } else { STONE }, if roof == Roof::Flat { LIMESTONE } else { TILE }, roof, 2),
            Era::Industrial => (BRICK, SLATE, if roof == Roof::Flat { Roof::Flat } else { Roof::Gable }, 3),
            Era::Modern => (CONCRETE, CONCRETE, Roof::Flat, 5),
        };
        Style { era, roof, temple, round: round && era == Era::Village, wall, roof_colour, accent, storeys }
    }
}

/// Faces orientées : chaque quadrilatère est retourné au besoin pour que sa
/// normale aille dans le sens donné.
struct Shape<'a> {
    b: &'a mut Builder,
}

impl Shape<'_> {
    fn face(&mut self, a: P, b: P, c: P, d: P, out: P, col: [f32; 3]) {
        let n = cross(sub(b, a), sub(c, a));
        if n[0] * out[0] + n[1] * out[1] + n[2] * out[2] >= 0.0 {
            self.b.quad(a, b, c, d, col);
        } else {
            self.b.quad(a, d, c, b, col);
        }
    }

    fn triangle(&mut self, a: P, b: P, c: P, out: P, col: [f32; 3]) {
        let n = cross(sub(b, a), sub(c, a));
        if n[0] * out[0] + n[1] * out[1] + n[2] * out[2] >= 0.0 {
            self.b.tri(a, b, c, col);
        } else {
            self.b.tri(a, c, b, col);
        }
    }

    /// Bloc de `x0..x1` × `y0..y1` × `z0..z1`, sans dessous ; `top` :
    /// fermé dessus.
    fn cuboid(&mut self, lo: P, hi: P, col: [f32; 3], top: bool) {
        let [x0, y0, z0] = lo;
        let [x1, y1, z1] = hi;
        self.face([x1, y0, z0], [x1, y0, z1], [x1, y1, z1], [x1, y1, z0], [1.0, 0.0, 0.0], col);
        self.face([x0, y0, z0], [x0, y0, z1], [x0, y1, z1], [x0, y1, z0], [-1.0, 0.0, 0.0], shade(col, 0.88));
        self.face([x0, y0, z1], [x1, y0, z1], [x1, y1, z1], [x0, y1, z1], [0.0, 0.0, 1.0], shade(col, 0.94));
        self.face([x0, y0, z0], [x1, y0, z0], [x1, y1, z0], [x0, y1, z0], [0.0, 0.0, -1.0], shade(col, 0.9));
        if top {
            self.face([x0, y1, z0], [x1, y1, z0], [x1, y1, z1], [x0, y1, z1], [0.0, 1.0, 0.0], col);
        }
    }

    /// Toit à deux pentes sur le rectangle `x0..x1` × `z0..z1` à la hauteur
    /// `y`, faîte le long de x, avec un débord.
    #[allow(clippy::too_many_arguments)]
    fn gable(&mut self, x0: f32, x1: f32, z0: f32, z1: f32, y: f32, h: f32, col: [f32; 3], wall: [f32; 3]) {
        let o = 0.35;
        let zm = (z0 + z1) / 2.0;
        let (xa, xb, za, zb) = (x0 - o, x1 + o, z0 - o, z1 + o);
        let ya = y - o * h / ((z1 - z0) / 2.0);
        self.face([xa, ya, za], [xb, ya, za], [xb, y + h, zm], [xa, y + h, zm], [0.0, 0.6, -1.0], shade(col, 0.86));
        self.face([xa, ya, zb], [xb, ya, zb], [xb, y + h, zm], [xa, y + h, zm], [0.0, 0.6, 1.0], col);
        // Pignons.
        self.triangle([x0, y, z0], [x0, y, z1], [x0, y + h, zm], [-1.0, 0.0, 0.0], shade(wall, 0.88));
        self.triangle([x1, y, z0], [x1, y, z1], [x1, y + h, zm], [1.0, 0.0, 0.0], wall);
    }

    /// Toit en croupe : quatre pans vers un faîte court.
    #[allow(clippy::too_many_arguments)]
    fn hip(&mut self, x0: f32, x1: f32, z0: f32, z1: f32, y: f32, h: f32, col: [f32; 3]) {
        let o = 0.35;
        let (xa, xb, za, zb) = (x0 - o, x1 + o, z0 - o, z1 + o);
        let zm = (z0 + z1) / 2.0;
        let inset = ((z1 - z0) / 2.0).min((x1 - x0) / 2.0 - 0.01);
        let (ra, rb) = (x0 + inset, x1 - inset);
        let top_a = [ra, y + h, zm];
        let top_b = [rb, y + h, zm];
        self.face([xa, y, za], [xb, y, za], top_b, top_a, [0.0, 0.6, -1.0], shade(col, 0.86));
        self.face([xa, y, zb], [xb, y, zb], top_b, top_a, [0.0, 0.6, 1.0], col);
        self.triangle([xa, y, za], [xa, y, zb], top_a, [-1.0, 0.6, 0.0], shade(col, 0.8));
        self.triangle([xb, y, za], [xb, y, zb], top_b, [1.0, 0.6, 0.0], shade(col, 0.95));
    }

    /// Terrasse : parapet bas autour du toit plat.
    fn parapet(&mut self, x0: f32, x1: f32, z0: f32, z1: f32, y: f32, col: [f32; 3]) {
        let (t, h) = (0.3, 0.6);
        self.cuboid([x0, y, z0], [x1, y + h, z0 + t], col, true);
        self.cuboid([x0, y, z1 - t], [x1, y + h, z1], col, true);
        self.cuboid([x0, y, z0], [x0 + t, y + h, z1], col, true);
        self.cuboid([x1 - t, y, z0], [x1, y + h, z1], col, true);
    }

    /// Toit pointu (cône à facettes) d'une maison ronde ou d'une tour.
    fn cone(&mut self, c: P, r: f32, h: f32, sides: usize, col: [f32; 3]) {
        let apex = add(c, [0.0, h, 0.0]);
        for k in 0..sides {
            let (t0, t1) = (TAU * k as f32 / sides as f32, TAU * (k + 1) as f32 / sides as f32);
            let a = add(c, [r * t0.cos(), 0.0, r * t0.sin()]);
            let b = add(c, [r * t1.cos(), 0.0, r * t1.sin()]);
            let mid = mul(add(sub(a, c), sub(b, c)), 0.5);
            self.triangle(a, b, apex, add(mid, [0.0, r, 0.0]), if k % 2 == 0 { col } else { shade(col, 0.9) });
        }
    }

    /// Dôme : calotte à facettes.
    fn dome(&mut self, c: P, r: f32, col: [f32; 3]) {
        let (rings, sides) = (4, 12);
        for i in 0..rings {
            let (p0, p1) = (TAU / 4.0 * i as f32 / rings as f32, TAU / 4.0 * (i + 1) as f32 / rings as f32);
            for k in 0..sides {
                let (t0, t1) = (TAU * k as f32 / sides as f32, TAU * (k + 1) as f32 / sides as f32);
                let pt = |p: f32, t: f32| add(c, [r * p.cos() * t.cos(), r * p.sin(), r * p.cos() * t.sin()]);
                let (a, b, cc, d) = (pt(p0, t0), pt(p0, t1), pt(p1, t1), pt(p1, t0));
                let out = sub(mul(add(a, cc), 0.5), c);
                if i + 1 == rings {
                    self.triangle(a, b, cc, out, col);
                } else {
                    self.face(a, b, cc, d, out, if k % 2 == 0 { col } else { shade(col, 0.92) });
                }
            }
        }
    }

    /// Cylindre fermé (colonne, cheminée, silo).
    fn cylinder(&mut self, base: P, r: f32, h: f32, sides: usize, col: [f32; 3]) {
        self.b.tube(base, add(base, [0.0, h, 0.0]), r, r, sides, col);
    }

    /// Fenêtres d'une façade : rangées par étage, à l'encre (au verre aux
    /// temps modernes). La façade va de `a` à `b` (au sol), normale `n`.
    #[allow(clippy::too_many_arguments)]
    fn windows(&mut self, a: P, b: P, n: P, storeys: u32, storey_h: f32, every: f32, size: (f32, f32), col: [f32; 3], skip_door: bool) {
        let len = (sub(b, a)[0].powi(2) + sub(b, a)[2].powi(2)).sqrt();
        let cols = ((len / every).floor() as usize).max(1);
        let u = norm(sub(b, a));
        let off = mul(n, 0.04);
        for s in 0..storeys {
            for k in 0..cols {
                if skip_door && s == 0 && k == cols / 2 {
                    continue;
                }
                let along = (k as f32 + 0.5) * len / cols as f32;
                let y = s as f32 * storey_h + storey_h * 0.45;
                let c = add(add(a, mul(u, along)), add([0.0, y, 0.0], off));
                let (w, h) = (size.0 / 2.0, size.1 / 2.0);
                let p = |du: f32, dv: f32| add(c, add(mul(u, du), [0.0, dv, 0.0]));
                self.face(p(-w, -h), p(w, -h), p(w, h), p(-w, h), n, col);
            }
        }
    }

    /// Porte au milieu de la façade avant (+x).
    fn door(&mut self, x: f32, w: f32, h: f32) {
        let x = x + 0.05;
        self.face([x, 0.0, -w / 2.0], [x, 0.0, w / 2.0], [x, h, w / 2.0], [x, h, -w / 2.0], [1.0, 0.0, 0.0], INK);
    }
}

fn shade(c: [f32; 3], k: f32) -> [f32; 3] {
    [c[0] * k, c[1] * k, c[2] * k]
}

/// Emprise au sol (longueur selon x, largeur selon z) d'un bâtiment, pour
/// la mise en place des rues.
pub fn footprint(kind: Kind, style: &Style) -> (f32, f32) {
    let era = style.era;
    match kind {
        Kind::House | Kind::Ruin => match era {
            Era::Village => {
                if style.round {
                    (6.5, 6.5)
                } else {
                    (7.0, 5.5)
                }
            }
            Era::Brick | Era::Monumental => (8.0, 7.0),
            Era::Classical => (10.0, 8.0),
            Era::Industrial => (7.0, 11.0),
            Era::Modern => (11.0, 9.0),
        },
        Kind::Block => match era {
            Era::Modern => (22.0, 22.0),
            Era::Industrial => (16.0, 14.0),
            _ => (13.0, 11.0),
        },
        Kind::Granary => (6.0, 6.0),
        Kind::Temple => match style.temple {
            TempleForm::Steps => (34.0, 34.0),
            TempleForm::Colonnade => (34.0, 20.0),
            TempleForm::Pagoda => (24.0, 24.0),
            TempleForm::Dome => (28.0, 28.0),
        },
        Kind::Palace => (44.0, 40.0),
        Kind::Market => (24.0, 14.0),
        Kind::WallSegment => (10.0, 2.6),
        Kind::WallTower => (7.0, 7.0),
        Kind::Workshop => (12.0, 8.0),
        Kind::Factory => (36.0, 22.0),
        Kind::Windmill => (7.0, 7.0),
    }
}

/// Construit le bâtiment `kind` de style `style`, tiré de `seed` (hauteurs,
/// fenêtres, détails).
pub fn mesh(kind: Kind, style: &Style, seed: u64) -> Mesh {
    let mut rng = DrawRng::new(mix64(seed ^ (kind as u64).wrapping_mul(0x9E37_79B9) ^ (style.era as u64) << 40));
    let mut b = Builder::default();
    let (l, w) = footprint(kind, style);
    let (x0, x1, z0, z1) = (-l / 2.0, l / 2.0, -w / 2.0, w / 2.0);
    let sink = -0.6;
    let era = style.era;
    let glass = if era == Era::Modern { GLASS } else { INK };
    {
        let mut s = Shape { b: &mut b };
        match kind {
            Kind::House | Kind::Ruin => {
                let ruin = kind == Kind::Ruin;
                let storeys = match era {
                    Era::Village | Era::Brick | Era::Monumental => 1,
                    Era::Classical => 1 + (rng.next_u64() % 2) as u32,
                    Era::Industrial => 2 + (rng.next_u64() % 2) as u32,
                    Era::Modern => 1 + (rng.next_u64() % 2) as u32,
                };
                let sh = if era == Era::Village { 2.4 } else { 3.0 };
                let h = storeys as f32 * sh;
                let wall = if ruin { shade(STONE, 0.85) } else { style.wall };
                if era == Era::Village && style.round {
                    let r = l / 2.0;
                    let wh = if ruin { 1.0 } else { 2.2 };
                    s.b.tube([0.0, sink, 0.0], [0.0, wh, 0.0], r, r, 10, wall);
                    if !ruin {
                        s.cone([0.0, wh - 0.3, 0.0], r + 0.6, rng.range(2.6, 3.4), 10, style.roof_colour);
                        s.door(r - 0.02, 1.0, 1.7);
                    }
                } else if ruin {
                    // Murs écroulés à des hauteurs inégales.
                    let t = 0.5;
                    for (k, (lo, hi)) in [
                        ([x0, sink, z0], [x1, 0.0, z0 + t]),
                        ([x0, sink, z1 - t], [x1, 0.0, z1]),
                        ([x0, sink, z0], [x0 + t, 0.0, z1]),
                        ([x1 - t, sink, z0], [x1, 0.0, z1]),
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        let top = h * rng.range(0.2, 0.75) * if k == 0 { 0.6 } else { 1.0 };
                        s.cuboid(lo, [hi[0], top, hi[2]], wall, true);
                    }
                } else {
                    s.cuboid([x0, sink, z0], [x1, h, z1], wall, style.roof == Roof::Flat);
                    match style.roof {
                        Roof::Flat => s.parapet(x0, x1, z0, z1, h, wall),
                        Roof::Gable => s.gable(x0, x1, z0, z1, h, w * 0.45, style.roof_colour, wall),
                        Roof::Hip => s.hip(x0, x1, z0, z1, h, w * 0.4, style.roof_colour),
                        Roof::Tiered => {
                            s.hip(x0, x1, z0, z1, h, w * 0.3, style.roof_colour);
                            s.hip(x0 + 1.2, x1 - 1.2, z0 + 1.2, z1 - 1.2, h + w * 0.18, w * 0.25, shade(style.roof_colour, 0.92));
                        }
                    }
                    let win = if era == Era::Village { 0 } else { storeys };
                    s.windows([x1, 0.0, z1], [x1, 0.0, z0], [1.0, 0.0, 0.0], win, sh, 3.0, (0.8, 1.1), glass, true);
                    s.windows([x0, 0.0, z1], [x1, 0.0, z1], [0.0, 0.0, 1.0], win, sh, 3.0, (0.8, 1.1), glass, false);
                    s.windows([x0, 0.0, z0], [x1, 0.0, z0], [0.0, 0.0, -1.0], win, sh, 3.0, (0.8, 1.1), glass, false);
                    s.door(x1, 1.1, 2.0);
                }
            }
            Kind::Block => {
                let storeys = match era {
                    Era::Modern => 6 + (rng.next_u64() % 22) as u32,
                    Era::Industrial => 4 + (rng.next_u64() % 3) as u32,
                    _ => style.storeys + 1 + (rng.next_u64() % 2) as u32,
                };
                let sh = 3.0;
                let h = storeys as f32 * sh;
                let flat = era == Era::Modern || style.roof == Roof::Flat;
                s.cuboid([x0, sink, z0], [x1, h, z1], style.wall, flat);
                if flat {
                    s.parapet(x0, x1, z0, z1, h, style.wall);
                } else {
                    s.hip(x0, x1, z0, z1, h, w * 0.35, style.roof_colour);
                }
                let (every, size) = if era == Era::Modern { (2.0, (1.6, 2.0)) } else { (3.2, (1.0, 1.5)) };
                for (a, bb, n) in [
                    ([x1, 0.0, z1], [x1, 0.0, z0], [1.0, 0.0, 0.0]),
                    ([x0, 0.0, z0], [x0, 0.0, z1], [-1.0, 0.0, 0.0]),
                    ([x0, 0.0, z1], [x1, 0.0, z1], [0.0, 0.0, 1.0]),
                    ([x0, 0.0, z0], [x1, 0.0, z0], [0.0, 0.0, -1.0]),
                ] {
                    s.windows(a, bb, n, storeys, sh, every, size, glass, n[0] > 0.5);
                }
                s.door(x1, 1.6, 2.4);
            }
            Kind::Granary => {
                if era <= Era::Monumental {
                    // Grenier sur pilotis.
                    for (px, pz) in [(-2.0, -2.0), (2.0, -2.0), (2.0, 2.0), (-2.0, 2.0)] {
                        s.cylinder([px, sink, pz], 0.18, 1.6 - sink, 5, TIMBER);
                    }
                    s.b.tube([0.0, 1.6, 0.0], [0.0, 3.6, 0.0], 2.6, 2.6, 10, if era == Era::Village { WATTLE } else { MUDBRICK });
                    s.cone([0.0, 3.6, 0.0], 3.1, 2.2, 10, style.roof_colour);
                } else {
                    s.cylinder([0.0, sink, 0.0], 2.8, 9.0 - sink, 12, if era >= Era::Industrial { CONCRETE } else { style.wall });
                    s.dome([0.0, 9.0, 0.0], 2.8, style.roof_colour);
                }
            }
            Kind::Temple => temple(&mut s, style, &mut rng, (x0, x1, z0, z1), sink),
            Kind::Palace => {
                let h = if era >= Era::Industrial { 14.0 } else { 9.0 };
                // Cour carrée : quatre ailes.
                let t = 9.0;
                s.cuboid([x0, sink, z0], [x1, h, z0 + t], style.wall, style.roof == Roof::Flat);
                s.cuboid([x0, sink, z1 - t], [x1, h, z1], style.wall, style.roof == Roof::Flat);
                s.cuboid([x0, sink, z0 + t], [x0 + t, h, z1 - t], style.wall, style.roof == Roof::Flat);
                s.cuboid([x1 - t, sink, z0 + t], [x1, h, z1 - t], style.wall, style.roof == Roof::Flat);
                for (a0, a1, c0, c1) in [(x0, x1, z0, z0 + t), (x0, x1, z1 - t, z1)] {
                    match style.roof {
                        Roof::Flat => s.parapet(a0, a1, c0, c1, h, style.wall),
                        _ => s.hip(a0, a1, c0, c1, h, t * 0.4, style.roof_colour),
                    }
                }
                for (a0, a1, c0, c1) in [(x0, x0 + t, z0 + t, z1 - t), (x1 - t, x1, z0 + t, z1 - t)] {
                    match style.roof {
                        Roof::Flat => s.parapet(a0, a1, c0, c1, h, style.wall),
                        // Faîte le long de z : toit en croupe tourné.
                        _ => s.hip(a0, a1, c0, c1, h, t * 0.3, style.roof_colour),
                    }
                }
                s.windows([x1, 0.0, z1], [x1, 0.0, z0], [1.0, 0.0, 0.0], 2, h / 2.0, 4.0, (1.0, 1.6), glass, true);
                s.windows([x0, 0.0, z1], [x1, 0.0, z1], [0.0, 0.0, 1.0], 2, h / 2.0, 4.0, (1.0, 1.6), glass, false);
                s.windows([x0, 0.0, z0], [x1, 0.0, z0], [0.0, 0.0, -1.0], 2, h / 2.0, 4.0, (1.0, 1.6), glass, false);
                s.door(x1, 3.0, 4.0);
                // Bannière aux couleurs de la société.
                s.cylinder([x1 - 2.0, h, 0.0], 0.12, 6.0, 4, TIMBER);
                s.face(
                    [x1 - 2.0, h + 3.5, 0.0],
                    [x1 - 2.0, h + 3.5, 2.5],
                    [x1 - 2.0, h + 6.0, 2.5],
                    [x1 - 2.0, h + 6.0, 0.0],
                    [1.0, 0.0, 0.0],
                    style.accent,
                );
                s.face(
                    [x1 - 2.0, h + 3.5, 0.0],
                    [x1 - 2.0, h + 3.5, 2.5],
                    [x1 - 2.0, h + 6.0, 2.5],
                    [x1 - 2.0, h + 6.0, 0.0],
                    [-1.0, 0.0, 0.0],
                    style.accent,
                );
            }
            Kind::Market => {
                let h = 5.0;
                let n = 6;
                for i in 0..n {
                    for z in [z0 + 0.6, z1 - 0.6] {
                        let x = x0 + 0.6 + (l - 1.2) * i as f32 / (n - 1) as f32;
                        s.cylinder([x, sink, z], 0.3, h - sink, 6, if era >= Era::Industrial { CONCRETE } else { TIMBER });
                    }
                }
                s.gable(x0, x1, z0, z1, h, w * 0.3, style.roof_colour, style.roof_colour);
            }
            Kind::WallSegment => {
                let h = if era >= Era::Classical { 9.0 } else { 6.5 };
                let col = if era == Era::Village {
                    TIMBER
                } else if era == Era::Brick {
                    MUDBRICK
                } else {
                    STONE
                };
                if era == Era::Village {
                    // Palissade de pieux.
                    for i in 0..12 {
                        let x = x0 + (i as f32 + 0.5) * l / 12.0;
                        s.cylinder([x, sink, 0.0], 0.38, 4.0 - sink, 5, col);
                    }
                } else {
                    s.cuboid([x0, sink, z0], [x1, h, z1], col, true);
                    for i in 0..5 {
                        let x = x0 + (i as f32 + 0.25) * l / 5.0;
                        s.cuboid([x, h, z0], [x + l / 10.0, h + 1.0, z0 + 0.6], col, true);
                    }
                }
            }
            Kind::WallTower => {
                let h = if era >= Era::Classical { 15.0 } else { 10.0 };
                let col = if era == Era::Village {
                    TIMBER
                } else if era == Era::Brick {
                    MUDBRICK
                } else {
                    STONE
                };
                if style.roof == Roof::Gable || style.roof == Roof::Tiered {
                    s.cuboid([x0, sink, z0], [x1, h, z1], col, false);
                    s.hip(x0, x1, z0, z1, h, 3.5, style.roof_colour);
                } else {
                    s.b.tube([0.0, sink, 0.0], [0.0, h, 0.0], l / 2.0, l / 2.0, 10, col);
                    for k in 0..8 {
                        let t = TAU * k as f32 / 8.0;
                        let c = [l / 2.0 * 0.85 * t.cos(), h, l / 2.0 * 0.85 * t.sin()];
                        s.cuboid(add(c, [-0.5, 0.0, -0.5]), add(c, [0.5, 1.2, 0.5]), col, true);
                    }
                }
                s.windows([x1, 0.0, z1 * 0.4], [x1, 0.0, z0 * 0.4], [1.0, 0.0, 0.0], 3, h / 3.0, 3.0, (0.3, 1.0), INK, false);
            }
            Kind::Workshop => {
                let h = 4.0;
                s.cuboid([x0, sink, z0], [x1, h, z1], style.wall, false);
                s.gable(x0, x1, z0, z1, h, 2.8, style.roof_colour, style.wall);
                s.cylinder([x0 + 2.0, h, z0 + 2.0], 0.5, 3.5, 6, if era >= Era::Industrial { BRICK } else { STONE });
                s.door(x1, 2.2, 2.6);
            }
            Kind::Factory => {
                let h = 8.0;
                s.cuboid([x0, sink, z0], [x1, h, z1], if era == Era::Modern { CONCRETE } else { BRICK }, false);
                // Toit en dents de scie.
                let n = 5;
                for i in 0..n {
                    let (a0, a1) = (x0 + l * i as f32 / n as f32, x0 + l * (i + 1) as f32 / n as f32);
                    s.face([a0, h, z0], [a0, h, z1], [a1, h + 3.0, z1], [a1, h + 3.0, z0], [-0.6, 1.0, 0.0], SLATE);
                    s.face([a1, h, z0], [a1, h, z1], [a1, h + 3.0, z1], [a1, h + 3.0, z0], [1.0, 0.0, 0.0], GLASS);
                    s.triangle([a0, h, z0], [a1, h, z0], [a1, h + 3.0, z0], [0.0, 0.0, -1.0], BRICK);
                    s.triangle([a0, h, z1], [a1, h, z1], [a1, h + 3.0, z1], [0.0, 0.0, 1.0], BRICK);
                }
                s.windows([x0, 0.0, z1], [x1, 0.0, z1], [0.0, 0.0, 1.0], 2, h / 2.0, 4.0, (1.6, 2.0), glass, false);
                s.windows([x0, 0.0, z0], [x1, 0.0, z0], [0.0, 0.0, -1.0], 2, h / 2.0, 4.0, (1.6, 2.0), glass, false);
                let ch = rng.range(22.0, 32.0);
                s.b.tube([x0 + 4.0, sink, z0 + 4.0], [x0 + 4.0, ch, z0 + 4.0], 1.4, 0.9, 8, BRICK);
                s.door(x1, 4.0, 4.5);
            }
            Kind::Windmill => {
                let h = 11.0;
                s.b.tube([0.0, sink, 0.0], [0.0, h, 0.0], 3.2, 2.2, 8, style.wall);
                s.cone([0.0, h, 0.0], 2.6, 2.8, 8, style.roof_colour);
                // Ailes en croix, face à +x.
                let hub = [2.6, h - 0.8, 0.0];
                for k in 0..4 {
                    let t = TAU * k as f32 / 4.0 + 0.4;
                    let d = [0.0, t.sin(), t.cos()];
                    let e = add(hub, mul(d, 8.0));
                    let side = mul(cross(d, [1.0, 0.0, 0.0]), 1.1);
                    s.face(add(hub, mul(d, 1.5)), e, add(e, side), add(add(hub, mul(d, 1.5)), side), [1.0, 0.0, 0.0], FIBRE_SAIL);
                    s.face(add(hub, mul(d, 1.5)), e, add(e, side), add(add(hub, mul(d, 1.5)), side), [-1.0, 0.0, 0.0], FIBRE_SAIL);
                }
                s.door(2.9, 1.0, 1.9);
            }
        }
    }
    b.mesh
}

const FIBRE_SAIL: [f32; 3] = [0.82, 0.77, 0.62];

/// Le temple de la culture, à son époque.
fn temple(s: &mut Shape, style: &Style, rng: &mut DrawRng, (x0, x1, z0, z1): (f32, f32, f32, f32), sink: f32) {
    let era = style.era;
    let stone = match era {
        Era::Village => WATTLE,
        Era::Brick => MUDBRICK,
        Era::Industrial | Era::Modern => LIMESTONE,
        _ => style.wall,
    };
    match style.temple {
        TempleForm::Steps => {
            let levels = if era == Era::Village { 2 } else { 4 + (rng.next_u64() % 2) as usize };
            let mut y = sink;
            let (mut a, mut c) = ((x1 - x0) / 2.0, (z1 - z0) / 2.0);
            let step = a / (levels as f32 + 1.5);
            for i in 0..levels {
                let h = if i == 0 { 5.0 - sink } else { 4.0 };
                s.cuboid([-a, y, -c], [a, y + h, c], if i % 2 == 0 { stone } else { shade(stone, 0.93) }, true);
                // Escalier sur la face avant.
                s.face([a, y, -2.0], [a, y, 2.0], [a - step, y + h, 2.0], [a - step, y + h, -2.0], [1.0, 1.0, 0.0], shade(stone, 0.8));
                y += h;
                a -= step;
                c -= step;
            }
            // Sanctuaire au sommet.
            s.cuboid([-a * 0.7, y, -c * 0.7], [a * 0.7, y + 4.0, c * 0.7], style.accent, false);
            s.hip(-a * 0.7, a * 0.7, -c * 0.7, c * 0.7, y + 4.0, 2.5, style.roof_colour);
        }
        TempleForm::Colonnade => {
            let base = 1.6;
            s.cuboid([x0, sink, z0], [x1, base, z1], stone, true);
            let h = if era == Era::Village { 5.0 } else { 9.0 };
            // Cella.
            s.cuboid([x0 + 5.0, base, z0 + 4.0], [x1 - 5.0, base + h, z1 - 4.0], shade(stone, 0.95), false);
            let (nx, nz) = (8, 5);
            for i in 0..nx {
                for z in [z0 + 1.2, z1 - 1.2] {
                    let x = x0 + 1.2 + (x1 - x0 - 2.4) * i as f32 / (nx - 1) as f32;
                    s.cylinder([x, base, z], 0.6, h, 8, stone);
                }
            }
            for j in 1..nz - 1 {
                for x in [x0 + 1.2, x1 - 1.2] {
                    let z = z0 + 1.2 + (z1 - z0 - 2.4) * j as f32 / (nz - 1) as f32;
                    s.cylinder([x, base, z], 0.6, h, 8, stone);
                }
            }
            s.cuboid([x0 + 0.4, base + h, z0 + 0.4], [x1 - 0.4, base + h + 1.2, z1 - 0.4], style.accent, false);
            s.gable(x0 + 0.4, x1 - 0.4, z0 + 0.4, z1 - 0.4, base + h + 1.2, (z1 - z0) * 0.18, style.roof_colour, stone);
        }
        TempleForm::Pagoda => {
            let mut y = sink;
            let tiers = if era == Era::Village { 2 } else { 4 };
            let (mut a, mut c) = ((x1 - x0) / 2.0, (z1 - z0) / 2.0);
            for i in 0..tiers {
                let h = if i == 0 { 5.0 - sink } else { 3.5 };
                s.cuboid([-a, y, -c], [a, y + h, c], if i == 0 { stone } else { style.accent }, false);
                y += h;
                // Toit débordant relevé.
                s.hip(-a - 1.5, a + 1.5, -c - 1.5, c + 1.5, y, 2.2, style.roof_colour);
                y += 1.6;
                a *= 0.72;
                c *= 0.72;
            }
            s.cylinder([0.0, y, 0.0], 0.25, 5.0, 6, GILT);
        }
        TempleForm::Dome => {
            let h = if era == Era::Village { 5.0 } else { 11.0 };
            s.cuboid([x0, sink, z0], [x1, h, z1], stone, true);
            let r = (x1 - x0) * 0.38;
            s.b.tube([0.0, h, 0.0], [0.0, h + 3.0, 0.0], r, r, 12, shade(stone, 0.95));
            s.dome([0.0, h + 3.0, 0.0], r, if era >= Era::Classical { COPPER } else { style.roof_colour });
            if era >= Era::Monumental {
                for (px, pz) in [(x0, z0), (x1, z0), (x1, z1), (x0, z1)] {
                    s.cylinder([px, h, pz], 1.0, h * 0.9, 8, stone);
                    s.cone([px, h + h * 0.9, pz], 1.3, 3.0, 8, style.accent);
                }
            }
            s.door(x1, 3.0, 5.0);
        }
    }
}

/// Hauteur du plus haut point du maillage (pour les étiquettes).
pub fn height_m(m: &Mesh) -> f32 {
    m.positions.iter().map(|p| p[1]).fold(0.0, f32::max)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chaque_batiment_de_chaque_epoque_se_construit() {
        for era in Era::ALL {
            for culture in 0..6 {
                let style = Style::of(era, culture, [0.6, 0.3, 0.2]);
                for kind in Kind::ALL {
                    let m = mesh(kind, &style, 7);
                    assert!(m.triangles() > 4, "{kind:?} {era:?}");
                    assert!(m.triangles() < 6000, "{kind:?} {era:?} : {} triangles", m.triangles());
                    assert!(m.positions.iter().all(|p| p.iter().all(|v| v.is_finite())));
                    assert_eq!(m.positions.len(), m.normals.len());
                }
            }
        }
    }

    #[test]
    fn les_faces_regardent_dehors() {
        // Un bloc fermé vu de l'avant : la face +x a sa normale vers +x.
        let style = Style::of(Era::Brick, 1, [0.5; 3]);
        let m = mesh(Kind::House, &style, 1);
        let (l, _) = footprint(Kind::House, &style);
        let front = m.positions.iter().zip(&m.normals).filter(|(p, _)| (p[0] - l / 2.0).abs() < 1e-4).count();
        assert!(front > 0);
        assert!(m
            .positions
            .iter()
            .zip(&m.normals)
            .filter(|(p, _)| (p[0] - l / 2.0).abs() < 1e-4)
            .all(|(_, n)| n[0] > 0.9 || n[1].abs() > 0.1 || n[2].abs() > 0.1));
    }

    #[test]
    fn la_culture_garde_son_allure_et_les_cultures_different() {
        let a = Style::of(Era::Classical, 3, [0.5; 3]);
        let b = Style::of(Era::Industrial, 3, [0.5; 3]);
        assert_eq!(a.temple, b.temple);
        let forms: std::collections::BTreeSet<_> =
            (0..40).map(|c| format!("{:?}", Style::of(Era::Classical, c, [0.5; 3]).temple)).collect();
        assert!(forms.len() >= 3);
        assert_eq!(mesh(Kind::Temple, &a, 5), mesh(Kind::Temple, &a, 5));
    }
}
