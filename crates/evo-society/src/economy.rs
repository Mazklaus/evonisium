//! Économie d'une cellule habitée pour une année : produire, stocker,
//! répartir.
//!
//! Les flux sont physiques d'abord (rations, tonnes, mégawattheures), en
//! valeur ensuite. La nourriture passe en premier : le travail va à la
//! cueillette, à la pêche, aux champs et aux troupeaux jusqu'à couvrir les
//! besoins, dans l'ordre du meilleur rendement à la marge. Le reste du
//! travail sert, par ordre de priorité, le combustible, les outils, le
//! logement, les vêtements, les armes de l'armée, les écrits, le luxe des
//! élites, les commandes de l'État et l'investissement en machines, chacun
//! par la recette la moins coûteuse en travail parmi celles que la cellule
//! connaît. L'énergie (eau, vent, charbon, pétrole, électricité) multiplie
//! le rendement de l'artisanat et de l'industrie à hauteur des machines
//! disponibles.
//!
//! Rien n'écrit le piège malthusien : tant que l'énergie reste musculaire,
//! un gain de rendement nourrit plus de monde et la production par tête
//! revient vers le minimum vital ; l'énergie fossile multiplie la
//! production par travailleur plus vite que la population ne peut croître.

use crate::catalog::{catalog, Effects, Energy, Known, Recipe, Resource};
use crate::goods::{self, Good, Goods, GOODS, ZERO};
use crate::land::{Deposits, Land};
use evo_core::math::Det;
use serde::{Deserialize, Serialize};

/// Rations par km² de champ cultivé, au rendement de départ (0,5 t de
/// à 0,7 t de grain par hectare, 250 kg par ration).
pub const BASE_YIELD_KM2: f64 = 120.0;
/// Travailleurs par km² cultivé à la houe : environ un hectare
/// chacun, deux rations par travailleur, de quoi nourrir à peine un
/// spécialiste pour cinq paysans.
pub const BASE_FIELD_LABOUR_KM2: f64 = 95.0;
/// Rations par km² et par kg C·m⁻²·an⁻¹ de production primaire, pour la
/// chasse et la cueillette (0,1 personne par km² en forêt tempérée).
pub const FORAGE_PER_NPP: f64 = 0.22;
/// Rations par travailleur à la cueillette quand le gibier abonde.
pub const FORAGE_PER_WORKER: f64 = 3.0;
pub const FISH_PER_WORKER: f64 = 3.5;
/// Bêtes (unités de gros bétail) par km² de pâture.
pub const LIVESTOCK_KM2: f64 = 8.0;
/// Énergie d'une tonne : bois, charbon de bois, charbon, pétrole, MWh.
pub const MWH_PER_T: [(Good, f64); 5] =
    [(Good::Wood, 4.2), (Good::Charcoal, 8.0), (Good::Coal, 7.5), (Good::Oil, 11.6), (Good::Electricity, 1.0)];
/// Part de la population qui travaille (les enfants aident aux champs).
pub const WORK_SHARE: f64 = 0.6;
/// Travail utile que délivre une unité de machines, MWh par an.
pub const MACHINE_MWH: f64 = 20.0;
/// Travail musculaire d'un travailleur, MWh par an (environ 75 W).
pub const MUSCLE_MWH: f64 = 0.3;

/// Ce que la société impose à la cellule.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Policy {
    /// Part des élites dans la population.
    pub elites: f64,
    /// Part des travailleurs sous les armes.
    pub army: f64,
    /// Part des travailleurs au service des institutions (prêtres, scribes,
    /// juges, maîtres).
    pub specialists: f64,
    /// Bâtiments commandés par l'État, par habitant.
    pub monuments: f64,
    /// Impôt ou tribut, part du surplus.
    pub tax: f64,
    /// Part des gens qui savent lire.
    pub literacy: f64,
    /// Marché : les prix existent.
    pub market: bool,
    /// Investissement en machines (une société industrielle épargne).
    pub invest: bool,
}

impl Default for Policy {
    fn default() -> Policy {
        Policy { elites: 0.0, army: 0.0, specialists: 0.0, monuments: 0.0, tax: 0.0, literacy: 0.0, market: false, invest: false }
    }
}

/// Ce qu'une cellule connaît de sa nourriture domestique.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Domesticates {
    /// Rendement relatif de la plante cultivée (sélection comprise) et son
    /// accord avec le climat de la cellule ; 0 sans plante.
    pub crop: f64,
    /// Bien produit par la plante.
    pub crop_good: Good,
    /// Bétail : accord avec le climat, 0 sans animal ; donne-t-il du lait ?
    pub herd: f64,
    pub milk: bool,
}

/// État économique d'une cellule d'une année à l'autre.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Economy {
    pub stocks: Goods,
    pub cropland_km2: f64,
    pub irrigated_km2: f64,
    pub livestock: f64,
    /// Fertilité des sols cultivés, de 0 à 1.
    pub soil: f64,
    /// Carbone des forêts de la cellule, t C.
    pub forest_c: f64,
    pub deposits: Deposits,
    /// Part du combustible des moteurs qui a manqué l'an passé, par source
    /// (charbon, pétrole, électricité) : on n'attend plus d'un moteur qu'il
    /// tourne sans ce qu'il brûle.
    #[serde(default)]
    pub engine_short: [f64; 3],
}

/// Flux d'une cellule vers la planète, pour une année.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Flux {
    /// CO₂ émis, t C (combustion, défrichement, ciment), net de la repousse
    /// des forêts.
    pub co2_c: f64,
    /// Méthane du bétail et des rizières, t C.
    pub ch4_c: f64,
    /// Aérosols soufrés du charbon, t.
    pub aerosol_t: f64,
    pub cropland_km2: f64,
    pub pasture_km2: f64,
    pub irrigated_km2: f64,
    /// Forêt défrichée ou coupée, t C.
    pub deforestation_c: f64,
    /// Sol emporté, t.
    pub erosion_t: f64,
    /// Part du CO₂ qui vient des carbonates (ciment), t C : ce carbone sort
    /// des roches, pas des stocks lents tenus ici.
    pub carbonate_c: f64,
}

/// Bilan d'une année.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Year {
    pub produced: Goods,
    pub consumed: Goods,
    /// Demande restée sans réponse.
    pub shortage: Goods,
    /// Rations disponibles par personne (les élites mangent double).
    pub food_pc: f64,
    /// Production en rations de référence, et par personne.
    pub gdp: f64,
    pub gdp_pc: f64,
    /// Travailleurs aux champs, aux troupeaux, à la cueillette et à la pêche.
    pub food_workers: f64,
    pub workers: f64,
    /// Travail utile non musculaire par travailleur, MWh.
    pub power_pc: f64,
    /// Part de l'énergie utile d'origine fossile ou mieux.
    pub modern_energy: f64,
    /// Part de l'énergie utile tirée du charbon et du pétrole.
    #[serde(default)]
    pub fossil_energy: f64,
    /// Combustible brûlé, par source, MWh.
    pub fuel_mwh: [f64; 5],
    /// Surplus imposable, en rations.
    pub surplus: f64,
    /// Dépenses publiques engagées dans la cellule, en rations.
    pub public: f64,
    pub flux: Flux,
    /// Prix (seulement avec un marché), en rations.
    pub prices: Option<Goods>,
}

/// Part des terres labourables qu'on sait travailler : à la houe, les sols
/// légers et les berges ; l'araire ouvre les plaines sèches, la charrue
/// lourde les terres grasses du Nord (White, 1962), le tracteur le reste.
/// Les berges des fleuves se cultivent dès la houe (le Nil, l'Indus).
pub fn tilled_share(known: &Known, land: &Land) -> f64 {
    let cat = catalog();
    let has = |k: &str| cat.id(k).map(|t| known.has(t)).unwrap_or(false);
    let tools = if has("tracteur") {
        1.0
    } else if has("charrue_lourde") {
        0.3
    } else if has("araire") {
        0.12
    } else {
        0.04
    };
    // Les plaines d'inondation se cultivent à la houe (Nil, Indus).
    (tools + 0.15 * land.river).min(1.0)
}

/// Effets d'énergie connus : rendement par source.
fn energy_efficiency(known: &Known) -> [f64; 10] {
    let cat = catalog();
    let mut e = [0.0; 10];
    for t in known.iter() {
        if let Some((src, eff)) = cat.get(t).energy {
            let i = Energy::ALL.iter().position(|&s| s == src).unwrap_or(0);
            e[i] = f64::max(e[i], eff);
        }
    }
    e
}

/// La cellule a-t-elle la ressource ?
pub fn has_resource(r: Resource, land: &Land, eco: &Economy, dom: &Domesticates, wild: &[&crate::land::WildSpecies]) -> bool {
    let d = &eco.deposits;
    match r {
        Resource::River => land.river > 0.1,
        Resource::Coast => land.coast,
        Resource::Forest => eco.forest_c > 500.0 * land.area_km2,
        Resource::Coal => d.coal_c > 1.0,
        Resource::Oil => d.oil_c > 1.0,
        Resource::Copper => d.copper > 0.1,
        Resource::Tin => d.tin > 0.1,
        Resource::Iron => d.iron > 0.1,
        Resource::Uranium => d.uranium > 0.0,
        Resource::Salt => d.salt > 0.0 || land.coast,
        Resource::SeedPlant => {
            (dom.crop > 0.0 && dom.crop_good == Good::Grain)
                || wild.iter().any(|w| w.good() == Good::Grain && w.cultivability() * w.suits(land.temp_c, land.rain_mm) > 0.15)
        }
        Resource::RootPlant => {
            (dom.crop > 0.0 && dom.crop_good == Good::Roots)
                || wild.iter().any(|w| w.good() == Good::Roots && w.cultivability() * w.suits(land.temp_c, land.rain_mm) > 0.15)
        }
        Resource::Animal => dom.herd > 0.0 || wild.iter().any(|w| w.tameness() * w.suits(land.temp_c, land.rain_mm) > 0.2),
        Resource::MilkAnimal => {
            dom.milk
                || wild.iter().any(|w| {
                    matches!(&w.kind, crate::land::WildKind::Animal(a) if a.milk) && w.tameness() * w.suits(land.temp_c, land.rain_mm) > 0.2
                })
        }
        Resource::Mount => {
            wild.iter().any(|w| matches!(&w.kind, crate::land::WildKind::Animal(a) if a.mass_kg >= 200.0) && w.tameness() > 0.3)
        }
        Resource::Herds => eco.livestock > 1.0,
        Resource::Wind => land.wind > 0.4,
        Resource::Sun => land.sun > 0.5,
        Resource::Geothermal => land.geothermal > 0.3,
        Resource::Grassland => land.pasture > 0.3,
        Resource::Abundance => {
            land.ocean
                || dom.crop > 0.0
                || (land.coast || land.river > 0.25) && land.npp() > 0.45
                || wild.iter().any(|w| {
                    matches!(w.kind, crate::land::WildKind::Plant(_)) && w.cultivability() * w.suits(land.temp_c, land.rain_mm) > 0.3
                })
        }
    }
}

/// Une recette est-elle praticable dans la cellule ?
fn recipe_ok(r: &Recipe, land: &Land, eco: &Economy, fire: bool) -> bool {
    if !fire
        && r.inputs.iter().any(|(g, _)| matches!(g, Good::Wood | Good::Charcoal | Good::Coal | Good::Oil))
        && r.output != Good::Charcoal
    {
        // Brûler demande du feu (l'électricité, elle, ne brûle rien ici).
        if r.output != Good::Machines || r.inputs.iter().any(|(g, _)| matches!(g, Good::Coal | Good::Oil)) {
            return false;
        }
    }
    if !fire && r.output == Good::Charcoal {
        return false;
    }
    match r.deposit {
        None => true,
        Some((res, _)) => {
            let d = &eco.deposits;
            match res {
                Resource::Copper => d.copper > 0.0,
                Resource::Tin => d.tin > 0.0,
                Resource::Iron => d.iron > 0.0,
                Resource::Coal => d.coal_c > 0.0,
                Resource::Oil => d.oil_c > 0.0,
                Resource::Uranium => d.uranium > 0.0,
                Resource::River => land.river > 0.1,
                _ => true,
            }
        }
    }
}

struct Plan<'a> {
    recipes: [Option<&'a Recipe>; GOODS],
    unit_labour: [f64; GOODS],
    /// Électricité de l'eau, du vent, du soleil, de la terre : recettes et
    /// ce qu'il en reste à tirer cette année, MWh, de la moins coûteuse à
    /// la plus coûteuse.
    renewables: Vec<(&'a Recipe, std::cell::Cell<f64>)>,
    /// La centrale qui brûle (ou fissionne) quand le reste ne suffit pas.
    elec_backup: Option<&'a Recipe>,
}

/// Électricité qu'un lieu peut tirer d'une source par an, MWh (ordres de
/// grandeur : le potentiel hydraulique du monde, environ 15 000 TWh, sur ses
/// fleuves ; le vent et le soleil, sur ce qu'on peut couvrir).
fn renewable_mwh(r: Resource, land: &Land) -> f64 {
    let a = land.area_km2;
    match r {
        Resource::River => a * land.river * 150.0,
        Resource::Wind => a * land.wind * 200.0,
        Resource::Sun => a * land.sun * 400.0,
        Resource::Geothermal => a * land.geothermal * 200.0,
        _ => f64::INFINITY,
    }
}

/// Choisit pour chaque bien la recette la moins coûteuse en travail,
/// entrées comprises.
fn plan<'a>(known: &Known, land: &Land, eco: &Economy, fire: bool, craft: f64, extra: &[(Good, f64)]) -> Plan<'a> {
    let cat = catalog();
    let mut recipes: [Option<&Recipe>; GOODS] = [None; GOODS];
    let mut unit = [f64::INFINITY; GOODS];
    // Biens tirés du milieu sans recette.
    for &(g, l) in extra {
        unit[g.index()] = l;
    }
    // Recettes utilisables ici, dans l'ordre des biens puis des techniques.
    let mut usable: Vec<(usize, &Recipe)> = Vec::with_capacity(48);
    for g in Good::ALL {
        for &(t, j) in &cat.producers[g.index()] {
            if !known.has(t) {
                continue;
            }
            let r = &cat.get(t).recipes[j];
            if recipe_ok(r, land, eco, fire) {
                usable.push((g.index(), r));
            }
        }
    }
    // Trois passes suffisent : la profondeur des recettes ne dépasse pas
    // trois. Une passe sans changement fixe le résultat : on s'arrête là.
    for _ in 0..4 {
        let mut changed = false;
        for &(g, r) in &usable {
            let mut cost = r.labour / craft;
            for &(i, q) in &r.inputs {
                cost += q * unit[i.index()];
            }
            let cost = cost / r.amount;
            if cost < unit[g] {
                unit[g] = cost;
                recipes[g] = Some(r);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    // Électricité : les sources du lieu ont un plafond ; au-delà, la
    // centrale thermique.
    let mut renewables: Vec<(&Recipe, f64, f64)> = Vec::new();
    let mut elec_backup: Option<(&Recipe, f64)> = None;
    let e = Good::Electricity.index();
    for &(t, j) in &cat.producers[e] {
        if !known.has(t) {
            continue;
        }
        let r = &cat.get(t).recipes[j];
        if !recipe_ok(r, land, eco, fire) {
            continue;
        }
        let mut cost = r.labour / craft;
        for &(i, q) in &r.inputs {
            cost += q * unit[i.index()];
        }
        let cost = cost / r.amount;
        if r.inputs.is_empty() && r.deposit.is_none() {
            let cap = cat.get(t).resources.iter().map(|&res| renewable_mwh(res, land)).fold(f64::INFINITY, f64::min);
            if cap.is_finite() {
                renewables.push((r, cost, cap));
                continue;
            }
        }
        if cost.is_finite() && elec_backup.is_none_or(|b| cost < b.1) {
            elec_backup = Some((r, cost));
        }
    }
    renewables.sort_by(|a, b| a.1.total_cmp(&b.1));
    Plan {
        recipes,
        unit_labour: unit,
        renewables: renewables.into_iter().map(|(r, _, cap)| (r, std::cell::Cell::new(cap))).collect(),
        elec_backup: elec_backup.map(|b| b.0),
    }
}

/// Ce qu'il faut pour une année dans une cellule.
pub struct Input<'a> {
    pub land: &'a Land,
    pub wild: &'a [&'a crate::land::WildSpecies],
    pub known: &'a Known,
    pub effects: &'a Effects,
    pub dom: Domesticates,
    pub pop: f64,
    pub sedentary: bool,
    pub policy: Policy,
    /// Surface où l'eau ne manque pas pour irriguer (zone aride arrosée).
    pub fire: bool,
    /// Une mine de charbon voisine, chez soi ou chez un partenaire,
    /// peut en livrer (canaux, puis rail).
    pub coal_nearby: bool,
    /// Le pétrole arrive par le marché national (rail, oléoducs).
    pub oil_nearby: bool,
    /// Ce qu'une mine ou un puits extrait en plus pour le reste du pays
    /// (charbon, pétrole), t : le manque de l'an passé, partagé.
    pub export: [f64; 2],
}

/// Une année d'économie. Rend l'état suivant et le bilan.
pub fn run(inp: &Input, eco: &Economy) -> (Economy, Year) {
    let land = inp.land;
    let eff = inp.effects;
    let pop = inp.pop.max(0.0);
    let mut next = eco.clone();
    let mut produced = ZERO;
    let mut consumed = ZERO;
    let mut shortage = ZERO;
    let mut flux = Flux::default();
    let elites = pop * inp.policy.elites;
    let workers = (pop - elites) * WORK_SHARE;
    let soldiers = workers * inp.policy.army;
    let specialists = workers * inp.policy.specialists;
    let mut free = (workers - soldiers - specialists).max(0.0);
    let area = land.area_km2;

    // ---------------------------------------------------------- Nourriture
    let need = pop + elites + soldiers * 0.2;
    let cultivating = inp.dom.crop > 0.0 && inp.sedentary;
    // Champs.
    let climate = (1.0 - land.variability).clamp(0.0, 1.0);
    let arid = (1.0 - land.rain_mm / 600.0).clamp(0.0, 1.0);
    let max_crop = if cultivating { area * land.arable * tilled_share(inp.known, land) } else { 0.0 };
    let fert_need = eco.cropland_km2 * eff.fertilizer_km2;
    let fert_share = if fert_need > 0.0 { (eco.stocks[Good::Fertilizer.index()] / fert_need).min(1.0) } else { 0.0 };
    let fuel_need = eco.cropland_km2 * eff.fuel_km2;
    let fuel_share = if fuel_need > 0.0 { (eco.stocks[Good::Oil.index()] / fuel_need).min(1.0) } else { 0.0 };
    let irrigated_share = if max_crop > 0.0 { (land.river * (0.3 + 0.7 * arid)).min(1.0) } else { 0.0 };
    let yield_km2 = BASE_YIELD_KM2
        * inp.dom.crop
        * eco.soil
        * climate
        * eff.crop_yield
        * (1.0 + 1.5 * fert_share)
        * (1.0 + eff.irrigation.min(2.0) * irrigated_share * (0.5 + arid))
        * (1.0 - 0.6 * arid * (1.0 - irrigated_share * eff.irrigation.min(1.0)));
    let traction = if eco.cropland_km2 > 0.0 { (eco.livestock / (eco.cropland_km2 * 1.5)).min(1.0) } else { 0.0 };
    let tractor = if eff.fuel_km2 > 0.0 { fuel_share } else { 1.0 };
    let labour_km2 = BASE_FIELD_LABOUR_KM2 * (1.0 - tractor + tractor * eff.field_labour) / (1.0 + eff.traction * traction);
    // Pâtures.
    let npp = land.npp();
    let herding = inp.dom.herd > 0.0;
    let pasture_km2 = area * land.pasture;
    let herd_cap = if herding {
        pasture_km2 * LIVESTOCK_KM2 * (npp / 0.5).min(2.0) * inp.dom.herd * eff.herd.sqrt() + eco.cropland_km2 * 0.5
    } else {
        0.0
    };
    let herd_workers = (eco.livestock * 0.02).min(free);
    free -= herd_workers;
    let herd_food = eco.livestock
        * 0.22
        * if inp.dom.milk { eff.herd } else { eff.herd.min(1.2) }
        * (herd_workers / (eco.livestock * 0.02).max(1e-9)).min(1.0);
    // Cueillette, chasse et pêche.
    let wild_area = (area - eco.cropland_km2 - eco.livestock / LIVESTOCK_KM2).max(0.0);
    let wild_cap = wild_area * npp * FORAGE_PER_NPP * eff.forage.sqrt();
    let fishing = inp.known.intersects(&catalog().fishing);
    let fish_cap = if fishing { area * (0.004 * f64::from(u8::from(land.coast)) + 0.002 * land.river) * eff.fishing * 10.0 } else { 0.0 };
    let mut food = herd_food;
    produced[Good::AnimalFood.index()] += herd_food;
    let mut f_wild = 0.0;
    let mut f_fish = 0.0;
    let mut f_crop = 0.0;
    let mut l_wild = 0.0;
    let mut l_fish = 0.0;
    let mut l_crop = 0.0;
    let crop_area = eco.cropland_km2.min(max_crop);
    // Le travail va par tranches au secteur qui rend le plus à la marge.
    let stored = eco.stocks[Good::Grain.index()] + eco.stocks[Good::Roots.index()];
    let target = (need * 1.1 - food).max(0.0);
    let steps = 24;
    let chunk = free / steps as f64;
    let wild_out =
        |l: f64| if wild_cap > 0.0 { wild_cap * (-(-l * FORAGE_PER_WORKER * eff.forage.sqrt() / wild_cap).dexp_m1()) } else { 0.0 };
    let fish_out = |l: f64| if fish_cap > 0.0 { fish_cap * (-(-l * FISH_PER_WORKER * eff.fishing / fish_cap).dexp_m1()) } else { 0.0 };
    let crop_out = |l: f64| {
        if crop_area <= 0.0 || labour_km2 <= 0.0 {
            return 0.0;
        }
        let full = crop_area * labour_km2;
        if l <= full {
            l / labour_km2 * yield_km2
        } else {
            // Intensification (Boserup) : plus de bras, un peu plus de grain.
            crop_area * yield_km2 * (1.0 + 0.4 * (-(-(l / full - 1.0)).dexp_m1()))
        }
    };
    let mut used = 0.0;
    // Rendement de la tranche suivante dans chaque secteur ; seul celui qui
    // vient de la recevoir change.
    let (mut nw, mut nf, mut nc) = (wild_out(l_wild + chunk), fish_out(l_fish + chunk), crop_out(l_crop + chunk));
    for _ in 0..steps {
        if f_wild + f_fish + f_crop >= target {
            break;
        }
        let gw = nw - f_wild;
        let gf = nf - f_fish;
        let gc = nc - f_crop;
        if gw <= 0.0 && gf <= 0.0 && gc <= 0.0 {
            break;
        }
        if gc >= gw && gc >= gf {
            l_crop += chunk;
            f_crop += gc;
            nc = crop_out(l_crop + chunk);
        } else if gw >= gf {
            l_wild += chunk;
            f_wild += gw;
            nw = wild_out(l_wild + chunk);
        } else {
            l_fish += chunk;
            f_fish += gf;
            nf = fish_out(l_fish + chunk);
        }
        used += chunk;
    }
    free -= used;
    food += f_wild + f_fish + f_crop;
    produced[Good::WildFood.index()] += f_wild;
    produced[Good::Fish.index()] += f_fish;
    produced[inp.dom.crop_good.index()] += f_crop;
    let food_workers = herd_workers + used;
    // Réserves : on y puise en cas de manque ; le surplus s'y ajoute.
    let mut available = food + stored;
    let eaten = available.min(need);
    available -= eaten;
    let storage = eff.storage.min(0.85);
    next.stocks[Good::Grain.index()] = 0.0;
    next.stocks[Good::Roots.index()] = 0.0;
    let keep_good = if inp.dom.crop > 0.0 { inp.dom.crop_good } else { Good::Grain };
    // Ne se garde que ce qui se stocke : le grain ou les tubercules.
    let storable = available.min(stored + f_crop);
    next.stocks[keep_good.index()] = storable * (1.0 - keep_good.decay() * (1.0 - storage));
    consumed[Good::Grain.index()] += eaten;
    let commoners = (pop - elites).max(1.0);
    let food_pc = if pop > 0.0 { ((eaten - elites * 2.0).max(0.0) / commoners).min(eaten / pop.max(1.0) * 1.5) } else { 0.0 };
    let surplus_food = (food - pop).max(0.0);
    for g in [Good::Grain, Good::Roots] {
        shortage[g.index()] = (need - eaten).max(0.0) * if g == keep_good { 1.0 } else { 0.0 };
    }

    // Champs et troupeaux de l'an prochain.
    if cultivating {
        // Les champs suivent les bras qui y travaillent, défrichés sur la
        // forêt ou la steppe.
        let wanted = (l_crop / labour_km2.max(1e-9)).max(eco.cropland_km2 * 0.97);
        let wanted = if food_pc < 1.05 { wanted * 1.03 + 0.002 * pop / yield_km2.max(1.0) } else { wanted };
        let new = wanted.clamp(0.0, max_crop.max(0.0));
        let cleared = (new - eco.cropland_km2).max(0.0);
        let forest_density = eco.forest_c / area.max(1e-9);
        let cut = (cleared * forest_density).min(eco.forest_c);
        next.forest_c -= cut;
        flux.deforestation_c += cut;
        next.stocks[Good::Wood.index()] += cut * 0.5 / Good::Wood.carbon();
        // L'autre moitié du bois défriché brûle sur place.
        flux.co2_c += cut * 0.5;
        next.cropland_km2 = new;
        next.irrigated_km2 = new * irrigated_share * f64::from(u8::from(eff.irrigation > 0.0));
    } else {
        next.cropland_km2 = eco.cropland_km2 * 0.9;
        next.irrigated_km2 = 0.0;
    }
    if herding {
        let growth = 0.15 * eco.livestock.max(10.0) * (1.0 - eco.livestock / herd_cap.max(1.0));
        next.livestock = (eco.livestock + growth).max(0.0).min(herd_cap.max(0.0) * 1.05);
    } else {
        next.livestock = eco.livestock * 0.8;
    }
    // Sols : l'épuisement suit l'intensité, la jachère, la fumure et les
    // engrais réparent ; le relief et la déforestation font l'érosion.
    if eco.cropland_km2 > 0.0 {
        let intensity = (l_crop / (crop_area * labour_km2).max(1e-9)).min(2.0);
        // Sans technique, la friche reconstitue le sol entre deux
        // cultures : l'équilibre est autour des deux tiers.
        let wear = 0.002 * intensity * (1.0 + land.relief);
        let repair = 0.004 + eff.soil + 0.006 * fert_share + if inp.dom.herd > 0.0 { 0.001 } else { 0.0 };
        next.soil = (eco.soil - wear * eco.soil + repair * (1.0 - eco.soil)).clamp(0.2, 1.0);
        flux.erosion_t = eco.cropland_km2 * 100.0 * (0.2 + land.relief) * intensity;
        // L'usure se répare lentement quand les champs reculent.
    } else {
        next.soil = (eco.soil + 0.01 * (1.0 - eco.soil)).min(1.0);
    }
    // Méthane : rumination (environ 30 kg par bête et par an dans un
    // troupeau mêlé de bovins, moutons et chèvres ; FAO, GLEAM) et rizières.
    flux.ch4_c += eco.livestock * 0.03 * 0.75
        + if inp.dom.crop_good == Good::Grain && land.rain_mm > 1200.0 { eco.irrigated_km2 * 20.0 * 0.75 } else { 0.0 };
    // Le méthane vient du carbone de l'air capté par le fourrage la même
    // année : il sort du CO₂ de l'atmosphère.
    flux.co2_c -= eco.livestock * 0.03 * 0.75
        + if inp.dom.crop_good == Good::Grain && land.rain_mm > 1200.0 { eco.irrigated_km2 * 20.0 * 0.75 } else { 0.0 };

    // ---------------------------------------------------------- Énergie
    let ee = energy_efficiency(inp.known);
    let machines = eco.stocks[Good::Machines.index()];
    let nonfood = (free + specialists).max(1.0);
    // Eau, vent, sources chaudes : à hauteur des moulins qu'on bâtit.
    let mills = ee[1] * land.river * 0.15 + ee[2] * land.wind * 0.1 + ee[9] * land.geothermal * 0.2 + ee[8] * land.sun * 0.05;
    let mill_work = mills * nonfood;
    // Moteurs : charbon, pétrole, électricité, limités par les machines.
    let engine_cap = machines * MACHINE_MWH;
    let fire = inp.fire;
    let coal_eff = if fire { ee[4] } else { 0.0 };
    let oil_eff = if fire { ee[5] } else { 0.0 };
    let elec_eff = ee[6];
    let p0 = plan(inp.known, land, eco, fire, 1.0, &[]);
    let has = |g: Good| {
        p0.recipes[g.index()].is_some()
            || eco.stocks[g.index()] > 0.0
            || g == Good::Coal && inp.coal_nearby
            || g == Good::Oil && inp.oil_nearby
    };
    let mut engine_fuel = [0.0f64; 3];
    let mut engine_work = 0.0;
    let mut engine_source = None;
    if engine_cap > 0.0 {
        // On prend la meilleure source dont on dispose, au vu de ce qui a
        // manqué l'an passé.
        let mut best = (0usize, 0.0, 0.0);
        for (i, (g, e)) in [(Good::Coal, coal_eff), (Good::Oil, oil_eff), (Good::Electricity, elec_eff)].iter().enumerate() {
            let expected = *e * (1.0 - eco.engine_short[i]);
            if *e > 0.0 && has(*g) && expected > best.2 {
                best = (i, *e, expected);
            }
        }
        if best.1 > 0.0 {
            let content = [7.5, 11.6, 1.0][best.0];
            engine_fuel[best.0] = engine_cap / (best.1 * content);
            engine_work = engine_cap * (1.0 - eco.engine_short[best.0]);
            engine_source = Some(best.0);
        }
    }
    let work = mill_work + engine_work;
    let power_pc = work / nonfood;
    let modern = if work > 0.0 { engine_work / (work + nonfood * MUSCLE_MWH) } else { 0.0 };
    let craft = eff.craft * (1.0 + power_pc / 0.4).dpowf(0.8);
    let fuel_labour = |g: Good| -> f64 {
        match g {
            Good::Wood => {
                if eco.forest_c > 1.0 {
                    0.08 / craft.sqrt() * (1.0 + 3.0 * (1.0 - (eco.forest_c / (land.forest_c_km2 * area).max(1.0)).min(1.0)))
                } else {
                    f64::INFINITY
                }
            }
            Good::Stone => 0.05 / craft.sqrt(),
            _ => f64::INFINITY,
        }
    };
    let extra = [(Good::Wood, fuel_labour(Good::Wood)), (Good::Stone, fuel_labour(Good::Stone))];
    let p = plan(inp.known, land, eco, fire, craft, &extra);

    // ---------------------------------------------------------- Demandes
    let cold = ((15.0 - land.temp_c) / 20.0).clamp(0.0, 1.0);
    let knows_fire = fire && inp.known.iter().any(|t| catalog().get(t).family == Some(evo_culture::catalog::Family::Fire));
    let heat_mwh = if knows_fire { pop * (0.6 + 2.5 * cold) * if inp.sedentary { 1.0 } else { 0.6 } } else { 0.0 };
    let mut demands: Vec<(Good, f64)> = Vec::with_capacity(20);
    // Combustible des moteurs : avant tout le reste, l'industrie tourne.
    demands.push((Good::Coal, engine_fuel[0]));
    demands.push((Good::Oil, engine_fuel[1] + fuel_need));
    demands.push((Good::Electricity, engine_fuel[2]));
    // Une mine extrait aussi pour ses voisins : le commerce emporte le
    // surplus là où il manque.
    // Ce qui s'extrait pour d'autres va au stock, pas au feu.
    let mut for_stock: Vec<(Good, f64)> = Vec::new();
    if p.recipes[Good::Coal.index()].is_some() && fire {
        for_stock.push((Good::Coal, (pop * 0.5 + inp.export[0] - eco.stocks[Good::Coal.index()]).max(0.0)));
    }
    if p.recipes[Good::Oil.index()].is_some() && inp.export[1] > 0.0 {
        for_stock.push((Good::Oil, (inp.export[1] - eco.stocks[Good::Oil.index()]).max(0.0)));
    }
    // Chauffage et cuisson, par le combustible le moins coûteux en travail.
    let mut heat_good = Good::Wood;
    let mut best = f64::INFINITY;
    for (g, mwh) in MWH_PER_T {
        if g == Good::Coal && !fire || g == Good::Oil && !fire {
            continue;
        }
        let u = p.unit_labour[g.index()] / mwh;
        if u < best {
            best = u;
            heat_good = g;
        }
    }
    // Charbon de terre au foyer dès qu'une mine voisine en livre (Londres
    // au XVIIe siècle, Wrigley, 2010).
    if inp.coal_nearby && fire && p.recipes[Good::Coal.index()].is_none() {
        heat_good = Good::Coal;
    }
    let heat_t = heat_mwh / MWH_PER_T.iter().find(|(g, _)| *g == heat_good).map(|x| x.1).unwrap_or(4.2);
    demands.push((heat_good, heat_t));
    // L'investissement en machines passe avant les outils et le confort :
    // c'est le pari des marchands et des États industriels, et la machine
    // remplace l'outil à main. Le parc visé, deux machines par travailleur
    // hors des champs, se comble d'un cinquième par an : le capital
    // britannique doublait en une génération au XIXe siècle (Feinstein,
    // 1978).
    let mut invest_at = usize::MAX;
    if inp.policy.invest {
        let target = nonfood * 2.0;
        let invest = ((target - machines) * 0.2).max(0.0) + machines * Good::Machines.decay();
        invest_at = demands.len();
        demands.push((Good::Machines, invest));
    }
    demands.push((Good::Tools, workers));
    let housing = pop * 0.03 * if inp.sedentary { 1.0 } else { 0.3 };
    demands.push((Good::Buildings, housing));
    let clothing = pop * (0.2 + 0.8 * cold);
    if p.recipes[Good::Textile.index()].is_some() {
        demands.push((Good::Textile, clothing));
    } else {
        demands.push((Good::Leather, clothing));
    }
    demands.push((Good::Fertilizer, fert_need));
    demands.push((Good::Pottery, pop * 0.3 * f64::from(u8::from(inp.sedentary))));
    demands.push((Good::Salt, pop * 0.005));
    demands.push((Good::Weapons, soldiers));
    demands.push((Good::Medicine, pop * 0.2));
    demands.push((Good::Books, pop * inp.policy.literacy * 0.3 + specialists * 0.5));
    demands.push((Good::Electricity, if elec_eff > 0.0 { pop * 1.5 } else { 0.0 }));
    demands.push((Good::Luxury, elites * 3.0));
    demands.push((Good::Buildings, pop * inp.policy.monuments));
    demands.push((Good::Luxury, (pop - elites) * 0.02 * (1.0 + power_pc)));

    // ---------------------------------------------------------- Production
    let mut stocks = next.stocks;
    let mut deposits = next.deposits;
    let mut forest = next.forest_c;
    let mut carbonate = 0.0;
    let mut labour_left = free;
    // Production récursive d'un bien, entrées d'abord, travail limité.
    #[allow(clippy::too_many_arguments)]
    fn make(
        g: Good,
        q: f64,
        depth: u8,
        p: &Plan,
        craft: f64,
        stocks: &mut Goods,
        produced: &mut Goods,
        consumed: &mut Goods,
        labour: &mut f64,
        deposits: &mut Deposits,
        forest: &mut f64,
        carbonate: &mut f64,
        extra: &[(Good, f64)],
    ) -> f64 {
        if q <= 0.0 {
            return 0.0;
        }
        let have = stocks[g.index()].min(q);
        stocks[g.index()] -= have;
        consumed[g.index()] += have;
        let mut want = q - have;
        if want <= 0.0 || depth > 4 {
            return have;
        }
        // Biens tirés du milieu.
        if let Some(&(_, l)) = extra.iter().find(|(e, _)| *e == g) {
            if l.is_finite() {
                let mut take = want.min(*labour / l.max(1e-12));
                if g == Good::Wood {
                    take = take.min(*forest / Good::Wood.carbon());
                    *forest -= take * Good::Wood.carbon();
                }
                *labour -= take * l;
                produced[g.index()] += take;
                consumed[g.index()] += take;
                return have + take;
            }
            if p.recipes[g.index()].is_none() {
                return have;
            }
        }
        if g == Good::Electricity && !p.renewables.is_empty() {
            // L'eau, le vent, le soleil d'abord, à hauteur de ce que le lieu
            // en donne ; le reste par la centrale thermique.
            let mut got = 0.0;
            for (r, cap) in &p.renewables {
                let take = want.min(cap.get());
                if take <= 0.0 {
                    continue;
                }
                let out = produce(g, r, take, depth, p, craft, stocks, produced, consumed, labour, deposits, forest, carbonate, extra);
                cap.set(cap.get() - out);
                got += out;
                want -= out;
            }
            if want > 0.0 {
                if let Some(r) = p.elec_backup {
                    got += produce(g, r, want, depth, p, craft, stocks, produced, consumed, labour, deposits, forest, carbonate, extra);
                }
            }
            return have + got;
        }
        let Some(r) = p.recipes[g.index()] else { return have };
        have + produce(g, r, want, depth, p, craft, stocks, produced, consumed, labour, deposits, forest, carbonate, extra)
    }
    /// Produit `want` de `g` par la recette `r`, entrées d'abord.
    #[allow(clippy::too_many_arguments)]
    fn produce(
        g: Good,
        r: &Recipe,
        want: f64,
        depth: u8,
        p: &Plan,
        craft: f64,
        stocks: &mut Goods,
        produced: &mut Goods,
        consumed: &mut Goods,
        labour: &mut f64,
        deposits: &mut Deposits,
        forest: &mut f64,
        carbonate: &mut f64,
        extra: &[(Good, f64)],
    ) -> f64 {
        let mut want = want;
        // Limite par le travail et par le gisement.
        let unit = r.labour / craft / r.amount;
        if unit > 0.0 {
            want = want.min(*labour / unit);
        }
        if let Some((res, per)) = r.deposit {
            let left = match res {
                Resource::Copper => deposits.copper,
                Resource::Tin => deposits.tin,
                Resource::Iron => deposits.iron,
                Resource::Coal => deposits.coal_c,
                Resource::Oil => deposits.oil_c,
                Resource::Uranium => deposits.uranium,
                _ => f64::INFINITY,
            };
            want = want.min(left / (per / r.amount).max(1e-12));
        }
        // Entrées.
        let mut frac: f64 = 1.0;
        for &(i, qi) in &r.inputs {
            let need = qi * want / r.amount;
            let got = make(i, need, depth + 1, p, craft, stocks, produced, consumed, labour, deposits, forest, carbonate, extra);
            if need > 0.0 {
                frac = frac.min(got / need);
            }
        }
        let out = want * frac.clamp(0.0, 1.0);
        *labour -= out * unit;
        if let Some((res, per)) = r.deposit {
            let used = out * per / r.amount;
            match res {
                Resource::Copper => deposits.copper -= used,
                Resource::Tin => deposits.tin -= used,
                Resource::Iron => deposits.iron -= used,
                Resource::Coal => deposits.coal_c -= used,
                Resource::Oil => deposits.oil_c -= used,
                Resource::Uranium => deposits.uranium -= used,
                _ => {}
            }
        }
        *carbonate += out * r.carbonate_c / r.amount;
        produced[g.index()] += out;
        consumed[g.index()] += out;
        out
    }
    // Extraction pour les voisins et le pays, payée par les acheteurs :
    // elle passe avant les besoins de la cellule, sur la moitié au plus de
    // son travail (les bassins houillers attiraient la main-d'œuvre).
    let mut exported: Vec<(Good, f64)> = Vec::with_capacity(for_stock.len());
    for &(g, q) in &for_stock {
        if q <= 0.0 {
            continue;
        }
        let reserve = labour_left * 0.5;
        labour_left -= reserve;
        let got = make(
            g,
            q,
            0,
            &p,
            craft,
            &mut stocks,
            &mut produced,
            &mut consumed,
            &mut labour_left,
            &mut deposits,
            &mut forest,
            &mut carbonate,
            &extra,
        );
        labour_left += reserve;
        consumed[g.index()] -= got;
        exported.push((g, got));
    }
    let mut fuel_mwh = [0.0f64; 5];
    let mut engine_got = [0.0f64; 3];
    for (k, &(g, q)) in demands.iter().enumerate() {
        if q <= 0.0 {
            continue;
        }
        // L'investissement se fabrique : le parc en place ne compte pas.
        let park = if k == invest_at { std::mem::take(&mut stocks[g.index()]) } else { 0.0 };
        let got = make(
            g,
            q,
            0,
            &p,
            craft,
            &mut stocks,
            &mut produced,
            &mut consumed,
            &mut labour_left,
            &mut deposits,
            &mut forest,
            &mut carbonate,
            &extra,
        );
        shortage[g.index()] += (q - got).max(0.0);
        if k < 3 {
            engine_got[k] = got;
        }
        // Les machines achetées s'ajoutent au parc.
        if k == invest_at {
            stocks[g.index()] += park + got;
            consumed[g.index()] -= got;
        }
        if let Some(i) = MWH_PER_T.iter().position(|(m, _)| *m == g) {
            fuel_mwh[i] += got * MWH_PER_T[i].1;
        }
    }
    // Ce que les moteurs ont vraiment brûlé : la leçon pour l'an prochain.
    for i in 0..3 {
        next.engine_short[i] = if Some(i) == engine_source && engine_fuel[i] > 0.0 {
            (1.0 - engine_got[i].min(engine_fuel[i]) / engine_fuel[i]).clamp(0.0, 1.0)
        } else {
            // Une source qu'on n'a pas essayée redevient tentante.
            eco.engine_short[i] * 0.9
        };
    }
    // Ce qui a été extrait pour le marché rejoint le stock après coup :
    // la mine ne le brûle pas elle-même.
    for (g, got) in exported {
        stocks[g.index()] += got;
    }
    // Le combustible consommé brûle : carbone à l'air.
    for g in [Good::Wood, Good::Charcoal, Good::Coal, Good::Oil] {
        flux.co2_c += consumed[g.index()] * g.carbon();
    }
    // Le charbon de bois produit avec du bois : le bois consommé pour le
    // faire est déjà compté brûlé ; le charbon de bois lui-même aussi, ce
    // qui compterait deux fois le carbone. On retire le carbone resté dans
    // le charbon de bois produit.
    flux.co2_c -= produced[Good::Charcoal.index()] * Good::Charcoal.carbon();
    flux.co2_c += carbonate;
    flux.carbonate_c = carbonate;
    flux.aerosol_t += consumed[Good::Coal.index()] * 0.015;
    // Repousse des forêts (puits de carbone).
    let forest_max = land.forest_c_km2 * (area - next.cropland_km2).max(0.0);
    let regrowth = (0.01 * forest.max(forest_max * 0.01) * (1.0 - forest / forest_max.max(1.0))).max(0.0);
    forest += regrowth;
    flux.co2_c -= regrowth;
    flux.deforestation_c += (next.forest_c - forest + regrowth).max(0.0);
    next.forest_c = forest;
    next.deposits = deposits;
    // Stocks : usure.
    for g in Good::ALL {
        if g == Good::Grain || g == Good::Roots {
            continue;
        }
        let i = g.index();
        let left = stocks[i];
        let lost = left * g.decay();
        stocks[i] = left - lost;
        if g.carbon() > 0.0 {
            // Le bois et le charbon de bois qui pourrissent rendent leur
            // carbone à l'air.
            flux.co2_c += lost * g.carbon();
        }
    }
    let keep = [next.stocks[Good::Grain.index()], next.stocks[Good::Roots.index()]];
    next.stocks = stocks;
    next.stocks[Good::Grain.index()] = keep[0];
    next.stocks[Good::Roots.index()] = keep[1];
    // Le bois défriché mis en stock plus haut.
    flux.cropland_km2 = next.cropland_km2;
    flux.pasture_km2 = next.livestock / LIVESTOCK_KM2;
    flux.irrigated_km2 = next.irrigated_km2;

    // ---------------------------------------------------------- Bilan
    let mut out_value = goods::value(&produced);
    out_value -= produced[Good::Grain.index()].min(0.0);
    let gdp = out_value;
    let gdp_pc = if pop > 0.0 { gdp / pop } else { 0.0 };
    let surplus = (surplus_food + (gdp - goods::food(&produced))).max(0.0);
    let public = soldiers * 1.5 + specialists * 1.5 + pop * inp.policy.monuments * Good::Buildings.value();
    let prices = if inp.policy.market {
        let mut pr = ZERO;
        for g in Good::ALL {
            let i = g.index();
            let s = produced[i] + eco.stocks[i];
            let d = consumed[i] + shortage[i];
            pr[i] = g.value().max(0.01) * ((d + 1.0) / (s + 1.0)).clamp(0.2, 5.0);
        }
        Some(pr)
    } else {
        None
    };
    let year = Year {
        produced,
        consumed,
        shortage,
        food_pc,
        gdp,
        gdp_pc,
        food_workers,
        workers,
        power_pc,
        modern_energy: modern,
        fossil_energy: if matches!(engine_source, Some(0 | 1)) { modern } else { 0.0 },
        fuel_mwh,
        surplus,
        public,
        flux,
        prices,
    };
    (next, year)
}

/// Carbone des stocks lents d'une cellule (forêt, gisements fossiles,
/// biens), t C : sert au bilan fermé.
pub fn carbon_stock(eco: &Economy) -> f64 {
    eco.forest_c + eco.deposits.coal_c + eco.deposits.oil_c + goods::carbon(&eco.stocks)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::catalog;
    use evo_culture::catalog::Milieu;

    pub(crate) fn plain() -> Land {
        Land {
            area_km2: 100_000.0,
            lat_deg: 35.0,
            lon_deg: 40.0,
            milieu: Milieu::Land,
            ocean: false,
            coast: false,
            temp_c: 16.0,
            rain_mm: 500.0,
            variability: 0.1,
            relief: 0.2,
            river: 0.6,
            arable: 0.3,
            pasture: 0.3,
            forest_c_km2: 3000.0,
            deposits: Deposits { coal_c: 1e9, oil_c: 0.0, copper: 1e5, tin: 0.0, iron: 1e7, uranium: 0.0, salt: 1.0 },
            wind: 0.5,
            sun: 0.6,
            geothermal: 0.0,
            o2_percent: 21.0,
            sky: true,
            wild: vec![],
        }
    }

    fn known(keys: &[&str]) -> Known {
        let cat = catalog();
        let mut k = Known::default();
        for key in keys {
            k.set(cat.id(key).unwrap_or_else(|| panic!("{key}")));
        }
        k
    }

    fn eco(land: &Land) -> Economy {
        Economy {
            stocks: ZERO,
            cropland_km2: 0.0,
            irrigated_km2: 0.0,
            livestock: 0.0,
            soil: 1.0,
            forest_c: land.forest_c_km2 * land.area_km2,
            deposits: land.deposits,
            engine_short: [0.0; 3],
        }
    }

    #[test]
    fn farming_feeds_a_hundred_times_more_than_foraging() {
        let land = plain();
        let forager = known(&["percussion", "eclat", "feu", "epieu", "corde", "abri"]);
        let eff = catalog().effects(&forager);
        let inp = Input {
            land: &land,
            wild: &[],
            known: &forager,
            effects: &eff,
            dom: Domesticates { crop: 0.0, crop_good: Good::Grain, herd: 0.0, milk: false },
            pop: 5000.0,
            sedentary: false,
            policy: Policy::default(),
            fire: true,
            coal_nearby: false,
            oil_nearby: false,
            export: [0.0; 2],
        };
        let (_, y) = run(&inp, &eco(&land));
        assert!(y.food_pc > 0.95, "les chasseurs-cueilleurs mangent : {}", y.food_pc);
        // À 100 fois plus, ils ont faim.
        let inp = Input { pop: 500_000.0, ..inp };
        let (_, y) = run(&inp, &eco(&land));
        assert!(y.food_pc < 0.5, "{}", y.food_pc);
        // Des agriculteurs, oui.
        let mut farmer = forager;
        // À la houe, les champs ne couvrent qu'une petite part des terres ;
        // avec l'araire, la plaine entière nourrit.
        for k in ["recipient", "stockage", "village", "meule", "recolte_sauvage", "culture_cereales", "araire"] {
            farmer.set(catalog().id(k).unwrap());
        }
        let eff = catalog().effects(&farmer);
        let mut e = eco(&land);
        e.cropland_km2 = land.area_km2 * land.arable;
        let inp = Input {
            known: &farmer,
            effects: &eff,
            dom: Domesticates { crop: 0.9, crop_good: Good::Grain, herd: 0.0, milk: false },
            sedentary: true,
            ..inp
        };
        let (_, y) = run(&inp, &e);
        assert!(y.food_pc > 0.95, "les agriculteurs mangent : {} ; {:?}", y.food_pc, y.produced);
    }
}
