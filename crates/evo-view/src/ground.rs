//! Scène au sol (document Globe, bande Z5 et « Descente au sol » ; Rendu du
//! vivant, « Présence dans le monde selon le zoom ») : un carré de quelques
//! kilomètres autour du point visé, son relief généré depuis la cellule, et
//! des individus animés.
//!
//! Les espèces vraies de la cellule assez grandes pour se voir sont des
//! **agents** du moteur (crate `evo-agents`, niveaux 4 et 5) : leurs
//! individus viennent de l'échantillon que le moteur tire de leur population
//! (génomes, naissances, morts et départs calibrés sur ses taux), et leurs
//! comportements de leurs traits ; leur action courante est celle que la
//! scène des agents publie dans le lexique commun. Ailleurs, et tant que le
//! vivant de la cellule est trop petit pour la scène, la scène est peuplée de
//! **figurants** : des individus de banc d'essai dont les corps viennent de
//! plans de construction au format du moteur (`evo_life::BodyPlan`, traduits
//! par [`crate::anatomy::from_simulated`]) et dont les comportements sont
//! tirés dans le client. Ni les uns ni les autres n'appartiennent à
//! l'histoire simulée : rien de ce qu'ils font ne remonte au moteur, et le
//! rejeu ne les voit pas.
//!
//! Les individus proches de la caméra sont animés (squelette et pose par
//! action du lexique), les autres sont des imposteurs (vues pré-rendues).
//! Seuls les proches ont des comportements complets (chasse, fuite, troupeau) ;
//! les lointains errent, comme le prévoit la règle de figuration du doc
//! Rendu du vivant.

use crate::anatomy::from_simulated;
use evo_life::body as sim;
use evo_morph::body::{self, pose, rig, Action, Affine, BodyPlan, Locomotion, Motion, Rig};
use evo_morph::canvas::{fbm, mix64, DrawRng};
use evo_sim::history::Organisation;
use std::f32::consts::{PI, TAU};

/// Côté du carré, en mètres, et sommets par côté.
pub const PATCH_SIZE_M: f32 = 2400.0;
pub const PATCH_RES: usize = 161;
/// Profondeur d'eau montrée au plus, m. [Simplification] Une scène au fond
/// d'un océan de 4 km serait noire : la scène sous-marine est ramenée à une
/// eau côtière.
pub const MAX_SHOWN_DEPTH_M: f32 = 40.0;

/// Ce que la scène lit de la cellule visée.
#[derive(Clone, Debug, PartialEq)]
pub struct Site {
    pub cell: u32,
    pub elevation_m: f32,
    pub is_ocean: bool,
    pub temperature_k: f32,
    pub ice_cover: f32,
    pub roughness_m: f32,
    /// Teinte du vivant de la cellule.
    pub life_rgb: Option<[u8; 3]>,
    pub biomass_per_m2: f32,
    pub sky: [f32; 3],
}

impl Site {
    /// La cellule visée telle que l'image publiée la décrit. Le relief fin
    /// n'est pas simulé : sa rugosité suit l'altitude (simplification).
    pub fn from_frame(frame: &crate::frame::Frame, cell: usize) -> Option<Site> {
        let c = frame.cells().get(cell)?;
        let g = &frame.state.globals;
        let habitat = crate::decor::Habitat {
            species: 0,
            cell,
            is_ocean: c.is_ocean,
            depth_m: 0.0,
            height_m: 0.0,
            temperature_k: c.temperature_k,
            ice_cover: c.ice_cover,
            light_w_m2: c.light_w_m2,
            vent: false,
            pigment_rgb: None,
            neighbours: Vec::new(),
            density: 0.0,
            star_temperature_k: frame.planet.star_temperature_k,
            o2_mixing: g.o2_mixing,
            ch4_ppb: g.ch4_ppb,
            years: 0.0,
        };
        Some(Site {
            cell: cell as u32,
            elevation_m: c.elevation_m,
            is_ocean: c.is_ocean,
            temperature_k: c.temperature_k,
            ice_cover: c.ice_cover,
            roughness_m: (60.0 + c.elevation_m.abs() * 0.12).min(600.0),
            life_rgb: c.pigment_rgb,
            biomass_per_m2: c.biomass_per_m2,
            sky: crate::decor::sky_colour(&habitat),
        })
    }
}

/// Maillage du terrain : positions, normales, couleurs, indices.
pub type PatchMesh = (Vec<[f32; 3]>, Vec<[f32; 3]>, Vec<[f32; 3]>, Vec<u32>);

/// Le terrain de la scène : altitudes sur une grille régulière, centrée sur
/// l'origine. Avec de l'eau, la surface est à y = 0.
#[derive(Clone, Debug)]
pub struct Patch {
    pub site: Site,
    pub heights: Vec<f32>,
    pub water: bool,
}

impl Patch {
    pub fn new(site: Site, seed: u64) -> Patch {
        let n = PATCH_RES;
        let mut heights = vec![0.0f32; n * n];
        let s = mix64(seed ^ site.cell as u64 ^ 0x0073_6F6C);
        let water = site.is_ocean;
        let depth = if water { (-site.elevation_m).clamp(4.0, MAX_SHOWN_DEPTH_M) } else { 0.0 };
        // Amplitude du détail : la rugosité de la cellule, ramenée à
        // l'échelle d'un carré de 2,4 km.
        let amp = if water { (0.15 * depth).max(1.5) } else { (site.roughness_m * 0.12).clamp(6.0, 140.0) };
        for j in 0..n {
            for i in 0..n {
                let (x, z) = Patch::xz(i, j);
                let u = x / 900.0;
                let v = z / 900.0;
                let broad = fbm(s, u, v) - 0.5;
                let ridge = 1.0 - (2.0 * fbm(s ^ 0x5249, u * 2.3, v * 2.3) - 1.0).abs();
                let fine = fbm(s ^ 0x4649, u * 9.0, v * 9.0) - 0.5;
                let h = amp * (1.4 * broad + 0.5 * (ridge - 0.5) + 0.12 * fine);
                heights[j * n + i] = if water { -depth + h } else { h };
            }
        }
        // Le centre reste praticable : un replat doux sous la caméra.
        Patch { site, heights, water }
    }

    fn xz(i: usize, j: usize) -> (f32, f32) {
        let step = PATCH_SIZE_M / (PATCH_RES - 1) as f32;
        (i as f32 * step - PATCH_SIZE_M / 2.0, j as f32 * step - PATCH_SIZE_M / 2.0)
    }

    /// Altitude du sol en (x, z), interpolée.
    pub fn height_at(&self, x: f32, z: f32) -> f32 {
        let n = PATCH_RES;
        let step = PATCH_SIZE_M / (n - 1) as f32;
        let fx = ((x + PATCH_SIZE_M / 2.0) / step).clamp(0.0, (n - 1) as f32 - 1e-3);
        let fz = ((z + PATCH_SIZE_M / 2.0) / step).clamp(0.0, (n - 1) as f32 - 1e-3);
        let (i, j) = (fx as usize, fz as usize);
        let (tx, tz) = (fx - i as f32, fz - j as f32);
        let h = |a: usize, b: usize| self.heights[b * n + a];
        let top = h(i, j) + (h(i + 1, j) - h(i, j)) * tx;
        let bottom = h(i, j + 1) + (h(i + 1, j + 1) - h(i, j + 1)) * tx;
        top + (bottom - top) * tz
    }

    /// Couleur du sol en un sommet : roche et sol ocre, teinte du vivant sur
    /// les replats, glace, sable sous l'eau.
    pub fn colour_at(&self, i: usize, j: usize, slope: f32) -> [f32; 3] {
        let n = PATCH_RES;
        let h = self.heights[j * n + i];
        let (x, z) = Patch::xz(i, j);
        let grain = fbm(0x5A4E ^ self.site.cell as u64, x / 60.0, z / 60.0);
        let sand = [0.80, 0.72, 0.55];
        let rock = [0.58, 0.52, 0.45];
        let soil = [0.70, 0.60, 0.44];
        let mut c = if self.water { sand } else { soil };
        let rocky = ((slope - 0.25) * 3.0).clamp(0.0, 1.0);
        for k in 0..3 {
            c[k] += (rock[k] - c[k]) * rocky;
        }
        if let Some(rgb) = self.site.life_rgb {
            // Tapis du vivant sur les replats, plus dense où la biomasse l'est.
            let cover = ((self.site.biomass_per_m2 * 4.0).sqrt()).clamp(0.0, 0.7) * (1.0 - rocky) * (0.6 + 0.4 * grain);
            for k in 0..3 {
                c[k] += (rgb[k] as f32 / 255.0 - c[k]) * cover;
            }
        }
        let ice = (self.site.ice_cover * 1.2 - 0.1 + 0.2 * (grain - 0.5) + if self.water { 0.0 } else { h / 2000.0 }).clamp(0.0, 1.0);
        for (k, v) in [0.93, 0.95, 0.97].iter().enumerate() {
            c[k] += (v - c[k]) * ice;
        }
        let shade = 0.9 + 0.2 * grain;
        [c[0] * shade, c[1] * shade, c[2] * shade]
    }

    /// Maillage du terrain : positions, normales, couleurs, triangles (sens
    /// direct vu d'en haut).
    pub fn mesh(&self) -> PatchMesh {
        let n = PATCH_RES;
        let step = PATCH_SIZE_M / (n - 1) as f32;
        let mut pos = Vec::with_capacity(n * n);
        let mut nor = Vec::with_capacity(n * n);
        let mut col = Vec::with_capacity(n * n);
        for j in 0..n {
            for i in 0..n {
                let (x, z) = Patch::xz(i, j);
                let h = self.heights[j * n + i];
                let hx = self.heights[j * n + (i + 1).min(n - 1)] - self.heights[j * n + i.saturating_sub(1)];
                let hz = self.heights[(j + 1).min(n - 1) * n + i] - self.heights[j.saturating_sub(1) * n + i];
                let nv = [-hx / (2.0 * step), 1.0, -hz / (2.0 * step)];
                let l = (nv[0] * nv[0] + nv[1] * nv[1] + nv[2] * nv[2]).sqrt();
                let normal = [nv[0] / l, nv[1] / l, nv[2] / l];
                pos.push([x, h, z]);
                nor.push(normal);
                col.push(self.colour_at(i, j, 1.0 - normal[1]));
            }
        }
        let mut idx = Vec::with_capacity((n - 1) * (n - 1) * 6);
        for j in 0..n - 1 {
            for i in 0..n - 1 {
                let a = (j * n + i) as u32;
                let b = a + 1;
                let c = a + n as u32;
                let d = c + 1;
                idx.extend_from_slice(&[a, c, b, b, c, d]);
            }
        }
        (pos, nor, col, idx)
    }
}

/// Familles de corps du banc d'essai.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BenchKind {
    /// Marcheur à 2 à 4 paires de pattes.
    Walker,
    /// Nageur à nageoires et queue.
    Swimmer,
    /// Corps en segments, sans membres.
    Worm,
    /// Corps radiaire à tentacules.
    Radial,
    /// Corps à grandes lames, sans eau : il vole.
    Flyer,
}

fn cell_type(pathways: u32, role: &str, nm: Option<f64>) -> sim::CellType {
    sim::CellType { pathways, role: role.into(), expressed_genes: 12, pigment_nm: nm }
}

#[allow(clippy::too_many_arguments)]
fn sim_module(
    id: u16,
    kind: sim::ModuleKind,
    parent: Option<u16>,
    attach: [f64; 3],
    axis: [f64; 3],
    dims: [f64; 3],
    profile: sim::Profile,
    symmetry: sim::SymmetryKind,
    covering: sim::Covering,
    pigment_nm: f64,
    cell_types: Vec<u8>,
) -> sim::Module {
    sim::Module {
        id,
        kind,
        cell_types,
        layers: vec![sim::Layer { depth: 0.0, weight: 1.0, cell_type: 0 }],
        master_regulator: None,
        parent,
        attach,
        axis,
        offset: [0.0; 3],
        dimensions: sim::Dimensions { length_m: dims[0], width_m: dims[1], thickness_m: dims[2], profile, carbon_mol: 0.0 },
        symmetry: sim::Symmetry { kind: symmetry, copies: 1, spacing: 0.0 },
        joints: vec![sim::Joint {
            kind: sim::JointKind::Flexible,
            axis: [0.0, 0.0, 1.0],
            amplitude_rad: 0.8,
            muscle_section_m2: 0.0,
            fast_fibres: 0.5,
        }],
        material: sim::Material::Hydrostatic,
        covering,
        pigments: vec![sim::PigmentPatch { absorption_nm: pigment_nm, concentration: 0.8, depth: 0.0 }],
        structural_colour: None,
        pattern: None,
        function: sim::Function { pathways: 0, maintenance_kj: 1.0, build_kj: 1.0 },
    }
}

/// Direction autour de l'axe du corps (degrés, 0 vers le dos, 90 vers la
/// droite) et inclinaison sur l'axe (degrés) : point d'attache et axe au
/// format du plan.
fn around(t: f64, around_deg: f64, elevation_deg: f64) -> ([f64; 3], [f64; 3]) {
    let (a, e) = (around_deg.to_radians(), elevation_deg.to_radians());
    let attach = [t, a.cos(), a.sin()];
    let axis = [e.cos(), e.sin() * a.cos(), e.sin() * a.sin()];
    (attach, axis)
}

/// Plan de construction de banc d'essai, au format du moteur : un tronc,
/// une tête et ses yeux, puis les appendices de la famille.
pub fn bench_plan(kind: BenchKind, seed: u64, size_m: f64) -> sim::BodyPlan {
    use sim::{Covering as C, ModuleKind as K, Profile as P, SymmetryKind as S};
    let mut rng = DrawRng::new(mix64(seed ^ 0x6265_6E63));
    let r = |rng: &mut DrawRng, a: f64, b: f64| rng.range(a as f32, b as f32) as f64;
    let skin_nm = r(&mut rng, 425.0, 470.0);
    let second_nm = r(&mut rng, 440.0, 500.0);
    let covering = match kind {
        BenchKind::Walker => [C::Cuticle, C::Scales, C::Hair][(rng.next_u64() % 3) as usize],
        BenchKind::Swimmer => C::Scales,
        BenchKind::Flyer => [C::Feathers, C::Cuticle][(rng.next_u64() % 2) as usize],
        BenchKind::Worm | BenchKind::Radial => C::Mucus,
    };
    let types = vec![cell_type(0, "nourricière", None), cell_type(0, "sensorielle", Some(500.0)), cell_type(0, "réserve", Some(second_nm))];
    let mut m: Vec<sim::Module> = Vec::new();
    let girth = size_m * r(&mut rng, 0.1, 0.2);
    let trunk_len = size_m * if kind == BenchKind::Worm { 0.25 } else { 0.5 };
    m.push(sim_module(
        0,
        K::Body,
        None,
        [0.0; 3],
        [1.0, 0.0, 0.0],
        [trunk_len, girth * 2.0, girth * 2.0],
        P::Elongated,
        S::Bilateral,
        covering,
        skin_nm,
        vec![0, 2],
    ));
    let mut next = 1u16;
    let mut add = |m: &mut Vec<sim::Module>, mut module: sim::Module| -> u16 {
        module.id = next;
        next += 1;
        m.push(module);
        next - 1
    };
    if kind == BenchKind::Radial {
        // Ombrelle et tentacules d'ordre n.
        let n = 4 + (rng.next_u64() % 5) as u8;
        let (at, ax) = around(0.1, 0.0, 150.0);
        add(
            &mut m,
            sim_module(
                0,
                K::Appendage,
                Some(0),
                at,
                ax,
                [size_m * 0.6, girth * 0.25, girth * 0.25],
                P::Elongated,
                S::Radial(n),
                covering,
                second_nm,
                vec![0],
            ),
        );
        let (at, ax) = around(0.9, 0.0, 60.0);
        add(
            &mut m,
            sim_module(
                0,
                K::Organ,
                Some(0),
                at,
                ax,
                [girth * 0.3, girth * 0.25, girth * 0.25],
                P::Round,
                S::Radial(n),
                covering,
                480.0,
                vec![1],
            ),
        );
        return sim::BodyPlan { version: sim::BODY_PLAN_VERSION, modules: m, cell_types: types };
    }
    // Tête à l'avant, deux yeux.
    let head_len = girth * r(&mut rng, 1.4, 2.2);
    let (at, ax) = around(1.0, 0.0, r(&mut rng, -10.0, 10.0));
    let head = add(
        &mut m,
        sim_module(0, K::Segment, Some(0), at, ax, [head_len, girth * 1.5, girth * 1.5], P::Elongated, S::None, covering, skin_nm, vec![0]),
    );
    let (at, ax) = around(0.6, 55.0, 80.0);
    add(
        &mut m,
        sim_module(
            0,
            K::Organ,
            Some(head),
            at,
            ax,
            [girth * 0.35, girth * 0.25, girth * 0.25],
            P::Round,
            S::Bilateral,
            covering,
            480.0,
            vec![1],
        ),
    );
    // Queue : segments vers l'arrière.
    let tail_segments = match kind {
        BenchKind::Worm => 5 + (rng.next_u64() % 4) as usize,
        BenchKind::Swimmer => 2,
        _ => 1,
    };
    let mut prev = 0u16;
    for k in 0..tail_segments {
        let taper = 1.0 - 0.15 * (k as f64 + 1.0);
        let (at, ax) = if k == 0 { around(0.0, 0.0, 180.0) } else { around(1.0, 0.0, r(&mut rng, -12.0, 12.0)) };
        let len = if kind == BenchKind::Worm { trunk_len } else { size_m * 0.22 };
        prev = add(
            &mut m,
            sim_module(
                0,
                K::Segment,
                Some(prev),
                at,
                ax,
                [len, girth * 2.0 * taper.max(0.3), girth * 2.0 * taper.max(0.3)],
                P::Elongated,
                S::None,
                covering,
                skin_nm,
                vec![0],
            ),
        );
    }
    match kind {
        BenchKind::Walker => {
            // Paires de pattes le long du tronc, vers le bas et le côté.
            let pairs = 2 + (rng.next_u64() % 3) as usize;
            for k in 0..pairs {
                let t = 0.15 + 0.7 * k as f64 / (pairs - 1).max(1) as f64;
                let (at, ax) = around(t, 150.0, 100.0);
                add(
                    &mut m,
                    sim_module(
                        0,
                        K::Appendage,
                        Some(0),
                        at,
                        ax,
                        [size_m * r(&mut rng, 0.3, 0.45), girth * 0.45, girth * 0.45],
                        P::Elongated,
                        S::Bilateral,
                        covering,
                        skin_nm,
                        vec![0],
                    ),
                );
            }
        }
        BenchKind::Swimmer => {
            let (at, ax) = around(0.75, 110.0, 120.0);
            add(
                &mut m,
                sim_module(
                    0,
                    K::Appendage,
                    Some(0),
                    at,
                    ax,
                    [size_m * 0.2, size_m * 0.12, girth * 0.1],
                    P::Flattened,
                    S::Bilateral,
                    covering,
                    second_nm,
                    vec![0],
                ),
            );
            let (at, ax) = around(1.0, 0.0, 180.0);
            add(
                &mut m,
                sim_module(
                    0,
                    K::Appendage,
                    Some(prev),
                    at,
                    ax,
                    [size_m * 0.2, size_m * 0.2, girth * 0.1],
                    P::Flattened,
                    S::None,
                    covering,
                    second_nm,
                    vec![0],
                ),
            );
            let (at, ax) = around(0.5, 0.0, 160.0);
            add(
                &mut m,
                sim_module(
                    0,
                    K::Appendage,
                    Some(0),
                    at,
                    ax,
                    [size_m * 0.15, size_m * 0.1, girth * 0.1],
                    P::Flattened,
                    S::None,
                    covering,
                    second_nm,
                    vec![0],
                ),
            );
        }
        BenchKind::Flyer => {
            let (at, ax) = around(0.6, 90.0, 90.0);
            add(
                &mut m,
                sim_module(
                    0,
                    K::Appendage,
                    Some(0),
                    at,
                    ax,
                    [size_m * 0.7, size_m * 0.35, girth * 0.08],
                    P::Flattened,
                    S::Bilateral,
                    covering,
                    second_nm,
                    vec![0],
                ),
            );
            let (at, ax) = around(0.35, 130.0, 100.0);
            add(
                &mut m,
                sim_module(
                    0,
                    K::Appendage,
                    Some(0),
                    at,
                    ax,
                    [size_m * 0.2, girth * 0.25, girth * 0.25],
                    P::Elongated,
                    S::Bilateral,
                    covering,
                    skin_nm,
                    vec![0],
                ),
            );
        }
        BenchKind::Worm | BenchKind::Radial => {}
    }
    sim::BodyPlan { version: sim::BODY_PLAN_VERSION, modules: m, cell_types: types }
}

/// Une espèce présente dans la scène.
pub struct SceneSpecies {
    pub name: String,
    /// Espèce simulée (signature) ou figurant de banc d'essai.
    pub signature: Option<u32>,
    pub plan: BodyPlan,
    pub body: body::Body,
    /// Squelette dans le repère normalisé du maillage (corps de taille 1,
    /// centré).
    pub rig: Rig,
    pub centre: [f32; 3],
    pub size_m: f32,
    /// Distance du centre du maillage normalisé à ses pieds (part de la
    /// taille).
    pub foot: f32,
    pub aquatic: bool,
    pub predator: bool,
    /// 0 : solitaire, 1 : troupeau serré.
    pub social: f32,
    pub kind: Option<BenchKind>,
    /// Espèce dont les individus sont des agents du moteur (`evo-agents`).
    pub engine: bool,
}

impl SceneSpecies {
    /// Construit une espèce depuis son plan d'affichage.
    pub fn new(name: String, signature: Option<u32>, plan: BodyPlan, size_m: f32, aquatic: bool, kind: Option<BenchKind>) -> SceneSpecies {
        let body = body::build(&plan, 3);
        let (lo, hi) = body.lods.first().map(|m| m.bounds()).unwrap_or(([0.0; 3], [0.0; 3]));
        let centre = [(lo[0] + hi[0]) / 2.0, (lo[1] + hi[1]) / 2.0, (lo[2] + hi[2]) / 2.0];
        let k = if body.extent_m > 0.0 { 1.0 / body.extent_m } else { 1.0 };
        let norm = |p: body::shape::V3| body::shape::V3((p.0 - centre[0]) * k, (p.1 - centre[1]) * k, (p.2 - centre[2]) * k);
        let bones: Vec<body::shape::Bone> = body
            .bones
            .iter()
            .map(|b| body::shape::Bone { module: b.module, parent: b.parent, head: norm(b.head), tail: norm(b.tail) })
            .collect();
        let rig = rig(&plan, &bones, aquatic);
        let foot = (centre[1] - lo[1]) * k;
        SceneSpecies { name, signature, plan, body, rig, centre, size_m, foot, aquatic, predator: false, social: 0.5, kind, engine: false }
    }

    /// Nombre de nombres par os dans la texture des poses.
    pub fn bone_count(&self) -> usize {
        self.rig.bones.len()
    }
}

/// Espèces vraies de la cellule dont les corps se voient à l'échelle de la
/// scène (au moins un millimètre). Elles n'ont pas encore de comportement
/// publié par le moteur : leurs gestes sont ceux des figurants.
pub fn real_species(
    list: &[(evo_sim::history::SpeciesView, Option<evo_sim::observation::PopulationView>)],
    game_seed: u64,
    aquatic: bool,
    max: usize,
) -> Vec<SceneSpecies> {
    let mut out = Vec::new();
    for (sv, pop) in list {
        if out.len() >= max {
            break;
        }
        if !sv.organisation.multicellular {
            continue;
        }
        let Some(plan) = crate::anatomy::plan_for_species(sv, pop.as_ref(), game_seed) else { continue };
        let probe = body::build(&plan, 1);
        if probe.extent_m < 1e-3 {
            continue;
        }
        let mut s = SceneSpecies::new(sv.name.clone(), Some(sv.signature), plan, probe.extent_m, aquatic, None);
        s.social = 0.8;
        out.push(s);
    }
    out
}

/// Figurants de banc d'essai pour une scène : familles selon le milieu.
pub fn bench_species(site: &Site, seed: u64, count: usize) -> Vec<SceneSpecies> {
    let kinds: &[BenchKind] = if site.is_ocean {
        &[BenchKind::Swimmer, BenchKind::Radial, BenchKind::Worm, BenchKind::Walker, BenchKind::Swimmer, BenchKind::Radial]
    } else {
        &[BenchKind::Walker, BenchKind::Walker, BenchKind::Flyer, BenchKind::Worm, BenchKind::Walker, BenchKind::Flyer]
    };
    let mut rng = DrawRng::new(mix64(seed ^ site.cell as u64 ^ 0x6669_6775));
    let mut out = Vec::new();
    for k in 0..count {
        let kind = kinds[k % kinds.len()];
        let size = match kind {
            BenchKind::Walker => 10f32.powf(rng.range(-1.3, 0.3)),
            BenchKind::Swimmer => 10f32.powf(rng.range(-1.2, 0.2)),
            BenchKind::Worm => 10f32.powf(rng.range(-1.5, -0.5)),
            BenchKind::Radial => 10f32.powf(rng.range(-1.3, -0.3)),
            BenchKind::Flyer => 10f32.powf(rng.range(-1.0, -0.2)),
        };
        let plan_seed = rng.next_u64();
        let sim_plan = bench_plan(kind, plan_seed, size as f64);
        let org = Organisation { body_cells: 1e6, cell_types: 3, eukaryote: true, multicellular: true, sexual: true, ..Default::default() };
        let plan = from_simulated(&sim_plan, &org, plan_seed);
        let label = match kind {
            BenchKind::Walker => "marcheur",
            BenchKind::Swimmer => "nageur",
            BenchKind::Worm => "ver",
            BenchKind::Radial => "radiaire",
            BenchKind::Flyer => "voilier",
        };
        let mut s = SceneSpecies::new(format!("Figurant d'essai n° {} ({label})", k + 1), None, plan, size, site.is_ocean, Some(kind));
        s.social = match kind {
            BenchKind::Swimmer | BenchKind::Flyer => 0.9,
            BenchKind::Walker => rng.range(0.2, 0.9),
            _ => 0.1,
        };
        out.push(s);
    }
    // Le plus grand marcheur ou nageur chasse les autres.
    if let Some(p) = out
        .iter_mut()
        .filter(|s| matches!(s.kind, Some(BenchKind::Walker | BenchKind::Swimmer)))
        .max_by(|a, b| a.size_m.total_cmp(&b.size_m))
    {
        p.predator = true;
        p.social = 0.1;
        p.size_m *= 1.6;
    }
    out
}

/// Un individu de la scène.
#[derive(Clone, Copy, Debug)]
pub struct Individual {
    pub species: u16,
    pub x: f32,
    pub z: f32,
    pub y: f32,
    pub heading: f32,
    /// Vitesse au sol, m/s.
    pub speed: f32,
    pub action: Action,
    /// Temps restant de l'action, s.
    pub timer: f32,
    pub time: f32,
    pub phase: f32,
    /// Taille propre (variation autour de celle de l'espèce).
    pub scale: f32,
    /// Cible : point (errance, troupeau) ou individu (chasse, fuite).
    pub goal: (f32, f32),
    pub other: u32,
    /// Action imposée (banc d'essai), sinon tirée.
    pub forced: bool,
}

/// La foule d'une scène.
pub struct Crowd {
    pub patch: Patch,
    pub species: Vec<SceneSpecies>,
    pub individuals: Vec<Individual>,
    /// Individus animés, les plus proches de la caméra.
    pub near: Vec<u32>,
    rng: DrawRng,
    pub clock: f32,
    is_near: Vec<bool>,
    tick: u32,
    /// Agents du moteur et leurs places dans la foule.
    agents: Option<AgentLink>,
}

/// Lien entre la scène des agents et la foule : chaque espèce d'agents a des
/// places réservées (individus de taille nulle quand elles sont libres), si
/// bien que le nombre d'individus de la foule ne change jamais.
struct AgentLink {
    scene: evo_agents::Scene,
    /// Espèce de la foule de chaque espèce de la scène.
    species: Vec<u16>,
    /// Places libres de chaque espèce de la scène.
    free: Vec<Vec<u32>>,
    slot: std::collections::HashMap<u64, u32>,
}

/// Événements de vie montrés par minute dans une scène d'agents
/// (horloge de la vie, affichage seul).
pub const LIFE_EVENTS_PER_SECOND: f64 = 0.5;

/// Action du lexique de la foule pour une action des agents (même ordre).
pub fn action_of(a: evo_agents::Act) -> Action {
    Action::from_index(a.index())
}

/// Les lointains avancent par tranches : une sur quatre à chaque image.
const FAR_SLICES: u32 = 4;

impl Crowd {
    /// Répartit `total` individus entre les espèces (les prédateurs sont
    /// rares), en troupeaux pour les espèces sociales.
    pub fn new(patch: Patch, species: Vec<SceneSpecies>, total: usize, seed: u64) -> Crowd {
        let mut rng = DrawRng::new(mix64(seed ^ 0x666F_756C));
        let weights: Vec<f32> = species.iter().map(|s| if s.predator { 0.15 } else { 1.0 / s.size_m.max(0.05).sqrt() }).collect();
        let sum: f32 = weights.iter().sum::<f32>().max(1e-9);
        let mut individuals = Vec::with_capacity(total);
        let half = PATCH_SIZE_M / 2.0 * 0.96;
        for (k, s) in species.iter().enumerate() {
            let n = ((total as f32) * weights[k] / sum).round() as usize;
            let herd = if s.social > 0.5 { 40 } else { 4 };
            let mut centre = (0.0, 0.0);
            for i in 0..n {
                if i % herd == 0 {
                    centre = (rng.range(-half, half), rng.range(-half, half));
                }
                let spread = 6.0 + s.size_m * 25.0;
                let x = (centre.0 + rng.range(-spread, spread)).clamp(-half, half);
                let z = (centre.1 + rng.range(-spread, spread)).clamp(-half, half);
                individuals.push(Individual {
                    species: k as u16,
                    x,
                    z,
                    y: 0.0,
                    heading: rng.range(0.0, TAU),
                    speed: 0.0,
                    action: Action::Feed,
                    timer: rng.range(0.0, 6.0),
                    time: rng.range(0.0, 100.0),
                    phase: rng.unit(),
                    scale: rng.range(0.85, 1.15),
                    goal: (x, z),
                    other: u32::MAX,
                    forced: false,
                });
            }
        }
        let mut c = Crowd { patch, species, individuals, near: Vec::new(), rng, clock: 0.0, is_near: Vec::new(), tick: 0, agents: None };
        for i in 0..c.individuals.len() {
            c.place_y(i);
        }
        c
    }

    /// Foule de figurants (`species`, `total` individus) à laquelle
    /// s'ajoutent des espèces d'agents du moteur : chaque espèce vraie
    /// arrive avec l'échantillon de sa population.
    pub fn with_agents(
        patch: Patch,
        species: Vec<SceneSpecies>,
        real: Vec<(SceneSpecies, evo_agents::Sample)>,
        total: usize,
        seed: u64,
    ) -> Crowd {
        let mut c = Crowd::new(patch, species, total, seed);
        if real.is_empty() {
            return c;
        }
        let half = PATCH_SIZE_M / 2.0 * 0.96;
        let mut samples = Vec::new();
        let mut map = Vec::new();
        let mut free = Vec::new();
        for (mut sp, sample) in real {
            sp.engine = true;
            sp.predator = sample.genotypes[0].traits.diet == evo_agents::Diet::Predator;
            sp.social = sample.genotypes[0].traits.sociality as f32;
            let k = c.species.len() as u16;
            c.species.push(sp);
            let cap = 2 * sample.nominal.max(sample.members.len()) + 16;
            let mut slots = Vec::with_capacity(cap);
            for _ in 0..cap {
                slots.push(c.individuals.len() as u32);
                c.individuals.push(Individual {
                    species: k,
                    x: 0.0,
                    z: 0.0,
                    y: 0.0,
                    heading: 0.0,
                    speed: 0.0,
                    action: Action::Rest,
                    timer: 0.0,
                    time: 0.0,
                    phase: c.rng.unit(),
                    scale: 0.0,
                    goal: (0.0, 0.0),
                    other: u32::MAX,
                    forced: false,
                });
            }
            slots.reverse();
            free.push(slots);
            map.push(k);
            samples.push(sample);
        }
        let scene = evo_agents::Scene::new(samples, half, seed, LIFE_EVENTS_PER_SECOND);
        c.agents = Some(AgentLink { scene, species: map, free, slot: std::collections::HashMap::new() });
        c.sync_agents(0.0);
        c
    }

    /// Recopie l'état des agents dans leurs places de la foule.
    fn sync_agents(&mut self, dt: f32) {
        let Some(link) = self.agents.as_mut() else { return };
        let mut seen = std::collections::HashSet::new();
        for a in &link.scene.agents {
            let k = evo_agents::agents::key(a.species, a.member);
            seen.insert(k);
            let slot = match link.slot.get(&k) {
                Some(&s) => Some(s),
                None => {
                    let s = link.free[a.species as usize].pop();
                    if let Some(s) = s {
                        link.slot.insert(k, s);
                    }
                    s
                }
            };
            let Some(slot) = slot else { continue };
            let sp = &self.species[link.species[a.species as usize] as usize];
            let t = link.scene.traits_of(a);
            let ind = &mut self.individuals[slot as usize];
            ind.x = a.x;
            ind.z = a.z;
            ind.heading = a.heading;
            ind.speed = a.speed;
            if !ind.forced {
                ind.action = action_of(a.act);
            }
            ind.time += dt;
            ind.scale = if matches!(a.fate, evo_agents::agents::Fate::Dead { .. }) {
                0.0
            } else {
                (t.length_m as f32 / sp.size_m.max(1e-9)).clamp(0.3, 3.0)
            };
        }
        let gone: Vec<(u64, u32)> = link.slot.iter().filter(|(k, _)| !seen.contains(k)).map(|(&k, &s)| (k, s)).collect();
        for (k, s) in gone {
            link.slot.remove(&k);
            self.individuals[s as usize].scale = 0.0;
            link.free[(k >> 32) as usize].push(s);
        }
    }

    /// Scène des agents du moteur, s'il y en a.
    pub fn agents(&self) -> Option<&evo_agents::Scene> {
        self.agents.as_ref().map(|l| &l.scene)
    }

    /// Vrai pour une place d'agent libre (individu invisible).
    pub fn is_vacant(&self, i: usize) -> bool {
        let ind = &self.individuals[i];
        self.species[ind.species as usize].engine && ind.scale == 0.0
    }

    fn place_y(&mut self, i: usize) {
        let ind = self.individuals[i];
        let s = &self.species[ind.species as usize];
        let ground = self.patch.height_at(ind.x, ind.z);
        let size = s.size_m * ind.scale;
        let lift = s.foot * size;
        let fly =
            matches!(s.rig.locomotion, Locomotion::Fly) && !matches!(ind.action, Action::Rest | Action::Feed | Action::Lay | Action::Care);
        self.individuals[i].y = if s.aquatic && !matches!(s.rig.locomotion, Locomotion::Walk { .. }) {
            // Nage entre le fond et la surface, à une hauteur propre.
            let top = -size;
            let bottom = ground + lift + size;
            (bottom + (top - bottom).max(0.0) * (0.15 + 0.5 * ind.phase)).min(top.max(bottom))
        } else if fly {
            ground + lift + 4.0 + 20.0 * ind.phase
        } else {
            ground + lift
        };
    }

    /// Impose une action à toute une espèce (banc d'essai, captures), ou
    /// rend la main aux comportements avec `None`.
    pub fn force(&mut self, species: Option<u16>, action: Option<Action>) {
        for ind in self.individuals.iter_mut() {
            if species.is_none_or(|s| s == ind.species) {
                match action {
                    Some(a) => {
                        ind.action = a;
                        ind.forced = true;
                    }
                    None => ind.forced = false,
                }
            }
        }
    }

    /// Avance la scène de `dt` secondes (temps réel : les gestes des
    /// figurants ne suivent pas la vitesse de la simulation). `camera` est la
    /// position de la caméra ; `animated` le nombre d'individus animés.
    pub fn step(&mut self, dt: f32, camera: [f32; 3], animated: usize) {
        let dt = dt.clamp(0.0, 0.1);
        self.clock += dt;
        if let Some(link) = self.agents.as_mut() {
            link.scene.step(dt);
        }
        self.sync_agents(dt);
        // Les plus proches de la caméra.
        let mut order: Vec<(f32, u32)> = self
            .individuals
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let d = (p.x - camera[0]).powi(2) + (p.y - camera[1]).powi(2) + (p.z - camera[2]).powi(2);
                // Places d'agents libres : jamais animées.
                (if p.scale == 0.0 { f32::INFINITY } else { d }, i as u32)
            })
            .collect();
        let k = animated.min(order.len());
        if k > 0 && k < order.len() {
            order.select_nth_unstable_by(k - 1, |a, b| a.0.total_cmp(&b.0));
        }
        self.near = order[..k].iter().map(|o| o.1).collect();
        self.near.sort_unstable();
        // Proches : comportements complets ; prédateurs proches à portée.
        let predators: Vec<u32> = self
            .near
            .iter()
            .copied()
            .filter(|&i| {
                let s = &self.species[self.individuals[i as usize].species as usize];
                s.predator && !s.engine
            })
            .collect();
        let near = self.near.clone();
        for &i in &near {
            self.decide(i as usize, dt, &predators, &near);
        }
        // Déplacement : les proches à chaque image, les lointains par quart,
        // un quart par image avec un pas quadruple (ils ne sont que des
        // imposteurs, rafraîchis moins souvent).
        let half = PATCH_SIZE_M / 2.0 * 0.98;
        self.is_near.clear();
        self.is_near.resize(self.individuals.len(), false);
        for &i in &near {
            self.is_near[i as usize] = true;
        }
        self.tick = self.tick.wrapping_add(1);
        let slice = (self.tick % FAR_SLICES) as usize;
        for i in 0..self.individuals.len() {
            let close = self.is_near[i];
            if self.species[self.individuals[i].species as usize].engine {
                // Agents : déplacés par leur scène, posés au sol ici.
                if close || i % FAR_SLICES as usize == slice {
                    self.place_y(i);
                }
                continue;
            }
            if !close && i % FAR_SLICES as usize != slice {
                continue;
            }
            let dt = if close { dt } else { dt * FAR_SLICES as f32 };
            let ind = &mut self.individuals[i];
            let s = &self.species[ind.species as usize];
            let size = s.size_m * ind.scale;
            if ind.timer <= 0.0 && !close {
                // Lointains : errance seule.
                ind.action = if self.rng.unit() < 0.6 { Action::Feed } else { Action::Migrate };
                ind.timer = self.rng.range(4.0, 12.0);
                ind.heading += self.rng.range(-1.0, 1.0);
            }
            ind.timer -= dt;
            ind.time += dt;
            let sessile = matches!(s.rig.locomotion, Locomotion::Sessile);
            let target = if sessile { 0.0 } else { ind.action.pace() * size * if s.aquatic { 1.0 } else { 1.2 } };
            ind.speed += (target - ind.speed) * (dt * 3.0).min(1.0);
            let (sn, cs) = ind.heading.sin_cos();
            ind.x += cs * ind.speed * dt;
            ind.z += sn * ind.speed * dt;
            if ind.x.abs() > half || ind.z.abs() > half {
                ind.x = ind.x.clamp(-half, half);
                ind.z = ind.z.clamp(-half, half);
                ind.heading = (-ind.z).atan2(-ind.x);
            }
            self.place_y(i);
        }
    }

    /// Comportement d'un individu proche : fuir un prédateur, chasser une
    /// proie, suivre son troupeau, sinon tirer la prochaine action.
    fn decide(&mut self, i: usize, dt: f32, predators: &[u32], near: &[u32]) {
        let me = self.individuals[i];
        let s = &self.species[me.species as usize];
        let size = s.size_m * me.scale;
        if me.forced || s.engine {
            return;
        }
        let steer = |ind: &mut Individual, gx: f32, gz: f32, rate: f32| {
            let want = (gz - ind.z).atan2(gx - ind.x);
            let mut d = want - ind.heading;
            while d > PI {
                d -= TAU;
            }
            while d < -PI {
                d += TAU;
            }
            ind.heading += d.clamp(-rate * dt, rate * dt);
        };
        if !s.predator {
            // Un prédateur à moins de 30 longueurs : fuite.
            let threat = predators.iter().map(|&p| (p, self.individuals[p as usize])).find(|(_, p)| {
                let ps = &self.species[p.species as usize];
                ps.size_m > s.size_m && (p.x - me.x).hypot(p.z - me.z) < 30.0 * size.max(ps.size_m)
            });
            if let Some((_, p)) = threat {
                let ind = &mut self.individuals[i];
                ind.action = Action::Flee;
                ind.timer = 2.0;
                steer(ind, 2.0 * ind.x - p.x, 2.0 * ind.z - p.z, 4.0);
                return;
            }
        } else if matches!(me.action, Action::Hunt) && me.other != u32::MAX {
            let prey = self.individuals[me.other as usize];
            let ind = &mut self.individuals[i];
            steer(ind, prey.x, prey.z, 3.0);
            if (prey.x - ind.x).hypot(prey.z - ind.z) < size * 0.8 {
                // Prise : une bouchée, puis le repos.
                ind.action = Action::Feed;
                ind.timer = 6.0;
                ind.other = u32::MAX;
            }
            return;
        }
        if me.timer > 0.0 {
            if matches!(me.action, Action::Migrate | Action::Feed | Action::Communicate) && s.social > 0.5 {
                // Troupeau : vers le centre des congénères proches.
                let (mut cx, mut cz, mut n) = (0.0, 0.0, 0.0);
                for &j in near.iter().take(400) {
                    let o = &self.individuals[j as usize];
                    if o.species == me.species && (o.x - me.x).hypot(o.z - me.z) < 40.0 * size {
                        cx += o.x;
                        cz += o.z;
                        n += 1.0;
                    }
                }
                if n > 1.0 {
                    let ind = &mut self.individuals[i];
                    steer(ind, cx / n, cz / n, 0.6 * s.social);
                }
            }
            return;
        }
        // Prochaine action.
        let r = self.rng.unit();
        let (action, timer) = if s.predator {
            let prey = near.iter().copied().filter(|&j| j as usize != i).find(|&j| {
                let o = &self.individuals[j as usize];
                let os = &self.species[o.species as usize];
                !os.predator && !os.engine && os.size_m < s.size_m && (o.x - me.x).hypot(o.z - me.z) < 60.0 * size
            });
            match (prey, r) {
                (Some(p), r) if r < 0.6 => {
                    self.individuals[i].other = p;
                    (Action::Hunt, 12.0)
                }
                (_, r) if r < 0.75 => (Action::Rest, 8.0),
                (_, r) if r < 0.85 => (Action::Migrate, 10.0),
                (_, r) if r < 0.92 => (Action::Communicate, 4.0),
                _ => (Action::Fight, 3.0),
            }
        } else {
            let table = [
                (Action::Feed, 0.36),
                (Action::Rest, 0.12),
                (Action::Migrate, 0.12),
                (Action::Communicate, 0.07),
                (Action::Mate, 0.06),
                (Action::Lay, 0.04),
                (Action::Care, 0.05),
                (Action::Hide, 0.05),
                (Action::Build, 0.05),
                (Action::Fight, 0.04),
                (Action::Hunt, 0.04),
            ];
            let mut acc = 0.0;
            let mut pick = Action::Feed;
            for (a, w) in table {
                acc += w;
                if r < acc {
                    pick = a;
                    break;
                }
            }
            (pick, self.rng.range(3.0, 9.0))
        };
        let ind = &mut self.individuals[i];
        ind.action = action;
        ind.timer = timer;
        if matches!(action, Action::Migrate | Action::Feed) {
            ind.heading += self.rng.range(-0.8, 0.8);
        }
    }

    /// Pose d'un individu, transformations par os.
    pub fn pose_of(&self, i: usize) -> Vec<Affine> {
        let ind = &self.individuals[i];
        let s = &self.species[ind.species as usize];
        let size = (s.size_m * ind.scale).max(1e-6);
        pose(&s.rig, &Motion { action: ind.action, time: ind.time, speed: ind.speed / size, phase: ind.phase })
    }

    /// Transformation d'instance (3 lignes de 4) : lacet, taille, position.
    pub fn instance_rows(&self, i: usize) -> [f32; 12] {
        let ind = &self.individuals[i];
        let s = &self.species[ind.species as usize];
        let k = s.size_m * ind.scale;
        // Le corps regarde vers +x ; lacet autour de y (x vers z positif).
        let (sn, cs) = (-ind.heading).sin_cos();
        [cs * k, 0.0, sn * k, ind.x, 0.0, k, 0.0, ind.y, -sn * k, 0.0, cs * k, ind.z]
    }

    /// Individu le plus proche d'un rayon (caméra), parmi les animés.
    pub fn pick(&self, origin: [f32; 3], dir: [f32; 3]) -> Option<u32> {
        let l = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]).sqrt().max(1e-9);
        let d = [dir[0] / l, dir[1] / l, dir[2] / l];
        self.near
            .iter()
            .filter_map(|&i| {
                let p = &self.individuals[i as usize];
                let s = &self.species[p.species as usize];
                let v = [p.x - origin[0], p.y - origin[1], p.z - origin[2]];
                let t = v[0] * d[0] + v[1] * d[1] + v[2] * d[2];
                if t <= 0.0 || p.scale == 0.0 {
                    return None;
                }
                let q = [v[0] - t * d[0], v[1] - t * d[1], v[2] - t * d[2]];
                let miss = (q[0] * q[0] + q[1] * q[1] + q[2] * q[2]).sqrt();
                let radius = (s.size_m * p.scale * 0.8).max(t * 0.01);
                (miss < radius).then_some((t, i))
            })
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .map(|x| x.1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn site(ocean: bool) -> Site {
        Site {
            cell: 42,
            elevation_m: if ocean { -30.0 } else { 300.0 },
            is_ocean: ocean,
            temperature_k: 290.0,
            ice_cover: 0.0,
            roughness_m: 300.0,
            life_rgb: Some([90, 130, 70]),
            biomass_per_m2: 0.1,
            sky: [0.8, 0.85, 0.9],
        }
    }

    #[test]
    fn bench_plans_give_animals_that_move() {
        let kinds = [BenchKind::Walker, BenchKind::Swimmer, BenchKind::Worm, BenchKind::Radial, BenchKind::Flyer];
        let org = Organisation { body_cells: 1e6, cell_types: 3, eukaryote: true, multicellular: true, ..Default::default() };
        let mut walkers = 0;
        for (k, kind) in kinds.iter().enumerate() {
            for seed in 0..4 {
                let plan = from_simulated(&bench_plan(*kind, seed + 10 * k as u64, 0.5), &org, seed);
                assert!(plan.is_valid(), "{kind:?}");
                let b = body::build(&plan, 1);
                assert!(b.lods[0].triangles() > 200, "{kind:?}");
                assert!(b.extent_m > 0.3 && b.extent_m < 2.0, "{kind:?} {}", b.extent_m);
                let r = rig(&plan, &b.bones, *kind == BenchKind::Swimmer);
                if matches!(r.locomotion, Locomotion::Walk { .. }) {
                    walkers += 1;
                }
                if *kind == BenchKind::Flyer {
                    assert_eq!(r.locomotion, Locomotion::Fly);
                }
                if *kind == BenchKind::Swimmer {
                    assert_eq!(r.locomotion, Locomotion::Swim);
                }
            }
        }
        assert!(walkers >= 4, "{walkers}");
    }

    #[test]
    fn engine_agents_drive_their_species_in_the_crowd() {
        use evo_agents::bench::{sample, with_body};
        use evo_agents::{Diet, PopRates};
        let s = site(false);
        let patch = Patch::new(s.clone(), 9);
        let figurants = bench_species(&s, 9, 2);
        let org = Organisation { body_cells: 1e6, cell_types: 3, eukaryote: true, multicellular: true, sexual: true, ..Default::default() };
        let body = |kind, size: f32, seed| {
            let plan = from_simulated(&bench_plan(kind, seed, size as f64), &org, seed);
            SceneSpecies::new(format!("essai {seed}"), Some(seed as u32), plan, size, false, None)
        };
        let prey = with_body(
            sample(PopRates { birth: 1.0, death: 1.0, predation: 0.7, emigration: 0.2 }, 80, 3, true),
            0.2,
            1.2,
            4.0,
            Diet::Grazer,
        );
        let pred = with_body(
            sample(PopRates { birth: 0.3, death: 0.3, predation: 0.0, emigration: 0.05 }, 6, 4, true),
            1.0,
            3.0,
            12.0,
            Diet::Predator,
        );
        let real = vec![(body(BenchKind::Walker, 0.2, 11), prey), (body(BenchKind::Walker, 1.0, 12), pred)];
        let mut crowd = Crowd::with_agents(patch, figurants, real, 500, 9);
        let n = crowd.individuals.len();
        let mut actions = std::collections::HashSet::new();
        for _ in 0..1200 {
            crowd.step(0.05, [0.0, 20.0, 0.0], 400);
            assert_eq!(crowd.individuals.len(), n, "la foule ne change jamais de taille");
            for (i, ind) in crowd.individuals.iter().enumerate() {
                if crowd.species[ind.species as usize].engine && !crowd.is_vacant(i) {
                    actions.insert(ind.action);
                    assert!(ind.x.is_finite() && ind.y.is_finite());
                }
            }
        }
        let scene = crowd.agents().expect("agents");
        let living = crowd
            .individuals
            .iter()
            .enumerate()
            .filter(|(i, ind)| crowd.species[ind.species as usize].engine && !crowd.is_vacant(*i))
            .count();
        let agents = scene.agents.iter().filter(|a| !matches!(a.fate, evo_agents::agents::Fate::Dead { .. })).count();
        assert_eq!(living, agents, "chaque agent a une place visible");
        assert!(scene.kills > 0, "aucune prise montrée");
        for a in [Action::Feed, Action::Hunt, Action::Migrate] {
            assert!(actions.contains(&a), "action {a:?} jamais montrée : {actions:?}");
        }
    }

    #[test]
    fn a_crowd_keeps_its_animals_on_the_ground_and_reacts_to_predators() {
        for ocean in [false, true] {
            let s = site(ocean);
            let patch = Patch::new(s.clone(), 7);
            let species = bench_species(&s, 7, 4);
            assert!(species.iter().any(|x| x.predator));
            let mut crowd = Crowd::new(patch, species, 3000, 7);
            for _ in 0..40 {
                crowd.step(0.05, [0.0, 50.0, 0.0], 300);
            }
            assert_eq!(crowd.near.len(), 300);
            for ind in &crowd.individuals {
                assert!(ind.x.is_finite() && ind.y.is_finite() && ind.z.is_finite());
                let ground = crowd.patch.height_at(ind.x, ind.z);
                assert!(ind.y >= ground - 1e-3, "sous le sol : {} < {ground}", ind.y);
                if ocean {
                    assert!(ind.y <= 0.0 + 1e-3 || crowd.species[ind.species as usize].aquatic);
                }
            }
            // Une proie près d'un prédateur fuit.
            let pred = crowd.individuals.iter().position(|i| crowd.species[i.species as usize].predator).unwrap();
            let prey = crowd.individuals.iter().position(|i| !crowd.species[i.species as usize].predator).unwrap();
            let p = crowd.individuals[pred];
            crowd.individuals[prey].x = p.x + 0.5;
            crowd.individuals[prey].z = p.z;
            crowd.step(0.05, [p.x, p.y + 5.0, p.z], 300);
            assert_eq!(crowd.individuals[prey].action, Action::Flee);
            // Les poses sont finies.
            for &i in crowd.near.iter().take(50) {
                assert!(crowd.pose_of(i as usize).iter().all(|m| m.rows().iter().all(|v| v.is_finite())));
            }
        }
    }

    #[test]
    fn the_patch_is_reproducible() {
        let a = Patch::new(site(false), 3);
        let b = Patch::new(site(false), 3);
        assert_eq!(a.heights, b.heights);
        let (pos, nor, col, idx) = a.mesh();
        assert_eq!(pos.len(), PATCH_RES * PATCH_RES);
        assert_eq!(nor.len(), pos.len());
        assert_eq!(col.len(), pos.len());
        assert_eq!(idx.len(), (PATCH_RES - 1) * (PATCH_RES - 1) * 6);
    }
}
