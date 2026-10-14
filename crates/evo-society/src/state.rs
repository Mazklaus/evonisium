//! État des sociétés publié, format `evo_society::SocietyState` v1.
//!
//! Le moteur le publie à chaque pas du mode historique, à côté de l'état du
//! monde et de `evo_culture::CulturalState`. Le client (calques, villes,
//! comparaison, galerie des rejeux, encyclopédie, bâtiments du palier 3) et le
//! son (villes) le lisent ; ils n'écrivent jamais dedans. Le lire ne change
//! rien à l'histoire : [`SocietyState::of`] ne prend qu'une référence.
//!
//! Règles du format :
//!
//! * tout champ ajouté plus tard sera facultatif (`#[serde(default)]`) ;
//!   un changement qui casse la lecture fait passer à la version 2 ;
//! * les identifiants (sociétés, cellules, villes, notables, maladies,
//!   techniques) sont stables pendant toute la partie ;
//! * les noms des techniques et des biens se lisent dans le catalogue
//!   ([`crate::catalog::catalog`], [`crate::goods::Good`]) par leur rang ;
//! * les événements publiés sont ceux de l'année écoulée ; le client garde
//!   la chronique.

use crate::catalog::Known;
use crate::chronicle::{Event, NotableKind};
use crate::economy::Flux;
use crate::goods::{Goods, GOODS};
use crate::science::{estimate, Domain, DOMAINS};
use crate::society::{Aggregates, Dimensions, Label, INSTITUTIONS};
use crate::world::{RelationKind, SocietyWorld};
use serde::{Deserialize, Serialize};

pub const FORMAT_VERSION: u32 = 1;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SocietyState {
    pub version: u32,
    pub year: i64,
    pub societies: Vec<SocietyView>,
    /// Cellules habitées seulement.
    pub cells: Vec<CellView>,
    /// Villes vivantes.
    pub cities: Vec<CityView>,
    /// Routes d'échange de l'année, entre cellules voisines.
    pub routes: Vec<RouteView>,
    pub relations: Vec<RelationView>,
    /// Notables vivants (au plus 5 000).
    pub notables: Vec<NotableView>,
    pub diseases: Vec<DiseaseView>,
    pub species: Vec<SpeciesView>,
    pub planet: PlanetView,
    /// Événements de l'année écoulée.
    pub events: Vec<Event>,
    /// Empreinte de l'état simulé complet, pour vérifier les rejeux.
    pub fingerprint: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SocietyView {
    pub id: u32,
    pub parent: Option<u32>,
    pub species: u32,
    /// Peuple dominant (identifiant d'`evo-culture`).
    pub people: u32,
    pub name: String,
    pub founded: i64,
    pub capital: u32,
    pub cells: Vec<u32>,
    /// Étiquette d'affichage et son nom français.
    pub label: Label,
    pub label_name: String,
    pub peak: Label,
    pub dims: Dimensions,
    pub agg: Aggregates,
    /// Force de chaque institution, de 0 à 1, dans l'ordre de
    /// [`crate::society::Institution::ALL`].
    pub institutions: [f64; INSTITUTIONS],
    pub religion: u32,
    pub in_crisis: bool,
    pub at_war: bool,
    pub chief: Option<u32>,
    pub sciences: [ScienceView; DOMAINS],
    /// Techniques connues (rangs du catalogue).
    pub known: Known,
}

/// Ce qu'une société croit d'un domaine : sa précision et son estimation
/// d'une grandeur de référence, à comparer à la vraie.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ScienceView {
    pub domain: Domain,
    pub precision: f64,
    pub paradigm: u16,
    pub estimate: f64,
    pub truth: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CellView {
    pub cell: u32,
    pub society: u32,
    pub species: u32,
    pub people: u32,
    pub pop: f64,
    /// Part urbaine.
    pub urban: f64,
    pub sedentary: bool,
    pub city: Option<u32>,
    /// Espèces sauvages domestiquées ici (culture, troupeau).
    pub crop: Option<u32>,
    pub herd: Option<u32>,
    pub food_pc: f64,
    pub gdp_pc: f64,
    /// Travail utile non musculaire par travailleur, MWh.
    pub power_pc: f64,
    pub modern_energy: f64,
    pub literacy: f64,
    /// Bilan des biens de l'année, par rang de [`crate::goods::Good`].
    pub produced: Goods,
    pub consumed: Goods,
    pub stocks: Goods,
    /// Prix, en rations, seulement avec un marché.
    pub prices: Option<Goods>,
    pub cropland_km2: f64,
    pub irrigated_km2: f64,
    pub livestock: f64,
    pub soil: f64,
    pub forest_c: f64,
    pub flux: Flux,
    pub deaths_epidemic: f64,
    pub deaths_war: f64,
    pub famine: bool,
    /// Maladies qui circulent (bits des identifiants).
    pub circulating: u32,
    pub known: Known,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CityView {
    pub id: u32,
    pub cell: u32,
    pub name: String,
    pub society: u32,
    pub founded: i64,
    pub pop: f64,
    pub peak: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct RouteView {
    pub a: u32,
    pub b: u32,
    /// Valeur échangée par la cellule `a`, en rations (somme de ses routes).
    pub value: f64,
    pub sea: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct RelationView {
    pub a: u32,
    pub b: u32,
    pub kind: RelationKind,
    pub since: i64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NotableView {
    pub id: u32,
    pub kind: NotableKind,
    pub society: u32,
    pub species: u32,
    pub name: String,
    pub born: i64,
    pub subject: u32,
    pub deeds: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DiseaseView {
    pub id: u32,
    pub name: String,
    pub r0: f64,
    pub lethality: f64,
    pub species: u32,
    pub origin_year: i64,
    /// Cellules où elle circule cette année.
    pub cells: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SpeciesView {
    pub id: u32,
    pub name: String,
    pub pop: f64,
    pub societies: u32,
    /// Sortie du piège malthusien.
    pub takeoff: Option<i64>,
    /// Fin de la première version.
    pub end_of_first_version: Option<i64>,
    /// Espèces intelligentes rencontrées.
    pub met: Vec<u32>,
}

/// Ce que les sociétés rendent à la planète.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlanetView {
    /// Flux de l'année, somme des cellules.
    pub year: Flux,
    pub cum_co2_c: f64,
    pub cum_ch4_c: f64,
    pub ch4_burden_c: f64,
    pub cum_deforestation_c: f64,
    pub cum_erosion_t: f64,
    /// Forçage radiatif des sociétés, W/m².
    pub forcing_w_m2: f64,
    /// Pas du climat demandé à Planète, années.
    pub climate_step_years: u32,
    /// Écart du bilan de carbone, t C (0 à l'arrondi près).
    pub carbon_balance: f64,
}

impl SocietyState {
    /// Lit l'état publié, sans rien changer.
    pub fn of(w: &SocietyWorld) -> SocietyState {
        let at_war = |id: u32| w.relations.iter().any(|(k, r)| (k.0 == id || k.1 == id) && matches!(r.kind, RelationKind::War { .. }));
        let societies = w
            .alive_societies()
            .map(|s| SocietyView {
                id: s.id,
                parent: s.parent,
                species: s.species,
                people: s.people,
                name: s.name(),
                founded: s.founded,
                capital: s.capital,
                cells: s.cells.clone(),
                label: s.label,
                label_name: s.label.label().to_string(),
                peak: s.peak,
                dims: s.dims,
                agg: s.agg,
                institutions: s.institutions,
                religion: s.religion,
                in_crisis: s.crisis > 0,
                at_war: at_war(s.id),
                chief: s.chief,
                sciences: std::array::from_fn(|k| {
                    let d = Domain::ALL[k];
                    let m = &s.sciences[k];
                    let truth = w.truths.0[k];
                    ScienceView {
                        domain: d,
                        precision: m.precision,
                        paradigm: m.paradigm,
                        estimate: estimate(truth, m.precision, s.bias[k]),
                        truth,
                    }
                }),
                known: s.known,
            })
            .collect();
        let mut cells = Vec::new();
        let mut routes = Vec::new();
        for c in &w.cells {
            let Some(s) = &c.settlement else { continue };
            let y = c.year.as_ref();
            cells.push(CellView {
                cell: c.id,
                society: s.society,
                species: s.species,
                people: s.people,
                pop: s.pop,
                urban: s.urban,
                sedentary: s.sedentary,
                city: s.city,
                crop: s.crop.map(|d| d.species),
                herd: s.herd.map(|d| d.species),
                food_pc: s.food_pc,
                gdp_pc: s.gdp_pc,
                power_pc: s.power_pc,
                modern_energy: s.modern,
                literacy: s.literacy,
                produced: y.map(|y| y.produced).unwrap_or([0.0; GOODS]),
                consumed: y.map(|y| y.consumed).unwrap_or([0.0; GOODS]),
                stocks: c.eco.stocks,
                prices: y.and_then(|y| y.prices),
                cropland_km2: c.eco.cropland_km2,
                irrigated_km2: c.eco.irrigated_km2,
                livestock: c.eco.livestock,
                soil: c.eco.soil,
                forest_c: c.eco.forest_c,
                flux: y.map(|y| y.flux).unwrap_or_default(),
                deaths_epidemic: s.deaths_epidemic,
                deaths_war: s.deaths_war,
                famine: s.famine > 0,
                circulating: s.circulating,
                known: s.known,
            });
            if c.trade > 0.0 {
                for l in &c.links {
                    if l.to > c.id && w.cells[l.to as usize].trade > 0.0 && w.cells[l.to as usize].settlement.is_some() {
                        routes.push(RouteView { a: c.id, b: l.to, value: c.trade, sea: l.sea });
                    }
                }
            }
        }
        let cities = w
            .cities
            .iter()
            .filter(|c| c.abandoned.is_none())
            .map(|c| CityView {
                id: c.id,
                cell: c.cell,
                name: c.name.clone(),
                society: c.society,
                founded: c.founded,
                pop: c.pop,
                peak: c.peak,
            })
            .collect();
        let relations = w.relations.iter().map(|(k, r)| RelationView { a: k.0, b: k.1, kind: r.kind, since: r.since }).collect();
        let notables = w
            .notables
            .iter()
            .filter(|n| n.died.is_none())
            .map(|n| NotableView {
                id: n.id,
                kind: n.kind,
                society: n.society,
                species: n.species,
                name: n.name.clone(),
                born: n.born,
                subject: n.subject,
                deeds: n.deeds.clone(),
            })
            .collect();
        let diseases = w
            .diseases
            .iter()
            .map(|d| DiseaseView {
                id: d.id,
                name: d.name.clone(),
                r0: d.r0,
                lethality: d.lethality,
                species: d.species,
                origin_year: d.origin_year,
                cells: w.cells.iter().filter_map(|c| c.settlement.as_ref()).filter(|s| s.circulating >> d.id & 1 == 1).count() as u32,
            })
            .collect();
        let species = w
            .species
            .iter()
            .map(|sp| SpeciesView {
                id: sp.id,
                name: sp.name.clone(),
                pop: w.population(sp.id),
                societies: w.alive_societies().filter(|s| s.species == sp.id).count() as u32,
                takeoff: w.takeoff.get(&sp.id).copied(),
                end_of_first_version: w.v1.get(&sp.id).copied(),
                met: w
                    .contacts
                    .iter()
                    .filter_map(|&(a, b)| {
                        if a == sp.id {
                            Some(b)
                        } else if b == sp.id {
                            Some(a)
                        } else {
                            None
                        }
                    })
                    .collect(),
            })
            .collect();
        let t = &w.totals;
        let planet = PlanetView {
            year: t.year,
            cum_co2_c: t.cum_co2_c,
            cum_ch4_c: t.cum_ch4_c,
            ch4_burden_c: t.ch4_burden_c,
            cum_deforestation_c: t.cum_deforestation_c,
            cum_erosion_t: t.cum_erosion_t,
            forcing_w_m2: w.forcing_w_m2(),
            climate_step_years: w.climate_step_years(),
            carbon_balance: w.carbon_balance(),
        };
        let events = w.events.iter().rev().take_while(|e| e.year == w.year).cloned().collect::<Vec<_>>().into_iter().rev().collect();
        SocietyState {
            version: FORMAT_VERSION,
            year: w.year,
            societies,
            cells,
            cities,
            routes,
            relations,
            notables,
            diseases,
            species,
            planet,
            events,
            fingerprint: w.fingerprint(),
        }
    }
}
