//! Catalogue des techniques jusqu'au XXIe siècle, chargé depuis
//! `data/techniques.txt`.
//!
//! Le graphe n'a pas de sommet : ajouter un nœud au fichier ne touche pas
//! au code. Chaque technique déclare ses prérequis, ce qu'elle demande du
//! corps et du milieu, les ressources locales, les sciences, le besoin qui
//! pousse à l'inventer, ses effets chiffrés et les recettes qu'elle ouvre.
//! Une technique n'apparaît que si tout cela est réuni ; l'ordre et le lieu
//! ne sont jamais fixés. Les voies de remplacement (mondes aquatiques,
//! souterrains, pauvres en oxygène, sans mains) sont des nœuds à part,
//! marqués par leur corps.
//!
//! Format, un bloc par technique :
//!
//! ```text
//! [cle]
//! nom = Nom affiché
//! ref = Référence terrestre
//! pre = a, b            # tous requis
//! ou = c, d             # au moins un
//! famille = Fire        # famille de l'étape 6 (evo-culture)
//! diff = 30             # difficulté de Henrich
//! inv = 1               # facilité d'invention relative
//! besoin = nourriture
//! corps = mains, terre, feu, ciel, sedentaire
//! res = riviere, charbon
//! sci = mecanique:0.3
//! effet = rendement:1.5, sante:0.95
//! energie = charbon:0.05
//! instrument = astronomie
//! drapeaux = agriculture
//! recette = bronze 1 <- cuivre 0.88, etain 0.12, charbon_bois 1.5 ; travail 0.3
//! ```

use crate::goods::{Good, GOODS};
use crate::science::{Domain, DOMAINS};
use evo_culture::catalog::{Family, Milieu};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

pub const MAX_TECHNIQUES: usize = 320;
const WORDS: usize = MAX_TECHNIQUES / 64;

/// Ensemble de techniques connues.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Known(pub [u64; WORDS]);

impl Known {
    pub fn has(&self, t: u16) -> bool {
        self.0[t as usize / 64] >> (t % 64) & 1 == 1
    }
    pub fn set(&mut self, t: u16) {
        self.0[t as usize / 64] |= 1 << (t % 64);
    }
    pub fn clear(&mut self, t: u16) {
        self.0[t as usize / 64] &= !(1 << (t % 64));
    }
    pub fn or(&self, o: &Known) -> Known {
        let mut k = *self;
        for i in 0..WORDS {
            k.0[i] |= o.0[i];
        }
        k
    }
    pub fn and_not(&self, o: &Known) -> Known {
        let mut k = *self;
        for i in 0..WORDS {
            k.0[i] &= !o.0[i];
        }
        k
    }
    pub fn and(&self, o: &Known) -> Known {
        let mut k = *self;
        for i in 0..WORDS {
            k.0[i] &= o.0[i];
        }
        k
    }
    pub fn contains_all(&self, o: &Known) -> bool {
        (0..WORDS).all(|i| self.0[i] & o.0[i] == o.0[i])
    }
    pub fn intersects(&self, o: &Known) -> bool {
        (0..WORDS).any(|i| self.0[i] & o.0[i] != 0)
    }
    pub fn is_empty(&self) -> bool {
        self.0.iter().all(|&w| w == 0)
    }
    pub fn count(&self) -> u32 {
        self.0.iter().map(|w| w.count_ones()).sum()
    }
    pub fn iter(&self) -> impl Iterator<Item = u16> + '_ {
        (0..WORDS).flat_map(move |i| {
            let mut w = self.0[i];
            std::iter::from_fn(move || {
                if w == 0 {
                    return None;
                }
                let b = w.trailing_zeros();
                w &= w - 1;
                Some((i * 64) as u16 + b as u16)
            })
        })
    }
}

/// Ce qui pousse à inventer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Need {
    Food,
    Fuel,
    Defence,
    Conquest,
    Administration,
    Transport,
    Health,
    Knowledge,
    Shelter,
    Prestige,
}

/// Ressource locale demandée.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Resource {
    River,
    Coast,
    Forest,
    Coal,
    Oil,
    Copper,
    Tin,
    Iron,
    Uranium,
    Salt,
    SeedPlant,
    RootPlant,
    Animal,
    MilkAnimal,
    Mount,
    Herds,
    Wind,
    Sun,
    Geothermal,
    Grassland,
    /// Assez de nourriture sauvage sur place pour ne plus partir : côte ou
    /// fleuve poissonneux, ou peuplements de plantes à récolter.
    Abundance,
}

/// Exigences du corps et du milieu.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Body {
    pub hands: bool,
    /// Voie réservée aux espèces sans mains.
    pub handless: bool,
    pub milieu: Option<Milieu>,
    pub combustion: bool,
    pub sky: bool,
    pub sedentary: bool,
    /// Voie réservée aux airs trop pauvres pour le feu.
    pub low_oxygen: bool,
}

/// Source d'énergie ouverte par une technique.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Energy {
    Animal,
    Water,
    Wind,
    Wood,
    Coal,
    Oil,
    Electric,
    Nuclear,
    Solar,
    Geothermal,
}

impl Energy {
    pub const ALL: [Energy; 10] = [
        Energy::Animal,
        Energy::Water,
        Energy::Wind,
        Energy::Wood,
        Energy::Coal,
        Energy::Oil,
        Energy::Electric,
        Energy::Nuclear,
        Energy::Solar,
        Energy::Geothermal,
    ];
    pub fn key(self) -> &'static str {
        match self {
            Energy::Animal => "animal",
            Energy::Water => "eau",
            Energy::Wind => "vent",
            Energy::Wood => "bois",
            Energy::Coal => "charbon",
            Energy::Oil => "petrole",
            Energy::Electric => "electrique",
            Energy::Nuclear => "nucleaire",
            Energy::Solar => "solaire",
            Energy::Geothermal => "geothermie",
        }
    }
    /// Fossile ou mieux : ce qui sort du piège malthusien.
    pub fn modern(self) -> bool {
        matches!(self, Energy::Coal | Energy::Oil | Energy::Electric | Energy::Nuclear | Energy::Solar | Energy::Geothermal)
    }
}

/// Effets chiffrés. Les facteurs valent 1 par défaut et se multiplient entre
/// techniques connues ; les ajouts valent 0 et s'additionnent.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Effects {
    pub forage: f64,
    pub fishing: f64,
    pub crop_yield: f64,
    pub field_labour: f64,
    /// Rendement ajouté sur les terres irrigables.
    pub irrigation: f64,
    /// Entretien de la fertilité (jachère, fumure, engrais), ajout.
    pub soil: f64,
    pub storage: f64,
    pub herd: f64,
    pub traction: f64,
    pub craft: f64,
    pub transport: f64,
    /// Fidélité de transmission ajoutée (l'écriture en est le cas type).
    pub fidelity: f64,
    pub admin: f64,
    pub military: f64,
    pub defence: f64,
    /// Facteur sur la mortalité ordinaire.
    pub health: f64,
    /// Facteur sur la létalité des épidémies.
    pub epidemic: f64,
    /// Facteur sur la part des gens sensibles (vaccins).
    pub immunity: f64,
    /// Facteur sur la natalité (contraception).
    pub birth: f64,
    pub science: f64,
    /// Part urbaine permise, ajout.
    pub urban: f64,
    /// Portée des échanges et des armées, ajout (en cellules).
    pub reach: f64,
    /// Carburant et engrais demandés par km² cultivé, t, ajouts : sans eux,
    /// les tracteurs et les engrais ne servent pas.
    pub fuel_km2: f64,
    pub fertilizer_km2: f64,
}

impl Default for Effects {
    fn default() -> Effects {
        Effects {
            forage: 1.0,
            fishing: 1.0,
            crop_yield: 1.0,
            field_labour: 1.0,
            irrigation: 0.0,
            soil: 0.0,
            storage: 0.0,
            herd: 1.0,
            traction: 0.0,
            craft: 1.0,
            transport: 1.0,
            fidelity: 0.0,
            admin: 0.0,
            military: 1.0,
            defence: 1.0,
            health: 1.0,
            epidemic: 1.0,
            immunity: 1.0,
            birth: 1.0,
            science: 1.0,
            urban: 0.0,
            reach: 0.0,
            fuel_km2: 0.0,
            fertilizer_km2: 0.0,
        }
    }
}

impl Effects {
    /// Composition de deux jeux d'effets.
    pub fn combine(&self, o: &Effects) -> Effects {
        Effects {
            forage: self.forage * o.forage,
            fishing: self.fishing * o.fishing,
            crop_yield: self.crop_yield * o.crop_yield,
            field_labour: self.field_labour * o.field_labour,
            irrigation: self.irrigation + o.irrigation,
            soil: self.soil + o.soil,
            storage: self.storage + o.storage,
            herd: self.herd * o.herd,
            traction: self.traction + o.traction,
            craft: self.craft * o.craft,
            transport: self.transport * o.transport,
            fidelity: self.fidelity + o.fidelity,
            admin: self.admin + o.admin,
            military: self.military * o.military,
            defence: self.defence * o.defence,
            health: self.health * o.health,
            epidemic: self.epidemic * o.epidemic,
            immunity: self.immunity * o.immunity,
            birth: self.birth * o.birth,
            science: self.science * o.science,
            urban: self.urban + o.urban,
            reach: self.reach + o.reach,
            fuel_km2: self.fuel_km2 + o.fuel_km2,
            fertilizer_km2: self.fertilizer_km2 + o.fertilizer_km2,
        }
    }
}

/// Une recette : des entrées et du travail donnent une sortie.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Recipe {
    pub output: Good,
    pub amount: f64,
    pub inputs: Vec<(Good, f64)>,
    /// Années de travail par unité produite.
    pub labour: f64,
    /// Gisement consommé par unité (minerais, fossiles), avec sa ressource.
    pub deposit: Option<(Resource, f64)>,
    /// CO₂ issu des carbonates (ciment), t C par unité.
    pub carbonate_c: f64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Flags {
    pub agriculture: bool,
    pub herding: bool,
    pub writing: bool,
    pub fossil: bool,
    pub network: bool,
    pub method: bool,
    pub printing: bool,
    pub money: bool,
    pub market: bool,
    pub industry: bool,
    pub navigation: bool,
    pub metal: bool,
    pub law: bool,
    pub school: bool,
    pub cavalry: bool,
    pub gunpowder: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Technique {
    pub id: u16,
    pub key: String,
    pub name: String,
    pub reference: String,
    pub pre: Vec<u16>,
    /// Groupes « au moins un de » (une ligne `ou` par groupe).
    pub any: Vec<Vec<u16>>,
    pub family: Option<Family>,
    pub difficulty: f64,
    pub inventiveness: f64,
    pub need: Option<Need>,
    pub body: Body,
    pub resources: Vec<Resource>,
    pub science: Vec<(Domain, f64)>,
    pub effects: Effects,
    pub energy: Option<(Energy, f64)>,
    pub instruments: Vec<Domain>,
    pub flags: Flags,
    pub recipes: Vec<Recipe>,
}

impl Technique {
    /// Masque des prérequis obligatoires.
    pub fn pre_mask(&self) -> Known {
        let mut k = Known::default();
        for &p in &self.pre {
            k.set(p);
        }
        k
    }
}

/// Le catalogue entier.
#[derive(Debug)]
pub struct Catalog {
    pub techniques: Vec<Technique>,
    pub pre_masks: Vec<Known>,
    pub any_masks: Vec<Vec<Known>>,
    /// Techniques qui produisent chaque bien.
    pub producers: [Vec<(u16, usize)>; GOODS],
    pub instrument_of: [Known; DOMAINS],
    pub families: Vec<(Family, u16)>,
    /// Profondeur : longueur de la plus longue chaîne de prérequis (un
    /// groupe « ou » compte par son option la plus courte).
    pub depth: Vec<u16>,
}

static CATALOG: OnceLock<Catalog> = OnceLock::new();

pub const DATA: &str = include_str!("../data/techniques.txt");

/// Le catalogue de la partie (chargé une fois).
pub fn catalog() -> &'static Catalog {
    CATALOG.get_or_init(|| Catalog::parse(DATA).unwrap_or_else(|e| panic!("data/techniques.txt : {e}")))
}

fn need(k: &str) -> Option<Need> {
    Some(match k {
        "nourriture" => Need::Food,
        "combustible" => Need::Fuel,
        "defense" => Need::Defence,
        "conquete" => Need::Conquest,
        "administration" => Need::Administration,
        "transport" => Need::Transport,
        "sante" => Need::Health,
        "savoir" => Need::Knowledge,
        "abri" => Need::Shelter,
        "prestige" => Need::Prestige,
        _ => return None,
    })
}

fn resource(k: &str) -> Option<Resource> {
    Some(match k {
        "riviere" => Resource::River,
        "cote" => Resource::Coast,
        "foret" => Resource::Forest,
        "charbon" => Resource::Coal,
        "petrole" => Resource::Oil,
        "cuivre" => Resource::Copper,
        "etain" => Resource::Tin,
        "fer" => Resource::Iron,
        "uranium" => Resource::Uranium,
        "sel" => Resource::Salt,
        "plante_graine" => Resource::SeedPlant,
        "plante_racine" => Resource::RootPlant,
        "animal" => Resource::Animal,
        "animal_lait" => Resource::MilkAnimal,
        "monture" => Resource::Mount,
        "troupeau" => Resource::Herds,
        "vent" => Resource::Wind,
        "soleil" => Resource::Sun,
        "geothermie" => Resource::Geothermal,
        "prairie" => Resource::Grassland,
        "abondance" => Resource::Abundance,
        _ => return None,
    })
}

fn family(k: &str) -> Option<Family> {
    Family::ALL.iter().copied().find(|f| format!("{f:?}") == k)
}

fn num(s: &str, line: usize) -> Result<f64, String> {
    s.trim().parse::<f64>().map_err(|_| format!("ligne {line} : nombre attendu, « {s} »"))
}

fn set_effect(e: &mut Effects, k: &str, v: f64, line: usize) -> Result<(), String> {
    match k {
        "cueillette" => e.forage = v,
        "peche" => e.fishing = v,
        "rendement" => e.crop_yield = v,
        "travail_champ" => e.field_labour = v,
        "irrigation" => e.irrigation = v,
        "sol" => e.soil = v,
        "stockage" => e.storage = v,
        "troupeau" => e.herd = v,
        "traction" => e.traction = v,
        "artisanat" => e.craft = v,
        "transport" => e.transport = v,
        "fidelite" => e.fidelity = v,
        "admin" => e.admin = v,
        "militaire" => e.military = v,
        "defense" => e.defence = v,
        "sante" => e.health = v,
        "epidemie" => e.epidemic = v,
        "immunite" => e.immunity = v,
        "natalite" => e.birth = v,
        "science" => e.science = v,
        "urbain" => e.urban = v,
        "portee" => e.reach = v,
        "carburant" => e.fuel_km2 = v,
        "engrais" => e.fertilizer_km2 = v,
        _ => return Err(format!("ligne {line} : effet inconnu « {k} »")),
    }
    Ok(())
}

fn parse_recipe(s: &str, line: usize) -> Result<Recipe, String> {
    // sortie q <- e q, e q ; travail w ; gisement r q ; carbonate c
    let mut parts = s.split(';');
    let head = parts.next().unwrap_or("");
    let (out, ins) = head.split_once("<-").map(|(a, b)| (a, Some(b))).unwrap_or((head, None));
    let mut o = out.split_whitespace();
    let output = o.next().and_then(Good::from_key).ok_or_else(|| format!("ligne {line} : bien inconnu dans « {out} »"))?;
    let amount = num(o.next().unwrap_or("1"), line)?;
    let mut inputs = Vec::new();
    if let Some(ins) = ins {
        for i in ins.split(',').map(str::trim).filter(|i| !i.is_empty()) {
            let mut w = i.split_whitespace();
            let g = w.next().and_then(Good::from_key).ok_or_else(|| format!("ligne {line} : bien inconnu « {i} »"))?;
            inputs.push((g, num(w.next().unwrap_or("1"), line)?));
        }
    }
    let mut r = Recipe { output, amount, inputs, labour: 0.0, deposit: None, carbonate_c: 0.0 };
    for p in parts {
        let mut w = p.split_whitespace();
        match w.next() {
            Some("travail") => r.labour = num(w.next().unwrap_or("0"), line)?,
            Some("gisement") => {
                let res = w.next().and_then(resource).ok_or_else(|| format!("ligne {line} : gisement inconnu « {p} »"))?;
                r.deposit = Some((res, num(w.next().unwrap_or("1"), line)?));
            }
            Some("carbonate") => r.carbonate_c = num(w.next().unwrap_or("0"), line)?,
            Some(x) => return Err(format!("ligne {line} : clé de recette inconnue « {x} »")),
            None => {}
        }
    }
    Ok(r)
}

impl Catalog {
    pub fn parse(text: &str) -> Result<Catalog, String> {
        struct Raw {
            key: String,
            line: usize,
            fields: Vec<(String, String, usize)>,
        }
        let mut raws: Vec<Raw> = Vec::new();
        for (n, l) in text.lines().enumerate() {
            let line = n + 1;
            let l = l.split('#').next().unwrap_or("").trim();
            if l.is_empty() {
                continue;
            }
            if let Some(k) = l.strip_prefix('[').and_then(|r| r.strip_suffix(']')) {
                raws.push(Raw { key: k.trim().to_string(), line, fields: Vec::new() });
                continue;
            }
            let (k, v) = l.split_once('=').ok_or_else(|| format!("ligne {line} : « clé = valeur » attendu"))?;
            let r = raws.last_mut().ok_or_else(|| format!("ligne {line} : champ avant le premier bloc"))?;
            r.fields.push((k.trim().to_string(), v.trim().to_string(), line));
        }
        if raws.len() > MAX_TECHNIQUES {
            return Err(format!("{} techniques, plus que {MAX_TECHNIQUES}", raws.len()));
        }
        let index = |k: &str| raws.iter().position(|r| r.key == k);
        for (i, r) in raws.iter().enumerate() {
            if index(&r.key) != Some(i) {
                return Err(format!("ligne {} : « {} » en double", r.line, r.key));
            }
        }
        let mut techniques = Vec::with_capacity(raws.len());
        for (i, r) in raws.iter().enumerate() {
            let mut t = Technique {
                id: i as u16,
                key: r.key.clone(),
                name: r.key.clone(),
                reference: String::new(),
                pre: Vec::new(),
                any: Vec::new(),
                family: None,
                difficulty: 20.0,
                inventiveness: 1.0,
                need: None,
                body: Body::default(),
                resources: Vec::new(),
                science: Vec::new(),
                effects: Effects::default(),
                energy: None,
                instruments: Vec::new(),
                flags: Flags::default(),
                recipes: Vec::new(),
            };
            for (k, v, line) in &r.fields {
                let line = *line;
                let list = || v.split(',').map(str::trim).filter(|s| !s.is_empty());
                match k.as_str() {
                    "nom" => t.name = v.clone(),
                    "ref" => t.reference = v.clone(),
                    "pre" | "ou" => {
                        let mut group = Vec::new();
                        for p in list() {
                            group.push(index(p).ok_or_else(|| format!("ligne {line} : prérequis inconnu « {p} »"))? as u16);
                        }
                        if k == "pre" {
                            t.pre.extend(group)
                        } else {
                            t.any.push(group)
                        }
                    }
                    "famille" => t.family = Some(family(v).ok_or_else(|| format!("ligne {line} : famille inconnue « {v} »"))?),
                    "diff" => t.difficulty = num(v, line)?,
                    "inv" => t.inventiveness = num(v, line)?,
                    "besoin" => t.need = Some(need(v).ok_or_else(|| format!("ligne {line} : besoin inconnu « {v} »"))?),
                    "corps" => {
                        for c in list() {
                            match c {
                                "mains" => t.body.hands = true,
                                "sans_mains" => t.body.handless = true,
                                "terre" => t.body.milieu = Some(Milieu::Land),
                                "eau" => t.body.milieu = Some(Milieu::Water),
                                "sous_terre" => t.body.milieu = Some(Milieu::Underground),
                                "feu" => t.body.combustion = true,
                                "ciel" => t.body.sky = true,
                                "sedentaire" => t.body.sedentary = true,
                                "peu_oxygene" => t.body.low_oxygen = true,
                                _ => return Err(format!("ligne {line} : exigence du corps inconnue « {c} »")),
                            }
                        }
                    }
                    "res" => {
                        for c in list() {
                            t.resources.push(resource(c).ok_or_else(|| format!("ligne {line} : ressource inconnue « {c} »"))?);
                        }
                    }
                    "sci" => {
                        for c in list() {
                            let (d, x) = c.split_once(':').ok_or_else(|| format!("ligne {line} : « domaine:niveau » attendu"))?;
                            let d = Domain::from_key(d.trim()).ok_or_else(|| format!("ligne {line} : domaine inconnu « {d} »"))?;
                            t.science.push((d, num(x, line)?));
                        }
                    }
                    "effet" => {
                        for c in list() {
                            let (e, x) = c.split_once(':').ok_or_else(|| format!("ligne {line} : « effet:valeur » attendu"))?;
                            set_effect(&mut t.effects, e.trim(), num(x, line)?, line)?;
                        }
                    }
                    "energie" => {
                        let (e, x) = v.split_once(':').ok_or_else(|| format!("ligne {line} : « source:rendement » attendu"))?;
                        let e = Energy::ALL
                            .iter()
                            .copied()
                            .find(|s| s.key() == e.trim())
                            .ok_or_else(|| format!("ligne {line} : énergie inconnue « {e} »"))?;
                        t.energy = Some((e, num(x, line)?));
                    }
                    "instrument" => {
                        for c in list() {
                            t.instruments.push(Domain::from_key(c).ok_or_else(|| format!("ligne {line} : domaine inconnu « {c} »"))?);
                        }
                    }
                    "drapeaux" => {
                        for c in list() {
                            let f = &mut t.flags;
                            match c {
                                "agriculture" => f.agriculture = true,
                                "elevage" => f.herding = true,
                                "ecriture" => f.writing = true,
                                "fossile" => f.fossil = true,
                                "reseau" => f.network = true,
                                "methode" => f.method = true,
                                "imprimerie" => f.printing = true,
                                "monnaie" => f.money = true,
                                "marche" => f.market = true,
                                "industrie" => f.industry = true,
                                "navigation" => f.navigation = true,
                                "metal" => f.metal = true,
                                "droit" => f.law = true,
                                "ecole" => f.school = true,
                                "cavalerie" => f.cavalry = true,
                                "poudre" => f.gunpowder = true,
                                _ => return Err(format!("ligne {line} : drapeau inconnu « {c} »")),
                            }
                        }
                    }
                    "recette" => t.recipes.push(parse_recipe(v, line)?),
                    _ => return Err(format!("ligne {line} : champ inconnu « {k} »")),
                }
            }
            techniques.push(t);
        }
        // Le graphe doit être sans cycle : un prérequis vient avant.
        let mut state = vec![0u8; techniques.len()];
        fn visit(i: usize, ts: &[Technique], st: &mut [u8]) -> Result<(), String> {
            match st[i] {
                1 => return Err(format!("cycle de prérequis par « {} »", ts[i].key)),
                2 => return Ok(()),
                _ => {}
            }
            st[i] = 1;
            for &p in ts[i].pre.iter().chain(ts[i].any.iter().flatten()) {
                visit(p as usize, ts, st)?;
            }
            st[i] = 2;
            Ok(())
        }
        for i in 0..techniques.len() {
            visit(i, &techniques, &mut state)?;
        }
        let pre_masks = techniques.iter().map(Technique::pre_mask).collect();
        let any_masks = techniques
            .iter()
            .map(|t| {
                t.any
                    .iter()
                    .map(|g| {
                        let mut k = Known::default();
                        for &p in g {
                            k.set(p);
                        }
                        k
                    })
                    .collect()
            })
            .collect();
        let mut producers: [Vec<(u16, usize)>; GOODS] = Default::default();
        let mut instrument_of = [Known::default(); DOMAINS];
        let mut families = Vec::new();
        for t in &techniques {
            for (j, r) in t.recipes.iter().enumerate() {
                producers[r.output.index()].push((t.id, j));
            }
            for d in &t.instruments {
                instrument_of[d.index()].set(t.id);
            }
            if let Some(f) = t.family {
                families.push((f, t.id));
            }
        }
        // Le graphe est sans cycle (vérifié plus haut) : récursion mémoïsée.
        fn depth_of(i: usize, ts: &[Technique], memo: &mut [Option<u16>]) -> u16 {
            if let Some(d) = memo[i] {
                return d;
            }
            let mut d = 0;
            for &p in &ts[i].pre {
                d = d.max(depth_of(p as usize, ts, memo) + 1);
            }
            for g in &ts[i].any {
                let m = g.iter().map(|&p| depth_of(p as usize, ts, memo) + 1).min().unwrap_or(0);
                d = d.max(m);
            }
            memo[i] = Some(d);
            d
        }
        let mut memo = vec![None; techniques.len()];
        let depth = (0..techniques.len()).map(|i| depth_of(i, &techniques, &mut memo)).collect();
        Ok(Catalog { techniques, pre_masks, any_masks, producers, instrument_of, families, depth })
    }

    pub fn len(&self) -> usize {
        self.techniques.len()
    }

    pub fn is_empty(&self) -> bool {
        self.techniques.is_empty()
    }

    pub fn id(&self, key: &str) -> Option<u16> {
        self.techniques.iter().position(|t| t.key == key).map(|i| i as u16)
    }

    pub fn get(&self, id: u16) -> &Technique {
        &self.techniques[id as usize]
    }

    /// Les prérequis sont-ils réunis ?
    pub fn ready(&self, id: u16, known: &Known) -> bool {
        known.contains_all(&self.pre_masks[id as usize]) && self.any_masks[id as usize].iter().all(|g| known.intersects(g))
    }

    /// Techniques de l'étape 6 correspondant à des familles connues.
    pub fn from_families(&self, families: &[Family]) -> Known {
        let mut k = Known::default();
        for &(f, id) in &self.families {
            if families.contains(&f) {
                k.set(id);
            }
        }
        k
    }

    /// Effets combinés d'un ensemble de techniques.
    pub fn effects(&self, known: &Known) -> Effects {
        known.iter().fold(Effects::default(), |e, t| e.combine(&self.techniques[t as usize].effects))
    }
}
