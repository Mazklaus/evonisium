//! Bancs d'essai : des mondes écrits à la main pour les tests historiques
//! de l'étape 7, en attendant que le moteur fournisse le milieu (Planète),
//! les espèces sauvages (populations) et les peuples (`evo-culture`).
//!
//! * [`earth`] : une Terre grossière en cellules de 5° (continents, déserts,
//!   montagnes, fleuves, gisements, espèces sauvages par région), partie de
//!   −15 000 ans avec des chasseurs-cueilleurs du Paléolithique supérieur ;
//!   le climat glaciaire, instable, cède à l'Holocène à −11 700 ans.
//! * [`desert`] : un monde désertique de la vague 1, sans charbon ni pétrole.
//! * [`ocean`] : un monde océan de la vague 1 peuplé d'une espèce aquatique.
//! * [`two_species`] : deux espèces intelligentes sur un même continent.
//! * [`modern`] : la Terre de l'an 1750, pour vérifier la fin de la
//!   première version.
//!
//! [Simplification] Les contours, les climats et les gisements sont des
//! boîtes en latitude et longitude, à l'échelle de la cellule ; les ordres de
//! grandeur suivent la Terre réelle (climat par la latitude et l'altitude,
//! désert subtropical, fleuves irrigables), sans prétendre à la carte.

use crate::catalog::{catalog, Known};
use crate::land::{AnimalTraits, Deposits, Land, PlantOrgan, PlantTraits, WildKind, WildSpecies};
use crate::science::Truths;
use crate::world::SocietyWorld;
use evo_core::math::Det;
use evo_culture::catalog::Milieu;
use evo_culture::cognition::presets;

/// Pas de la grille, degrés.
pub const STEP_DEG: f64 = 5.0;
/// Année de départ du banc Terre.
pub const EARTH_START: i64 = -15_000;
/// Fin du dernier âge glaciaire.
pub const HOLOCENE: i64 = -11_700;
/// Variabilité du climat glaciaire, puis holocène.
pub const GLACIAL_VARIABILITY: f64 = 0.7;
pub const HOLOCENE_VARIABILITY: f64 = 0.1;

/// Identifiants des espèces intelligentes des bancs.
pub const HUMANS: u32 = 1;
pub const OTHERS: u32 = 2;

/// Boîte : latitude min, max, longitude min, max.
pub type Area = (f64, f64, f64, f64);
/// Espèce sauvage et ses aires.
pub type Range = (WildSpecies, Vec<Area>);

fn inside(lat: f64, lon: f64, b: Area) -> bool {
    lat >= b.0 && lat < b.1 && lon >= b.2 && lon < b.3
}

fn any(lat: f64, lon: f64, boxes: &[Area]) -> bool {
    boxes.iter().any(|&b| inside(lat, lon, b))
}

/// Terres émergées : (latitude min, max, longitude min, max).
const EARTH_LAND: &[Area] = &[
    // Afrique.
    (5.0, 37.0, -17.0, 35.0),
    (0.0, 12.0, 35.0, 51.0),
    (-20.0, 5.0, 9.0, 42.0),
    (-35.0, -20.0, 15.0, 35.0),
    (-25.0, -12.0, 43.0, 50.0),
    // Europe.
    (36.0, 44.0, -10.0, 3.0),
    (43.0, 55.0, -5.0, 30.0),
    (38.0, 46.0, 8.0, 18.0),
    (37.0, 46.0, 18.0, 28.0),
    (55.0, 71.0, 5.0, 30.0),
    (50.0, 59.0, -6.0, 2.0),
    // Russie et Sibérie.
    (45.0, 72.0, 30.0, 180.0),
    // Moyen-Orient, Iran, Asie centrale.
    (36.0, 42.0, 26.0, 45.0),
    (29.0, 37.0, 34.0, 48.0),
    (13.0, 30.0, 36.0, 58.0),
    (25.0, 40.0, 44.0, 63.0),
    (37.0, 55.0, 50.0, 90.0),
    // Inde, Asie du Sud-Est, Chine, Corée, Japon, Mongolie.
    (8.0, 30.0, 68.0, 90.0),
    (25.0, 37.0, 60.0, 100.0),
    (0.0, 25.0, 92.0, 110.0),
    (20.0, 45.0, 98.0, 123.0),
    (34.0, 42.0, 125.0, 130.0),
    (31.0, 45.0, 130.0, 145.0),
    (40.0, 55.0, 87.0, 135.0),
    // Insulinde, Nouvelle-Guinée, Australie.
    (-8.0, 5.0, 95.0, 120.0),
    (-10.0, -1.0, 131.0, 150.0),
    (-39.0, -11.0, 113.0, 153.0),
    // Amérique du Nord et centrale.
    (50.0, 72.0, -168.0, -60.0),
    (30.0, 50.0, -125.0, -67.0),
    (25.0, 30.0, -110.0, -80.0),
    (15.0, 30.0, -117.0, -87.0),
    (7.0, 18.0, -92.0, -77.0),
    // Amérique du Sud.
    (-5.0, 12.0, -80.0, -50.0),
    (-25.0, -5.0, -75.0, -35.0),
    (-40.0, -25.0, -72.0, -48.0),
    (-55.0, -40.0, -75.0, -63.0),
];

/// Mers intérieures et golfes retirés des boîtes de terres.
const EARTH_SEA: &[Area] = &[
    (40.0, 46.0, 28.0, 41.0),   // mer Noire
    (37.0, 47.0, 47.0, 54.0),   // Caspienne
    (24.0, 30.0, 48.0, 56.0),   // golfe Persique
    (12.0, 28.0, 33.0, 43.0),   // mer Rouge (étroite : une rangée)
    (31.0, 36.0, 0.0, 30.0),    // Méditerranée orientale et centrale
    (55.0, 66.0, 15.0, 25.0),   // Baltique
    (51.0, 63.0, -95.0, -77.0), // baie d'Hudson
];

/// Déserts : pluie forcée, mm.
const EARTH_DESERTS: &[(Area, f64)] = &[
    ((16.0, 32.0, -15.0, 30.0), 40.0),
    ((15.0, 30.0, 40.0, 58.0), 70.0),
    ((25.0, 33.0, 55.0, 70.0), 150.0),
    ((37.0, 48.0, 52.0, 110.0), 220.0),
    ((-30.0, -18.0, 120.0, 145.0), 220.0),
    ((-28.0, -18.0, 15.0, 26.0), 300.0),
    ((28.0, 38.0, -118.0, -105.0), 280.0),
    ((-30.0, -15.0, -72.0, -68.0), 50.0),
];

/// Montagnes : relief et altitude, km.
const EARTH_MOUNTAINS: &[(Area, f64, f64)] = &[
    ((28.0, 38.0, 75.0, 100.0), 0.9, 4.2),
    ((-50.0, 10.0, -80.0, -70.0), 0.7, 2.8),
    ((35.0, 60.0, -125.0, -105.0), 0.6, 1.8),
    ((44.0, 48.0, 5.0, 15.0), 0.5, 1.2),
    ((30.0, 40.0, 44.0, 50.0), 0.45, 1.3),
    ((5.0, 15.0, 35.0, 42.0), 0.5, 2.0),
    ((15.0, 22.0, -105.0, -95.0), 0.4, 1.8),
];

/// Fleuves irrigables : part des terres irrigables.
const EARTH_RIVERS: &[(Area, f64)] = &[
    ((22.0, 32.0, 28.0, 34.0), 0.6),   // Nil
    ((29.0, 37.0, 38.0, 48.0), 0.6),   // Tigre et Euphrate
    ((23.0, 33.0, 65.0, 76.0), 0.5),   // Indus
    ((21.0, 30.0, 76.0, 92.0), 0.5),   // Gange
    ((32.0, 41.0, 103.0, 122.0), 0.5), // fleuve Jaune
    ((26.0, 33.0, 100.0, 123.0), 0.5), // Yangzi
    ((8.0, 22.0, 95.0, 108.0), 0.4),   // Mékong, Irrawaddy
    ((30.0, 45.0, -96.0, -85.0), 0.4), // Mississippi
    ((-5.0, 0.0, -70.0, -50.0), 0.3),  // Amazone
    ((42.0, 49.0, 15.0, 30.0), 0.3),   // Danube
    ((10.0, 18.0, -10.0, 5.0), 0.3),   // Niger
    ((45.0, 58.0, 35.0, 50.0), 0.3),   // Volga
];

/// Gisements : (boîte, charbon t C, pétrole t C, cuivre, étain, fer,
/// uranium) par cellule.
type DepositBox = (Area, Deposits);

fn earth_deposits() -> Vec<DepositBox> {
    let coal = |c: f64| Deposits { coal_c: c, ..Default::default() };
    let oil = |o: f64| Deposits { oil_c: o, ..Default::default() };
    // Le fer est partout abondant (minerais de fer rubanés, fer des marais).
    let ore = |cu: f64, sn: f64, fe: f64| Deposits { copper: cu, tin: sn, iron: fe * 100.0, ..Default::default() };
    vec![
        ((50.0, 56.0, -5.0, 2.0), coal(3e10)),
        ((48.0, 53.0, 2.0, 25.0), coal(3e10)),
        ((36.0, 42.0, -88.0, -75.0), coal(6e10)),
        ((34.0, 42.0, 105.0, 120.0), coal(8e10)),
        ((20.0, 26.0, 82.0, 88.0), coal(2e10)),
        ((50.0, 58.0, 60.0, 90.0), coal(4e10)),
        ((-35.0, -20.0, 145.0, 153.0), coal(3e10)),
        ((-30.0, -24.0, 26.0, 32.0), coal(2e10)),
        ((24.0, 32.0, 44.0, 56.0), oil(8e10)),
        ((38.0, 42.0, 45.0, 52.0), oil(1e10)),
        ((27.0, 34.0, -104.0, -94.0), oil(2e10)),
        ((55.0, 65.0, 65.0, 80.0), oil(4e10)),
        ((5.0, 11.0, -72.0, -62.0), oil(3e10)),
        ((0.0, 8.0, 0.0, 10.0), oil(1e10)),
        ((34.0, 42.0, 30.0, 45.0), ore(2e6, 0.0, 5e6)),
        ((28.0, 36.0, 50.0, 60.0), ore(2e6, 1e5, 5e6)),
        ((-30.0, -15.0, -75.0, -65.0), ore(5e7, 3e6, 1e7)),
        ((-15.0, -8.0, 22.0, 30.0), ore(3e7, 0.0, 1e7)),
        ((48.0, 52.0, -6.0, 15.0), ore(1e6, 2e6, 2e7)),
        ((0.0, 12.0, 95.0, 108.0), ore(1e6, 5e6, 5e6)),
        ((40.0, 50.0, -95.0, -75.0), ore(2e7, 0.0, 5e7)),
        ((50.0, 60.0, 55.0, 62.0), ore(1e7, 0.0, 3e7)),
        ((-30.0, -20.0, 115.0, 125.0), ore(5e6, 0.0, 8e7)),
        ((-5.0, 5.0, 20.0, 30.0), Deposits { uranium: 1e5, ..Default::default() }),
        ((55.0, 60.0, -110.0, -100.0), Deposits { uranium: 1e5, ..Default::default() }),
        ((42.0, 50.0, 60.0, 75.0), Deposits { uranium: 1e5, ..Default::default() }),
    ]
}

#[allow(clippy::too_many_arguments)]
fn plant(
    id: u32,
    name: &str,
    organ: PlantOrgan,
    seed_mg: f64,
    annual: bool,
    selfing: bool,
    yield_t_ha: f64,
    temp_c: f64,
    rain_mm: f64,
) -> WildSpecies {
    WildSpecies {
        id,
        name: name.to_string(),
        kind: WildKind::Plant(PlantTraits { organ, seed_mg, annual, selfing, wild_yield_t_ha: yield_t_ha, temp_c, rain_mm }),
    }
}

#[allow(clippy::too_many_arguments)]
fn animal(
    id: u32,
    name: &str,
    mass_kg: f64,
    herbivory: f64,
    maturity_years: f64,
    captive_breeding: bool,
    aggression: f64,
    panic: f64,
    herd: f64,
    milk: bool,
) -> WildSpecies {
    WildSpecies {
        id,
        name: name.to_string(),
        kind: WildKind::Animal(AnimalTraits { mass_kg, herbivory, maturity_years, captive_breeding, aggression, panic, herd, milk }),
    }
}

/// Espèces sauvages de la Terre et leurs aires.
fn earth_wild() -> Vec<Range> {
    use PlantOrgan::*;
    vec![
        (plant(100, "le blé sauvage", Seed, 30.0, true, true, 0.8, 16.0, 450.0), vec![(30.0, 40.0, 33.0, 50.0)]),
        (plant(101, "l'orge sauvage", Seed, 35.0, true, true, 0.7, 15.0, 400.0), vec![(28.0, 40.0, 30.0, 60.0)]),
        (plant(102, "le riz sauvage", Seed, 25.0, true, true, 0.6, 20.0, 1300.0), vec![(20.0, 33.0, 100.0, 123.0)]),
        (plant(103, "le millet sauvage", Seed, 8.0, true, true, 0.6, 12.0, 500.0), vec![(33.0, 46.0, 100.0, 125.0)]),
        (plant(104, "le téosinte", Seed, 20.0, true, false, 0.5, 21.0, 900.0), vec![(15.0, 25.0, -106.0, -88.0)]),
        (plant(105, "la pomme de terre sauvage", Root, 0.0, false, false, 0.6, 10.0, 800.0), vec![(-25.0, -5.0, -80.0, -65.0)]),
        (plant(106, "le taro", Root, 0.0, false, false, 0.6, 25.0, 2500.0), vec![(-10.0, 5.0, 120.0, 150.0)]),
        (plant(107, "le sorgho sauvage", Seed, 25.0, true, true, 0.5, 26.0, 650.0), vec![(8.0, 17.0, -10.0, 40.0)]),
        (plant(108, "l'igname", Root, 0.0, false, false, 0.5, 26.0, 1400.0), vec![(4.0, 10.0, -12.0, 10.0)]),
        (plant(109, "le tournesol sauvage", Seed, 6.0, true, false, 0.4, 14.0, 900.0), vec![(33.0, 42.0, -95.0, -78.0)]),
        (plant(110, "l'avoine sauvage", Seed, 18.0, true, true, 0.4, 10.0, 700.0), vec![(40.0, 55.0, -10.0, 40.0)]),
        (animal(200, "la chèvre sauvage", 50.0, 1.0, 1.0, true, 0.2, 0.3, 0.9, true), vec![(30.0, 42.0, 30.0, 60.0)]),
        (animal(201, "l'aurochs", 700.0, 1.0, 2.0, true, 0.5, 0.3, 0.8, true), vec![(30.0, 55.0, -10.0, 120.0), (10.0, 30.0, 68.0, 90.0)]),
        (animal(202, "le sanglier", 100.0, 0.6, 1.0, true, 0.4, 0.3, 0.5, false), vec![(10.0, 55.0, -10.0, 125.0)]),
        (animal(203, "le cheval sauvage", 400.0, 1.0, 2.0, true, 0.3, 0.5, 0.8, true), vec![(42.0, 56.0, 30.0, 100.0)]),
        (animal(204, "le chameau sauvage", 500.0, 1.0, 4.0, true, 0.3, 0.2, 0.6, true), vec![(15.0, 45.0, 35.0, 100.0)]),
        (animal(205, "le guanaco", 120.0, 1.0, 2.0, true, 0.2, 0.3, 0.8, false), vec![(-35.0, -5.0, -80.0, -63.0)]),
        (animal(206, "le zèbre", 350.0, 1.0, 2.0, true, 0.9, 0.8, 0.7, false), vec![(-35.0, 15.0, 10.0, 45.0)]),
        (animal(207, "le bison", 800.0, 1.0, 3.0, true, 0.7, 0.6, 0.8, false), vec![(30.0, 60.0, -120.0, -80.0)]),
        (animal(208, "le kangourou", 60.0, 1.0, 2.0, true, 0.3, 0.8, 0.3, false), vec![(-39.0, -11.0, 113.0, 153.0)]),
        (animal(209, "le cerf", 150.0, 1.0, 2.0, true, 0.3, 0.9, 0.4, false), vec![(30.0, 60.0, -125.0, 140.0)]),
        (animal(210, "l'éléphant", 4000.0, 1.0, 14.0, false, 0.5, 0.3, 0.7, false), vec![(-30.0, 25.0, -15.0, 105.0)]),
    ]
}

/// Climat d'une cellule de Terre, sans les cas particuliers.
fn earth_climate(lat: f64, alt_km: f64) -> (f64, f64) {
    let a = lat.abs();
    let temp = 27.0 - 0.55 * (a - 12.0).max(0.0) - 6.5 * alt_km;
    let rain = if a < 10.0 {
        2000.0
    } else if a < 18.0 {
        2000.0 - (a - 10.0) * 180.0
    } else if a < 32.0 {
        550.0
    } else if a < 60.0 {
        750.0
    } else {
        400.0
    };
    (temp, rain)
}

/// Fabrique les milieux d'une grille.
struct Grid {
    rows: usize,
    cols: usize,
    /// Index de cellule du monde, par case.
    cell: Vec<Option<u32>>,
    lat: Vec<f64>,
}

impl Grid {
    fn new() -> Grid {
        let rows = (180.0 / STEP_DEG) as usize;
        let cols = (360.0 / STEP_DEG) as usize;
        Grid { rows, cols, cell: vec![None; rows * cols], lat: (0..rows).map(|r| -90.0 + STEP_DEG * (r as f64 + 0.5)).collect() }
    }
    fn lon(&self, c: usize) -> f64 {
        -180.0 + STEP_DEG * (c as f64 + 0.5)
    }
    fn area(&self, r: usize) -> f64 {
        // 111,2 km par degré.
        let side = 111.2 * STEP_DEG;
        side * side * self.lat[r].to_radians().dcos()
    }
    fn neighbours(&self, r: usize, c: usize) -> Vec<(usize, usize)> {
        let mut out = Vec::new();
        for (dr, dc) in [(-1i64, 0i64), (1, 0), (0, -1), (0, 1)] {
            let rr = r as i64 + dr;
            if rr < 0 || rr >= self.rows as i64 {
                continue;
            }
            let cc = (c as i64 + dc).rem_euclid(self.cols as i64);
            out.push((rr as usize, cc as usize));
        }
        out
    }
}

/// Description d'un monde de banc.
pub struct Spec {
    pub land: fn(f64, f64) -> bool,
    pub climate: fn(f64, f64) -> Option<Land>,
    pub wild: Vec<Range>,
    /// Liens par mer à travers une case d'océan (détroits).
    pub straits: bool,
}

fn earth_land(lat: f64, lon: f64) -> bool {
    any(lat, lon, EARTH_LAND) && !any(lat, lon, EARTH_SEA) && lat > -56.0 && lat < 72.0
}

fn earth_cell(lat: f64, lon: f64) -> Option<Land> {
    let mut relief: f64 = 0.1;
    let mut alt: f64 = 0.2;
    for &(b, r, a) in EARTH_MOUNTAINS {
        if inside(lat, lon, b) {
            relief = relief.max(r);
            alt = alt.max(a);
        }
    }
    let (mut temp, mut rain) = earth_climate(lat, alt);
    // Intérieur des continents : plus froid l'hiver, plus sec.
    if inside(lat, lon, (45.0, 72.0, 60.0, 140.0)) {
        temp -= 4.0;
        rain *= 0.6;
    }
    for &(b, r) in EARTH_DESERTS {
        if inside(lat, lon, b) {
            rain = rain.min(r);
        }
    }
    let mut river: f64 = 0.0;
    for &(b, r) in EARTH_RIVERS {
        if inside(lat, lon, b) {
            river = river.max(r);
        }
    }
    Some(make_land(lat, lon, temp, rain, relief, river))
}

fn make_land(lat: f64, lon: f64, temp: f64, rain: f64, relief: f64, river: f64) -> Land {
    let warm = ((temp - 2.0) / 8.0).clamp(0.0, 1.0);
    let wet = ((rain - 250.0) / 400.0).clamp(0.0, 1.0);
    let arable = (0.35 * (1.0 - relief) * warm * wet).max(0.25 * river);
    let pasture = (0.45 * warm.max(0.3) * ((rain - 120.0) / 300.0).clamp(0.0, 1.0) * (1.0 - 0.5 * wet)).max(0.05);
    let forest = if rain > 500.0 && temp > -2.0 {
        let npp_like = (rain / 2000.0).min(1.0) * ((temp + 5.0) / 30.0).clamp(0.2, 1.0);
        (16_000.0 * npp_like).min(16_000.0)
    } else {
        0.0
    };
    Land {
        area_km2: 0.0,
        lat_deg: lat,
        lon_deg: lon,
        milieu: Milieu::Land,
        ocean: false,
        coast: false,
        temp_c: temp,
        rain_mm: rain,
        variability: HOLOCENE_VARIABILITY,
        relief,
        river,
        arable,
        pasture,
        forest_c_km2: forest,
        deposits: Deposits::default(),
        wind: if lat.abs() > 40.0 || relief > 0.4 { 0.6 } else { 0.35 },
        sun: if rain < 400.0 { 0.85 } else { 0.5 },
        geothermal: if relief > 0.6 { 0.4 } else { 0.05 },
        o2_percent: 21.0,
        sky: true,
        wild: Vec::new(),
    }
}

/// Un monde construit : le monde des sociétés et la position de chaque
/// cellule.
#[derive(Clone)]
pub struct Bench {
    pub world: SocietyWorld,
    /// (latitude, longitude) de chaque cellule.
    pub pos: Vec<(f64, f64)>,
    /// Le climat est-il glaciaire au départ (variabilité jusqu'à
    /// [`HOLOCENE`]) ?
    pub glacial: bool,
}

fn build(seed: u64, spec: &Spec, deposits: &[DepositBox]) -> Bench {
    let mut world = SocietyWorld::new(seed, Truths::earth());
    let mut g = Grid::new();
    let mut pos = Vec::new();
    for r in 0..g.rows {
        for c in 0..g.cols {
            let (lat, lon) = (g.lat[r], g.lon(c));
            if !(spec.land)(lat, lon) {
                continue;
            }
            let Some(mut land) = (spec.climate)(lat, lon) else { continue };
            land.area_km2 = g.area(r);
            land.coast = g.neighbours(r, c).iter().any(|&(rr, cc)| !(spec.land)(g.lat[rr], g.lon(cc)));
            for (b, d) in deposits {
                if inside(lat, lon, *b) {
                    let x = &mut land.deposits;
                    x.coal_c += d.coal_c;
                    x.oil_c += d.oil_c;
                    x.copper += d.copper;
                    x.tin += d.tin;
                    x.iron += d.iron;
                    x.uranium += d.uranium;
                }
            }
            // Un peu de fer et de sel presque partout.
            let n = world.noise((r * 1000 + c) as u64, 1);
            land.deposits.iron += 2e8 * n;
            if n > 0.7 {
                land.deposits.salt += 1e4;
            }
            for (w, boxes) in &spec.wild {
                if any(lat, lon, boxes) {
                    land.wild.push(w.id);
                }
            }
            let id = world.add_cell(land);
            g.cell[r * g.cols + c] = Some(id);
            pos.push((lat, lon));
        }
    }
    for (w, _) in &spec.wild {
        world.add_wild(w.clone());
    }
    // Liens par terre entre cases voisines.
    for r in 0..g.rows {
        for c in 0..g.cols {
            let Some(a) = g.cell[r * g.cols + c] else { continue };
            for (rr, cc) in g.neighbours(r, c) {
                if let Some(b) = g.cell[rr * g.cols + cc] {
                    if a < b {
                        let (la, lb) = (&world.cells[a as usize].land, &world.cells[b as usize].land);
                        let rough = 1.0 + 1.5 * la.relief.max(lb.relief) + if la.rain_mm.min(lb.rain_mm) < 150.0 { 1.0 } else { 0.0 };
                        world.link(a, b, rough, false);
                    }
                }
            }
        }
    }
    // Détroits : deux côtes séparées par une ou deux cases de mer.
    if spec.straits {
        for r in 0..g.rows {
            for c in 0..g.cols {
                let Some(a) = g.cell[r * g.cols + c] else { continue };
                if !world.cells[a as usize].land.coast {
                    continue;
                }
                for (dr, dc, cost) in [(0i64, 2i64, 1.5), (2, 0, 1.5), (0, 3, 2.5), (3, 0, 2.5), (2, 2, 2.5), (2, -2, 2.5)] {
                    let rr = r as i64 + dr;
                    if rr < 0 || rr >= g.rows as i64 {
                        continue;
                    }
                    let cc = (c as i64 + dc).rem_euclid(g.cols as i64) as usize;
                    let Some(b) = g.cell[rr as usize * g.cols + cc] else { continue };
                    if world.cells[a as usize].links.iter().any(|l| l.to == b) {
                        continue;
                    }
                    // Seulement si les cases entre les deux sont de la mer.
                    let mid = (((r as i64 + rr) / 2) as usize, ((c as i64 + dc / 2).rem_euclid(g.cols as i64)) as usize);
                    if g.cell[mid.0 * g.cols + mid.1].is_some() {
                        continue;
                    }
                    world.link(a, b, cost, true);
                }
            }
        }
    }
    let _ = &mut g;
    Bench { world, pos, glacial: false }
}

/// Bagage de la fin du Paléolithique supérieur (Kébarien, vers −20 000 :
/// arc, microlithes, meules, récolte des céréales sauvages à Ohalo II ;
/// hameaux permanents là où la nourriture abonde, comme les premiers
/// Natoufiens).
pub fn upper_palaeolithic() -> Known {
    let cat = catalog();
    let mut k = Known::default();
    for key in [
        "sonde",
        "percussion",
        "eclat",
        "biface",
        "corde",
        "abri",
        "recipient",
        "feu",
        "cuisson",
        "parure",
        "epieu",
        "radeau",
        "propulseur",
        "outil_os",
        "vetement",
        "peche",
        "aiguille",
        "art_rupestre",
        "harpon",
        "arc",
        "microlithes",
        "filet",
        "hamecon",
        "pieges",
        "meule",
        "stockage",
        "recolte_sauvage",
        "village",
    ] {
        if let Some(t) = cat.id(key) {
            k.set(t);
        }
    }
    k
}

/// Capacité de cueillette d'une cellule, en personnes.
fn forage_capacity(land: &Land) -> f64 {
    land.area_km2 * land.npp() * crate::economy::FORAGE_PER_NPP
}

impl Bench {
    /// Climat de l'année : glaciaire puis holocène.
    pub fn climate(&mut self) {
        if !self.glacial {
            return;
        }
        let v = if self.world.year < HOLOCENE { GLACIAL_VARIABILITY } else { HOLOCENE_VARIABILITY };
        if self.world.cells.first().map(|c| c.land.variability) != Some(v) {
            for c in &mut self.world.cells {
                c.land.variability = v;
            }
        }
    }

    /// Avance d'un an, climat compris.
    pub fn step(&mut self) {
        self.climate();
        self.world.step();
    }

    /// Distance en km entre deux cellules (grand cercle).
    pub fn km(&self, a: u32, b: u32) -> f64 {
        let (la, oa) = self.pos[a as usize];
        let (lb, ob) = self.pos[b as usize];
        let (la, lb) = (la.to_radians(), lb.to_radians());
        let d = (ob - oa).to_radians();
        let c = (la.dsin() * lb.dsin() + la.dcos() * lb.dcos() * d.dcos()).clamp(-1.0, 1.0);
        6371.0 * c.dacos()
    }

    /// Cellule la plus proche d'un point.
    pub fn at(&self, lat: f64, lon: f64) -> u32 {
        let mut best = (f64::MAX, 0u32);
        for (i, &(a, o)) in self.pos.iter().enumerate() {
            let d = (a - lat) * (a - lat) + (o - lon) * (o - lon);
            if d < best.0 {
                best = (d, i as u32);
            }
        }
        best.1
    }
}

/// La Terre à −15 000 ans : des chasseurs-cueilleurs dans l'Ancien Monde et
/// en Australie, l'Amérique encore vide sauf la Béringie.
pub fn earth(seed: u64) -> Bench {
    let spec = Spec { land: earth_land, climate: earth_cell, wild: earth_wild(), straits: true };
    let mut b = build(seed, &spec, &earth_deposits());
    b.world.year = EARTH_START;
    b.glacial = true;
    b.world.add_species(HUMANS, "humains", presets::human(), Milieu::Land);
    let known = upper_palaeolithic();
    let n = b.world.cells.len();
    for i in 0..n {
        let (lat, lon) = b.pos[i];
        let americas = lon < -30.0;
        let beringia = lat > 55.0 && lon < -150.0;
        if americas && !beringia {
            continue;
        }
        let land = &b.world.cells[i].land;
        let cap = forage_capacity(land);
        if cap < 500.0 {
            continue;
        }
        b.world.settle(i as u32, HUMANS, cap * 0.5, known, i as u32);
    }
    b.climate();
    b
}

fn desert_land(lat: f64, lon: f64) -> bool {
    // Deux continents et un archipel.
    inside(lat, lon, (-40.0, 45.0, -60.0, 40.0))
        || inside(lat, lon, (-30.0, 30.0, 70.0, 140.0))
        || inside(lat, lon, (-5.0, 10.0, 150.0, 175.0))
}

fn desert_cell(lat: f64, lon: f64) -> Option<Land> {
    let (temp, _) = earth_climate(lat, 0.3);
    let temp = temp + 3.0;
    // Pluie rare, sauf une bande et des oasis.
    let band = if lat.abs() < 8.0 { 650.0 } else { 160.0 };
    let river = if inside(lat, lon, (10.0, 30.0, -20.0, -10.0)) || inside(lat, lon, (-20.0, 0.0, 95.0, 105.0)) { 0.5 } else { 0.0 };
    let mut l = make_land(lat, lon, temp, band, 0.15, river);
    l.sun = 0.9;
    l.wind = 0.6;
    Some(l)
}

/// Monde désertique de la vague 1 : pas de charbon, pas de pétrole, pas de
/// grand herbivore domesticable ; du soleil et du vent.
pub fn desert(seed: u64) -> Bench {
    use PlantOrgan::*;
    let wild = vec![
        (plant(300, "le mil des sables", Seed, 22.0, true, true, 0.5, 26.0, 450.0), vec![(-10.0, 30.0, -60.0, 140.0)]),
        (plant(301, "la racine d'oasis", Root, 0.0, false, false, 0.5, 25.0, 600.0), vec![(-10.0, 10.0, -60.0, 175.0)]),
        (animal(302, "le coureur des dunes", 40.0, 1.0, 1.5, true, 0.2, 0.4, 0.8, true), vec![(-40.0, 45.0, -60.0, 140.0)]),
    ];
    let ore = vec![((-40.0, 45.0, -60.0, 40.0), Deposits { copper: 3e6, tin: 2e5, iron: 1e7, ..Default::default() })];
    let spec = Spec { land: desert_land, climate: desert_cell, wild, straits: true };
    let mut b = build(seed, &spec, &ore);
    b.world.year = 0;
    b.world.add_species(HUMANS, "humains", presets::human(), Milieu::Land);
    seed_peoples(&mut b, HUMANS, |lat, lon| inside(lat, lon, (-10.0, 10.0, -20.0, 20.0)));
    b
}

fn ocean_land(lat: f64, _lon: f64) -> bool {
    lat.abs() < 40.0
}

fn ocean_cell(lat: f64, lon: f64) -> Option<Land> {
    let (temp, _) = earth_climate(lat, 0.0);
    let mut l = make_land(lat, lon, temp, 1000.0, 0.1, 0.0);
    l.milieu = Milieu::Water;
    l.ocean = true;
    l.coast = inside(lat, lon, (-10.0, 10.0, -20.0, 20.0));
    l.sky = true;
    l.forest_c_km2 = 0.0;
    l.arable = 0.25;
    l.pasture = 0.0;
    l.geothermal = if (lon / 30.0).fract().abs() < 0.2 { 0.6 } else { 0.05 };
    l.deposits = Deposits { copper: 2e5, iron: 5e6, ..Default::default() };
    Some(l)
}

/// Monde océan de la vague 1 : une espèce aquatique, sans feu.
pub fn ocean(seed: u64) -> Bench {
    let spec = Spec { land: ocean_land, climate: ocean_cell, wild: Vec::new(), straits: false };
    let mut b = build(seed, &spec, &[]);
    b.world.year = 0;
    let mut c = presets::tentacled();
    c.manipulation.quality = 0.85;
    b.world.add_species(OTHERS, "céphalopodes", c, Milieu::Water);
    // Peuples d'essai de l'étape 6 : coquillages et coraux taillés.
    let mut known = upper_palaeolithic();
    for key in ["aq_coquilles", "aq_corail"] {
        if let Some(t) = catalog().id(key) {
            known.set(t);
        }
    }
    seed_with(&mut b, OTHERS, known, |lat, lon| inside(lat, lon, (-15.0, 15.0, -30.0, 30.0)));
    // Les cellules d'eau se pêchent : la « cueillette » de l'eau.
    b
}

/// Installe des bandes du Paléolithique supérieur dans une région.
fn seed_peoples(b: &mut Bench, species: u32, region: impl Fn(f64, f64) -> bool) {
    seed_with(b, species, upper_palaeolithic(), region);
}

fn seed_with(b: &mut Bench, species: u32, known: Known, region: impl Fn(f64, f64) -> bool) {
    let n = b.world.cells.len();
    for i in 0..n {
        let (lat, lon) = b.pos[i];
        if !region(lat, lon) {
            continue;
        }
        let land = &b.world.cells[i].land;
        let cap = forage_capacity(land).max(land.area_km2 * 0.05);
        b.world.settle(i as u32, species, cap * 0.5, known, i as u32);
    }
}

fn two_land(lat: f64, lon: f64) -> bool {
    inside(lat, lon, (-20.0, 40.0, -40.0, 40.0))
}

fn two_cell(lat: f64, lon: f64) -> Option<Land> {
    let (temp, rain) = earth_climate(lat, 0.2);
    Some(make_land(lat, lon, temp, rain.max(600.0), 0.15, if lon.abs() < 10.0 { 0.3 } else { 0.0 }))
}

/// Deux espèces intelligentes aux deux bouts d'un continent.
pub fn two_species(seed: u64) -> Bench {
    use PlantOrgan::*;
    let wild = vec![
        (plant(400, "l'herbe à grain", Seed, 30.0, true, true, 0.7, 18.0, 700.0), vec![(-20.0, 40.0, -40.0, 40.0)]),
        (animal(401, "le brouteur", 80.0, 1.0, 1.0, true, 0.2, 0.3, 0.9, true), vec![(-20.0, 40.0, -40.0, 40.0)]),
    ];
    let spec = Spec { land: two_land, climate: two_cell, wild, straits: false };
    let mut b = build(seed, &spec, &[]);
    b.world.year = 0;
    b.world.add_species(HUMANS, "humains", presets::human(), Milieu::Land);
    b.world.add_species(OTHERS, "tentaculés", presets::tentacled(), Milieu::Land);
    seed_peoples(&mut b, HUMANS, |lat, lon| inside(lat, lon, (0.0, 20.0, -40.0, -30.0)));
    seed_peoples(&mut b, OTHERS, |lat, lon| inside(lat, lon, (0.0, 20.0, 30.0, 40.0)));
    b
}

/// Les techniques connues vers 1750 dans les sociétés les plus avancées.
pub fn early_modern() -> Known {
    let cat = catalog();
    let mut k = Known::default();
    let mut todo: Vec<u16> = [
        "imprimerie",
        "navigation_hauturiere",
        "methode_scientifique",
        "lunette",
        "microscope",
        "thermometre",
        "balance_precision",
        "academie",
        "banque",
        "universite",
        "canon",
        "mousquet",
        "moulin_vent",
        "moulin_eau",
        "charrue_lourde",
        "assolement",
        "acier_trempe",
        "verre",
        "cartographie",
        "boussole",
        "algebre",
        "variolisation",
        "quarantaine",
        "routes",
        "urbanisme",
        "bureaucratie",
        "recensement",
        "droit_ecrit",
        "pieces",
        "ecole",
        "bibliotheque",
        "extraction_charbon",
        "haut_fourneau",
        "revue_scientifique",
        "lait",
        "laine",
        "equitation",
        "collier_epaule",
        "papier_monnaie",
        // Les débuts de la vapeur, comme l'Angleterre de 1750.
        "coke",
        "pompe_vapeur",
        "machine_outil",
    ]
    .iter()
    .filter_map(|key| cat.id(key))
    .collect();
    // Avec tous leurs prérequis.
    while let Some(t) = todo.pop() {
        if k.has(t) {
            continue;
        }
        k.set(t);
        let tech = cat.get(t);
        todo.extend(tech.pre.iter().copied());
        for group in &tech.any {
            if let Some(&first) = group.first() {
                todo.push(first);
            }
        }
    }
    k
}

/// La Terre de 1750 : sociétés agricoles et marchandes denses, savoirs de
/// l'époque moderne.
pub fn modern(seed: u64) -> Bench {
    let spec = Spec { land: earth_land, climate: earth_cell, wild: earth_wild(), straits: true };
    let mut b = build(seed, &spec, &earth_deposits());
    b.world.year = 1750;
    b.world.add_species(HUMANS, "humains", presets::human(), Milieu::Land);
    let known = early_modern();
    let n = b.world.cells.len();
    let mut kingdoms: std::collections::BTreeMap<(i64, i64), u32> = std::collections::BTreeMap::new();
    for i in 0..n {
        let land = &b.world.cells[i].land;
        // Les champs que la charrue permet, au rendement de 1750.
        let fields = land.area_km2 * land.arable * crate::economy::tilled_share(&known, land);
        let cap = fields * crate::economy::BASE_YIELD_KM2 * 1.5 + forage_capacity(land);
        if cap < 1_000.0 {
            continue;
        }
        // Des royaumes d'environ 15° de côté, comme l'Europe des
        // monarchies : une cellule par bloc fonde l'État, les autres le
        // rejoignent.
        let (lat, lon) = b.pos[i];
        let block = (((lat + 90.0) / 15.0) as i64, ((lon + 180.0) / 15.0) as i64);
        let s = match kingdoms.get(&block) {
            Some(&k) => {
                let own = b.world.settle(i as u32, HUMANS, cap * 0.4, known, i as u32);
                if let Some(st) = b.world.cells[i].settlement.as_mut() {
                    st.society = k;
                }
                let _ = own;
                k
            }
            None => {
                let k = b.world.settle(i as u32, HUMANS, cap * 0.4, known, i as u32);
                kingdoms.insert(block, k);
                k
            }
        };
        b.world.prime_modern(s, i as u32);
    }
    b
}
