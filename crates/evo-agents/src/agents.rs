//! Agents détaillés (niveau 5) : les individus d'un échantillon, posés dans
//! une scène locale, avec un corps, des sens, des besoins et une action
//! courante tirée du lexique commun.
//!
//! La scène est hors histoire. Ses naissances, ses morts et ses départs sont
//! ceux de l'échantillon du niveau 4 ([`crate::sample::Sample`]), donc
//! calibrés sur la population : un prédateur ne tue que lorsque le niveau 4
//! tire une mort par prédation chez une espèce qu'il peut manger ; ses
//! autres chasses échouent. Les comportements ne décident que de la forme :
//! qui chasse qui, qui fuit, qui se nourrit et combien de temps.
//!
//! Le temps des gestes est celui de l'écran (secondes). Le temps de la vie
//! (naissances, morts) avance à `life_years_per_second` années par seconde,
//! réglé pour que la scène montre quelques événements par minute : c'est un
//! choix d'affichage, sans effet sur le monde.
//!
//! Chaque agent décide toutes les `reaction_s` secondes (son système
//! nerveux), voit à `perception_m` (ses sens), et se déplace selon sa
//! locomotion. Ses besoins : énergie (la part du temps passée à se nourrir
//! suit le bilan d'énergie de sa population), fatigue, peur, envie de
//! s'accoupler.

use crate::sample::{LifeEvent, LifeEventKind, Sample, Sex};
use crate::traits::{Diet, Traits};
use evo_core::rng::{rng_for, SimRng, Stream};
use rand::Rng;
use std::collections::HashMap;
use std::f32::consts::{PI, TAU};

/// Lexique commun des actions (fixé par Vision avec Intelligence et
/// Organismes), dans l'ordre de `evo_morph::body::Action`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Act {
    Feed,
    Hunt,
    Flee,
    Hide,
    Mate,
    Lay,
    Care,
    Migrate,
    Fight,
    Communicate,
    Build,
    Rest,
}

impl Act {
    pub const ALL: [Act; 12] = [
        Act::Feed,
        Act::Hunt,
        Act::Flee,
        Act::Hide,
        Act::Mate,
        Act::Lay,
        Act::Care,
        Act::Migrate,
        Act::Fight,
        Act::Communicate,
        Act::Build,
        Act::Rest,
    ];

    pub fn index(self) -> usize {
        self as usize
    }

    /// Allure de l'action, en part de la vitesse de pointe.
    pub fn pace(self) -> f32 {
        match self {
            Act::Flee | Act::Hunt => 1.0,
            Act::Migrate => 0.3,
            Act::Fight => 0.2,
            Act::Mate => 0.15,
            Act::Feed => 0.1,
            Act::Care | Act::Build => 0.05,
            Act::Hide | Act::Lay | Act::Communicate | Act::Rest => 0.0,
        }
    }
}

/// Sort d'un agent dans la scène.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Fate {
    Living,
    /// Arrive du bord (immigrant).
    Arriving,
    /// Quitte la scène (émigrant).
    Leaving,
    /// Doit mourir (niveau 4) : un prédateur le chasse, ou il s'éteint.
    Doomed {
        timer: f32,
        predator: Option<u64>,
    },
    /// Mort : disparaît à la fin du délai.
    Dead {
        timer: f32,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Agent {
    pub species: u16,
    pub member: u32,
    pub genotype: u32,
    pub x: f32,
    pub z: f32,
    pub heading: f32,
    pub speed: f32,
    pub act: Act,
    /// Temps restant de l'action, s.
    pub timer: f32,
    /// Prochaine décision, s.
    pub think: f32,
    pub energy: f32,
    pub fatigue: f32,
    pub fear: f32,
    pub drive: f32,
    /// Cible (proie, partenaire, petit, rival) : clé d'agent.
    pub target: Option<u64>,
    pub fate: Fate,
    pub hidden: bool,
    pub age_s: f32,
    /// Durée cumulée de chaque action, s (calibrage et tests).
    pub time_in: [f32; 12],
}

/// Clé stable d'un agent : espèce et identifiant de l'individu.
pub fn key(species: u16, member: u32) -> u64 {
    ((species as u64) << 32) | member as u64
}

/// Une espèce de la scène et son échantillon.
pub struct SpeciesState {
    pub sample: Sample,
    /// Centre et rayon de son groupe dans la scène, m.
    pub anchor: (f32, f32),
    pub radius_m: f32,
    /// Part du temps à se nourrir, d'après le bilan d'énergie.
    pub feed_share: f32,
}

/// Énergie brûlée au repos, par seconde (1 = réserves pleines).
const BURN: f32 = 1.0 / 150.0;

pub struct Scene {
    pub species: Vec<SpeciesState>,
    pub agents: Vec<Agent>,
    /// Demi-côté de la scène, m.
    pub half_m: f32,
    pub life_years_per_second: f64,
    pub clock: f32,
    index: HashMap<u64, usize>,
    rng: SimRng,
    /// Grille de voisinage : côté d'une case, m, et contenu.
    grid_cell: f32,
    grid: HashMap<(i32, i32), Vec<u32>>,
    /// Événements de vie appliqués (calibrage).
    pub applied: Vec<LifeEvent>,
    /// Prises : proies condamnées attrapées, et proies échappées.
    pub kills: u32,
    pub escapes: u32,
}

/// Part du temps passée à se nourrir : plus la croissance prend d'énergie,
/// plus il faut manger. [Simplification] La relation est linéaire.
fn feed_share(sample: &Sample) -> f32 {
    let r = sample.resident_rates();
    let surplus = if r.energy_kj > 0.0 { (r.birth * r.biomass_cost_kj / r.energy_kj).clamp(0.0, 1.0) } else { 0.0 };
    (0.2 + 0.6 * (1.0 - surplus)).clamp(0.2, 0.8) as f32
}

impl Scene {
    /// Pose les échantillons dans une scène de demi-côté `half_m`.
    /// `events_per_second` règle l'horloge de la vie (affichage).
    pub fn new(samples: Vec<Sample>, half_m: f32, seed: u64, events_per_second: f64) -> Scene {
        let mut rng = rng_for(seed, Stream::Individuals, &[0x5343_454E]);
        let mut species = Vec::new();
        let mut total_rate = 0.0;
        for (k, sample) in samples.into_iter().enumerate() {
            let t = sample.genotypes[0].traits;
            let n = sample.members.len().max(1) as f32;
            let spacing = 12.0 * t.length_m as f32 * (1.0 + 2.0 * (1.0 - t.sociality as f32));
            let radius_m = (0.5 * spacing * n.sqrt()).clamp(0.05, 0.4 * half_m);
            let anchor = if k == 0 || t.diet == Diet::Predator {
                (0.0, 0.0)
            } else {
                let a = rng.random::<f32>() * TAU;
                let r = rng.random::<f32>() * 0.3 * radius_m;
                (r * a.cos(), r * a.sin())
            };
            let r = sample.rates;
            total_rate += n as f64 * (r.birth + r.death + 2.0 * r.emigration);
            species.push(SpeciesState { feed_share: feed_share(&sample), sample, anchor, radius_m });
        }
        // Les ancres se rejoignent : les prédateurs au centre, les autres
        // près d'eux, pour que les uns voient les autres.
        let max_r = species.iter().map(|s| s.radius_m).fold(0.0f32, f32::max);
        for s in species.iter_mut() {
            s.anchor.0 = s.anchor.0.clamp(-max_r, max_r);
            s.anchor.1 = s.anchor.1.clamp(-max_r, max_r);
        }
        let life_years_per_second = if total_rate > 0.0 { events_per_second / total_rate } else { 0.0 };
        let mut scene = Scene {
            species,
            agents: Vec::new(),
            half_m,
            life_years_per_second,
            clock: 0.0,
            index: HashMap::new(),
            rng,
            grid_cell: 1.0,
            grid: HashMap::new(),
            applied: Vec::new(),
            kills: 0,
            escapes: 0,
        };
        for k in 0..scene.species.len() {
            let members: Vec<_> = scene.species[k].sample.members.clone();
            for m in members {
                let s = &scene.species[k];
                let rk = s.sample.radius_km.max(1e-9);
                let x = s.anchor.0 + (m.position_km[0] / rk) as f32 * s.radius_m;
                let z = s.anchor.1 + (m.position_km[1] / rk) as f32 * s.radius_m;
                scene.spawn(k as u16, m.id, m.genotype, x, z, Fate::Living);
            }
        }
        scene.reindex();
        scene
    }

    fn spawn(&mut self, species: u16, member: u32, genotype: u32, x: f32, z: f32, fate: Fate) {
        let heading = self.rng.random::<f32>() * TAU;
        let think = self.rng.random::<f32>() * 0.5;
        let energy = 0.5 + 0.5 * self.rng.random::<f32>();
        let drive = self.rng.random::<f32>() * 0.5;
        let time_in = [0.0; 12];
        self.agents.push(Agent {
            species,
            member,
            genotype,
            x,
            z,
            heading,
            speed: 0.0,
            act: Act::Feed,
            timer: 0.0,
            think,
            energy,
            fatigue: 0.0,
            fear: 0.0,
            drive,
            target: None,
            fate,
            hidden: false,
            age_s: 0.0,
            time_in,
        });
    }

    fn reindex(&mut self) {
        self.index.clear();
        for (i, a) in self.agents.iter().enumerate() {
            self.index.insert(key(a.species, a.member), i);
        }
    }

    pub fn traits_of(&self, a: &Agent) -> &Traits {
        &self.species[a.species as usize].sample.genotypes[a.genotype as usize].traits
    }

    pub fn agent(&self, k: u64) -> Option<&Agent> {
        self.index.get(&k).map(|&i| &self.agents[i])
    }

    /// Avance la scène de `dt` secondes.
    pub fn step(&mut self, dt: f32) {
        let dt = dt.clamp(0.0, 0.25);
        self.clock += dt;
        // 1. La vie : naissances, morts, départs et arrivées du niveau 4.
        let years = dt as f64 * self.life_years_per_second;
        for k in 0..self.species.len() {
            self.species[k].sample.advance(years);
            let events = self.species[k].sample.take_events();
            for e in events {
                self.apply(k as u16, e);
            }
        }
        // 2. Voisinage.
        self.build_grid();
        // 3. Décisions.
        for i in 0..self.agents.len() {
            self.agents[i].think -= dt;
            self.agents[i].timer -= dt;
            if self.agents[i].think <= 0.0 {
                self.decide(i);
                let t = self.traits_of(&self.agents[i]).reaction_s as f32;
                self.agents[i].think = t * (0.8 + 0.4 * self.rng.random::<f32>());
            }
        }
        // 4. Mouvement, besoins, prises.
        for i in 0..self.agents.len() {
            self.update(i, dt);
        }
        // 5. Disparitions.
        let before = self.agents.len();
        let half = self.half_m;
        let species = &self.species;
        self.agents.retain(|a| match a.fate {
            Fate::Dead { timer } => timer > 0.0,
            Fate::Leaving => {
                let s = &species[a.species as usize];
                (a.x - s.anchor.0).hypot(a.z - s.anchor.1) < 2.5 * s.radius_m && a.x.abs() < half && a.z.abs() < half
            }
            _ => true,
        });
        if self.agents.len() != before {
            self.reindex();
        }
    }

    /// Applique un événement du niveau 4.
    fn apply(&mut self, species: u16, e: LifeEvent) {
        self.applied.push(e);
        match e.kind {
            LifeEventKind::Birth { child, mother, father } => {
                let s = &self.species[species as usize];
                let Some(m) = s.sample.members.iter().find(|m| m.id == child).copied() else { return };
                let mi = self.index.get(&key(species, mother)).copied();
                let (x, z) = match mi {
                    Some(i) => (self.agents[i].x, self.agents[i].z),
                    None => (s.anchor.0, s.anchor.1),
                };
                let care = self.traits_of_genotype(species, m.genotype).care as f32;
                let nervous = self.traits_of_genotype(species, m.genotype).nervous as f32;
                if let Some(i) = mi {
                    let fi = self.index.get(&key(species, father)).copied();
                    let a = &mut self.agents[i];
                    a.target = Some(key(species, child));
                    a.drive = 0.0;
                    if care > 0.5 && nervous > 0.5 {
                        a.act = Act::Build;
                        a.timer = 3.0;
                    } else {
                        a.act = Act::Lay;
                        a.timer = 2.0;
                    }
                    a.think = a.timer;
                    let (mx, mz) = (a.x, a.z);
                    let reach = self.traits_of_genotype(species, m.genotype).perception_m as f32;
                    let fi = fi.filter(|&f| (self.agents[f].x - mx).hypot(self.agents[f].z - mz) < reach);
                    if let Some(f) = fi {
                        let b = &mut self.agents[f];
                        b.act = Act::Mate;
                        b.timer = 2.0;
                        b.think = 2.0;
                        b.drive = 0.0;
                        b.target = Some(key(species, mother));
                    }
                }
                let len = self.traits_of_genotype(species, m.genotype).length_m as f32;
                let dx = (self.rng.random::<f32>() - 0.5) * len;
                let dz = (self.rng.random::<f32>() - 0.5) * len;
                self.spawn(species, child, m.genotype, x + dx, z + dz, Fate::Living);
                let n = self.agents.len() - 1;
                self.agents[n].energy = 1.0;
                self.agents[n].act = Act::Rest;
                self.agents[n].timer = 2.0;
                self.agents[n].think = 2.0;
                if let Some(mi) = mi {
                    self.agents[n].target = Some(key(species, self.agents[mi].member));
                }
                self.index.insert(key(species, child), n);
            }
            LifeEventKind::Death { id, predation } => {
                let Some(&i) = self.index.get(&key(species, id)) else { return };
                let predator = if predation { self.find_predator(i) } else { None };
                if let Some(p) = predator {
                    let pk = key(self.agents[p].species, self.agents[p].member);
                    let vk = key(species, id);
                    let a = &mut self.agents[p];
                    a.act = Act::Hunt;
                    a.target = Some(vk);
                    a.timer = 20.0;
                    a.think = 20.0;
                    self.agents[i].fate = Fate::Doomed { timer: 20.0, predator: Some(pk) };
                } else {
                    let v = &mut self.agents[i];
                    v.fate = Fate::Doomed { timer: 2.0, predator: None };
                    v.act = Act::Rest;
                    v.timer = 2.0;
                    v.think = 2.0;
                }
            }
            LifeEventKind::Emigration { id } => {
                let Some(&i) = self.index.get(&key(species, id)) else { return };
                let s = &self.species[species as usize];
                let a = &mut self.agents[i];
                a.fate = Fate::Leaving;
                a.act = Act::Migrate;
                a.timer = f32::INFINITY;
                a.heading = (a.z - s.anchor.1).atan2(a.x - s.anchor.0);
            }
            LifeEventKind::Immigration { id } => {
                let s = &self.species[species as usize];
                let Some(m) = s.sample.members.iter().find(|m| m.id == id).copied() else { return };
                let ang = self.rng.random::<f32>() * TAU;
                let r = 2.0 * s.radius_m;
                let (x, z) = (s.anchor.0 + r * ang.cos(), s.anchor.1 + r * ang.sin());
                let (x, z) = (x.clamp(-self.half_m, self.half_m), z.clamp(-self.half_m, self.half_m));
                self.spawn(species, id, m.genotype, x, z, Fate::Arriving);
                let n = self.agents.len() - 1;
                let a = &mut self.agents[n];
                a.act = Act::Migrate;
                a.heading = ang + PI;
                a.timer = f32::INFINITY;
                self.index.insert(key(species, id), n);
            }
        }
    }

    fn traits_of_genotype(&self, species: u16, genotype: u32) -> &Traits {
        &self.species[species as usize].sample.genotypes[genotype as usize].traits
    }

    /// Le prédateur libre le plus proche qui peut manger l'agent `i`.
    fn find_predator(&self, i: usize) -> Option<usize> {
        let v = &self.agents[i];
        let len = self.traits_of(v).length_m;
        self.agents
            .iter()
            .enumerate()
            .filter(|(_, p)| {
                let t = self.traits_of(p);
                t.can_eat(len) && t.moves() && matches!(p.fate, Fate::Living | Fate::Arriving) && !matches!(p.act, Act::Hunt)
            })
            .min_by(|a, b| {
                let da = (a.1.x - v.x).hypot(a.1.z - v.z);
                let db = (b.1.x - v.x).hypot(b.1.z - v.z);
                da.total_cmp(&db)
            })
            .map(|(j, _)| j)
    }

    fn build_grid(&mut self) {
        // Côté des cases : la médiane des portées de perception.
        let mut per: Vec<f32> = self.species.iter().map(|s| s.sample.genotypes[0].traits.perception_m as f32).collect();
        per.sort_by(f32::total_cmp);
        self.grid_cell = per.get(per.len() / 2).copied().unwrap_or(1.0).clamp(0.01, 50.0);
        self.grid.clear();
        for (i, a) in self.agents.iter().enumerate() {
            let c = ((a.x / self.grid_cell).floor() as i32, (a.z / self.grid_cell).floor() as i32);
            self.grid.entry(c).or_default().push(i as u32);
        }
    }

    /// Agents à moins de `r` mètres de (x, z), sauf `me`.
    fn around(&self, x: f32, z: f32, r: f32, me: usize) -> Vec<u32> {
        let reach = ((r / self.grid_cell).ceil() as i32).clamp(1, 6);
        let (cx, cz) = ((x / self.grid_cell).floor() as i32, (z / self.grid_cell).floor() as i32);
        let mut out = Vec::new();
        for dx in -reach..=reach {
            for dz in -reach..=reach {
                if let Some(v) = self.grid.get(&(cx + dx, cz + dz)) {
                    for &j in v {
                        if j as usize != me {
                            let o = &self.agents[j as usize];
                            if (o.x - x).hypot(o.z - z) <= r {
                                out.push(j);
                            }
                        }
                    }
                }
            }
        }
        out.sort_unstable();
        out
    }

    /// Choix de l'action d'un agent.
    fn decide(&mut self, i: usize) {
        let me = self.agents[i];
        if matches!(me.fate, Fate::Dead { .. }) {
            return;
        }
        let t = *self.traits_of(&me);
        let len = t.length_m as f32;
        let per = t.perception_m as f32;
        let near = self.around(me.x, me.z, per, i);
        // Menace : un prédateur qui peut le manger, en vue (un individu
        // caché ne se voit qu'à une longueur).
        // Un prédateur repu qui passe n'inquiète que de près ; un prédateur
        // en chasse, dès qu'il est en vue.
        let threat = near.iter().copied().find(|&j| {
            let o = &self.agents[j as usize];
            let ot = self.traits_of(o);
            let d = (o.x - me.x).hypot(o.z - me.z);
            ot.can_eat(t.length_m)
                && !matches!(o.fate, Fate::Dead { .. })
                && (o.target == Some(key(me.species, me.member))
                    || (matches!(o.act, Act::Hunt) && d < 0.5 * per)
                    || d < 2.0 * ot.length_m as f32)
        });
        if let Some(j) = threat {
            let p = self.agents[j as usize];
            let pt = *self.traits_of(&p);
            let d = (p.x - me.x).hypot(p.z - me.z);
            let doomed_by_it = matches!(me.fate, Fate::Doomed { predator: Some(k), .. } if k == key(p.species, p.member));
            let a = &mut self.agents[i];
            a.fear = 1.0;
            if d < len.max(pt.length_m as f32 * 0.5) && t.defence > 0.4 && !doomed_by_it {
                a.act = Act::Fight;
                a.timer = 1.5;
            } else if !t.moves() || (t.sprint_m_s < 0.7 * pt.sprint_m_s && t.length_m < 0.3 * pt.length_m && !doomed_by_it) {
                a.act = Act::Hide;
                a.hidden = true;
                a.timer = 3.0;
            } else {
                a.act = Act::Flee;
                a.hidden = false;
                a.timer = 2.0;
                a.heading = (me.z - p.z).atan2(me.x - p.x);
            }
            // Cri d'alarme : les congénères proches prennent peur.
            if t.sociality > 0.4 && t.nervous > 0.3 && me.fear < 0.5 {
                self.agents[i].act = Act::Communicate;
                self.agents[i].timer = 0.6;
                for &k in &near {
                    let o = &mut self.agents[k as usize];
                    if o.species == me.species {
                        o.fear = o.fear.max(0.8);
                        o.think = o.think.min(0.1);
                    }
                }
            }
            return;
        }
        if me.timer > 0.0 && !matches!(me.act, Act::Feed | Act::Rest) {
            return;
        }
        if matches!(me.fate, Fate::Leaving | Fate::Arriving) || (matches!(me.act, Act::Hunt) && me.timer > 0.0) {
            return;
        }
        let a = &mut self.agents[i];
        a.hidden = false;
        // Petit qui suit sa mère.
        if a.age_s < 20.0 && a.target.is_some() && t.care > 0.3 {
            a.act = Act::Migrate;
            a.timer = 1.0;
            return;
        }
        // Parent qui soigne.
        if t.care > 0.3 && a.target.is_some() && matches!(a.act, Act::Lay | Act::Mate | Act::Build | Act::Care) && a.age_s > 0.0 {
            if matches!(a.act, Act::Care) && a.timer <= 0.0 {
                a.target = None;
            } else if !matches!(a.act, Act::Care) {
                a.act = Act::Care;
                a.timer = 10.0 * t.care as f32;
                return;
            }
        }
        if a.fatigue > 0.85 || (matches!(a.act, Act::Rest) && a.fatigue > 0.2 && a.timer > 0.0) {
            a.act = Act::Rest;
            a.timer = a.timer.max(3.0);
            return;
        }
        let share = self.species[me.species as usize].feed_share;
        let hungry = a.energy < 0.4 || (matches!(a.act, Act::Feed | Act::Hunt) && a.energy < 0.9);
        if hungry {
            if t.diet == Diet::Predator {
                // Chasse : la proie mangeable la plus proche en vue.
                let prey = near
                    .iter()
                    .copied()
                    .filter(|&j| {
                        let o = &self.agents[j as usize];
                        let ot = self.traits_of(o);
                        t.can_eat(ot.length_m) && (!o.hidden || (o.x - me.x).hypot(o.z - me.z) < len)
                    })
                    .min_by(|&x, &y| {
                        let dx = (self.agents[x as usize].x - me.x).hypot(self.agents[x as usize].z - me.z);
                        let dy = (self.agents[y as usize].x - me.x).hypot(self.agents[y as usize].z - me.z);
                        dx.total_cmp(&dy)
                    });
                let a = &mut self.agents[i];
                match prey {
                    Some(j) => {
                        let o = self.agents[j as usize];
                        let a = &mut self.agents[i];
                        a.act = Act::Hunt;
                        a.target = Some(key(o.species, o.member));
                        a.timer = 8.0;
                    }
                    None => {
                        a.act = Act::Migrate;
                        a.timer = 4.0;
                        a.heading += self.rng.random::<f32>() - 0.5;
                    }
                }
            } else {
                a.act = Act::Feed;
                a.timer = 2.0;
            }
            return;
        }
        let _ = share;
        // Accouplement : partenaire du sexe opposé en vue.
        let sex = self.species[me.species as usize].sample.members.iter().find(|m| m.id == me.member).map(|m| m.sex);
        if t.sexual && me.drive > 0.8 && me.energy > 0.6 && me.age_s > 30.0 {
            let want = match sex {
                Some(Sex::Female) => Some(Sex::Male),
                Some(Sex::Male) => Some(Sex::Female),
                _ => None,
            };
            if let Some(want) = want {
                let members = &self.species[me.species as usize].sample.members;
                let partner = near.iter().copied().find(|&j| {
                    let o = &self.agents[j as usize];
                    o.species == me.species && members.iter().any(|m| m.id == o.member && m.sex == want)
                });
                let rival = near.iter().copied().find(|&j| {
                    let o = &self.agents[j as usize];
                    o.species == me.species
                        && matches!(o.act, Act::Mate)
                        && Some(want) != members.iter().find(|m| m.id == o.member).map(|m| m.sex)
                });
                if let (Some(_), true) = (rival, t.nervous > 0.2 && self.rng.random::<f32>() < 0.3) {
                    let r = rival.unwrap_or(0) as usize;
                    let rk = key(self.agents[r].species, self.agents[r].member);
                    let a = &mut self.agents[i];
                    a.act = Act::Fight;
                    a.target = Some(rk);
                    a.timer = 2.0;
                    let b = &mut self.agents[r];
                    b.act = Act::Fight;
                    b.timer = 2.0;
                    return;
                }
                if let Some(j) = partner {
                    let o = self.agents[j as usize];
                    let a = &mut self.agents[i];
                    a.act = Act::Mate;
                    a.target = Some(key(o.species, o.member));
                    a.timer = 3.0;
                    a.drive = 0.0;
                    return;
                }
            }
        }
        // Le reste du temps : se nourrir, se reposer, suivre le groupe,
        // communiquer.
        let r: f32 = self.rng.random();
        let a = &mut self.agents[i];
        let (act, timer) = if !t.moves() {
            if r < 0.85 {
                (Act::Feed, 4.0)
            } else {
                (Act::Rest, 3.0)
            }
        } else if t.sociality > 0.5 && t.nervous > 0.3 && r < 0.06 {
            (Act::Communicate, 1.0)
        } else if r < 0.15 {
            (Act::Rest, 3.0)
        } else if r < 0.35 {
            (Act::Migrate, 3.0)
        } else {
            (Act::Feed, 3.0)
        };
        a.act = act;
        a.timer = timer;
        if matches!(act, Act::Migrate | Act::Feed) {
            a.heading += (self.rng.random::<f32>() - 0.5) * 1.6;
        }
    }

    /// Mouvement, besoins et contacts d'un agent.
    fn update(&mut self, i: usize, dt: f32) {
        let me = self.agents[i];
        let t = *self.traits_of(&me);
        let s = &self.species[me.species as usize];
        let (anchor, radius) = (s.anchor, s.radius_m);
        let share = s.feed_share;
        let len = t.length_m as f32;
        let sprint = t.sprint_m_s as f32;
        let mut heading = me.heading;
        let steer = |h: &mut f32, gx: f32, gz: f32, rate: f32| {
            let want = (gz - me.z).atan2(gx - me.x);
            let mut d = want - *h;
            while d > PI {
                d -= TAU;
            }
            while d < -PI {
                d += TAU;
            }
            *h += d.clamp(-rate * dt, rate * dt);
        };
        let target = me.target.and_then(|k| self.index.get(&k).copied());
        let mut caught = None;
        match me.act {
            Act::Hunt => {
                if let Some(j) = target {
                    let o = self.agents[j];
                    steer(&mut heading, o.x, o.z, 6.0);
                    if (o.x - me.x).hypot(o.z - me.z) < len.max(1e-4) {
                        caught = Some(j);
                    }
                }
            }
            Act::Mate | Act::Fight | Act::Care => {
                if let Some(j) = target {
                    let o = self.agents[j];
                    if (o.x - me.x).hypot(o.z - me.z) > len {
                        steer(&mut heading, o.x, o.z, 4.0);
                    }
                }
            }
            Act::Migrate if matches!(me.fate, Fate::Arriving) => {
                steer(&mut heading, anchor.0, anchor.1, 2.0);
            }
            Act::Migrate if me.age_s < 20.0 && target.is_some() => {
                if let Some(j) = target {
                    let o = self.agents[j];
                    steer(&mut heading, o.x, o.z, 4.0);
                }
            }
            // Retour vers le groupe au-delà de son rayon.
            Act::Migrate | Act::Feed if !matches!(me.fate, Fate::Leaving) && (me.x - anchor.0).hypot(me.z - anchor.1) > radius => {
                steer(&mut heading, anchor.0, anchor.1, 1.5);
            }
            _ => {}
        }
        let mut pace = me.act.pace();
        if matches!(me.act, Act::Care | Act::Mate)
            && target.is_some_and(|j| (self.agents[j].x - me.x).hypot(self.agents[j].z - me.z) > 2.0 * len)
        {
            pace = 0.3;
        }
        if me.age_s < 20.0 && matches!(me.act, Act::Migrate) && target.is_some() {
            pace = 0.4;
        }
        let want = pace * sprint;
        let a = &mut self.agents[i];
        a.heading = heading;
        a.speed += (want - a.speed) * (dt * 4.0).min(1.0);
        let (sn, cs) = a.heading.sin_cos();
        a.x += cs * a.speed * dt;
        a.z += sn * a.speed * dt;
        if !matches!(a.fate, Fate::Leaving) {
            a.x = a.x.clamp(-self.half_m, self.half_m);
            a.z = a.z.clamp(-self.half_m, self.half_m);
        } else if a.x.abs() >= self.half_m || a.z.abs() >= self.half_m {
            // Sorti du carré : il disparaîtra.
            a.x = a.x.clamp(-self.half_m * 1.01, self.half_m * 1.01);
        }
        if matches!(a.fate, Fate::Arriving) && (a.x - anchor.0).hypot(a.z - anchor.1) < radius {
            a.fate = Fate::Living;
            a.timer = 0.0;
            a.act = Act::Feed;
        }
        // Besoins.
        let effort = if sprint > 0.0 { a.speed / sprint } else { 0.0 };
        a.energy -= BURN * (1.0 + 2.0 * effort) * dt;
        if matches!(a.act, Act::Feed) && t.diet != Diet::Predator {
            a.energy += BURN / share * dt;
        }
        a.energy = a.energy.clamp(0.0, 1.0);
        a.fatigue = if matches!(a.act, Act::Rest) { a.fatigue - dt / 20.0 } else { a.fatigue + effort * effort * dt / 30.0 + dt / 600.0 };
        a.fatigue = a.fatigue.clamp(0.0, 1.0);
        a.fear = (a.fear - dt / 5.0).max(0.0);
        if t.sexual {
            a.drive = (a.drive + dt / 120.0).min(1.0);
        }
        a.age_s += dt;
        a.time_in[a.act.index()] += dt;
        if let Fate::Doomed { timer, predator } = a.fate {
            let timer = timer - dt;
            a.fate = if timer <= 0.0 { Fate::Dead { timer: 1.0 } } else { Fate::Doomed { timer, predator } };
            if timer <= 0.0 {
                a.act = Act::Rest;
            }
        } else if let Fate::Dead { timer } = a.fate {
            a.fate = Fate::Dead { timer: timer - dt };
        }
        // Prise : seule une proie condamnée par le niveau 4 meurt ; les
        // autres s'échappent.
        if let Some(j) = caught {
            let mk = key(me.species, me.member);
            let doomed_for_me = matches!(self.agents[j].fate, Fate::Doomed { predator: Some(k), .. } if k == mk);
            if doomed_for_me {
                self.kills += 1;
                self.agents[j].fate = Fate::Dead { timer: 1.0 };
                self.agents[j].act = Act::Rest;
                let a = &mut self.agents[i];
                a.act = Act::Feed;
                a.timer = 6.0;
                a.think = 6.0;
                a.energy = (a.energy + 0.6).min(1.0);
                a.target = None;
            } else {
                self.escapes += 1;
                let p = (me.x, me.z);
                let o = &mut self.agents[j];
                o.act = Act::Flee;
                o.fear = 1.0;
                o.timer = 2.0;
                o.heading = (o.z - p.1).atan2(o.x - p.0);
                let a = &mut self.agents[i];
                a.act = Act::Rest;
                a.timer = 2.0;
                a.think = 2.0;
                a.target = None;
            }
        }
    }

    /// Action courante de chaque agent : ce que le moteur publie dans le
    /// lexique commun.
    pub fn actions(&self) -> impl Iterator<Item = (u64, Act)> + '_ {
        self.agents.iter().map(|a| (key(a.species, a.member), a.act))
    }
}
