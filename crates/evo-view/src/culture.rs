//! Peuples et cultures à l'écran (étape 6) : l'état culturel publié
//! (`evo_culture::CulturalState`) mis en forme pour le globe (G5), les
//! fiches de peuple et de personnage, l'arbre des cultures et les mythes.
//!
//! Le moteur ne publie pas encore d'état culturel : l'intégration d'evo-culture
//! au monde vient après la porte de l'étape 5. En attendant, [`bench_world`]
//! pose des peuples d'essai sur les terres de la planète en cours, avec le
//! modèle agrégé d'evo-culture et les lectures cognitives de banc d'essai.
//! Rien de ce banc ne remonte vers le monde ; l'écran le dit au joueur.

use crate::format::{duration, number_in, Lang};
use crate::frame::LineageFrame;
use evo_culture::catalog::{Context, Family};
use evo_culture::cognition::presets;
use evo_culture::state::{GroupView, NotableView, PeopleView};
use evo_culture::traits::TraitKind;
use evo_culture::world::NotableKind;
use evo_culture::{CultureWorld, CulturalState, Organ, Phenomenon};
use evo_planet::grid::GeodesicGrid;
use evo_sim::history::CellView;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// Espèces du banc d'essai : des identifiants hors de ceux du moteur.
pub const BENCH_HUMAN: u32 = 0x00C0_0001;
pub const BENCH_APE: u32 = 0x00C0_0002;

/// Nom d'une espèce du banc.
pub fn bench_species_name(species: u32, lang: Lang) -> &'static str {
    match (species, lang) {
        (BENCH_HUMAN, Lang::Fr) => "Hominidé d'essai",
        (BENCH_HUMAN, Lang::En) => "Test hominid",
        (BENCH_APE, Lang::Fr) => "Grand singe d'essai",
        (BENCH_APE, Lang::En) => "Test great ape",
        (_, Lang::Fr) => "Espèce",
        (_, Lang::En) => "Species",
    }
}

/// Réglages du banc.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BenchSpec {
    pub seed: u64,
    /// Cellules de l'espèce à langage.
    pub human_cells: usize,
    /// Cellules de l'espèce à traditions sans langage.
    pub ape_cells: usize,
    /// Années jouées pas à pas après la bascule.
    pub historical_years: u32,
}

impl Default for BenchSpec {
    fn default() -> Self {
        BenchSpec { seed: 2026, human_cells: 160, ape_cells: 36, historical_years: 1500 }
    }
}

/// Cellule habitable : terre hors des glaces, ni trop froide ni trop chaude.
fn habitable(c: &CellView) -> bool {
    !c.is_ocean && c.ice_cover < 0.3 && (265.0..315.0).contains(&c.temperature_k)
}

/// Région connexe de terres habitables autour de `start`, en largeur
/// d'abord, avec la profondeur de chaque cellule.
fn region(grid: &GeodesicGrid, cells: &[CellView], start: usize, n: usize, taken: &BTreeSet<usize>) -> Vec<(usize, u32)> {
    let mut out = Vec::new();
    let mut seen = BTreeSet::new();
    let mut queue = VecDeque::from([(start, 0u32)]);
    seen.insert(start);
    while let Some((c, d)) = queue.pop_front() {
        out.push((c, d));
        if out.len() >= n {
            break;
        }
        for nb in grid.neighbours_of(c) {
            if nb < cells.len() && habitable(&cells[nb]) && !taken.contains(&nb) && seen.insert(nb) {
                queue.push_back((nb, d + 1));
            }
        }
    }
    out
}

/// Meilleure cellule de départ : la plus douce (près de 290 K) parmi les
/// terres habitables dont la région est assez grande, hors de `taken`.
fn best_start(grid: &GeodesicGrid, cells: &[CellView], n: usize, taken: &BTreeSet<usize>, away_from: Option<usize>) -> Option<usize> {
    let mut candidates: Vec<(f32, usize)> = cells
        .iter()
        .enumerate()
        .filter(|(i, c)| habitable(c) && !taken.contains(i))
        .map(|(i, c)| {
            let mut score = (c.temperature_k - 290.0).abs() - 0.002 * c.rain_mm_yr.min(2000.0);
            if let Some(a) = away_from {
                // Assez loin de l'autre espèce pour que les deux ne se
                // mêlent pas, assez près pour tenir sur une vue.
                let (u, v) = (grid.centers[i], grid.centers[a]);
                let d = (u[0] * v[0] + u[1] * v[1] + u[2] * v[2]).clamp(-1.0, 1.0).acos();
                score += if (0.25..0.7).contains(&d) { 0.0 } else { 50.0 };
            }
            (score, i)
        })
        .collect();
    candidates.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    candidates.into_iter().take(200).map(|(_, i)| i).find(|&i| region(grid, cells, i, n, taken).len() >= n * 3 / 4)
}

/// Effectif d'un groupe selon le climat de sa cellule.
fn population(c: &CellView) -> f64 {
    let warm = 1.0 - ((c.temperature_k - 292.0).abs() / 25.0).min(0.8) as f64;
    let wet = (c.rain_mm_yr as f64 / 1200.0).clamp(0.3, 1.2);
    (900.0 * warm * wet).max(120.0)
}

/// Un monde culturel d'essai posé sur les terres de la planète : une espèce
/// à langage (avec une île qui se coupe puis se relie, d'où un peuple fils
/// et des passeurs), et une espèce à traditions sans langage plus loin.
/// Renvoie le monde et les cellules de chaque espèce.
pub fn bench_world(grid: &GeodesicGrid, cells: &[CellView], spec: BenchSpec) -> Option<CultureWorld> {
    let mut taken = BTreeSet::new();
    let start = best_start(grid, cells, spec.human_cells, &taken, None)?;
    let humans = region(grid, cells, start, spec.human_cells, &taken);
    taken.extend(humans.iter().map(|x| x.0));
    let mut w = CultureWorld::new(spec.seed);
    let land = Context::land(Organ::Hand);
    w.add_species(BENCH_HUMAN, presets::human());
    // L'île : les cellules les plus lointaines d'un même côté.
    let max_depth = humans.iter().map(|x| x.1).max().unwrap_or(0);
    let island: BTreeSet<usize> = humans.iter().filter(|x| x.1 + 1 >= max_depth && max_depth >= 4).map(|x| x.0).collect();
    let member: BTreeSet<usize> = humans.iter().map(|x| x.0).collect();
    for &(c, _) in &humans {
        // Le groupe descend d'un voisin déjà posé (le premier fonde le peuple).
        let from = grid.neighbours_of(c).find(|nb| member.contains(nb) && w.group(*nb as u32, BENCH_HUMAN).is_some()).map(|nb| nb as u32);
        w.add_group(c as u32, BENCH_HUMAN, population(&cells[c]), land, from);
    }
    let link_all = |w: &mut CultureWorld, weight_island: f64| {
        for &(c, _) in &humans {
            for nb in grid.neighbours_of(c) {
                if nb > c && member.contains(&nb) {
                    let across = island.contains(&c) != island.contains(&nb);
                    w.link(c as u32, nb as u32, if across { weight_island } else { 0.3 });
                }
            }
        }
    };
    link_all(&mut w, 0.3);
    let core: Vec<u32> = humans.iter().take(4).map(|x| x.0 as u32).collect();
    for f in [Family::Percussion, Family::Fire] {
        for &c in &core {
            w.seed_trait(c, BENCH_HUMAN, f as u32, 0.5, 0.8);
        }
    }
    // Les grands singes, ailleurs, avec leurs seules traditions.
    if spec.ape_cells > 0 {
        if let Some(s) = best_start(grid, cells, spec.ape_cells, &taken, Some(start)) {
            let apes = region(grid, cells, s, spec.ape_cells, &taken);
            w.add_species(BENCH_APE, presets::chimpanzee());
            let am: BTreeSet<usize> = apes.iter().map(|x| x.0).collect();
            for &(c, _) in &apes {
                let from = grid.neighbours_of(c).find(|nb| am.contains(nb) && w.group(*nb as u32, BENCH_APE).is_some()).map(|nb| nb as u32);
                w.add_group(c as u32, BENCH_APE, population(&cells[c]) * 0.4, land, from);
            }
            for &(c, _) in &apes {
                for nb in grid.neighbours_of(c) {
                    if nb > c && am.contains(&nb) {
                        w.link(c as u32, nb as u32, 0.5);
                    }
                }
            }
        }
    }
    // Longue préhistoire à l'équilibre : les traditions s'installent et
    // l'espèce à langage bascule en mode historique.
    for _ in 0..4 {
        w.step(10_000.0);
    }
    // Puis an par an : l'île se coupe, devient un autre peuple, puis se
    // relie ; un phénomène vu fait naître un mythe.
    let cut = spec.historical_years / 5;
    let relink = spec.historical_years * 3 / 5;
    let seen_at = spec.historical_years / 10;
    for y in 0..spec.historical_years {
        if y == cut && !island.is_empty() {
            link_all(&mut w, 0.0);
        }
        if y == relink && !island.is_empty() {
            link_all(&mut w, 0.15);
        }
        if y == seen_at {
            w.witness(start as u32, Phenomenon::Light, true);
        }
        if y == spec.historical_years * 7 / 10 {
            if let Some(&c) = island.iter().next() {
                w.witness(c as u32, Phenomenon::Eruption, false);
            }
        }
        w.step(1.0);
    }
    Some(w)
}

/// Couleurs des peuples : des lavis de l'Atlas, assez distincts côte à côte.
pub const PEOPLE_COLOURS: [[f32; 3]; 10] = [
    [0.69, 0.36, 0.22],
    [0.27, 0.43, 0.58],
    [0.42, 0.53, 0.27],
    [0.56, 0.33, 0.52],
    [0.78, 0.6, 0.27],
    [0.25, 0.55, 0.52],
    [0.5, 0.36, 0.25],
    [0.33, 0.33, 0.6],
    [0.63, 0.45, 0.4],
    [0.4, 0.4, 0.33],
];

/// Rang de couleur d'un peuple (1 à 10 ; 0 : aucun).
pub fn people_colour_index(state: &CulturalState, people: u32) -> u32 {
    // Rang dans l'ordre des identifiants : un peuple garde sa couleur.
    let rank = state.peoples.iter().position(|p| p.id == people).unwrap_or(0);
    (rank % PEOPLE_COLOURS.len()) as u32 + 1
}

pub fn people_colour(state: &CulturalState, people: u32) -> [f32; 3] {
    PEOPLE_COLOURS[(people_colour_index(state, people) - 1) as usize]
}

/// Groupe le plus peuplé de chaque cellule.
pub fn main_groups(state: &CulturalState) -> BTreeMap<u32, &GroupView> {
    let mut out: BTreeMap<u32, &GroupView> = BTreeMap::new();
    for g in &state.groups {
        match out.get(&g.cell) {
            Some(o) if o.population >= g.population => {}
            _ => {
                out.insert(g.cell, g);
            }
        }
    }
    out
}

/// Texture des territoires, RGBA en flottants (mise en page de
/// [`crate::layers::texture_size`]) : rang de couleur du peuple, log₁₀ de
/// l'effectif, frontière (1 si une voisine a un autre peuple ou aucun),
/// espèce en mode historique (1) ou non (0).
pub fn territory_texture(state: &CulturalState, grid: &GeodesicGrid, cells: usize) -> Vec<f32> {
    let (w, h) = crate::layers::texture_size(cells);
    let mut t = vec![0.0f32; w * h * 4];
    let main = main_groups(state);
    let historical: BTreeSet<u32> = state.species.iter().filter(|s| s.historical_since.is_some()).map(|s| s.species).collect();
    for (&cell, g) in &main {
        let c = cell as usize;
        if c >= cells {
            continue;
        }
        let i = c * 4;
        t[i] = people_colour_index(state, g.people) as f32;
        t[i + 1] = (g.population.max(1.0)).log10();
        let border = grid.neighbours_of(c).any(|nb| main.get(&(nb as u32)).is_none_or(|o| o.people != g.people));
        t[i + 2] = if border { 1.0 } else { 0.0 };
        t[i + 3] = if historical.contains(&g.species) { 1.0 } else { 0.0 };
    }
    t
}

/// Étiquette d'un peuple vivant sur le globe : nom, cellule du cœur,
/// effectif, couleur.
#[derive(Clone, Debug, PartialEq)]
pub struct PeopleLabel {
    pub id: u32,
    pub name: String,
    pub cell: u32,
    pub population: f32,
    pub colour: [f32; 3],
}

pub fn labels(state: &CulturalState) -> Vec<PeopleLabel> {
    state
        .peoples
        .iter()
        .filter(|p| p.ended_year.is_none() && p.groups > 0)
        .map(|p| PeopleLabel { id: p.id, name: p.name.clone(), cell: p.core_cell, population: p.population, colour: people_colour(state, p.id) })
        .collect()
}

/// Une technique, une croyance ou un rite d'un peuple.
#[derive(Clone, Debug, PartialEq)]
pub struct TraitLine {
    pub id: u32,
    pub kind: String,
    pub name: String,
    /// Part du peuple qui le porte (pondérée par l'effectif).
    pub share: f32,
    /// Maîtrise moyenne (techniques).
    pub mastery: f32,
    /// Plus longue transmission, en générations.
    pub generations: f32,
}

/// Fiche d'un peuple.
#[derive(Clone, Debug, PartialEq)]
pub struct PeopleSheet {
    pub id: u32,
    pub name: String,
    pub species: u32,
    pub species_name: String,
    pub parent: Option<(u32, String)>,
    pub children: Vec<(u32, String)>,
    pub born_year: f64,
    pub ended_year: Option<f64>,
    pub population: f32,
    pub groups: u32,
    pub core_cell: u32,
    pub colour: [f32; 3],
    pub language: Option<String>,
    pub words: Vec<(String, String)>,
    pub techniques: Vec<TraitLine>,
    pub customs: Vec<TraitLine>,
    pub myths: Vec<(u32, String)>,
    pub notables: Vec<(u32, String, bool)>,
    /// Résumé en une phrase.
    pub summary: String,
}

fn kind_label(k: TraitKind, lang: Lang) -> String {
    match lang {
        Lang::Fr => k.label().to_string(),
        Lang::En => match k {
            TraitKind::Technique(_) => "technique",
            TraitKind::Belief => "belief",
            TraitKind::Rite => "rite",
            TraitKind::Art => "art",
            TraitKind::Taboo(_) => "taboo",
            TraitKind::Myth(_) => "myth",
        }
        .to_string(),
    }
}

pub fn people_sheet(state: &CulturalState, id: u32, lang: Lang) -> Option<PeopleSheet> {
    let p = state.peoples.iter().find(|p| p.id == id)?;
    let name_of = |id: u32| state.peoples.iter().find(|q| q.id == id).map(|q| q.name.clone()).unwrap_or_default();
    let groups: Vec<&GroupView> = state.groups.iter().filter(|g| g.people == id).collect();
    let total: f32 = groups.iter().map(|g| g.population).sum::<f32>().max(1.0);
    let mut agg: BTreeMap<u32, (f32, f32, f32, f32)> = BTreeMap::new();
    for g in &groups {
        for t in &g.traits {
            let e = agg.entry(t.id).or_insert((0.0, 0.0, 0.0, 0.0));
            e.0 += t.freq * g.population;
            e.1 += t.mastery * t.freq * g.population;
            e.2 = e.2.max(t.generations);
            e.3 += t.freq * g.population;
        }
    }
    let mut techniques = Vec::new();
    let mut customs = Vec::new();
    for (tid, (carried, mastery, gens, w)) in agg {
        let Some(tv) = state.traits.iter().find(|t| t.id == tid) else { continue };
        let line = TraitLine {
            id: tid,
            kind: kind_label(tv.kind, lang),
            name: tv.name.clone(),
            share: carried / total,
            mastery: if w > 0.0 { mastery / w } else { 0.0 },
            generations: gens,
        };
        if line.share < 0.02 {
            continue;
        }
        if matches!(tv.kind, TraitKind::Technique(_)) {
            techniques.push(line);
        } else if !matches!(tv.kind, TraitKind::Myth(_)) {
            customs.push(line);
        }
    }
    techniques.sort_by(|a, b| b.share.total_cmp(&a.share).then(a.id.cmp(&b.id)));
    customs.sort_by(|a, b| b.share.total_cmp(&a.share).then(a.id.cmp(&b.id)));
    let lang_view = p.language.and_then(|l| state.languages.iter().find(|x| x.id == l));
    let species_name = bench_species_name(p.species, lang).to_string();
    let summary = people_summary(p, &techniques, lang_view.map(|l| l.name.as_str()), &species_name, lang);
    Some(PeopleSheet {
        id,
        name: p.name.clone(),
        species: p.species,
        species_name,
        parent: p.parent.map(|q| (q, name_of(q))),
        children: state.peoples.iter().filter(|q| q.parent == Some(id)).map(|q| (q.id, q.name.clone())).collect(),
        born_year: p.born_year,
        ended_year: p.ended_year,
        population: p.population,
        groups: p.groups,
        core_cell: p.core_cell,
        colour: people_colour(state, id),
        language: lang_view.map(|l| l.name.clone()),
        words: lang_view.map(|l| l.words.clone()).unwrap_or_default(),
        techniques,
        customs,
        myths: state.myths.iter().filter(|m| m.people == id).map(|m| (m.id, m.title.clone())).collect(),
        notables: state.notables.iter().filter(|n| n.people == id).map(|n| (n.id, n.name.clone(), n.alive)).collect(),
        summary,
    })
}

fn people_summary(p: &PeopleView, techniques: &[TraitLine], language: Option<&str>, species: &str, lang: Lang) -> String {
    let n = number_in(lang, p.population.round() as f64, 0);
    let tech = techniques.len();
    match (lang, language) {
        (Lang::Fr, Some(l)) => format!("{} : {n} individus en {} groupes ({species}), qui parlent le {l} et connaissent {tech} technique{}.", p.name, p.groups, if tech > 1 { "s" } else { "" }),
        (Lang::Fr, None) => format!("{} : {n} individus en {} groupes ({species}), {tech} technique{} transmise{}.", p.name, p.groups, if tech > 1 { "s" } else { "" }, if tech > 1 { "s" } else { "" }),
        (Lang::En, Some(l)) => format!("{}: {n} individuals in {} groups ({species}), speaking {l}, with {tech} technique{}.", p.name, p.groups, if tech == 1 { "" } else { "s" }),
        (Lang::En, None) => format!("{}: {n} individuals in {} groups ({species}), {tech} technique{} handed down.", p.name, p.groups, if tech == 1 { "" } else { "s" }),
    }
}

/// Nom d'un trait, même s'il n'est plus porté : une technique garde le nom
/// de sa voie ordinaire.
pub fn trait_name(state: &CulturalState, id: u32, lang: Lang) -> String {
    if let Some(t) = state.traits.iter().find(|t| t.id == id) {
        return t.name.clone();
    }
    match Family::from_index(id as usize) {
        Some(f) if id < evo_culture::traits::FIRST_GENERATED => f.technique().variants[0].name.to_string(),
        _ => match lang {
            Lang::Fr => "une coutume oubliée".into(),
            Lang::En => "a forgotten custom".into(),
        },
    }
}

/// Fiche d'un personnage notable.
#[derive(Clone, Debug, PartialEq)]
pub struct NotableSheet {
    pub id: u32,
    pub name: String,
    pub role: String,
    pub people: (u32, String),
    pub species_name: String,
    pub cell: u32,
    pub trait_name: String,
    pub born_year: f64,
    pub death_year: f64,
    pub alive: bool,
    /// Sa vie en deux phrases.
    pub story: String,
}

pub fn notable_sheet(state: &CulturalState, id: u32, lang: Lang) -> Option<NotableSheet> {
    let n: &NotableView = state.notables.iter().find(|n| n.id == id)?;
    let people = state.peoples.iter().find(|p| p.id == n.people).map(|p| p.name.clone()).unwrap_or_default();
    let trait_name = trait_name(state, n.trait_id, lang);
    let role = match (n.kind, lang) {
        (NotableKind::Inventor, Lang::Fr) => "inventeur",
        (NotableKind::Inventor, Lang::En) => "inventor",
        (NotableKind::Transmitter, Lang::Fr) => "passeur",
        (NotableKind::Transmitter, Lang::En) => "carrier",
    };
    let age = (n.death_year.min(state.year) - n.born_year).max(0.0).round();
    let story = match (n.kind, lang, n.alive) {
        (NotableKind::Inventor, Lang::Fr, _) => format!(
            "{} invente « {trait_name} » chez les {people}. De son vivant, l'invention ne peut pas se perdre ; elle se transmet ensuite comme les autres.",
            n.name
        ),
        (NotableKind::Transmitter, Lang::Fr, _) => format!(
            "{} fait entrer « {trait_name} » chez les {people}, qui ne la connaissaient pas. Là où il vit, elle se transmet deux fois mieux.",
            n.name
        ),
        (NotableKind::Inventor, Lang::En, _) => format!(
            "{} invents “{trait_name}” among the {people}. While they live the invention cannot be lost; afterwards it is handed down like any other.",
            n.name
        ),
        (NotableKind::Transmitter, Lang::En, _) => format!(
            "{} brings “{trait_name}” to the {people}, who did not know it. Where they live, it spreads twice as well.",
            n.name
        ),
    };
    let story = format!(
        "{story} {}",
        match (lang, n.alive) {
            (Lang::Fr, true) => format!("{age} ans."),
            (Lang::Fr, false) => format!("Mort à {age} ans."),
            (Lang::En, true) => format!("Aged {age}."),
            (Lang::En, false) => format!("Died aged {age}."),
        }
    );
    Some(NotableSheet {
        id,
        name: n.name.clone(),
        role: role.to_string(),
        people: (n.people, people),
        species_name: bench_species_name(n.species, lang).to_string(),
        cell: n.cell,
        trait_name,
        born_year: n.born_year,
        death_year: n.death_year,
        alive: n.alive,
        story,
    })
}

/// Lignées de l'arbre des cultures : peuples (ou langues, `languages`), au
/// format de l'arbre du vivant. Le « signature » porte l'identifiant.
pub fn tree_lineages(state: &CulturalState, languages: bool) -> Vec<LineageFrame> {
    let mut out: Vec<LineageFrame> = if languages {
        state
            .languages
            .iter()
            .map(|l| LineageFrame { id: l.id, parent: l.parent.unwrap_or(l.id), born_years: l.born_year, extinct_years: l.ended_year, origin_cell: 0, signature: l.id })
            .collect()
    } else {
        state
            .peoples
            .iter()
            .map(|p| LineageFrame { id: p.id, parent: p.parent.unwrap_or(p.id), born_years: p.born_year, extinct_years: p.ended_year, origin_cell: p.core_cell, signature: p.id })
            .collect()
    };
    out.sort_by_key(|l| l.id);
    out
}

/// Ligne de l'onglet des mythes.
#[derive(Clone, Debug, PartialEq)]
pub struct MythLine {
    pub id: u32,
    pub title: String,
    pub text: String,
    pub people: String,
    pub year: f64,
    pub cell: u32,
    pub intervention: bool,
    pub theme: String,
    pub figure: String,
}

pub fn myths(state: &CulturalState) -> Vec<MythLine> {
    state
        .myths
        .iter()
        .map(|m| MythLine {
            id: m.id,
            title: m.title.clone(),
            text: m.text.clone(),
            people: state.peoples.iter().find(|p| p.id == m.people).map(|p| p.name.clone()).unwrap_or_default(),
            year: m.year,
            cell: m.cell,
            intervention: m.intervention,
            theme: m.theme.label().to_string(),
            figure: m.figure.clone(),
        })
        .collect()
}

/// Ce que dit l'inspecteur d'une cellule habitée par un peuple.
pub fn cell_line(state: &CulturalState, cell: u32, lang: Lang) -> Option<(u32, String)> {
    let g = main_groups(state).get(&cell).copied()?;
    let p = state.peoples.iter().find(|p| p.id == g.people)?;
    let tech = g.traits.iter().filter(|t| t.id < evo_culture::traits::FIRST_GENERATED && t.freq > 0.25).count();
    let n = number_in(lang, g.population.round() as f64, 0);
    Some((
        p.id,
        match lang {
            Lang::Fr => format!("{} : {n} individus, {tech} technique{}", p.name, if tech > 1 { "s" } else { "" }),
            Lang::En => format!("{}: {n} individuals, {tech} technique{}", p.name, if tech == 1 { "" } else { "s" }),
        },
    ))
}

/// Ligne de la barre du temps : mode historique ou non.
pub fn mode_line(state: &CulturalState, lang: Lang) -> String {
    let since = state.species.iter().filter_map(|s| s.historical_since).fold(f64::INFINITY, f64::min);
    match (state.historical, lang) {
        (true, Lang::Fr) => format!("Mode historique, année par année (depuis {})", duration(state.year - since, lang)),
        (true, Lang::En) => format!("Historical mode, year by year (for {})", duration(state.year - since, lang)),
        (false, Lang::Fr) => "Culture à l'équilibre".into(),
        (false, Lang::En) => "Culture at equilibrium".into(),
    }
}

/// Ce que montre la scène au sol d'une cellule peuplée : techniques portées
/// (pour les objets et les gestes) et cohésion.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CampView {
    pub people: u32,
    pub population: f32,
    pub cohesion: f32,
    /// Techniques portées par plus d'un quart du groupe.
    pub techniques: Vec<Family>,
    /// Le groupe a des rites, des arts ou des mythes.
    pub rites: bool,
}

pub fn camp(state: &CulturalState, cell: u32) -> Option<CampView> {
    let g = main_groups(state).get(&cell).copied()?;
    let mut techniques = Vec::new();
    let mut rites = false;
    for t in &g.traits {
        if t.freq < 0.25 {
            continue;
        }
        match state.traits.iter().find(|x| x.id == t.id).map(|x| x.kind) {
            Some(TraitKind::Technique(f)) => techniques.push(f),
            Some(TraitKind::Rite | TraitKind::Art | TraitKind::Myth(_)) => rites = true,
            _ => {}
        }
    }
    Some(CampView { people: g.people, population: g.population, cohesion: g.cohesion, techniques, rites })
}

#[cfg(test)]
mod tests {
    use super::*;
    use evo_sim::{World, WorldConfig};

    fn bench() -> (GeodesicGrid, CulturalState) {
        let mut w = World::new(WorldConfig::new(7, 4));
        w.seed_life();
        w.step();
        let state = w.publication.current.clone().unwrap();
        let grid = GeodesicGrid::new(4);
        let spec = BenchSpec { historical_years: 600, human_cells: 80, ape_cells: 20, ..Default::default() };
        let cw = bench_world(&grid, &state.cells, spec).expect("des terres habitables");
        (grid, CulturalState::of(&cw))
    }

    #[test]
    fn the_bench_gives_peoples_languages_and_a_myth() {
        let (grid, st) = bench();
        assert!(st.historical, "l'espèce à langage bascule");
        assert!(st.peoples.len() >= 2, "{} peuples", st.peoples.len());
        assert!(!st.languages.is_empty());
        assert!(!st.myths.is_empty(), "le phénomène vu fait un mythe");
        let t = territory_texture(&st, &grid, grid.len());
        let painted = t.chunks(4).filter(|p| p[0] > 0.0).count();
        assert!(painted >= 80, "{painted} cellules peintes");
        assert!(t.chunks(4).any(|p| p[2] > 0.5), "des frontières");
        let p = &st.peoples[0];
        let sheet = people_sheet(&st, p.id, Lang::Fr).unwrap();
        assert!(!sheet.techniques.is_empty(), "{sheet:?}");
        assert!(sheet.summary.starts_with(&p.name));
        if let Some(n) = st.notables.first() {
            let ns = notable_sheet(&st, n.id, Lang::Fr).unwrap();
            assert!(ns.story.contains(&ns.trait_name));
        }
        let tree = crate::tree::build(&tree_lineages(&st, false), st.year, 40);
        assert_eq!(tree.nodes.len(), st.peoples.len().min(40));
    }

    #[test]
    #[ignore]
    fn bench_report() {
        let mut w = World::new(WorldConfig::new(2026, 6));
        w.seed_life();
        w.step();
        let state = w.publication.current.clone().unwrap();
        let grid = GeodesicGrid::new(6);
        let t0 = std::time::Instant::now();
        let cw = bench_world(&grid, &state.cells, BenchSpec::default()).unwrap();
        let st = CulturalState::of(&cw);
        eprintln!("{:?} ; {} peuples, {} langues, {} notables, {} mythes, {} groupes, historique {}", t0.elapsed(), st.peoples.len(), st.languages.len(), st.notables.len(), st.myths.len(), st.groups.len(), st.historical);
        for p in &st.peoples {
            eprintln!("{} {} parent {:?} {} groupes {} né {}", p.id, p.name, p.parent, p.groups, p.population, p.born_year);
        }
        for m in &st.myths { eprintln!("{} : {}", m.title, m.text); }
        let s = people_sheet(&st, st.peoples[0].id, Lang::Fr).unwrap();
        eprintln!("{s:#?}");
        if let Some(n) = st.notables.first() { eprintln!("{:#?}", notable_sheet(&st, n.id, Lang::Fr)); }
    }

    #[test]
    fn the_bench_is_deterministic() {
        let (_, a) = bench();
        let (_, b) = bench();
        assert_eq!(a.fingerprint(), b.fingerprint());
    }
}
