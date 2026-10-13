//! Le monde des sociétés : cellules habitées, sociétés, relations,
//! maladies, notables et chronique, avancés d'un an à chaque pas.
//!
//! Ordre d'une année :
//!
//! 1. économie et démographie de chaque cellule, en parallèle sur l'état
//!    d'avant le pas ;
//! 2. migrations et colonisation des terres vides ;
//! 3. échanges entre cellules (redistribution dans une société, commerce
//!    entre sociétés amies, tribut) ;
//! 4. diffusion des techniques et des maladies par les contacts ;
//! 5. inventions, pertes de savoir-faire ;
//! 6. sociétés : dimensions, institutions, élites, dette, légitimité,
//!    tension politique, crises, scissions, effondrements, étiquettes ;
//! 7. guerre et diplomatie entre voisines ;
//! 8. sciences, notables, nouvelles maladies, flux vers la planète, fin de
//!    la première version.
//!
//! Chaque tirage vient de `rng_for(graine, Stream::Society, [année, objet,
//! identifiant])` : l'histoire ne dépend ni du nombre de cœurs ni de ce que
//! la caméra regarde.

use crate::catalog::{catalog, Effects, Known, Need, Resource, Technique};
use crate::chronicle::{Event, EventKind, Notable, NotableKind, MAX_NOTABLES};
use crate::disease::{final_size, Disease, CRITICAL_COMMUNITY, MAX_DISEASES};
use crate::economy::{self, Domesticates, Economy, Flux, Input, Policy, Year};
use crate::goods::{Good, ZERO};
use crate::land::{Land, WildSpecies};
use crate::science::{Domain, Model, Truths, DOMAINS};
use crate::society::{self, Aggregates, Dimensions, Institution, Label, Society, INSTITUTIONS};
use evo_core::math::Det;
use evo_core::rng::{rng_for, SimRng, Stream};
use evo_culture::catalog::Milieu;
use evo_culture::cognition::{Channel, Cognition, Learning};
use evo_culture::language::Phonology;
use rand::Rng;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Taux d'invention de base, par habitant connecté et par an.
pub const INVENTION_RATE: f64 = 1.5e-10;
/// Coût de chaque marche de prérequis pour l'invention.
pub const DEPTH_COST: f64 = 0.6;
/// Plafond des morts par maladies endémiques, par an.
pub const ENDEMIC_CAP: f64 = 0.003;
/// Létalité d'une maladie de foule dans une population qui l'a déjà eue.
pub const ADAPTED_LETHALITY: f64 = 0.2;
/// Létalité d'une peste qui revient, rapportée à la première vague.
pub const RETURNING_PLAGUE: f64 = 0.3;
/// Avant la méthode expérimentale, la science avance par à-coups lents.
pub const PRE_METHOD_SCIENCE: f64 = 0.3;
/// Probabilité annuelle de domestiquer, pour une société fixée devant la
/// meilleure espèce possible.
pub const DOMESTICATION_RATE: f64 = 3.0e-4;
/// Probabilité annuelle qu'un voisin transmette une technique, à contact
/// ordinaire.
pub const DIFFUSION_RATE: f64 = 0.004;
/// Facteur de la diffusion par l'écrit dans un pays, sur la part des
/// lettrés : un pays tout lettré apprend en quelques décennies.
pub const WRITTEN_DIFFUSION: f64 = 10.0;
/// Facteur de la diffusion par l'écrit entre partenaires de commerce.
pub const PARTNER_DIFFUSION: f64 = 3.0;
/// Rythme des sous-systèmes lents (inventions, pertes, sciences, choix de
/// paix ou de guerre), en années : chaque société, chaque paire de
/// voisins y passe une fois tous les `SLOW_EVERY` ans, à son tour, avec
/// les chances de ces années cumulées. Le tour ne dépend que du temps
/// simulé et de l'identifiant.
pub const SLOW_EVERY: i64 = 5;

/// Est-ce le tour de cet identifiant cette année ?
fn due(year: i64, id: u64) -> bool {
    (year + (id % SLOW_EVERY as u64) as i64).rem_euclid(SLOW_EVERY) == 0
}

/// Chance qu'un événement de probabilité annuelle `p` arrive au moins une
/// fois en `SLOW_EVERY` ans.
fn over_slow(p: f64) -> f64 {
    let q = 1.0 - p.clamp(0.0, 1.0);
    let mut k = 1.0;
    for _ in 0..SLOW_EVERY {
        k *= q;
    }
    1.0 - k
}
const NEEDS: [Need; 10] = [
    Need::Food,
    Need::Fuel,
    Need::Defence,
    Need::Conquest,
    Need::Administration,
    Need::Transport,
    Need::Health,
    Need::Knowledge,
    Need::Shelter,
    Need::Prestige,
];
/// Densité, habitants par km² de terres labourables, au-dessus de laquelle
/// toute la population échange ses idées.
pub const DENSE_KM2: f64 = 10.0;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Species {
    pub id: u32,
    pub name: String,
    pub cognition: Cognition,
    pub learning: Learning,
    pub milieu: Milieu,
    /// Organe de manipulation fine (mains, trompe, tentacules).
    pub hands: bool,
    pub channel: Channel,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Link {
    pub to: u32,
    /// Coût d'un passage : 1 en plaine, plus en montagne ou au désert.
    pub cost: f64,
    /// Bras de mer : demande de quoi naviguer.
    pub sea: bool,
}

/// Une espèce domestiquée par une cellule.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Domestic {
    pub species: u32,
    /// Qualité acquise par la sélection, de la forme sauvage à 1,2.
    pub quality: f64,
    pub good: Good,
    pub milk: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Settlement {
    pub species: u32,
    pub society: u32,
    pub people: u32,
    pub pop: f64,
    pub known: Known,
    pub effects: Effects,
    pub crop: Option<Domestic>,
    pub herd: Option<Domestic>,
    pub sedentary: bool,
    pub urban: f64,
    pub city: Option<u32>,
    pub founded: i64,
    pub immune: [f32; MAX_DISEASES],
    /// Maladies qui circulent cette année (bits).
    pub circulating: u32,
    /// Maladies déjà rencontrées (bits) : la population s'y est faite.
    #[serde(default)]
    pub met: u32,
    pub food_pc: f64,
    pub gdp_pc: f64,
    pub power_pc: f64,
    pub modern: f64,
    pub literacy: f64,
    pub deaths_epidemic: f64,
    pub deaths_war: f64,
    pub famine: u16,
}

/// Une ville : la population urbaine d'une cellule dépasse
/// [`CITY_POP`] (Uruk, vers −3500, comptait quelques dizaines de milliers
/// d'habitants).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct City {
    pub id: u32,
    pub cell: u32,
    pub name: String,
    pub founded: i64,
    pub abandoned: Option<i64>,
    pub society: u32,
    pub pop: f64,
    pub peak: f64,
}

/// Population urbaine qui fait une ville, et sous laquelle elle se vide.
pub const CITY_POP: f64 = 20_000.0;
pub const CITY_ABANDON: f64 = 8_000.0;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Cell {
    pub id: u32,
    pub land: Land,
    pub links: Vec<Link>,
    pub eco: Economy,
    pub settlement: Option<Settlement>,
    pub year: Option<Year>,
    /// Valeur échangée avec les voisins l'an passé, en rations.
    pub trade: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelationKind {
    Peace,
    Trade,
    Alliance,
    Tribute { payer: u32 },
    War { attacker: u32 },
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Relation {
    pub kind: RelationKind,
    pub since: i64,
    pub weariness: f64,
}

/// Totaux des flux vers la planète.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Totals {
    pub year: Flux,
    pub cum_co2_c: f64,
    pub cum_ch4_c: f64,
    pub cum_carbonate_c: f64,
    /// Méthane présent dans l'air, t C (durée de vie de 12 ans).
    pub ch4_burden_c: f64,
    pub cum_deforestation_c: f64,
    pub cum_erosion_t: f64,
    /// Part fossile du CO₂ cumulé (charbon et pétrole brûlés), t C.
    #[serde(default)]
    pub cum_fossil_c: f64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Counters {
    pub inventions: u64,
    pub diffusions: u64,
    pub losses: u64,
    pub wars: u64,
    pub conquests: u64,
    pub collapses: u64,
    pub scissions: u64,
    pub epidemics: u64,
    pub crises: u64,
    pub raids: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SocietyWorld {
    pub seed: u64,
    pub year: i64,
    pub cells: Vec<Cell>,
    pub species: Vec<Species>,
    pub wild: Vec<WildSpecies>,
    pub societies: Vec<Society>,
    pub relations: BTreeMap<(u32, u32), Relation>,
    pub notables: Vec<Notable>,
    pub diseases: Vec<Disease>,
    pub events: Vec<Event>,
    pub truths: Truths,
    pub totals: Totals,
    /// Carbone des stocks lents au départ, t C (bilan fermé).
    pub carbon_initial: f64,
    /// CO₂ de l'air au départ, ppm (vient de Planète).
    pub co2_ppm0: f64,
    /// Fin de la première version, par espèce.
    pub v1: BTreeMap<u32, i64>,
    /// Sortie du piège malthusien, par espèce.
    pub takeoff: BTreeMap<u32, i64>,
    pub contacts: BTreeSet<(u32, u32)>,
    pub counters: Counters,
    pub religions: u32,
    pub cities: Vec<City>,
    /// Les grands jalons, jamais oubliés : foyers agricoles, États,
    /// effondrements, rencontres, sortie industrielle, fin de la v1.
    pub milestones: Vec<Event>,
    /// Événements produits depuis le début (les plus anciens de `events`
    /// tombent au-delà de `event_cap`).
    pub events_total: u64,
    /// Sociétés en guerre au début de l'année (cache, recalculé).
    #[serde(skip)]
    pub war: Vec<bool>,
    /// Relations de chaque société (cache, recalculé avec `war`).
    #[serde(skip)]
    pub adj: Vec<Vec<(u32, Relation)>>,
    /// Événements gardés (les plus anciens tombent au-delà).
    pub event_cap: usize,
}

/// Nombre de cellules qu'une société peut tenir : une chefferie, deux ou
/// trois régions ; un État, selon son administration et ses routes
/// (Turchin et al., 2022 : la portée suit l'écriture, les routes, le
/// cheval).
pub fn max_cells(soc: &Society) -> usize {
    let d = &soc.dims;
    match soc.label.rank() {
        0 | 1 => 1,
        2 => 2,
        3 => 3 + (24.0 * d.admin * d.centralisation) as usize,
        _ => 8 + (60.0 * d.admin) as usize,
    }
}

/// La société peut-elle tenir une cellule de plus ?
fn room(soc: &Society) -> bool {
    soc.cells.len() < max_cells(soc)
}

fn key(a: u32, b: u32) -> (u32, u32) {
    if a < b {
        (a, b)
    } else {
        (b, a)
    }
}

/// Graine dérivée pour un objet et une année.
fn rng(seed: u64, year: i64, what: u64, id: u64) -> SimRng {
    rng_for(seed, Stream::Society, &[year as u64, what, id])
}

/// Bruit déterministe entre 0 et 1.
fn noise(seed: u64, a: u64, b: u64) -> f64 {
    let h = evo_core::rng::derive_seed(seed, Stream::Society, &[a, b, 0x5eed]);
    (h >> 11) as f64 / (1u64 << 53) as f64
}

impl SocietyWorld {
    pub fn new(seed: u64, truths: Truths) -> SocietyWorld {
        SocietyWorld {
            seed,
            year: 0,
            cells: Vec::new(),
            species: Vec::new(),
            wild: Vec::new(),
            societies: Vec::new(),
            relations: BTreeMap::new(),
            notables: Vec::new(),
            diseases: Vec::new(),
            events: Vec::new(),
            truths,
            totals: Totals::default(),
            carbon_initial: 0.0,
            co2_ppm0: 280.0,
            v1: BTreeMap::new(),
            takeoff: BTreeMap::new(),
            contacts: BTreeSet::new(),
            counters: Counters::default(),
            religions: 0,
            cities: Vec::new(),
            milestones: Vec::new(),
            events_total: 0,
            war: Vec::new(),
            adj: Vec::new(),
            event_cap: 20_000,
        }
    }

    pub fn add_species(&mut self, id: u32, name: &str, cognition: Cognition, milieu: Milieu) {
        let learning = Learning::of(&cognition);
        let hands = !cognition.manipulation.organ.handless() && cognition.manipulation.quality >= 0.5;
        let channel = cognition.channels.best().0;
        self.species.push(Species { id, name: name.to_string(), cognition, learning, milieu, hands, channel });
    }

    pub fn species(&self, id: u32) -> Option<&Species> {
        self.species.iter().find(|s| s.id == id)
    }

    pub fn add_wild(&mut self, w: WildSpecies) {
        self.wild.push(w);
    }

    pub fn add_cell(&mut self, land: Land) -> u32 {
        let id = self.cells.len() as u32;
        let eco = Economy {
            stocks: ZERO,
            cropland_km2: 0.0,
            irrigated_km2: 0.0,
            livestock: 0.0,
            soil: 1.0,
            forest_c: land.forest_c_km2 * land.area_km2,
            deposits: land.deposits,
            engine_short: [0.0; 3],
        };
        self.carbon_initial += economy::carbon_stock(&eco);
        self.cells.push(Cell { id, land, links: Vec::new(), eco, settlement: None, year: None, trade: 0.0 });
        id
    }

    pub fn link(&mut self, a: u32, b: u32, cost: f64, sea: bool) {
        for (x, y) in [(a, b), (b, a)] {
            let links = &mut self.cells[x as usize].links;
            if let Some(l) = links.iter_mut().find(|l| l.to == y) {
                l.cost = cost;
                l.sea = sea;
            } else {
                links.push(Link { to: y, cost, sea });
            }
        }
    }

    fn wild_of(&self, cell: u32) -> Vec<&WildSpecies> {
        let ids = &self.cells[cell as usize].land.wild;
        self.wild.iter().filter(|w| ids.contains(&w.id)).collect()
    }

    fn new_society(&mut self, species: u32, people: u32, cell: u32, parent: Option<u32>, phonology: Option<Phonology>) -> u32 {
        let id = self.societies.len() as u32;
        let mut r = rng(self.seed, self.year, 1, id as u64);
        let phonology = match phonology {
            Some(mut p) => {
                // Une société fille garde la langue de sa mère, à un son près.
                p.shift(&mut r);
                p
            }
            None => Phonology::random(&mut r),
        };
        let mut bias = [0.0; DOMAINS];
        for b in &mut bias {
            *b = r.random_range(-1.0..1.0);
        }
        let (dims, sciences, institutions, religion) = match parent.and_then(|p| self.societies.get(p as usize)) {
            Some(p) => (p.dims, p.sciences, p.institutions, p.religion),
            None => (
                Dimensions { cohesion: 0.7, participation: 0.8, ..Default::default() },
                [Model::default(); DOMAINS],
                [0.0; INSTITUTIONS],
                0,
            ),
        };
        let agg = Aggregates { legitimacy: 0.7, ..Default::default() };
        self.societies.push(Society {
            id,
            parent,
            species,
            people,
            phonology,
            name_form: r.random(),
            founded: self.year,
            ended: None,
            capital: cell,
            cells: vec![cell],
            dims,
            agg,
            institutions,
            label: Label::Bands,
            sciences,
            bias,
            religion,
            crisis: 0,
            before_crisis: None,
            known: Known::default(),
            chief: None,
            victories: 0,
            defeats: 0,
            peak: Label::Bands,
        });
        id
    }

    /// Installe une population dans une cellule vide, avec sa propre
    /// société (bandes acéphales).
    /// Les techniques que l'espèce ne peut pas pratiquer ici sont
    /// retirées (pas de feu sous l'eau).
    pub fn settle(&mut self, cell: u32, species: u32, pop: f64, known: Known, people: u32) -> u32 {
        let society = self.new_society(species, people, cell, None, None);
        self.place(cell, species, society, people, pop, known, None, None);
        let cat = catalog();
        let c = &self.cells[cell as usize];
        if let Some(s) = &c.settlement {
            let mut kept = known;
            for t in known.iter() {
                if !self.feasible(cat.get(t), c, s) {
                    kept.clear(t);
                }
            }
            let effects = cat.effects(&kept);
            if let Some(s) = self.cells[cell as usize].settlement.as_mut() {
                s.known = kept;
                s.effects = effects;
            }
            self.societies[society as usize].known = kept;
        }
        society
    }

    /// Banc d'essai : une société déjà agricole, lettrée et marchande,
    /// comme vers 1750. Les champs défrichés avant le départ comptent dans
    /// le carbone initial.
    pub fn prime_modern(&mut self, society: u32, cell: u32) {
        let c = cell as usize;
        // Dix mille ans de sélection : plantes et bêtes au meilleur.
        let crop = self.best_wild(&self.cells[c], false, None).map(|d| Domestic { quality: 1.2, ..d });
        let herd = self.best_wild(&self.cells[c], true, None).map(|d| Domestic { quality: 1.2, ..d });
        let before = economy::carbon_stock(&self.cells[c].eco);
        {
            let cl = &mut self.cells[c];
            let area = cl.land.area_km2;
            if crop.is_some() {
                let share = cl.settlement.as_ref().map(|s| economy::tilled_share(&s.known, &cl.land)).unwrap_or(0.0);
                cl.eco.cropland_km2 = area * cl.land.arable * share;
                let cleared = (cl.land.forest_c_km2 * cl.eco.cropland_km2).min(cl.eco.forest_c);
                cl.eco.forest_c -= cleared;
            }
            if herd.is_some() {
                cl.eco.livestock = area * cl.land.pasture * economy::LIVESTOCK_KM2 * 0.5;
            }
            cl.eco.soil = 0.8;
            cl.eco.stocks[Good::Grain.index()] = cl.settlement.as_ref().map(|s| s.pop * 0.3).unwrap_or(0.0);
            if let Some(s) = cl.settlement.as_mut() {
                s.crop = crop;
                s.herd = herd;
                s.sedentary = true;
                s.urban = 0.1;
            }
        }
        self.carbon_initial += economy::carbon_stock(&self.cells[c].eco) - before;
        let s = &mut self.societies[society as usize];
        s.dims = Dimensions { centralisation: 0.6, inequality: 0.5, coercion: 0.4, participation: 0.3, admin: 0.6, cohesion: 0.6 };
        s.institutions = [0.4, 0.6, 0.5, 0.5, 0.5, 0.6, 0.4];
        s.agg.elites = 0.03;
        s.agg.legitimacy = 0.6;
        s.label = Label::State;
        s.peak = Label::State;
        for (k, m) in s.sciences.iter_mut().enumerate() {
            m.precision = Domain::ALL[k].naked_eye().min(0.45);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn place(
        &mut self,
        cell: u32,
        species: u32,
        society: u32,
        people: u32,
        pop: f64,
        known: Known,
        crop: Option<Domestic>,
        herd: Option<Domestic>,
    ) {
        let effects = catalog().effects(&known);
        self.cells[cell as usize].settlement = Some(Settlement {
            species,
            society,
            people,
            pop,
            known,
            effects,
            crop,
            herd,
            sedentary: false,
            urban: 0.0,
            city: None,
            founded: self.year,
            immune: [0.0; MAX_DISEASES],
            circulating: 0,
            met: 0,
            food_pc: 1.0,
            gdp_pc: 1.0,
            power_pc: 0.0,
            modern: 0.0,
            literacy: 0.0,
            deaths_epidemic: 0.0,
            deaths_war: 0.0,
            famine: 0,
        });
    }

    #[allow(clippy::too_many_arguments)]
    fn event(
        &mut self,
        kind: EventKind,
        society: Option<u32>,
        other: Option<u32>,
        cell: Option<u32>,
        technique: Option<u16>,
        notable: Option<u32>,
        text: String,
        interest: f64,
    ) {
        let e = Event { year: self.year, kind, society, other, cell, technique, notable, text, interest };
        let milestone = match kind {
            EventKind::AgricultureHearth
            | EventKind::Collapse
            | EventKind::FirstContact
            | EventKind::IndustrialTakeoff
            | EventKind::EndOfFirstVersion => true,
            EventKind::Label => interest >= 0.7,
            _ => false,
        };
        if milestone {
            self.milestones.push(e.clone());
        }
        self.events.push(e);
        self.events_total += 1;
        if self.events.len() > self.event_cap {
            let cut = self.events.len() - self.event_cap;
            self.events.drain(..cut);
        }
    }

    /// Population d'une espèce.
    pub fn population(&self, species: u32) -> f64 {
        self.cells.iter().filter_map(|c| c.settlement.as_ref()).filter(|s| s.species == species).map(|s| s.pop).sum()
    }

    pub fn alive_societies(&self) -> impl Iterator<Item = &Society> {
        self.societies.iter().filter(|s| s.alive())
    }

    /// Une technique est-elle praticable dans cette cellule par cette espèce ?
    pub fn feasible(&self, t: &Technique, cell: &Cell, s: &Settlement) -> bool {
        let Some(sp) = self.species(s.species) else { return false };
        let land = &cell.land;
        let b = &t.body;
        if b.hands && !sp.hands || b.handless && sp.hands {
            return false;
        }
        if let Some(m) = b.milieu {
            if m != sp.milieu {
                return false;
            }
        }
        // Pas d'agriculture dans un climat glaciaire, qui change d'une
        // décennie à l'autre (Richerson, Boyd et Bettinger, 2001).
        if (t.flags.agriculture || t.flags.herding) && land.variability > 0.4 {
            return false;
        }
        if b.combustion && !land.fire() {
            return false;
        }
        if b.low_oxygen && land.o2_percent >= evo_culture::catalog::FIRE_O2_PERCENT {
            return false;
        }
        if b.sky && (!land.sky || sp.milieu == Milieu::Underground) {
            return false;
        }
        if b.sedentary && !s.known.has(catalog().keys.village) && sp.milieu == Milieu::Land {
            return false;
        }
        let dom = self.domesticates(cell, s);
        let wild = self.wild_of(cell.id);
        t.resources.iter().all(|&r| economy::has_resource(r, land, &cell.eco, &dom, &wild) || self.borrowed(r, cell))
    }

    /// Une plante ou un bête domestiquée chez un voisin, qui pousse ou vit
    /// ici : l'agriculture voyage avec ses semences (le blé du Croissant
    /// fertile en Europe).
    fn borrowed(&self, r: Resource, cell: &Cell) -> bool {
        let good = match r {
            Resource::SeedPlant => Some(Good::Grain),
            Resource::RootPlant => Some(Good::Roots),
            Resource::Abundance => return [Resource::SeedPlant, Resource::RootPlant].iter().any(|&x| self.borrowed(x, cell)),
            Resource::Animal | Resource::MilkAnimal => None,
            // Le charbon voyage aussi : une mine voisine ou du même pays
            // suffit (la Hollande et la Prusse rhénane brûlaient celui des
            // autres, Pomeranz, 2000).
            Resource::Coal => {
                let t = catalog().keys.extraction_charbon;
                let mine = |c: u32| {
                    let c = &self.cells[c as usize];
                    c.eco.deposits.coal_c > 1.0 && c.settlement.as_ref().is_some_and(|s| s.known.has(t))
                };
                let society = cell.settlement.as_ref().and_then(|s| self.societies.get(s.society as usize));
                return cell.links.iter().any(|l| mine(l.to)) || society.is_some_and(|s| s.cells.iter().any(|&c| mine(c)));
            }
            _ => return false,
        };
        let land = &cell.land;
        cell.links.iter().any(|l| {
            let Some(o) = &self.cells[l.to as usize].settlement else { return false };
            let d = match good {
                Some(g) => o.crop.filter(|c| c.good == g),
                None => o.herd.filter(|h| r != Resource::MilkAnimal || h.milk),
            };
            d.and_then(|d| self.wild.iter().find(|w| w.id == d.species)).map(|w| w.suits(land.temp_c, land.rain_mm) > 0.3).unwrap_or(false)
        })
    }

    fn domesticates(&self, cell: &Cell, s: &Settlement) -> Domesticates {
        let land = &cell.land;
        let suit = |d: &Domestic| self.wild.iter().find(|w| w.id == d.species).map(|w| w.suits(land.temp_c, land.rain_mm)).unwrap_or(0.0);
        let farming = s.known.intersects(&catalog().flagged.agriculture);
        let herding = s.known.intersects(&catalog().flagged.herding);
        let (crop, crop_good) = match (&s.crop, farming) {
            (Some(c), true) => (c.quality * suit(c), c.good),
            _ => (0.0, Good::Grain),
        };
        let (herd, milk) = match (&s.herd, herding) {
            (Some(h), true) => (h.quality * suit(h), h.milk),
            _ => (0.0, false),
        };
        // Les voies sans plante (algues, champignons) cultivent sans espèce
        // sauvage repérée.
        let (crop, crop_good) =
            if crop == 0.0 && farming && s.crop.is_none() && self.species(s.species).map(|sp| sp.milieu != Milieu::Land).unwrap_or(false) {
                (0.6, Good::Roots)
            } else {
                (crop, crop_good)
            };
        Domesticates { crop, crop_good, herd, milk }
    }

    /// Inventions d'une société dans l'année : (société, technique, cellule).
    fn invent_in(&self, soc: &Society, year: i64, seed: u64, half_steps: &[f64]) -> Vec<(u32, u16, u32)> {
        let cat = catalog();
        let mut found: Vec<(u32, u16, u32)> = Vec::new();
        let Some(sp) = self.species(soc.species) else { return found };
        let pop = soc.agg.population;
        if pop < 50.0 || !due(year, soc.id as u64) {
            return found;
        }
        let mut candidates: Vec<(u16, f64)> = Vec::new();
        {
            let mut r = rng(seed, year, 5, soc.id as u64);
            let eff = cat.effects(&soc.known);
            // Population qui échange vraiment des idées : la densité compte
            // (Henrich, 2004 ; Powell, Shennan et Thomas, 2009) ; des
            // chasseurs épars inventent peu.
            let mut dense = 0.0;
            for &c in &soc.cells {
                let cell = &self.cells[c as usize];
                if let Some(s) = &cell.settlement {
                    // Densité sur les terres qui nourrissent.
                    let density = s.pop / (cell.land.area_km2 * cell.land.arable.max(0.05)).max(1.0);
                    dense += s.pop * ((density / DENSE_KM2).min(1.0) * (1.0 - s.urban) + s.urban);
                }
            }
            let connected = dense + 0.25 * self.partners_dense(soc.id);
            let rank = soc.label.rank();
            // Après la méthode expérimentale, le savoir appelle le savoir
            // (Mokyr, 2002) : chaque marche déjà gravie efface le tiers du
            // coût d'une marche. Avant, chaque marche coûte plein tarif.
            let method = soc.known.intersects(&cat.flagged.method);
            let frontier = if method { soc.known.iter().map(|t| cat.depth[t as usize]).max().unwrap_or(0) as usize } else { 0 };
            let needs: [f64; 10] = std::array::from_fn(|i| self.need_factor(soc, Some(NEEDS[i])));
            let need = |n: Option<Need>| n.map(|n| needs[n as usize]).unwrap_or(1.0);
            // Le rythme des inventions suit la population qui échange
            // (Kremer, 1993) : à population égale, le monde invente autant
            // qu'il soit morcelé ou uni. Des chasseurs épars comptent pour
            // quelques centièmes.
            let connected = connected + 0.03 * pop;
            // Lire et la méthode expérimentale aident à inventer (Mokyr,
            // 2002 : les Lumières industrielles).
            let lettered = (1.0 + 3.0 * soc.agg.literacy) * if method { 2.0 } else { 1.0 };
            let base = INVENTION_RATE
                * lettered
                * connected
                * (0.3 + sp.learning.inventiveness)
                * eff.science.min(400.0).sqrt()
                * (0.6 + soc.agg.gdp / pop.max(1.0) / 2.0).clamp(0.5, 3.0);
            candidates.clear();
            let mut total = 0.0;
            for t in 0..cat.len() as u16 {
                if soc.known.has(t) || !cat.ready(t, &soc.known) {
                    continue;
                }
                let tech = cat.get(t);
                if tech.science.iter().any(|&(d, lvl)| soc.sciences[d.index()].precision < lvl) {
                    continue;
                }
                // Écrire, juger, battre monnaie : il faut un pouvoir central.
                let f = &tech.flags;
                if rank < 2 && (tech.need == Some(Need::Administration) || f.writing || f.law || f.money || f.market || f.school) {
                    continue;
                }
                let ease = 30.0 / tech.difficulty.max(5.0);
                let rate = if f.agriculture || f.herding {
                    // Domestiquer est une occasion du milieu plus qu'une
                    // idée : là où poussent de bonnes plantes, où vivent de
                    // bons animaux, des chasseurs fixés s'y mettent en
                    // quelques siècles (Diamond, 1997).
                    let q = self.opportunity(soc, f.herding);
                    DOMESTICATION_RATE * q * q * need(tech.need) * tech.inventiveness
                } else {
                    // Chaque marche de la chaîne des prérequis coûte : une
                    // idée lointaine demande tout un savoir autour d'elle
                    // (Kremer, 1993 ; Mokyr, 2002).
                    base * tech.inventiveness
                        * need(tech.need)
                        * ease
                        * ease
                        * half_steps[(2 * cat.depth[t as usize] as usize).saturating_sub(2 * frontier / 3).min(95)]
                };
                let rate = rate * SLOW_EVERY as f64;
                if rate > 0.0 {
                    candidates.push((t, rate));
                    total += rate;
                }
            }
            // Autant de tirages que d'inventions attendues, au poids de
            // chaque taux : un seul nombre au hasard dans l'année calme.
            let mut tries = total as usize + usize::from(r.random::<f64>() < total.fract());
            while tries > 0 && total > 0.0 && !candidates.is_empty() {
                tries -= 1;
                let mut x = r.random::<f64>() * total;
                let mut k = candidates.len() - 1;
                for (j, &(_, w)) in candidates.iter().enumerate() {
                    if x < w {
                        k = j;
                        break;
                    }
                    x -= w;
                }
                let (t, w) = candidates.swap_remove(k);
                total -= w;
                let tech = cat.get(t);
                // Où ? La plus grande cellule où elle se pratique.
                let mut best: Option<(f64, u32)> = None;
                for &c in &soc.cells {
                    let cell = &self.cells[c as usize];
                    let Some(s) = &cell.settlement else { continue };
                    if cat.ready(t, &s.known) && self.feasible(tech, cell, s) && best.is_none_or(|b| s.pop > b.0) {
                        best = Some((s.pop, c));
                    }
                }
                if let Some((_, c)) = best {
                    found.push((soc.id, t, c));
                }
            }
        }
        found
    }

    /// Meilleure espèce sauvage à domestiquer dans la cellule.
    fn best_wild(&self, cell: &Cell, animal: bool, good: Option<Good>) -> Option<Domestic> {
        let land = &cell.land;
        let mut best: Option<(f64, Domestic)> = None;
        for w in self.wild.iter().filter(|w| land.wild.contains(&w.id)) {
            let (score, g, milk) = match (&w.kind, animal) {
                (crate::land::WildKind::Animal(a), true) => (w.tameness(), Good::Livestock, a.milk),
                (crate::land::WildKind::Plant(_), false) => (w.cultivability(), w.good(), false),
                _ => continue,
            };
            if let Some(g0) = good {
                if g0 != g {
                    continue;
                }
            }
            let s = score * w.suits(land.temp_c, land.rain_mm);
            if s > 0.15 && best.as_ref().is_none_or(|b| s > b.0) {
                best = Some((s, Domestic { species: w.id, quality: score, good: g, milk }));
            }
        }
        best.map(|b| b.1)
    }

    /// La société est-elle en guerre ?
    pub fn at_war(&self, soc: u32) -> bool {
        self.war.get(soc as usize).copied().unwrap_or(false)
    }

    /// Recalcule qui est en guerre (relu tout au long de l'année).
    fn refresh_wars(&mut self) {
        let n = self.societies.len();
        // On garde les tampons d'une année sur l'autre.
        let mut war = std::mem::take(&mut self.war);
        war.clear();
        war.resize(n, false);
        let mut adj = std::mem::take(&mut self.adj);
        adj.resize_with(n, Vec::new);
        for v in &mut adj {
            v.clear();
        }
        for (k, r) in &self.relations {
            if matches!(r.kind, RelationKind::War { .. }) {
                war[k.0 as usize] = true;
                war[k.1 as usize] = true;
            }
            adj[k.0 as usize].push((k.1, *r));
            adj[k.1 as usize].push((k.0, *r));
        }
        self.war = war;
        self.adj = adj;
    }

    /// Politique qu'une société impose à ses cellules.
    fn policy(&self, soc: &Society) -> Policy {
        let i = |x: Institution| soc.institution(x);
        let cat = catalog();
        let writing = soc.known.intersects(&cat.flagged.writing);
        let printing = soc.known.intersects(&cat.flagged.printing);
        let school = soc.known.intersects(&cat.flagged.school);
        let compulsory = soc.known.has(cat.keys.ecole_obligatoire);
        let literacy = if writing {
            (0.004 + 0.03 * i(Institution::School) + if printing { 0.12 } else { 0.0 } + if compulsory { 0.75 } else { 0.0 }).min(0.98)
        } else {
            0.0
        };
        let at_war = self.at_war(soc.id);
        let market = i(Institution::Market) > 0.3;
        let industry = soc.known.intersects(&cat.flagged.industry);
        Policy {
            elites: soc.agg.elites,
            army: (0.01 + 0.05 * i(Institution::Army) + if at_war { 0.05 } else { 0.0 }) * soc.dims.centralisation.max(0.2),
            specialists: 0.005
                + 0.02 * (i(Institution::Temple) + i(Institution::Court) + i(Institution::School)) / 3.0
                + 0.02 * soc.dims.admin * soc.dims.centralisation,
            monuments: 0.004 * i(Institution::Ruler) * soc.dims.inequality,
            tax: 0.03 + 0.25 * soc.dims.centralisation * soc.dims.admin.max(0.2),
            literacy: literacy * if school { 1.0 } else { 0.5 },
            market,
            invest: industry && (market || soc.dims.centralisation > 0.6),
        }
    }

    // ------------------------------------------------------------------ Pas
    /// Avance d'un an.
    pub fn step(&mut self) {
        self.refresh_wars();
        self.year += 1;
        let year = self.year;
        let seed = self.seed;
        // Les sociétés éteintes n'ont plus de cellules : pas de politique à tirer.
        let policies: Vec<Policy> = self.societies.iter().map(|s| if s.alive() { self.policy(s) } else { Policy::default() }).collect();

        // Mines de charbon en activité.
        let coal_t = catalog().keys.extraction_charbon;
        let mines: Vec<bool> =
            self.cells.iter().map(|c| c.eco.deposits.coal_c > 1.0 && c.settlement.as_ref().is_some_and(|s| s.known.has(coal_t))).collect();
        let market = self.national_markets(&mines);

        // 1. Économie et démographie, en parallèle.
        let this = &*self;
        let results: Vec<Option<(Economy, Year, Settlement)>> = this
            .cells
            .par_iter()
            .map(|cell| {
                let s = cell.settlement.as_ref()?;
                let policy = policies.get(s.society as usize).copied().unwrap_or_default();
                let dom = this.domesticates(cell, s);
                let wild = this.wild_of(cell.id);
                let land = &cell.land;
                let village = s.known.has(catalog().keys.village);
                // Chasseurs-cueilleurs sédentaires : là seulement où l'eau
                // nourrit toute l'année (Natoufiens, Jomon, côte Nord-Ouest).
                let rich = (land.coast || land.river > 0.25) && land.npp() > 0.45;
                let sedentary =
                    village && (dom.crop > 0.0 || rich || this.species(s.species).map(|sp| sp.milieu != Milieu::Land).unwrap_or(false));
                let inp = Input {
                    land,
                    wild: &wild,
                    known: &s.known,
                    effects: &s.effects,
                    dom,
                    pop: s.pop,
                    sedentary,
                    policy,
                    fire: land.fire(),
                    coal_nearby: mines[cell.id as usize]
                        || market.coal[s.society as usize]
                        || cell.links.iter().any(|l| {
                            mines[l.to as usize]
                                && this.cells[l.to as usize].settlement.as_ref().is_some_and(|o| this.trades(o.society, s.society))
                        }),
                    oil_nearby: market.oil[s.society as usize],
                    export: market.export(cell.id as usize, s.society as usize),
                };
                let (eco, y) = economy::run(&inp, &cell.eco);
                let mut s2 = s.clone();
                s2.deaths_war = 0.0;
                s2.sedentary = sedentary;
                s2.food_pc = y.food_pc;
                s2.gdp_pc = y.gdp_pc;
                s2.power_pc = y.power_pc;
                s2.modern = y.modern_energy;
                s2.literacy = policy.literacy;
                // Démographie.
                let eff = &s.effects;
                let base_birth = if sedentary { 0.046 } else { 0.036 };
                // Transition démographique : l'école et la santé, pas la
                // seule richesse, font baisser les naissances (Caldwell,
                // 1980 ; Lee, 2003).
                let transition = {
                    let x = ((policy.literacy - 0.35).max(0.0) / 0.25) * (1.0 - eff.health).max(0.0) / 0.3;
                    1.0 / (1.0 + x * x)
                };
                let fed = y.food_pc.clamp(0.0, 1.3);
                let birth = base_birth
                    * eff.birth
                    * transition
                    * (0.6 + 0.4 * (fed / 1.1).min(1.0))
                    * (1.0 - 0.5 * policy.literacy * (1.0 - transition));
                let crowd = 1.0 + s.urban * 0.6 * eff.epidemic;
                let base_death = if sedentary { 0.039 } else { 0.031 };
                // La transition s'arrête près du renouvellement : les
                // familles visent deux enfants, pas moins.
                let other = (s.deaths_epidemic + s.deaths_war) / s.pop.max(1.0);
                let birth = if transition < 1.0 || eff.birth < 1.0 {
                    birth.max(base_death * eff.health * crowd + other.min(0.02) + 0.002)
                } else {
                    birth
                };
                let famine = (1.0 - y.food_pc).max(0.0);
                let death = (base_death * eff.health * crowd + 0.6 * famine * famine + 0.15 * famine).min(0.6);
                s2.pop = (s.pop * (1.0 + birth - death)).max(0.0);
                s2.famine = if y.food_pc < 0.85 { s.famine.saturating_add(1) } else { 0 };
                // L'immunité se dilue par les naissances.
                for d in 0..MAX_DISEASES {
                    let lasting = this.diseases.get(d).map(|x| x.lasting).unwrap_or(true);
                    s2.immune[d] *= (1.0 - birth as f32) * if lasting { 1.0 } else { 0.9 };
                }
                // Villes : la part urbaine suit les bras libérés des champs et
                // ce que l'administration et le marché tiennent.
                let nonfood = 1.0 - (y.food_workers / y.workers.max(1.0)).min(1.0);
                let urban_cap =
                    (0.02 + eff.urban + 0.15 * policy.tax + if policy.market { 0.03 } else { 0.0 } + 0.6 * y.modern_energy).min(0.92);
                // Sans champs, des hameaux et non des villes.
                let farmers =
                    if dom.crop > 0.0 || this.species(s.species).map(|sp| sp.milieu != Milieu::Land).unwrap_or(false) { 1.0 } else { 0.1 };
                let urban_target = (nonfood * 0.9).min(urban_cap) * f64::from(u8::from(sedentary)) * farmers;
                s2.urban += (urban_target - s.urban) * 0.03;
                // Sélection des domestiques (pression de sélection publiée).
                if let Some(c) = &mut s2.crop {
                    if dom.crop > 0.0 {
                        c.quality = (c.quality + 0.0004 * (1.25 - c.quality)).min(1.2);
                    }
                }
                if let Some(h) = &mut s2.herd {
                    if dom.herd > 0.0 {
                        h.quality = (h.quality + 0.0003 * (1.2 - h.quality)).min(1.2);
                    }
                }
                Some((eco, y, s2))
            })
            .collect();
        let mut flux = Flux::default();
        let mut fossil = 0.0;
        for (cell, r) in self.cells.iter_mut().zip(results) {
            if let Some((eco, y, s)) = r {
                let f = y.flux;
                flux.co2_c += f.co2_c;
                flux.ch4_c += f.ch4_c;
                flux.aerosol_t += f.aerosol_t;
                flux.cropland_km2 += f.cropland_km2;
                flux.pasture_km2 += f.pasture_km2;
                flux.irrigated_km2 += f.irrigated_km2;
                flux.deforestation_c += f.deforestation_c;
                flux.erosion_t += f.erosion_t;
                flux.carbonate_c += f.carbonate_c;
                fossil += y.consumed[Good::Coal.index()] * Good::Coal.carbon() + y.consumed[Good::Oil.index()] * Good::Oil.carbon();
                cell.eco = eco;
                cell.year = Some(y);
                cell.settlement = if s.pop >= 20.0 { Some(s) } else { None };
            }
        }
        self.pool_fuels(&market);
        let t = &mut self.totals;
        t.year = flux;
        t.cum_co2_c += flux.co2_c;
        t.cum_ch4_c += flux.ch4_c;
        t.cum_carbonate_c += flux.carbonate_c;
        t.ch4_burden_c = t.ch4_burden_c * (1.0 - 1.0 / 12.0) + flux.ch4_c;
        t.cum_deforestation_c += flux.deforestation_c;
        t.cum_erosion_t += flux.erosion_t;
        t.cum_fossil_c += fossil;

        // 2 à 4.
        self.migrate();
        self.trade();
        self.diffuse();
        self.epidemics();
        // 5.
        self.invent();
        self.lose();
        // 6 à 8.
        self.update_societies();
        self.diplomacy();
        self.refresh_wars();
        self.sciences();
        self.notables();
        self.new_diseases();
        self.cities();
        self.check_milestones();
        let _ = (year, seed);
    }

    // ------------------------------------------------------------------ Marché national
    /// Les canaux, puis le rail et les oléoducs, font un marché national du
    /// charbon et du pétrole : une société industrielle, qui a une mine ou un puits
    /// chez elle ou chez un partenaire du rail, en livre toutes ses
    /// cellules (Wrigley, 2010 : le charbon britannique voyage par canal
    /// puis par rail). Les mines et les puits extraient pour tout le pays
    /// ce qui y a manqué l'an passé.
    fn national_markets(&self, mines: &[bool]) -> Market {
        let cat = catalog();
        let n = self.societies.len();
        let industrial =
            |s: &Society| s.alive() && (s.known.intersects(&cat.flagged.industry) || cat.keys.locomotive.is_some_and(|t| s.known.has(t)));
        let mut has = vec![[false; 2]; n];
        let mut short = vec![[0.0f64; 2]; n];
        let mut sources = vec![[0u32; 2]; n];
        for c in &self.cells {
            let Some(s) = &c.settlement else { continue };
            let k = s.society as usize;
            if k >= n {
                continue;
            }
            let well = c.year.as_ref().is_some_and(|y| y.produced[Good::Oil.index()] > 0.0);
            if mines[c.id as usize] {
                has[k][0] = true;
                sources[k][0] += 1;
            }
            if well {
                has[k][1] = true;
                sources[k][1] += 1;
            }
            if let Some(y) = &c.year {
                short[k][0] += y.shortage[Good::Coal.index()];
                short[k][1] += y.shortage[Good::Oil.index()];
            }
        }
        // Marchés : les sociétés industrielles et les pays miniers qui
        // commercent avec elles, réunis par leurs traités de commerce.
        let ind: Vec<bool> = self.societies.iter().map(industrial).collect();
        let mut member = ind.clone();
        let mut root: Vec<u32> = (0..n as u32).collect();
        fn find(root: &mut [u32], x: u32) -> u32 {
            let mut x = x;
            while root[x as usize] != x {
                root[x as usize] = root[root[x as usize] as usize];
                x = root[x as usize];
            }
            x
        }
        for (k, r) in &self.relations {
            let (a, b) = (k.0 as usize, k.1 as usize);
            if a >= n || b >= n || !matches!(r.kind, RelationKind::Trade | RelationKind::Alliance) {
                continue;
            }
            let ok = |x: usize, y: usize| ind[x] && (ind[y] || has[y][0] || has[y][1]) && self.societies[y].alive();
            if ok(a, b) || ok(b, a) {
                member[a] = true;
                member[b] = true;
                let (ra, rb) = (find(&mut root, k.0), find(&mut root, k.1));
                if ra != rb {
                    root[ra.max(rb) as usize] = ra.min(rb);
                }
            }
        }
        // Les navires à vapeur font un seul marché de tous les ports
        // industriels et de leurs fournisseurs (le charbon de Cardiff
        // chauffe Buenos Aires).
        let steam = cat.keys.bateau_vapeur;
        let mut world_root: Option<u32> = None;
        for (k, soc) in self.societies.iter().enumerate() {
            if !member[k] || !soc.alive() {
                continue;
            }
            if steam.is_some_and(|t| soc.known.has(t)) {
                let rk = find(&mut root, k as u32);
                match world_root {
                    None => world_root = Some(rk),
                    Some(w) => {
                        let w = find(&mut root, w);
                        if w != rk {
                            root[w.max(rk) as usize] = w.min(rk);
                        }
                    }
                }
            }
        }
        let group: Vec<u32> = (0..n as u32).map(|k| find(&mut root, k)).collect();
        let mut g_has = vec![[false; 2]; n];
        let mut g_short = vec![[0.0f64; 2]; n];
        let mut g_sources = vec![[0u32; 2]; n];
        for k in (0..n).filter(|&k| member[k]) {
            let g = group[k] as usize;
            for j in 0..2 {
                g_has[g][j] |= has[k][j];
                g_short[g][j] += short[k][j];
                g_sources[g][j] += sources[k][j];
            }
        }
        let coal = (0..n).map(|k| ind[k] && g_has[group[k] as usize][0]).collect();
        let oil = (0..n).map(|k| ind[k] && g_has[group[k] as usize][1]).collect();
        Market { coal, oil, short: g_short, sources: g_sources, member, group, mines: mines.to_vec() }
    }

    /// Le marché national répartit les stocks de charbon et de pétrole
    /// entre les cellules d'une société du rail, au prorata du manque ;
    /// 5 % se perdent en route (leur carbone part à l'air).
    fn pool_fuels(&mut self, market: &Market) {
        let n = self.societies.len();
        let goods = [Good::Coal, Good::Oil];
        let mut stock = vec![[0.0f64; 2]; n];
        let mut want = vec![[0.0f64; 2]; n];
        let group_of = |soc: u32| -> Option<usize> {
            let k = soc as usize;
            (k < n && market.member[k]).then(|| market.group[k] as usize)
        };
        for c in &self.cells {
            let (Some(s), Some(y)) = (&c.settlement, &c.year) else { continue };
            let Some(g) = group_of(s.society) else { continue };
            for (j, good) in goods.iter().enumerate() {
                stock[g][j] += c.eco.stocks[good.index()];
                want[g][j] += y.shortage[good.index()];
            }
        }
        for c in self.cells.iter_mut() {
            let (Some(s), Some(y)) = (&c.settlement, &c.year) else { continue };
            let Some(g) = group_of(s.society) else { continue };
            for (j, good) in goods.iter().enumerate() {
                let (st, w) = (stock[g][j], want[g][j]);
                if st <= 0.0 || w <= 0.0 {
                    continue;
                }
                // Chaque stock donne sa part de ce qui est livré ; chaque
                // manque en reçoit sa part.
                let delivered = st.min(w);
                let i = good.index();
                let out = c.eco.stocks[i] / st * delivered;
                let into = y.shortage[i] / w * delivered;
                c.eco.stocks[i] += into * 0.95 - out;
                let lost = into * 0.05 * good.carbon();
                self.totals.year.co2_c += lost;
                self.totals.cum_co2_c += lost;
                self.totals.cum_fossil_c += lost;
            }
        }
    }

    // ------------------------------------------------------------------ Migrations
    fn can_cross(&self, s: &Settlement, l: &Link) -> bool {
        if !l.sea {
            return true;
        }
        let cat = catalog();
        let nav = s.known.intersects(&cat.flagged.navigation);
        let raft = cat.keys.radeau.map(|r| s.known.has(r)).unwrap_or(false);
        nav || raft && l.cost <= 2.0
    }

    /// Capacité qu'auraient des arrivants dans une cellule, en personnes.
    fn capacity_for(&self, cell: &Cell, s: &Settlement) -> f64 {
        let land = &cell.land;
        if land.ocean != (self.species(s.species).map(|sp| sp.milieu == Milieu::Water).unwrap_or(false)) {
            return 0.0;
        }
        let forage = land.area_km2 * land.npp() * economy::FORAGE_PER_NPP * s.effects.forage.sqrt();
        let fish = if land.coast { land.area_km2 * 0.04 * s.effects.fishing } else { 0.0 };
        let farm = if s.crop.is_some() && s.sedentary {
            let suit = s
                .crop
                .and_then(|c| self.wild.iter().find(|w| w.id == c.species).map(|w| w.suits(land.temp_c, land.rain_mm) * c.quality))
                .unwrap_or(0.0);
            land.area_km2 * land.arable * economy::BASE_YIELD_KM2 * suit * (1.0 - land.variability) * s.effects.crop_yield * 0.25
        } else {
            0.0
        };
        forage + fish + farm
    }

    fn migrate(&mut self) {
        let year = self.year;
        let this = &*self;
        // Plans : chaque cellule propose un départ vers son meilleur voisin.
        let plans: Vec<Option<(u32, u32, f64)>> = this
            .cells
            .par_iter()
            .map(|cell| {
                let s = cell.settlement.as_ref()?;
                let here_cap = this.capacity_for(cell, s).max(1.0);
                let pressure = (1.0 - s.food_pc).max(0.0) + (s.pop / here_cap - 0.8).max(0.0) * 0.5;
                let mut best: Option<(u32, f64)> = None;
                for l in &cell.links {
                    if !this.can_cross(s, l) {
                        continue;
                    }
                    let other = &this.cells[l.to as usize];
                    let cap = this.capacity_for(other, s);
                    if cap <= 0.0 {
                        continue;
                    }
                    let occupied = other.settlement.as_ref().map(|o| o.pop).unwrap_or(0.0);
                    let room = (cap - occupied) / cap;
                    let attract = room / l.cost;
                    if attract > 0.2 && best.is_none_or(|b| attract > b.1) {
                        best = Some((l.to, attract));
                    }
                }
                let (to, attract) = best?;
                let rate = (0.002 + 0.03 * pressure.min(1.0)) * attract.min(1.0);
                let movers = s.pop * rate;
                if movers < 30.0 && this.cells[to as usize].settlement.is_some() {
                    return None;
                }
                Some((cell.id, to, movers.max(if this.cells[to as usize].settlement.is_none() { (s.pop * 0.01).min(200.0) } else { 0.0 })))
            })
            .collect();
        for (from, to, movers) in plans.into_iter().flatten() {
            let Some(src) = self.cells[from as usize].settlement.clone() else { continue };
            let movers = movers.min(src.pop * 0.2);
            if movers < 20.0 {
                continue;
            }
            let dst = self.cells[to as usize].settlement.clone();
            match dst {
                None => {
                    // Colonisation : une chefferie garde ses colonies, des
                    // bandes en font des sociétés filles.
                    let parent = &self.societies[src.society as usize];
                    let keep = parent.alive() && room(parent);
                    let society = if keep {
                        src.society
                    } else {
                        let ph = parent.phonology.clone();
                        let id = self.new_society(src.species, src.people, to, Some(src.society), Some(ph));
                        self.societies[id as usize].known = src.known;
                        id
                    };
                    let crop = src.crop.and_then(|c| {
                        // Une plante qui ne pousse pas ici est remplacée par
                        // une plante locale, si elle existe.
                        let land = &self.cells[to as usize].land;
                        let suit = self.wild.iter().find(|w| w.id == c.species).map(|w| w.suits(land.temp_c, land.rain_mm)).unwrap_or(0.0);
                        if suit > 0.3 {
                            Some(c)
                        } else {
                            self.best_wild(&self.cells[to as usize], false, Some(c.good)).or(Some(c))
                        }
                    });
                    self.place(to, src.species, society, src.people, movers, src.known, crop, src.herd);
                    if let Some(d) = self.cells[to as usize].settlement.as_mut() {
                        d.immune = src.immune;
                        d.met = src.met;
                        d.sedentary = src.sedentary;
                    }
                }
                Some(d) if d.species == src.species => {
                    if let Some(dd) = self.cells[to as usize].settlement.as_mut() {
                        dd.pop += movers;
                    }
                }
                Some(d) => {
                    // Concurrence entre espèces : pas de métissage ; le plus
                    // nombreux finit par évincer l'autre.
                    let sp = self.species(src.species).map(|s| s.milieu);
                    let dp = self.species(d.species).map(|s| s.milieu);
                    if sp != dp {
                        continue;
                    }
                    let cap = self.capacity_for(&self.cells[to as usize], &d).max(1.0);
                    if let Some(dd) = self.cells[to as usize].settlement.as_mut() {
                        dd.pop -= movers * 0.5;
                    }
                    if d.pop - movers * 0.5 < 0.08 * cap {
                        let parent = &self.societies[src.society as usize];
                        let society = if room(parent) {
                            src.society
                        } else {
                            let ph = parent.phonology.clone();
                            self.new_society(src.species, src.people, to, Some(src.society), Some(ph))
                        };
                        let txt = format!(
                            "Les {} évincent les {} d'une région, sans guerre, par le nombre.",
                            self.species(src.species).map(|s| s.name.clone()).unwrap_or_default(),
                            self.species(d.species).map(|s| s.name.clone()).unwrap_or_default()
                        );
                        self.place(to, src.species, society, src.people, movers, src.known, src.crop, src.herd);
                        self.event(EventKind::Conquest, Some(society), Some(d.society), Some(to), None, None, txt, 0.6);
                    }
                }
            }
            if let Some(s) = self.cells[from as usize].settlement.as_mut() {
                s.pop -= movers;
            }
        }
        let _ = year;
    }

    // ------------------------------------------------------------------ Échanges
    fn relation(&self, a: u32, b: u32) -> Option<&Relation> {
        self.relations.get(&key(a, b))
    }

    fn trades(&self, a: u32, b: u32) -> bool {
        a == b
            || matches!(
                self.relation(a, b).map(|r| r.kind),
                Some(RelationKind::Trade | RelationKind::Alliance | RelationKind::Tribute { .. })
            )
    }

    fn trade(&mut self) {
        const MOVED: [Good; 16] = [
            Good::Grain,
            Good::Roots,
            Good::Copper,
            Good::Tin,
            Good::Bronze,
            Good::Iron,
            Good::Coal,
            Good::Oil,
            Good::Tools,
            Good::Weapons,
            Good::Salt,
            Good::Textile,
            Good::Luxury,
            Good::Machines,
            Good::Fertilizer,
            Good::Medicine,
        ];
        let n = self.cells.len();
        for c in &mut self.cells {
            c.trade = 0.0;
        }
        for a in 0..n {
            let links = self.cells[a].links.clone();
            for l in links {
                let b = l.to as usize;
                if b <= a {
                    continue;
                }
                let (Some(sa), Some(sb)) = (&self.cells[a].settlement, &self.cells[b].settlement) else { continue };
                if !self.trades(sa.society, sb.society) {
                    continue;
                }
                if l.sea && !(self.can_cross(sa, &l) && self.can_cross(sb, &l)) {
                    continue;
                }
                let tr = sa.effects.transport.max(sb.effects.transport);
                let reach = sa.effects.reach.max(sb.effects.reach);
                let eff = (tr / (tr + l.cost * 1.5)) * (0.3 + 0.1 * reach).min(1.0);
                let mut value = 0.0;
                for g in MOVED {
                    let i = g.index();
                    let (ya, yb) = (self.cells[a].year.as_ref(), self.cells[b].year.as_ref());
                    let (Some(ya), Some(yb)) = (ya, yb) else { continue };
                    let (short_a, short_b) = (ya.shortage[i], yb.shortage[i]);
                    let (stock_a, stock_b) = (self.cells[a].eco.stocks[i], self.cells[b].eco.stocks[i]);
                    let (from, to, q) = if short_b > 0.0 && stock_a > 0.0 {
                        (a, b, (stock_a * 0.5).min(short_b))
                    } else if short_a > 0.0 && stock_b > 0.0 {
                        (b, a, (stock_b * 0.5).min(short_a))
                    } else {
                        continue;
                    };
                    self.cells[from].eco.stocks[i] -= q;
                    // Ce qui se perd en route se perd pour de bon (le
                    // carbone du charbon ou du pétrole perdu part à l'air).
                    self.cells[to].eco.stocks[i] += q * eff;
                    let lost = q * (1.0 - eff);
                    self.totals.year.co2_c += lost * g.carbon();
                    self.totals.cum_co2_c += lost * g.carbon();
                    if matches!(g, Good::Coal | Good::Oil) {
                        self.totals.cum_fossil_c += lost * g.carbon();
                    }
                    value += q * g.value();
                }
                self.cells[a].trade += value;
                self.cells[b].trade += value;
            }
        }
        // Tribut : le vassal verse une part de son surplus à la capitale du
        // suzerain.
        let tributes: Vec<(u32, u32)> = self
            .relations
            .iter()
            .filter_map(|(k, r)| {
                if let RelationKind::Tribute { payer } = r.kind {
                    Some((payer, if k.0 == payer { k.1 } else { k.0 }))
                } else {
                    None
                }
            })
            .collect();
        for (payer, lord) in tributes {
            let (Some(p), Some(l)) = (self.societies.get(payer as usize), self.societies.get(lord as usize)) else { continue };
            if !p.alive() || !l.alive() {
                continue;
            }
            let (pc, lc) = (p.capital as usize, l.capital as usize);
            let g = Good::Grain.index();
            let q = self.cells[pc].eco.stocks[g] * 0.2;
            self.cells[pc].eco.stocks[g] -= q;
            self.cells[lc].eco.stocks[g] += q * 0.8;
        }
    }

    // ------------------------------------------------------------------ Diffusion
    fn compatible(&self, a: u32, b: u32) -> f64 {
        if a == b {
            return 1.0;
        }
        let (Some(x), Some(y)) = (self.species(a), self.species(b)) else { return 0.0 };
        if x.channel == y.channel {
            0.3
        } else {
            0.08
        }
    }

    fn diffuse(&mut self) {
        let year = self.year;
        let seed = self.seed;
        let cat = catalog();
        // Partenaires de commerce et alliés, par société.
        let mut partners: Vec<Vec<u32>> = vec![Vec::new(); self.societies.len()];
        for (k, r) in &self.relations {
            if matches!(r.kind, RelationKind::Trade | RelationKind::Alliance) {
                let (a, b) = (k.0 as usize, k.1 as usize);
                if a < partners.len() && b < partners.len() && self.societies[a].alive() && self.societies[b].alive() {
                    partners[a].push(k.1);
                    partners[b].push(k.0);
                }
            }
        }
        let this = &*self;
        let plans: Vec<Vec<(u16, u32)>> = this
            .cells
            .par_iter()
            .map(|cell| {
                let mut out = Vec::new();
                let Some(s) = cell.settlement.as_ref() else { return out };
                let mut r = rng(seed, year, 2, cell.id as u64);
                for l in &cell.links {
                    let Some(o) = this.cells[l.to as usize].settlement.as_ref() else { continue };
                    if l.sea && !(this.can_cross(s, l) || this.can_cross(o, l)) {
                        continue;
                    }
                    let new = o.known.and_not(&s.known);
                    if new.is_empty() {
                        continue;
                    }
                    let same = s.society == o.society;
                    let rel = this.relation(s.society, o.society).map(|r| r.kind);
                    let contact = if same {
                        3.0
                    } else {
                        match rel {
                            Some(RelationKind::Trade | RelationKind::Alliance) => 2.0,
                            Some(RelationKind::Tribute { .. }) => 2.0,
                            Some(RelationKind::War { .. }) => 1.2,
                            _ => 1.0,
                        }
                    } * this.compatible(s.species, o.species)
                        * (1.0 + (cell.trade / 1e5).min(2.0))
                        / l.cost;
                    for t in new.iter() {
                        if r.random::<f64>() < DIFFUSION_RATE * contact {
                            let tech = cat.get(t);
                            if cat.ready(t, &s.known) && this.feasible(tech, cell, s) {
                                out.push((t, l.to));
                            }
                        }
                    }
                }
                // Le savoir écrit voyage dans tout le pays, pas seulement de
                // proche en proche : livres, ingénieurs, brevets (Mokyr,
                // 2002). Il va d'autant plus vite que l'on sait lire.
                if s.literacy > 0.0 {
                    if let Some(soc) = this.societies.get(s.society as usize) {
                        let p = DIFFUSION_RATE * WRITTEN_DIFFUSION * s.literacy;
                        for t in soc.known.and_not(&s.known).iter() {
                            if r.random::<f64>() < p {
                                let tech = cat.get(t);
                                if cat.ready(t, &s.known) && this.feasible(tech, cell, s) {
                                    out.push((t, cell.id));
                                }
                            }
                        }
                    }
                    // Et chez les partenaires de commerce, plus lentement :
                    // traductions, ingénieurs venus d'ailleurs (le Japon de
                    // Meiji, la Russie de Pierre le Grand).
                    for &o in partners.get(s.society as usize).map(Vec::as_slice).unwrap_or(&[]) {
                        let other = &this.societies[o as usize];
                        let p = DIFFUSION_RATE * PARTNER_DIFFUSION * s.literacy * this.compatible(s.species, other.species);
                        for t in other.known.and_not(&s.known).iter() {
                            if r.random::<f64>() < p {
                                let tech = cat.get(t);
                                if cat.ready(t, &s.known) && this.feasible(tech, cell, s) {
                                    out.push((t, cell.id));
                                }
                            }
                        }
                    }
                }
                out
            })
            .collect();
        for (cell, adds) in self.cells.iter_mut().zip(plans) {
            if adds.is_empty() {
                continue;
            }
            let Some(s) = cell.settlement.as_mut() else { continue };
            for (t, _from) in adds {
                if !s.known.has(t) {
                    s.known.set(t);
                    self.counters.diffusions += 1;
                }
            }
            s.effects = cat.effects(&s.known);
        }
        // Les domestiques voyagent avec les techniques.
        let n = self.cells.len();
        for i in 0..n {
            let Some(s) = self.cells[i].settlement.clone() else { continue };
            let farming = s.known.intersects(&cat.flagged.agriculture);
            let herding = s.known.intersects(&cat.flagged.herding);
            if farming && s.crop.is_none() || herding && s.herd.is_none() {
                let local_crop = if farming && s.crop.is_none() { self.best_wild(&self.cells[i], false, None) } else { None };
                let local_herd = if herding && s.herd.is_none() { self.best_wild(&self.cells[i], true, None) } else { None };
                let mut crop = local_crop;
                let mut herd = local_herd;
                for l in &self.cells[i].links {
                    if let Some(o) = &self.cells[l.to as usize].settlement {
                        let land = &self.cells[i].land;
                        let fit = |d: &Domestic| {
                            self.wild
                                .iter()
                                .find(|w| w.id == d.species)
                                .map(|w| w.suits(land.temp_c, land.rain_mm) * d.quality)
                                .unwrap_or(0.0)
                        };
                        if farming && s.crop.is_none() {
                            if let Some(c) = o.crop {
                                if fit(&c) > crop.map(|x| fit(&x)).unwrap_or(0.0) {
                                    crop = Some(c);
                                }
                            }
                        }
                        if herding && s.herd.is_none() {
                            if let Some(h) = o.herd {
                                if fit(&h) > herd.map(|x| fit(&x)).unwrap_or(0.0) {
                                    herd = Some(h);
                                }
                            }
                        }
                    }
                }
                if let Some(st) = self.cells[i].settlement.as_mut() {
                    if st.crop.is_none() {
                        st.crop = crop;
                    }
                    if st.herd.is_none() {
                        st.herd = herd;
                        if herd.is_some() && self.cells[i].eco.livestock < 10.0 {
                            self.cells[i].eco.livestock = 10.0;
                        }
                    }
                }
            }
        }
    }

    // ------------------------------------------------------------------ Épidémies
    fn epidemics(&mut self) {
        let year = self.year;
        let seed = self.seed;
        if self.diseases.is_empty() {
            return;
        }
        // Population connectée qui porte chaque maladie, par société.
        let mut carriers: BTreeMap<(u32, usize), f64> = BTreeMap::new();
        for c in &self.cells {
            if let Some(s) = &c.settlement {
                for d in 0..self.diseases.len() {
                    if s.circulating >> d & 1 == 1 {
                        *carriers.entry((s.society, d)).or_default() += s.pop;
                    }
                }
            }
        }
        // Propagation : de chaque cellule où circule une maladie vers ses
        // voisins, selon les contacts.
        let n = self.cells.len();
        let mut intro = vec![0u32; n];
        for i in 0..n {
            let Some(s) = &self.cells[i].settlement else { continue };
            if s.circulating == 0 {
                continue;
            }
            let mut r = rng(seed, year, 3, i as u64);
            for l in &self.cells[i].links {
                let Some(o) = &self.cells[l.to as usize].settlement else { continue };
                if o.species != s.species {
                    continue;
                }
                if l.sea && !self.can_cross(s, l) {
                    continue;
                }
                let contact = (0.25 + (self.cells[i].trade / 2e5).min(0.6)) / l.cost;
                for d in 0..self.diseases.len() {
                    if s.circulating >> d & 1 == 1 && o.circulating >> d & 1 == 0 && r.random::<f64>() < contact {
                        intro[l.to as usize] |= 1 << d;
                    }
                }
            }
        }
        let diseases = self.diseases.clone();
        for (i, cell) in self.cells.iter_mut().enumerate() {
            let Some(s) = cell.settlement.as_mut() else { continue };
            s.deaths_epidemic = 0.0;
            let mut next = 0u32;
            // Les maladies d'enfance s'additionnent mal : elles tuent les
            // mêmes enfants fragiles, et la mortalité de base les compte
            // déjà en partie.
            let mut endemic = 0.0;
            for (d, dis) in diseases.iter().enumerate() {
                let here = s.circulating >> d & 1 == 1 || intro[i] >> d & 1 == 1;
                if !here || dis.species != s.species {
                    continue;
                }
                let susceptible = (1.0 - s.immune[d] as f64 * s.effects.immunity.recip().min(4.0)).clamp(0.0, 1.0);
                // Les maladies de foule vivent des villes (McNeill, 1976).
                let density = (0.15 + s.urban * 3.0).min(1.5);
                let r_eff = dis.r0 * susceptible * density * s.effects.epidemic.sqrt();
                if r_eff > 1.0 {
                    let z = final_size(r_eff);
                    let hit = s.pop * susceptible * z;
                    // Sur un sol vierge, toute la létalité ; une maladie de
                    // foule déjà connue devient une maladie d'enfance
                    // (McNeill, 1976), et elles tuent les mêmes enfants.
                    // Une peste qui revient tue moins : les plus fragiles sont
                    // partis, le germe s'est adouci (vagues de la peste
                    // noire après 1348).
                    let known_before = s.met >> d & 1 == 1;
                    let mut deaths = hit * dis.lethality * s.effects.epidemic;
                    if known_before && dis.lasting {
                        deaths = (deaths * ADAPTED_LETHALITY).min((ENDEMIC_CAP * s.pop - endemic).max(0.0));
                        endemic += deaths;
                    } else if known_before {
                        deaths *= RETURNING_PLAGUE;
                    }
                    s.met |= 1 << d;
                    s.pop -= deaths;
                    s.deaths_epidemic += deaths;
                    if dis.lasting {
                        s.immune[d] = (s.immune[d] as f64 + susceptible * z).min(1.0) as f32;
                    } else {
                        s.immune[d] = (s.immune[d] as f64 + susceptible * z * 0.7).min(1.0) as f32;
                    }
                    next |= 1 << d;
                } else {
                    // Endémique si la population connectée suffit : les
                    // enfants l'attrapent, quelques-uns en meurent.
                    let conn = carriers.get(&(s.society, d)).copied().unwrap_or(0.0) + s.pop;
                    if conn >= CRITICAL_COMMUNITY && dis.lasting {
                        let deaths = (s.pop * 0.01 * dis.lethality * s.effects.epidemic).min((ENDEMIC_CAP * s.pop - endemic).max(0.0));
                        endemic += deaths;
                        s.pop -= deaths;
                        s.deaths_epidemic += deaths;
                        next |= 1 << d;
                    }
                }
            }
            s.circulating = next;
        }
        // Chronique : les grandes vagues.
        let mut waves: BTreeMap<u32, (f64, f64)> = BTreeMap::new();
        for c in &self.cells {
            if let Some(s) = &c.settlement {
                let e = waves.entry(s.society).or_default();
                e.0 += s.deaths_epidemic;
                e.1 += s.pop;
            }
        }
        for (soc, (deaths, pop)) in waves {
            if deaths > 0.08 * (pop + deaths) && deaths > 5_000.0 {
                self.counters.epidemics += 1;
                let name = self.societies[soc as usize].name();
                self.event(
                    EventKind::Epidemic,
                    Some(soc),
                    None,
                    None,
                    None,
                    None,
                    format!("Une épidémie emporte {:.0} % des {name}.", 100.0 * deaths / (pop + deaths)),
                    0.7,
                );
            }
        }
    }

    fn new_diseases(&mut self) {
        if self.diseases.len() >= MAX_DISEASES {
            return;
        }
        let year = self.year;
        let seed = self.seed;
        for i in 0..self.cells.len() {
            let Some(s) = &self.cells[i].settlement else { continue };
            let urban = s.pop * s.urban;
            if urban < 20_000.0 || self.cells[i].eco.livestock < 1_000.0 {
                continue;
            }
            let mut r = rng(seed, year, 4, i as u64);
            if r.random::<f64>() < 3e-6 * (urban / 1e5).min(10.0) {
                let id = self.diseases.len() as u32;
                let plague = s.known.intersects(&catalog().flagged.navigation) && r.random_bool(0.3);
                let (r0, lethality, lasting) = if plague {
                    (r.random_range(1.8..3.5), r.random_range(0.15..0.4), false)
                } else {
                    (r.random_range(4.0..16.0), r.random_range(0.02..0.2), true)
                };
                let soc = &self.societies[s.society as usize];
                let name = format!("mal {}", soc.phonology.word(0x4d41_4c00 + id));
                let species = s.species;
                self.diseases.push(Disease {
                    id,
                    name: name.clone(),
                    r0,
                    lethality,
                    lasting,
                    origin_year: year,
                    origin_cell: i as u32,
                    species,
                });
                if let Some(s) = self.cells[i].settlement.as_mut() {
                    s.circulating |= 1 << id;
                }
                let sname = self.societies[self.cells[i].settlement.as_ref().map(|s| s.society).unwrap_or(0) as usize].name();
                self.event(
                    EventKind::NewDisease,
                    self.cells[i].settlement.as_ref().map(|s| s.society),
                    None,
                    Some(i as u32),
                    None,
                    None,
                    format!("Une maladie nouvelle, le {name}, passe du bétail aux hommes chez les {sname}."),
                    0.5,
                );
            }
        }
    }

    // ------------------------------------------------------------------ Inventions
    fn need_factor(&self, soc: &Society, need: Option<Need>) -> f64 {
        let Some(n) = need else { return 1.0 };
        let a = &soc.agg;
        let at_war = self.at_war(soc.id);
        1.0 + match n {
            Need::Food => 2.5 * ((1.1 - a.food_pc) / 0.3).clamp(0.0, 1.0),
            Need::Fuel => {
                let mut short = 0.0;
                let mut want = 0.0;
                for &c in &soc.cells {
                    if let Some(y) = &self.cells[c as usize].year {
                        short += y.shortage[Good::Wood.index()] + y.shortage[Good::Charcoal.index()];
                        want += y.consumed[Good::Wood.index()] + y.consumed[Good::Charcoal.index()] + 1.0;
                    }
                }
                3.0 * (short / want).min(1.0)
            }
            Need::Defence => {
                if at_war {
                    2.0
                } else {
                    0.3
                }
            }
            Need::Conquest => 1.5 * soc.dims.centralisation,
            Need::Administration => (libm::log10(a.population.max(1.0)) - 4.0).clamp(0.0, 3.0),
            Need::Transport => (libm::log10(a.population.max(1.0)) - 4.5).clamp(0.0, 2.0) + 0.5 * f64::from(u8::from(soc.cells.len() > 3)),
            Need::Health => 2.0 * (a.urban * 3.0).min(1.0),
            Need::Knowledge => 3.0 * a.literacy.sqrt(),
            Need::Shelter => 0.5,
            Need::Prestige => soc.dims.inequality * 2.0,
        }
    }

    fn invent(&mut self) {
        let year = self.year;
        let seed = self.seed;
        let cat = catalog();
        // Coût par demi-marche : DEPTH_COST^(k/2).
        let half = DEPTH_COST.sqrt();
        let half_steps: Vec<f64> = (0..96)
            .scan(1.0, |x, _| {
                let v = *x;
                *x *= half;
                Some(v)
            })
            .collect();
        // Chaque société tire ses inventions de son propre flux : les
        // plans se calculent en parallèle et s'appliquent dans l'ordre.
        let this = &*self;
        let found: Vec<(u32, u16, u32)> =
            this.societies.iter().filter(|s| s.alive()).flat_map(|soc| this.invent_in(soc, year, seed, &half_steps)).collect();
        for (soc, t, c) in found {
            let tech = cat.get(t);
            let first_agri = tech.flags.agriculture && !self.societies[soc as usize].known.intersects(&cat.flagged.agriculture);
            if let Some(s) = self.cells[c as usize].settlement.as_mut() {
                s.known.set(t);
                s.effects = cat.effects(&s.known);
            }
            self.societies[soc as usize].known.set(t);
            self.counters.inventions += 1;
            // Domestication : l'espèce locale entre dans la cellule.
            if tech.flags.agriculture || tech.flags.herding {
                let animal = tech.flags.herding;
                let good = if tech.resources.contains(&Resource::RootPlant) {
                    Some(Good::Roots)
                } else if tech.resources.contains(&Resource::SeedPlant) {
                    Some(Good::Grain)
                } else {
                    None
                };
                let dom = self.best_wild(&self.cells[c as usize], animal, good);
                if let Some(s) = self.cells[c as usize].settlement.as_mut() {
                    if animal {
                        if s.herd.is_none() {
                            s.herd = dom;
                        }
                    } else if s.crop.is_none() {
                        s.crop = dom;
                    }
                }
                if animal && dom.is_some() && self.cells[c as usize].eco.livestock < 10.0 {
                    self.cells[c as usize].eco.livestock = 10.0;
                }
            }
            let inventor = self.draw_notable(NotableKind::Inventor, soc, t as u32);
            let name = self.societies[soc as usize].name();
            let who = inventor.map(|i| self.notables[i as usize].name.clone());
            let text = match &who {
                Some(w) => format!("{w}, chez les {name}, invente : {}.", tech.name),
                None => format!("Les {name} inventent : {}.", tech.name),
            };
            let interest = 0.3 + 0.4 * (tech.difficulty / 70.0).min(1.0);
            self.event(EventKind::Invention, Some(soc), None, Some(c), Some(t), inventor, text, interest);
            if first_agri {
                let wild = self.cells[c as usize]
                    .settlement
                    .as_ref()
                    .and_then(|s| s.crop)
                    .and_then(|d| self.wild.iter().find(|w| w.id == d.species))
                    .map(|w| w.name.clone())
                    .unwrap_or_default();
                self.event(
                    EventKind::AgricultureHearth,
                    Some(soc),
                    None,
                    Some(c),
                    Some(t),
                    None,
                    format!("Un foyer agricole naît chez les {name} : {wild} domestiqué."),
                    0.9,
                );
            }
        }
    }

    /// Meilleure espèce sauvage à domestiquer dans une société : qualité
    /// et climat (0 à 1).
    fn opportunity(&self, soc: &Society, animal: bool) -> f64 {
        let mut best: f64 = 0.0;
        for &c in &soc.cells {
            let land = &self.cells[c as usize].land;
            for w in self.wild.iter().filter(|w| land.wild.contains(&w.id)) {
                let q = match (&w.kind, animal) {
                    (crate::land::WildKind::Animal(_), true) => w.tameness(),
                    (crate::land::WildKind::Plant(_), false) => w.cultivability(),
                    _ => continue,
                };
                best = best.max(q * w.suits(land.temp_c, land.rain_mm));
            }
            if !animal && self.species(soc.species).map(|sp| sp.milieu != Milieu::Land).unwrap_or(false) {
                best = best.max(0.6);
            }
        }
        best.min(1.0)
    }

    /// Population dense des partenaires d'échange.
    fn partners_dense(&self, soc: u32) -> f64 {
        self.adj
            .get(soc as usize)
            .map(|v| {
                v.iter()
                    .filter(|(_, r)| matches!(r.kind, RelationKind::Trade | RelationKind::Alliance | RelationKind::Tribute { .. }))
                    .map(|&(o, _)| {
                        self.societies
                            .get(o as usize)
                            .map(|s| s.agg.population * (s.agg.urban + 0.2 * f64::from(s.label.rank()) / 4.0))
                            .unwrap_or(0.0)
                    })
                    .sum()
            })
            .unwrap_or(0.0)
    }

    /// Perte de savoir-faire : une technique difficile ne se maintient pas
    /// dans trop peu de praticiens (Henrich, 2004).
    fn lose(&mut self) {
        let year = self.year;
        let seed = self.seed;
        let cat = catalog();
        let n = cat.len();
        let this = &*self;
        let lost: Vec<(usize, u16)> = this
            .societies
            .iter()
            .filter(|s| s.alive())
            .flat_map(|soc| {
                let mut lost: Vec<(usize, u16)> = Vec::new();
                let Some(sp) = this.species(soc.species) else { return lost };
                if !due(year, soc.id as u64) {
                    return lost;
                }
                let mut practitioners = vec![0.0f64; n];
                for &c in &soc.cells {
                    if let Some(s) = &this.cells[c as usize].settlement {
                        for t in s.known.iter() {
                            practitioners[t as usize] += s.pop;
                        }
                    }
                }
                let eff = cat.effects(&soc.known);
                let mut learning = sp.learning;
                learning.fidelity = (learning.fidelity + eff.fidelity).min(0.995);
                let mut r = rng(seed, year, 6, soc.id as u64);
                for t in soc.known.iter() {
                    let tech = cat.get(t);
                    // Les techniques savantes n'ont qu'un praticien sur cent.
                    let share = if tech.difficulty > 30.0 { 0.01 } else { 0.2 };
                    let gain = learning.skill_gain(tech.difficulty * 3.0, practitioners[t as usize] * share + 1.0);
                    if gain < 0.0 && r.random::<f64>() < over_slow(0.02) {
                        for &c in &soc.cells {
                            lost.push((c as usize, t));
                        }
                    }
                }
                lost
            })
            .collect();
        for (c, t) in lost {
            if let Some(s) = self.cells[c].settlement.as_mut() {
                if s.known.has(t) {
                    s.known.clear(t);
                    s.effects = cat.effects(&s.known);
                    self.counters.losses += 1;
                }
            }
        }
    }

    // ------------------------------------------------------------------ Sociétés
    fn update_societies(&mut self) {
        let year = self.year;
        let seed = self.seed;
        let cat = catalog();
        // Cellules de chaque société.
        for s in self.societies.iter_mut() {
            s.cells.clear();
        }
        for c in &self.cells {
            if let Some(s) = &c.settlement {
                if let Some(soc) = self.societies.get_mut(s.society as usize) {
                    soc.cells.push(c.id);
                }
            }
        }
        let ids: Vec<u32> = self.societies.iter().filter(|s| s.alive()).map(|s| s.id).collect();
        // Chaque société ne lit que ses propres cellules, que les autres
        // sociétés ne touchent pas pendant la boucle : agrégats d'avance.
        let this = &*self;
        let mut gathers: Vec<Option<Gather>> = ids.par_iter().map(|&id| Some(this.gather(&this.societies[id as usize]))).collect();
        for (k, id) in ids.into_iter().enumerate() {
            let i = id as usize;
            if self.societies[i].cells.is_empty() {
                self.societies[i].ended = Some(year);
                continue;
            }
            // Agrégats.
            let mut agg = self.societies[i].agg;
            let policy = self.policy(&self.societies[i]);
            let Some(Gather {
                known,
                eff,
                pop,
                gdp,
                surplus,
                public,
                food,
                urban,
                lit,
                modern,
                sedentary,
                storable,
                peoples,
                military,
                circumscribed,
                biggest,
            }) = gathers[k].take()
            else {
                continue;
            };
            if pop <= 0.0 {
                self.societies[i].ended = Some(year);
                continue;
            }
            let soc = &self.societies[i];
            let dims = soc.dims;
            let inst = soc.institutions;
            let in_crisis = soc.crisis > 0;
            let fpc = food / pop;
            let sedentary_share = sedentary / pop;
            let surplus_share = (surplus / (gdp.max(1.0))).clamp(0.0, 1.0);
            let storable_pc = (storable / pop).min(1.0);
            let threat = self.threat(id);
            let circ = circumscribed / pop;
            let dominant = peoples.values().cloned().fold(0.0, f64::max) / pop;
            let writing = known.intersects(&cat.flagged.writing);
            let market_known = known.intersects(&cat.flagged.market) && known.intersects(&cat.flagged.money);
            let school_known = known.intersects(&cat.flagged.school);
            let law = known.intersects(&cat.flagged.law);
            let sym = self.species(soc.species).map(|s| s.learning.symbolic).unwrap_or(0.0);
            let chief_skill = soc.chief.and_then(|c| self.notables.get(c as usize)).map(|n| n.skill).unwrap_or(0.5);

            // Dimensions.
            let size = (libm::log10(pop.max(1.0)) - 3.0).max(0.0);
            let reach = (1.0 + eff.reach * 0.3 + eff.admin).min(3.0);
            let cells_pen = (libm::log10(soc.cells.len() as f64) / reach).clamp(0.0, 1.5);
            let c_t = ((0.13 * size + 0.35 * storable_pc + 0.2 * threat + 0.25 * circ * sedentary_share + 0.25 * dims.admin
                - 0.25 * cells_pen)
                * (0.35 + 0.65 * sedentary_share))
                .clamp(0.0, 1.0);
            let i_t = (0.08 + 0.45 * storable_pc * sedentary_share + 0.2 * dims.centralisation + 0.1 * f64::from(u8::from(market_known))
                - 0.15 * dims.participation
                + 0.1 * (1.0 - dominant))
                .clamp(0.0, 1.0);
            let k_t = (0.5 * dims.centralisation + 0.3 * inst[Institution::Army as usize] + 0.2 * circ - 0.3 * dims.participation)
                .clamp(0.0, 1.0);
            let lit_share = lit / pop;
            let p_t = (0.35 * inst[Institution::Council as usize]
                + 0.35 * lit_share
                + 0.15 * f64::from(u8::from(market_known))
                + 0.35 * (1.0 - dims.centralisation)
                - 0.2 * dims.coercion)
                .clamp(0.0, 1.0);
            let a_t = (eff.admin * (0.6 + 0.4 * inst[Institution::School as usize])).clamp(0.0, 1.0);
            let coh_t = (0.45 * dominant + 0.3 * agg.legitimacy + 0.15 * inst[Institution::Temple as usize] + 0.15 - 0.25 * cells_pen)
                .clamp(0.0, 1.0);
            let rate = if in_crisis { 0.02 } else { 0.01 };
            let mut d = dims;
            let toward = |x: f64, t: f64, r: f64| x + (t - x) * r;
            d.centralisation = toward(d.centralisation, if in_crisis { c_t * 0.6 } else { c_t }, rate);
            d.inequality = toward(d.inequality, i_t, 0.01);
            d.coercion = toward(d.coercion, k_t, 0.01);
            d.participation = toward(d.participation, p_t, 0.01);
            d.admin = toward(d.admin, a_t, if a_t > d.admin { 0.01 } else { 0.03 });
            d.cohesion = toward(d.cohesion, if in_crisis { coh_t * 0.7 } else { coh_t }, 0.03);

            // Institutions.
            let mut it = inst;
            let targets = [
                (0.3 + 0.5 * d.participation) * f64::from(u8::from(pop > 400.0)),
                (d.centralisation - 0.2).max(0.0) * 1.3,
                if law || writing { d.admin.min(1.0) * d.centralisation.max(0.3) } else { 0.0 },
                ((0.5 * d.inequality + 0.4 * sym) * sedentary_share).min(1.0),
                ((threat + 0.3) * d.centralisation * 1.2).min(1.0) * f64::from(u8::from(d.centralisation > 0.3)),
                if market_known { (0.4 + 0.6 * (1.0 - d.coercion)).min(1.0) } else { 0.0 },
                if school_known {
                    (0.3 + surplus_share).min(1.0)
                } else if writing {
                    0.15
                } else {
                    0.0
                },
            ];
            for k in 0..INSTITUTIONS {
                let r = if in_crisis && targets[k] < it[k] { 0.08 } else { 0.03 };
                it[k] = toward(it[k], targets[k].clamp(0.0, 1.0), r);
            }

            // Élites, fisc et tension politique (Goldstone, Turchin).
            let e_target = (0.002 + 0.12 * d.inequality * sedentary_share * storable_pc.sqrt()).min(0.2);
            let elite_income = d.inequality * surplus / (agg.elites * pop).max(1.0);
            let mut elites = agg.elites;
            if in_crisis {
                elites *= 0.96;
            } else {
                elites += 0.03 * elites.max(0.001) * (elite_income / 4.0 - 1.0).clamp(-1.0, 1.0);
            }
            elites = elites.clamp(0.0005, 0.25).min(e_target * 2.5).max(e_target * 0.3);
            let revenue = surplus * policy.tax * (0.4 + 0.6 * d.admin) / (1.0 + 0.3 * cells_pen);
            let expenses = public + elites * pop * 0.5 * d.centralisation;
            let mut debt = agg.debt * 1.03 + expenses - revenue;
            debt = debt.max(0.0);
            let mmp = ((1.05 - fpc) / 0.3).clamp(0.0, 1.0) * (0.5 + urban / pop);
            let emp = (elites / e_target.max(1e-4) - 1.0).max(0.0);
            let sfd = (debt / (revenue + 1.0) / 10.0).min(2.0);
            let stress = (1.0 + 4.0 * mmp) * (0.2 + emp) * (1.0 + sfd) / agg.legitimacy.max(0.1);
            // Légitimité.
            let l_t = (0.25
                + 0.2 * it[Institution::Temple as usize]
                + 0.15 * it[Institution::Court as usize]
                + 0.15 * it[Institution::Council as usize]
                + 0.2 * (fpc - 0.95).clamp(-0.5, 0.25) * 2.0
                - 0.3 * (d.inequality - 0.45).max(0.0)
                + 0.15 * (chief_skill - 0.5)
                + 0.05 * f64::from(soc.victories.min(4))
                - 0.08 * f64::from(soc.defeats.min(4)))
            .clamp(0.05, 1.0);
            let legitimacy = toward(agg.legitimacy, if in_crisis { l_t * 0.6 } else { l_t }, 0.05);

            let modern_known = known.intersects(&cat.flagged.industry);
            let lbl = society::label(
                &d,
                &Aggregates { population: pop, cells: soc.cells.len() as u32, ..agg },
                sedentary_share > 0.5,
                peoples.len(),
                modern_known && modern / pop > 0.3,
            );

            agg = Aggregates {
                population: pop,
                elites,
                debt,
                legitimacy,
                surplus,
                techniques: known.count(),
                revenue,
                expenses,
                gdp,
                food_pc: fpc,
                military,
                stress,
                cells: soc.cells.len() as u32,
                urban: urban / pop,
                literacy: lit_share,
                modern_energy: modern / pop,
            };
            let old_label = soc.label;
            let crisis = soc.crisis;
            let before = soc.before_crisis;
            let capital_ok = soc.cells.contains(&soc.capital);
            {
                let s = &mut self.societies[i];
                s.agg = agg;
                s.dims = d;
                s.institutions = it;
                s.known = known;
                s.label = lbl;
                if !capital_ok || biggest.0 > 3.0 * self.cells[s.capital as usize].settlement.as_ref().map(|x| x.pop).unwrap_or(0.0) {
                    s.capital = biggest.1;
                }
                if lbl > s.peak {
                    s.peak = lbl;
                }
                s.victories = s.victories.saturating_sub(u16::from(year % 50 == 0));
                s.defeats = s.defeats.saturating_sub(u16::from(year % 50 == 0));
            }
            if lbl != old_label && lbl.rank() > old_label.rank() {
                let interest = 0.4 + 0.1 * f64::from(lbl.rank());
                let name = self.societies[i].name();
                self.event(
                    EventKind::Label,
                    Some(id),
                    None,
                    Some(self.societies[i].capital),
                    None,
                    None,
                    format!("Les {name} forment un(e) {}.", lbl.label()),
                    interest,
                );
            }
            // Crises.
            // Le flux de l'année ne se tire que s'il sert : mêmes nombres.
            let mut lazy: Option<SimRng> = None;
            let mut r = || lazy.get_or_insert_with(|| rng(seed, year, 7, id as u64)).random::<f64>();
            if crisis == 0 {
                let p = ((stress - 1.6) * 0.03).clamp(0.0, 0.08);
                if lbl.rank() >= 2 && r() < p {
                    let s = &mut self.societies[i];
                    s.crisis = 1;
                    s.before_crisis = Some((lbl, pop, s.cells.len() as u32));
                    s.agg.debt = 0.0;
                    self.counters.crises += 1;
                    let name = self.societies[i].name();
                    self.event(
                        EventKind::CrisisStart,
                        Some(id),
                        None,
                        None,
                        None,
                        None,
                        format!("Crise chez les {name} : famines, élites rivales, trésor vide."),
                        0.5,
                    );
                }
            } else {
                let s = &mut self.societies[i];
                s.crisis = s.crisis.saturating_add(1);
                // Guerre civile : morts et fuite.
                for &c in &s.cells.clone() {
                    if let Some(st) = self.cells[c as usize].settlement.as_mut() {
                        st.deaths_war += st.pop * 0.004;
                        st.pop *= 0.996;
                    }
                }
                let s = &self.societies[i];
                if s.crisis > 15 && (stress < 1.0 || s.crisis > 80) {
                    let (blabel, bpop, bcells) = before.unwrap_or((lbl, pop, s.cells.len() as u32));
                    // Effondrement (Tainter, 1988) : un État perd son rang,
                    // une grande part de ses gens ou de ses terres. Les
                    // chefferies, elles, se font et se défont sans cesse
                    // (cycles d'Anderson, 1994) : ce n'est pas un
                    // effondrement.
                    let lost = pop < 0.6 * bpop || (s.cells.len() as u32) * 3 < bcells * 2;
                    let collapsed = blabel.rank() >= 3 && (lbl.rank() < blabel.rank() || lost);
                    let s = &mut self.societies[i];
                    s.crisis = 0;
                    s.before_crisis = None;
                    if collapsed {
                        self.counters.collapses += 1;
                        let name = self.societies[i].name();
                        self.event(
                            EventKind::Collapse,
                            Some(id),
                            None,
                            None,
                            None,
                            None,
                            format!(
                                "Effondrement des {name} : de {} à {}, {:.0} % de la population perdue.",
                                blabel.label(),
                                lbl.label(),
                                100.0 * (1.0 - pop / bpop.max(1.0)).max(0.0)
                            ),
                            0.85,
                        );
                    } else {
                        let name = self.societies[i].name();
                        self.event(EventKind::CrisisEnd, Some(id), None, None, None, None, format!("Les {name} sortent de la crise."), 0.3);
                    }
                }
            }
            // Scission.
            let s = &self.societies[i];
            let over = s.cells.len() > max_cells(s) + 1;
            if (s.dims.cohesion < 0.32 || over) && s.cells.len() >= 2 && r() < if over { 0.02 } else { 0.05 } {
                self.split(id);
            }
        }
        // Assimilation : les peuples conquis prennent peu à peu la culture du
        // peuple dominant de leur société (même espèce seulement).
        let mut r = rng(seed, year, 8, 0);
        for c in 0..self.cells.len() {
            let Some(s) = &self.cells[c].settlement else { continue };
            let Some(soc) = self.societies.get(s.society as usize) else { continue };
            if s.people != soc.people && s.species == soc.species && r.random::<f64>() < 0.002 * soc.dims.cohesion * (0.5 + soc.dims.admin)
            {
                let p = soc.people;
                if let Some(s) = self.cells[c].settlement.as_mut() {
                    s.people = p;
                }
            }
        }
    }

    /// Agrégats d'une société sur ses cellules (lecture seule, en parallèle).
    fn gather(&self, soc: &Society) -> Gather {
        let mut known = Known::default();
        let (mut pop, mut gdp, mut surplus, mut public, mut food, mut urban, mut lit, mut modern, mut sedentary, mut storable) =
            (0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
        let mut peoples: BTreeMap<u32, f64> = BTreeMap::new();
        let mut military = 0.0;
        let policy = self.policy(soc);
        let mut circumscribed = 0.0;
        let mut biggest = (0.0, soc.capital);
        for &c in &soc.cells {
            let cell = &self.cells[c as usize];
            let Some(s) = &cell.settlement else { continue };
            known = known.or(&s.known);
            pop += s.pop;
            *peoples.entry(s.people).or_default() += s.pop;
            urban += s.pop * s.urban;
            lit += s.pop * s.literacy;
            modern += s.pop * s.modern;
            if s.sedentary {
                sedentary += s.pop;
            }
            if s.pop > biggest.0 {
                biggest = (s.pop, c);
            }
            if let Some(y) = &cell.year {
                gdp += y.gdp;
                surplus += y.surplus;
                public += y.public;
                food += y.food_pc * s.pop;
                storable += cell.eco.stocks[Good::Grain.index()] + cell.eco.stocks[Good::Roots.index()];
            }
            military += s.pop * economy::WORK_SHARE * policy.army.max(0.02) * s.effects.military * (0.5 + soc.dims.centralisation);
            // Circonscription : voisins pauvres (désert, mer, montagne).
            let poor = cell
                .links
                .iter()
                .filter(|l| {
                    let o = &self.cells[l.to as usize];
                    o.land.ocean || o.land.npp() < 0.15 || l.cost >= 2.0
                })
                .count();
            circumscribed += s.pop * poor as f64 / cell.links.len().max(1) as f64;
        }
        let eff = catalog().effects(&known);
        Gather {
            eff,
            known,
            pop,
            gdp,
            surplus,
            public,
            food,
            urban,
            lit,
            modern,
            sedentary,
            storable,
            peoples,
            military,
            circumscribed,
            biggest,
        }
    }

    /// Menace extérieure : guerres en cours et voisins plus forts.
    fn threat(&self, soc: u32) -> f64 {
        let me = &self.societies[soc as usize];
        let mut t: f64 = 0.0;
        for &(o, r) in self.adj.get(soc as usize).map(|v| v.as_slice()).unwrap_or(&[]) {
            let Some(os) = self.societies.get(o as usize) else { continue };
            let ratio = os.agg.military / me.agg.military.max(1.0);
            t += match r.kind {
                RelationKind::War { .. } => 0.6 * os.dims.centralisation.min(0.32) / 0.32,
                _ => 0.15 * ratio.min(2.0),
            };
        }
        t.min(1.0)
    }

    /// Une société se scinde : les cellules les plus éloignées de la
    /// capitale, ou d'un autre peuple, partent ensemble.
    fn split(&mut self, id: u32) {
        let year = self.year;
        let soc = self.societies[id as usize].clone();
        let capital = soc.capital;
        // Distance en pas depuis la capitale, par les liens internes.
        let mut dist: BTreeMap<u32, u32> = BTreeMap::new();
        dist.insert(capital, 0);
        let mut frontier = vec![capital];
        while let Some(c) = frontier.pop() {
            let d = dist[&c];
            for l in &self.cells[c as usize].links {
                if soc.cells.contains(&l.to) && !dist.contains_key(&l.to) {
                    dist.insert(l.to, d + 1);
                    frontier.insert(0, l.to);
                }
            }
        }
        let mut leaving: Vec<u32> = soc
            .cells
            .iter()
            .copied()
            .filter(|c| {
                let other_people = self.cells[*c as usize].settlement.as_ref().map(|s| s.people != soc.people).unwrap_or(false);
                let far = dist.get(c).map(|&d| d >= 2).unwrap_or(true);
                *c != capital && (other_people || far)
            })
            .collect();
        if leaving.is_empty() {
            // À défaut, la moitié la plus lointaine.
            let mut v: Vec<(u32, u32)> =
                soc.cells.iter().filter(|&&c| c != capital).map(|&c| (dist.get(&c).copied().unwrap_or(99), c)).collect();
            v.sort();
            leaving = v.iter().rev().take(v.len().div_ceil(2)).map(|x| x.1).collect();
        }
        if leaving.is_empty() {
            return;
        }
        leaving.sort();
        let seat = leaving[0];
        let people = self.cells[seat as usize].settlement.as_ref().map(|s| s.people).unwrap_or(soc.people);
        let new = self.new_society(soc.species, people, seat, Some(id), Some(soc.phonology.clone()));
        for &c in &leaving {
            if let Some(s) = self.cells[c as usize].settlement.as_mut() {
                s.society = new;
            }
        }
        {
            let n = &mut self.societies[new as usize];
            n.cells = leaving.clone();
            n.known = soc.known;
            n.dims.cohesion = 0.6;
            n.dims.centralisation = soc.dims.centralisation * 0.7;
            n.agg.legitimacy = 0.6;
            n.agg.elites = soc.agg.elites;
        }
        let s = &mut self.societies[id as usize];
        s.cells.retain(|c| !leaving.contains(c));
        s.dims.cohesion = (s.dims.cohesion + 0.2).min(1.0);
        self.counters.scissions += 1;
        let (a, b) = (self.societies[id as usize].name(), self.societies[new as usize].name());
        self.event(EventKind::Scission, Some(id), Some(new), Some(seat), None, None, format!("Les {b} font sécession des {a}."), 0.55);
        self.relations.insert(key(id, new), Relation { kind: RelationKind::Peace, since: year, weariness: 0.0 });
    }

    // ------------------------------------------------------------------ Guerre et diplomatie
    /// Paires de sociétés voisines, triées, sans doublon.
    fn neighbours(&self) -> Vec<(u32, u32)> {
        let mut out = Vec::new();
        for c in &self.cells {
            let Some(s) = &c.settlement else { continue };
            for l in &c.links {
                if let Some(o) = &self.cells[l.to as usize].settlement {
                    if o.society != s.society && (!l.sea || self.can_cross(s, l) || self.can_cross(o, l)) {
                        out.push(key(s.society, o.society));
                    }
                }
            }
        }
        out.sort_unstable();
        out.dedup();
        out
    }

    fn diplomacy(&mut self) {
        let year = self.year;
        let seed = self.seed;
        let pairs = self.neighbours();
        // Les relations entre sociétés qui ne se touchent plus s'éteignent.
        let gone: Vec<(u32, u32)> = self
            .relations
            .keys()
            .copied()
            .filter(|k| pairs.binary_search(k).is_err() || !self.societies[k.0 as usize].alive() || !self.societies[k.1 as usize].alive())
            .collect();
        for k in gone {
            self.relations.remove(&k);
        }
        // Qui est en guerre avec qui, tenu à jour pendant la boucle : un
        // ennemi commun se lit sans parcourir toutes les relations.
        let mut war_with: Vec<Vec<u32>> = vec![Vec::new(); self.societies.len()];
        for (k, r) in &self.relations {
            if matches!(r.kind, RelationKind::War { .. }) {
                war_with[k.0 as usize].push(k.1);
                war_with[k.1 as usize].push(k.0);
            }
        }
        let is_war = |r: Option<&Relation>| r.is_some_and(|r| matches!(r.kind, RelationKind::War { .. }));
        for (a, b) in pairs {
            let (sa, sb) = (&self.societies[a as usize], &self.societies[b as usize]);
            if !sa.alive() || !sb.alive() {
                continue;
            }
            if sa.species != sb.species {
                let k = key(sa.species, sb.species);
                if self.contacts.insert(k) {
                    let (na, nb) = (
                        self.species(sa.species).map(|s| s.name.clone()).unwrap_or_default(),
                        self.species(sb.species).map(|s| s.name.clone()).unwrap_or_default(),
                    );
                    let txt = format!("Premier contact : les {na} ({}) rencontrent les {nb} ({}).", sa.name(), sb.name());
                    self.event(EventKind::FirstContact, Some(a), Some(b), Some(sa.capital), None, None, txt, 1.0);
                }
            }
            let was_war = is_war(self.relations.get(&(a, b)));
            'pair: {
                let rel =
                    self.relations.get(&(a, b)).copied().unwrap_or(Relation { kind: RelationKind::Peace, since: year, weariness: 0.0 });
                // La guerre se mène chaque année ; la paix se décide à son tour.
                if !matches!(rel.kind, RelationKind::War { .. }) && !due(year, a as u64 * 7 + b as u64) {
                    break 'pair;
                }
                let mut r = rng(seed, year, 9, ((a as u64) << 32) | b as u64);
                let (sa, sb) = (&self.societies[a as usize], &self.societies[b as usize]);
                let (pa, pb) = (sa.agg.military.max(1.0), sb.agg.military.max(1.0));
                let species_gap = sa.species != sb.species;
                let compat = self.compatible(sa.species, sb.species);
                match rel.kind {
                    RelationKind::War { attacker } => {
                        let defender = if attacker == a { b } else { a };
                        self.battle(attacker, defender, &mut r);
                        let mut rel = rel;
                        rel.weariness += 0.02 + 0.05 * f64::from(u8::from(self.societies[attacker as usize].crisis > 0));
                        let still = self.societies[defender as usize].alive() && self.societies[attacker as usize].alive();
                        if !still {
                            self.relations.remove(&(a, b));
                        } else if r.random::<f64>() < rel.weariness * 0.1 {
                            // Paix : le vaincu paie tribut si l'écart est grand.
                            let (pa2, pd2) =
                                (self.societies[attacker as usize].agg.military, self.societies[defender as usize].agg.military);
                            let kind = if pa2 > 2.0 * pd2 && self.societies[attacker as usize].dims.centralisation > 0.4 {
                                RelationKind::Tribute { payer: defender }
                            } else {
                                RelationKind::Peace
                            };
                            self.relations.insert((a, b), Relation { kind, since: year, weariness: 0.0 });
                            let (na, nd) = (self.societies[attacker as usize].name(), self.societies[defender as usize].name());
                            if rel.since < year - 1 && self.societies[attacker as usize].dims.centralisation > 0.3 {
                                self.event(
                                    EventKind::WarEnd,
                                    Some(attacker),
                                    Some(defender),
                                    None,
                                    None,
                                    None,
                                    format!("Paix entre les {na} et les {nd}."),
                                    0.4,
                                );
                            }
                            if matches!(kind, RelationKind::Tribute { .. }) {
                                self.event(
                                    EventKind::Tribute,
                                    Some(defender),
                                    Some(attacker),
                                    None,
                                    None,
                                    None,
                                    format!("Les {nd} paient tribut aux {na}."),
                                    0.4,
                                );
                            }
                        } else {
                            self.relations.insert((a, b), rel);
                        }
                    }
                    _ => {
                        // Guerre ? Dans les deux sens.
                        let mut declared = false;
                        for (att, def, patt, pdef) in [(a, b, pa, pb), (b, a, pb, pa)] {
                            let s = &self.societies[att as usize];
                            let d = &self.societies[def as usize];
                            let ambition = s.chief.and_then(|c| self.notables.get(c as usize)).map(|n| n.ambition).unwrap_or(0.3);
                            let hunger = ((1.0 - s.agg.food_pc) * 3.0).clamp(0.0, 1.0);
                            let elites = (s.agg.stress / 2.0).min(1.0);
                            let distance = if s.people != d.people { 1.4 } else { 0.8 } * if species_gap { 1.5 } else { 1.0 };
                            let peaceful = match rel.kind {
                                RelationKind::Alliance => 0.1,
                                RelationKind::Trade => 0.4,
                                RelationKind::Tribute { payer } if payer == def => 0.3,
                                _ => 1.0,
                            };
                            let organised = if s.dims.centralisation > 0.3 { 1.0 } else { 0.03 };
                            let ratio = (patt / pdef / d.dims.cohesion.max(0.3)).min(4.0);
                            let p =
                                0.004 * ratio * ratio * (1.0 + 3.0 * ambition) * (1.0 + hunger + elites) * distance * peaceful * organised;
                            if r.random::<f64>() < over_slow(p.min(0.3)) {
                                self.relations
                                    .insert((a, b), Relation { kind: RelationKind::War { attacker: att }, since: year, weariness: 0.0 });
                                // Les razzias entre bandes ne font pas la chronique.
                                if organised == 1.0 {
                                    self.counters.wars += 1;
                                    let (na, nd) = (s.name(), d.name());
                                    self.event(
                                        EventKind::WarStart,
                                        Some(att),
                                        Some(def),
                                        Some(d.capital),
                                        None,
                                        s.chief,
                                        format!("Les {na} entrent en guerre contre les {nd}."),
                                        0.5,
                                    );
                                } else {
                                    self.counters.raids += 1;
                                }
                                declared = true;
                                break;
                            }
                        }
                        if declared {
                            break 'pair;
                        }
                        // Commerce et alliances.
                        let (sa, sb) = (&self.societies[a as usize], &self.societies[b as usize]);
                        let market = sa.institution(Institution::Market).max(sb.institution(Institution::Market));
                        match rel.kind {
                            RelationKind::Peace => {
                                let p = (0.002 + 0.03 * market + 0.01 * f64::from(u8::from(sa.agg.surplus > 1e4 && sb.agg.surplus > 1e4)))
                                    * compat;
                                if r.random::<f64>() < over_slow(p) {
                                    self.relations.insert((a, b), Relation { kind: RelationKind::Trade, since: year, weariness: 0.0 });
                                    let txt = format!("Les {} et les {} commercent.", sa.name(), sb.name());
                                    self.event(
                                        EventKind::Trade,
                                        Some(a),
                                        Some(b),
                                        None,
                                        None,
                                        None,
                                        txt,
                                        if species_gap { 0.8 } else { 0.2 },
                                    );
                                }
                            }
                            RelationKind::Trade => {
                                // Un ennemi commun fait une alliance.
                                let common = war_with[a as usize].iter().any(|e| war_with[b as usize].contains(e));
                                if common && r.random::<f64>() < over_slow(0.1 * compat.max(0.3)) {
                                    self.relations.insert((a, b), Relation { kind: RelationKind::Alliance, since: year, weariness: 0.0 });
                                    let txt = format!("Alliance des {} et des {}.", sa.name(), sb.name());
                                    self.event(EventKind::Alliance, Some(a), Some(b), None, None, None, txt, 0.4);
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            let now_war = is_war(self.relations.get(&(a, b)));
            if was_war && !now_war {
                war_with[a as usize].retain(|&e| e != b);
                war_with[b as usize].retain(|&e| e != a);
            } else if now_war && !was_war {
                war_with[a as usize].push(b);
                war_with[b as usize].push(a);
            }
        }
    }

    /// Une année de guerre : combats sur les cellules de contact.
    fn battle(&mut self, att: u32, def: u32, r: &mut SimRng) {
        let year = self.year;
        let fronts: Vec<(u32, u32)> = self.societies[def as usize]
            .cells
            .iter()
            .flat_map(|&c| {
                self.cells[c as usize]
                    .links
                    .iter()
                    .filter(|l| self.cells[l.to as usize].settlement.as_ref().map(|s| s.society == att).unwrap_or(false))
                    .map(move |l| (c, l.to))
            })
            .collect();
        let Some(&(target, _from)) = fronts.first() else { return };
        let (pa, pd) = (self.societies[att as usize].agg.military, self.societies[def as usize].agg.military);
        let defence = self.cells[target as usize].settlement.as_ref().map(|s| s.effects.defence).unwrap_or(1.0);
        let p = pa / (pa + pd * defence * 1.3).max(1.0);
        let win = r.random::<f64>() < p;
        let casualties = |w: &mut Self, c: u32, f: f64| {
            if let Some(s) = w.cells[c as usize].settlement.as_mut() {
                let d = s.pop * f;
                s.pop -= d;
                s.deaths_war += d;
            }
        };
        casualties(self, target, if win { 0.03 } else { 0.01 });
        if !win {
            self.societies[def as usize].victories += 1;
            return;
        }
        // Bandes et tribus razzient sans annexer : il faut un pouvoir
        // central pour tenir une terre conquise, et une administration pour
        // en tenir beaucoup.
        if !room(&self.societies[att as usize]) {
            casualties(self, target, 0.01);
            self.societies[att as usize].victories += 1;
            return;
        }
        // Le vainqueur prend la cellule.
        let a_species = self.societies[att as usize].species;
        if let Some(s) = self.cells[target as usize].settlement.as_mut() {
            s.society = att;
            if s.species != a_species {
                // Pas de métissage : les vaincus restent eux-mêmes, sujets.
                s.pop *= 0.7;
            }
        }
        self.societies[def as usize].cells.retain(|&c| c != target);
        self.societies[att as usize].cells.push(target);
        self.societies[att as usize].victories += 1;
        self.societies[def as usize].defeats += 1;
        self.counters.conquests += 1;
        let gone = self.societies[def as usize].cells.is_empty();
        let (na, nd) = (self.societies[att as usize].name(), self.societies[def as usize].name());
        if gone {
            self.societies[def as usize].ended = Some(year);
            let conqueror = self.societies[att as usize].chief;
            if let Some(c) = conqueror {
                if let Some(n) = self.notables.get_mut(c as usize) {
                    if n.kind == NotableKind::Chief {
                        n.kind = NotableKind::Conqueror;
                    }
                    n.deeds.push(format!("conquiert les {nd}"));
                }
            }
            self.event(
                EventKind::Conquest,
                Some(att),
                Some(def),
                Some(target),
                None,
                conqueror,
                format!("Les {na} conquièrent les {nd}."),
                0.7,
            );
        }
    }

    // ------------------------------------------------------------------ Sciences
    fn sciences(&mut self) {
        let year = self.year;
        let seed = self.seed;
        let cat = catalog();
        // Les savants vivants.
        let mut savants: BTreeMap<u32, Vec<usize>> = BTreeMap::new();
        for n in &self.notables {
            if n.kind == NotableKind::Savant && n.died.is_none() {
                savants.entry(n.society).or_default().push(n.subject as usize);
            }
        }
        // Chaque société avance ses modèles de son propre flux : calcul en
        // parallèle, application dans l'ordre.
        let this = &*self;
        let plans: Vec<(u32, [Model; DOMAINS], Vec<Domain>, bool)> = this
            .societies
            .iter()
            .filter(|s| s.alive())
            .filter_map(|soc| {
                let id = soc.id;
                let literates = soc.agg.population * soc.agg.literacy;
                if literates < 1.0 && soc.agg.population < 1000.0 || !due(year, id as u64) {
                    return None;
                }
                let eff = cat.effects(&soc.known);
                let method = soc.known.intersects(&cat.flagged.method);
                let freedom = (1.0 - 0.6 * soc.dims.coercion * (1.0 - soc.dims.participation)).clamp(0.2, 1.0);
                let funding = (0.3 + soc.institution(Institution::School) + 0.3 * soc.institution(Institution::Temple)).min(1.5);
                let base = 2.5e-6
                    * (literates + 0.001 * soc.agg.population).sqrt()
                    * eff.science
                    * freedom
                    * funding
                    * if method { 3.0 } else { PRE_METHOD_SCIENCE };
                let temple = soc.institution(Institution::Temple);
                let mut r = rng(seed, year, 10, id as u64);
                let mut shifts = Vec::new();
                let mut conflict = false;
                let mut sciences = soc.sciences;
                for d in Domain::ALL {
                    let k = d.index();
                    let cap = if cat.instrument_of[k].intersects(&soc.known) { 1.0 } else { d.naked_eye() };
                    let boost = if savants.get(&id).map(|v| v.contains(&k)).unwrap_or(false) { 2.0 } else { 1.0 };
                    let m = &mut sciences[k];
                    let room = (cap - m.precision).max(0.0);
                    m.anomalies += SLOW_EVERY as f64 * base * boost * room * r.random_range(0.5..1.5);
                    // Le modèle dominant résiste aux anomalies, d'autant plus
                    // que le temple est fort ; puis il bascule.
                    let resistance = 0.01 + 0.03 * temple * (1.0 - m.precision);
                    if m.anomalies > resistance {
                        let before = m.precision;
                        m.precision = (m.precision + m.anomalies).min(cap);
                        m.anomalies = 0.0;
                        m.paradigm += 1;
                        if before < 0.5 && m.precision >= 0.5 {
                            shifts.push(d);
                            if matches!(d, Domain::Biology | Domain::Astronomy | Domain::Geology) && temple > 0.4 {
                                conflict = true;
                            }
                        }
                    }
                }
                Some((id, sciences, shifts, conflict))
            })
            .collect();
        for (id, sciences, shifts, conflict) in plans {
            let s = &mut self.societies[id as usize];
            s.sciences = sciences;
            for d in shifts {
                let name = self.societies[id as usize].name();
                self.event(
                    EventKind::Paradigm,
                    Some(id),
                    None,
                    None,
                    None,
                    None,
                    format!("Les savants des {name} renouvellent leur {} : le modèle ancien cède.", d.label()),
                    0.5,
                );
            }
            if conflict {
                let s = &mut self.societies[id as usize];
                s.agg.legitimacy = (s.agg.legitimacy - 0.08).max(0.05);
                s.dims.cohesion = (s.dims.cohesion - 0.05).max(0.0);
                s.institutions[Institution::Temple as usize] *= 0.9;
                let name = self.societies[id as usize].name();
                self.event(
                    EventKind::ScienceConflict,
                    Some(id),
                    None,
                    None,
                    None,
                    None,
                    format!("Chez les {name}, la science heurte les croyances du temple."),
                    0.6,
                );
            }
        }
        // Les partenaires d'échange rattrapent le meilleur modèle.
        let pairs: Vec<(u32, u32)> = self
            .relations
            .iter()
            .filter(|(_, r)| matches!(r.kind, RelationKind::Trade | RelationKind::Alliance))
            .map(|(k, _)| *k)
            .collect();
        for (a, b) in pairs {
            for k in 0..DOMAINS {
                let (ma, mb) = (self.societies[a as usize].sciences[k].precision, self.societies[b as usize].sciences[k].precision);
                let best = ma.max(mb);
                for x in [a, b] {
                    let m = &mut self.societies[x as usize].sciences[k];
                    m.precision += (best - m.precision) * 0.01;
                }
            }
        }
    }

    // ------------------------------------------------------------------ Notables
    fn alive_notables(&self) -> usize {
        self.notables.iter().filter(|n| n.died.is_none()).count()
    }

    fn draw_notable(&mut self, kind: NotableKind, society: u32, subject: u32) -> Option<u32> {
        if self.alive_notables() >= MAX_NOTABLES {
            return None;
        }
        let year = self.year;
        let id = self.notables.len() as u32;
        let mut r = rng(self.seed, year, 11, id as u64);
        let soc = &self.societies[society as usize];
        let name = format!("{} {}", soc.phonology.name(r.random()), soc.phonology.name(r.random()));
        let life = r.random_range(15..45);
        self.notables.push(Notable {
            id,
            kind,
            society,
            species: soc.species,
            name,
            born: year - r.random_range(18..40),
            died: None,
            until: year + life,
            ambition: r.random(),
            skill: r.random(),
            subject,
            deeds: Vec::new(),
        });
        Some(id)
    }

    fn notables(&mut self) {
        let year = self.year;
        let seed = self.seed;
        // Morts.
        for n in self.notables.iter_mut() {
            if n.died.is_none() && year >= n.until {
                n.died = Some(year);
            }
        }
        for s in self.societies.iter_mut() {
            if let Some(c) = s.chief {
                if self.notables.get(c as usize).map(|n| n.died.is_some()).unwrap_or(true) || !s.alive() {
                    s.chief = None;
                    // Une succession sans règle écrite ébranle le pouvoir.
                    if s.institution(Institution::Court) < 0.3 {
                        s.agg.legitimacy = (s.agg.legitimacy - 0.05).max(0.05);
                    }
                }
            }
        }
        // Les morts anciens ne sont plus gardés au-delà de 2 000.
        let ids: Vec<u32> = self.societies.iter().filter(|s| s.alive()).map(|s| s.id).collect();
        for id in ids {
            let s = &self.societies[id as usize];
            let mut r = rng(seed, year, 12, id as u64);
            // Chef.
            if s.chief.is_none() && s.institution(Institution::Ruler) > 0.25 {
                let n = self.draw_notable(NotableKind::Chief, id, 0);
                self.societies[id as usize].chief = n;
            }
            let s = &self.societies[id as usize];
            // Prophète : dans la crise, chez une espèce qui pense en symboles.
            let sym = self.species(s.species).map(|x| x.learning.symbolic).unwrap_or(0.0);
            if s.crisis > 0 && sym > 0.5 && r.random::<f64>() < 0.01 {
                self.religions += 1;
                let rel = self.religions;
                if let Some(n) = self.draw_notable(NotableKind::Prophet, id, rel) {
                    let name = self.notables[n as usize].name.clone();
                    let sname = self.societies[id as usize].name();
                    let s = &mut self.societies[id as usize];
                    s.religion = rel;
                    s.dims.cohesion = (s.dims.cohesion + 0.15).min(1.0);
                    s.agg.legitimacy = (s.agg.legitimacy + 0.1).min(1.0);
                    self.event(
                        EventKind::NewReligion,
                        Some(id),
                        None,
                        None,
                        None,
                        Some(n),
                        format!("{name} prêche une foi nouvelle chez les {sname}."),
                        0.6,
                    );
                }
            }
            // Savant.
            let s = &self.societies[id as usize];
            let literates = s.agg.population * s.agg.literacy;
            if s.institution(Institution::School) > 0.2 && r.random::<f64>() < (literates / 2e6).min(0.2) {
                let domain = r.random_range(0..DOMAINS) as u32;
                self.draw_notable(NotableKind::Savant, id, domain);
            }
        }
        // Une religion gagne les voisins pacifiques à faible cohésion.
        let pairs: Vec<(u32, u32)> =
            self.relations.iter().filter(|(_, r)| !matches!(r.kind, RelationKind::War { .. })).map(|(k, _)| *k).collect();
        let mut r = rng(seed, year, 13, 0);
        for (a, b) in pairs {
            let (ra, rb) = (self.societies[a as usize].religion, self.societies[b as usize].religion);
            if ra == rb {
                continue;
            }
            for (from, to, rel) in [(a, b, ra), (b, a, rb)] {
                if rel != 0
                    && self.societies[to as usize].dims.cohesion < 0.5
                    && self.societies[from as usize].dims.cohesion > 0.6
                    && r.random::<f64>() < 0.003
                {
                    let s = &mut self.societies[to as usize];
                    s.religion = rel;
                    s.dims.cohesion = (s.dims.cohesion + 0.1).min(1.0);
                }
            }
        }
        // Mémoire bornée : on garde les vivants et les 2 000 derniers morts.
        let dead: Vec<usize> = self.notables.iter().enumerate().filter(|(_, n)| n.died.is_some()).map(|(i, _)| i).collect();
        if dead.len() > 4_000 {
            for &i in &dead[..dead.len() - 2_000] {
                let n = &mut self.notables[i];
                n.deeds.clear();
                n.deeds.shrink_to_fit();
            }
        }
    }

    // ------------------------------------------------------------------ Villes
    fn cities(&mut self) {
        let year = self.year;
        for c in 0..self.cells.len() {
            let Some(s) = &self.cells[c].settlement else { continue };
            let urban = s.pop * s.urban;
            let (society, city) = (s.society, s.city);
            match city {
                Some(id) => {
                    let ct = &mut self.cities[id as usize];
                    ct.pop = urban;
                    ct.society = society;
                    ct.peak = ct.peak.max(urban);
                    if urban < CITY_ABANDON {
                        ct.abandoned = Some(year);
                        if let Some(s) = self.cells[c].settlement.as_mut() {
                            s.city = None;
                        }
                    }
                }
                None if urban >= CITY_POP => {
                    let id = self.cities.len() as u32;
                    let soc = &self.societies[society as usize];
                    let name = soc.phonology.name(0x7a11_0000 ^ (c as u32).wrapping_mul(2_654_435_761));
                    let sname = soc.name();
                    self.cities.push(City {
                        id,
                        cell: c as u32,
                        name: name.clone(),
                        founded: year,
                        abandoned: None,
                        society,
                        pop: urban,
                        peak: urban,
                    });
                    if let Some(s) = self.cells[c].settlement.as_mut() {
                        s.city = Some(id);
                    }
                    self.event(
                        EventKind::CityFounded,
                        Some(society),
                        None,
                        Some(c as u32),
                        None,
                        None,
                        format!("{name}, ville des {sname}, passe {:.0} habitants.", CITY_POP),
                        0.45,
                    );
                }
                None => {}
            }
        }
        // Une cellule vidée perd sa ville.
        for ct in self.cities.iter_mut().filter(|ct| ct.abandoned.is_none()) {
            if self.cells[ct.cell as usize].settlement.as_ref().and_then(|s| s.city) != Some(ct.id) {
                ct.abandoned = Some(year);
            }
        }
    }

    // ------------------------------------------------------------------ Jalons
    /// Forçage radiatif des sociétés, W/m² : CO₂ (Myhre et al., 1998),
    /// méthane, aérosols soufrés.
    pub fn forcing_w_m2(&self) -> f64 {
        let t = &self.totals;
        // 2,13 milliards de t C par ppm ; 45 % des émissions restent dans
        // l'air (le reste va aux océans et à la végétation).
        let ppm = self.co2_ppm0 + 0.45 * (t.cum_co2_c + t.cum_ch4_c) / 2.13e9;
        let co2 = 5.35 * (ppm / self.co2_ppm0).dln();
        // 2,78 Tg de méthane par ppb ; forçage ≈ 0,036 (√M − √M₀).
        let m0 = 700.0;
        let m = m0 + t.ch4_burden_c / 0.75 / 2.78e6;
        let ch4 = 0.036 * (m.sqrt() - m0.sqrt());
        let aerosol = -0.5 * t.year.aerosol_t / 1e8;
        co2 + ch4 + aerosol
    }

    /// Le forçage par source, W/m² : CO₂ fossile, CO₂ des terres (feu,
    /// défrichement, sols), méthane, aérosols. Le CO₂ fossile est ce que le
    /// seul carbone fossile brûlé aurait donné ; les terres, le reste.
    pub fn forcing_parts(&self) -> ForcingParts {
        let t = &self.totals;
        let ppm = self.co2_ppm0 + 0.45 * (t.cum_co2_c + t.cum_ch4_c) / 2.13e9;
        let co2 = 5.35 * (ppm / self.co2_ppm0).dln();
        // Le fossile seul, et le reste du CO₂ pour les terres.
        let fossil = 5.35 * ((self.co2_ppm0 + 0.45 * t.cum_fossil_c / 2.13e9) / self.co2_ppm0).dln();
        let m0 = 700.0;
        let m = m0 + t.ch4_burden_c / 0.75 / 2.78e6;
        ForcingParts {
            fossil_co2: fossil,
            land_co2: co2 - fossil,
            methane: 0.036 * (m.sqrt() - m0.sqrt()),
            aerosol: -0.5 * t.year.aerosol_t / 1e8,
        }
    }

    /// Pas du climat demandé à Planète : 10 ans dès que le forçage humain
    /// dépasse 0,1 W/m², 100 sinon (règle de Vision).
    pub fn climate_step_years(&self) -> u32 {
        if self.forcing_w_m2() > 0.1 {
            10
        } else {
            100
        }
    }

    /// Bilan de carbone : stocks lents + émissions cumulées − carbonates,
    /// comparé au départ. Vaut 0 à l'arrondi près.
    pub fn carbon_balance(&self) -> f64 {
        let now: f64 = self.cells.iter().map(|c| economy::carbon_stock(&c.eco)).sum();
        now + self.totals.cum_co2_c + self.totals.cum_ch4_c - self.totals.cum_carbonate_c - self.carbon_initial
    }

    fn check_milestones(&mut self) {
        let year = self.year;
        let cat = catalog();
        let species: Vec<u32> = self.species.iter().map(|s| s.id).collect();
        for sp in species {
            let mut pop = 0.0;
            let mut networked = 0.0;
            for c in &self.cells {
                if let Some(s) = &c.settlement {
                    if s.species != sp {
                        continue;
                    }
                    pop += s.pop;
                    if s.known.intersects(&cat.flagged.network) {
                        networked += s.pop;
                    }
                }
            }
            if pop <= 0.0 {
                continue;
            }
            let name = self.species(sp).map(|s| s.name.clone()).unwrap_or_default();
            // La sortie industrielle : une première société d'un million
            // d'habitants au moins tire de ses moteurs à charbon ou à pétrole
            // plus d'un sixième de son travail (l'Angleterre vers 1800,
            // selon Wrigley, 2010) ; l'eau et le vent n'y suffisent pas, ni
            // les pompes de Newcomen sans la machine de Watt.
            let fossil = |s: &Society| {
                let f: f64 = s
                    .cells
                    .iter()
                    .filter_map(|&c| {
                        let c = &self.cells[c as usize];
                        Some(c.settlement.as_ref()?.pop * c.year.as_ref()?.fossil_energy)
                    })
                    .sum();
                f / s.agg.population.max(1.0)
            };
            let first = if self.takeoff.contains_key(&sp) {
                None
            } else {
                self.societies
                    .iter()
                    .filter(|s| {
                        s.alive()
                            && s.species == sp
                            && s.agg.population >= 1e6
                            && cat.keys.machine_vapeur.is_some_and(|t| s.known.has(t))
                            && s.agg.modern_energy > 0.15
                            && fossil(s) > 0.15
                    })
                    .map(|s| s.id)
                    .next()
            };
            if let (false, Some(id)) = (self.takeoff.contains_key(&sp), first) {
                self.takeoff.insert(sp, year);
                let soc = self.societies[id as usize].name();
                self.event(EventKind::IndustrialTakeoff, Some(id), None, None, None, None, format!("Les {soc} brûlent assez de charbon pour sortir du piège malthusien : la révolution industrielle des {name} commence."), 0.9);
            }
            if self.v1.contains_key(&sp) {
                continue;
            }
            let mut sciences = [0.0f64; DOMAINS];
            let mut leader: Option<(f64, u32)> = None;
            for s in self.societies.iter().filter(|s| s.alive() && s.species == sp) {
                for (best, m) in sciences.iter_mut().zip(&s.sciences) {
                    *best = best.max(m.precision);
                }
                if s.known.intersects(&cat.flagged.network) && leader.is_none_or(|l| s.agg.population > l.0) {
                    leader = Some((s.agg.population, s.id));
                }
            }
            let science_ok = sciences.iter().all(|&p| p >= crate::science::MODERN);
            // Une civilisation de tête, reliée par un réseau mondial,
            // industrielle et à la science moderne : le XXIe siècle.
            let lead_ok = leader.is_some_and(|l| l.0 >= 1e6 && self.societies[l.1 as usize].agg.modern_energy > 0.2);
            if lead_ok && science_ok {
                self.v1.insert(sp, year);
                let n = self.alive_societies().filter(|s| s.species == sp).count();
                let lead = leader.map(|l| self.societies[l.1 as usize].name()).unwrap_or_default();
                let text = format!(
                    "Les {name} atteignent l'équivalent de notre XXIe siècle : {n} sociétés, {:.2} milliard(s) d'habitants, une science moderne dans les huit domaines, et les {lead} en tête, qui tirent {:.0} % de leur énergie utile des moteurs et dont le réseau relie déjà {:.0} % de l'espèce. Ici s'arrête la première version d'Evonisium ; la simulation continue, sans les techniques spéculatives, qui viendront plus tard.",
                    pop / 1e9,
                    100.0 * leader.map(|l| self.societies[l.1 as usize].agg.modern_energy).unwrap_or(0.0),
                    100.0 * networked / pop
                );
                self.event(EventKind::EndOfFirstVersion, leader.map(|l| l.1), None, None, None, None, text, 1.0);
            }
        }
    }

    /// Somme des pertes et des gains d'une technique dans le monde.
    pub fn knowers(&self, t: u16) -> f64 {
        self.cells.iter().filter_map(|c| c.settlement.as_ref()).filter(|s| s.known.has(t)).map(|s| s.pop).sum()
    }

    /// Cellule d'un foyer de technique agricole : premier événement.
    pub fn hearths(&self) -> Vec<&Event> {
        self.milestones.iter().filter(|e| e.kind == EventKind::AgricultureHearth).collect()
    }

    /// Empreinte de l'état simulé : deux histoires identiques au bit près
    /// ont la même.
    pub fn fingerprint(&self) -> u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        let mut eat = |x: u64| {
            for b in x.to_le_bytes() {
                h ^= b as u64;
                h = h.wrapping_mul(0x100_0000_01b3);
            }
        };
        eat(self.year as u64);
        for c in &self.cells {
            eat(c.eco.forest_c.to_bits());
            eat(c.eco.cropland_km2.to_bits());
            eat(c.eco.soil.to_bits());
            for g in &c.eco.stocks {
                eat(g.to_bits());
            }
            if let Some(s) = &c.settlement {
                eat(u64::from(s.society));
                eat(s.pop.to_bits());
                eat(s.urban.to_bits());
                for w in s.known.0 {
                    eat(w);
                }
            }
        }
        for s in &self.societies {
            let d = &s.dims;
            for x in [
                d.centralisation,
                d.inequality,
                d.coercion,
                d.participation,
                d.admin,
                d.cohesion,
                s.agg.legitimacy,
                s.agg.elites,
                s.agg.debt,
            ] {
                eat(x.to_bits());
            }
            eat(s.ended.map(|e| e as u64).unwrap_or(u64::MAX));
            for m in &s.sciences {
                eat(m.precision.to_bits());
            }
        }
        for (k, r) in &self.relations {
            eat(u64::from(k.0) << 32 | u64::from(k.1));
            eat(r.since as u64);
        }
        for n in &self.notables {
            eat(u64::from(n.id) ^ n.until as u64);
        }
        eat(self.events.len() as u64);
        eat(self.diseases.len() as u64);
        eat(self.totals.cum_co2_c.to_bits());
        eat(self.totals.cum_ch4_c.to_bits());
        h
    }

    /// Bruit déterministe pour les bancs.
    pub fn noise(&self, a: u64, b: u64) -> f64 {
        noise(self.seed, a, b)
    }
}

/// Ce qu'une société tire de ses cellules dans l'année.
struct Gather {
    known: Known,
    eff: Effects,
    pop: f64,
    gdp: f64,
    surplus: f64,
    public: f64,
    food: f64,
    urban: f64,
    lit: f64,
    modern: f64,
    sedentary: f64,
    storable: f64,
    peoples: BTreeMap<u32, f64>,
    military: f64,
    circumscribed: f64,
    biggest: (f64, u32),
}

/// Forçage humain par source, W/m².
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ForcingParts {
    pub fossil_co2: f64,
    pub land_co2: f64,
    pub methane: f64,
    pub aerosol: f64,
}

impl ForcingParts {
    /// Le CO₂ fossile est-il la première source de forçage (les aérosols,
    /// qui refroidissent, à part) ?
    pub fn fossil_leads(&self) -> bool {
        self.fossil_co2 > self.land_co2 && self.fossil_co2 > self.methane
    }
}

/// Les marchés du charbon et du pétrole dans l'année.
struct Market {
    /// Par société : le charbon, le pétrole arrivent-ils par le marché ?
    coal: Vec<bool>,
    oil: Vec<bool>,
    /// Manque de l'an passé, par marché (charbon, pétrole), t.
    short: Vec<[f64; 2]>,
    /// Mines et puits de chaque marché.
    sources: Vec<[u32; 2]>,
    /// La société est-elle d'un marché, et duquel ?
    member: Vec<bool>,
    group: Vec<u32>,
    mines: Vec<bool>,
}

impl Market {
    /// Ce qu'une cellule extrait en plus pour son marché.
    fn export(&self, cell: usize, society: usize) -> [f64; 2] {
        if !self.member.get(society).copied().unwrap_or(false) {
            return [0.0; 2];
        }
        let g = self.group[society] as usize;
        let mut e = [0.0; 2];
        if self.mines[cell] && self.sources[g][0] > 0 {
            e[0] = self.short[g][0] / f64::from(self.sources[g][0]);
        }
        if self.sources[g][1] > 0 {
            e[1] = self.short[g][1] / f64::from(self.sources[g][1]);
        }
        e
    }
}
