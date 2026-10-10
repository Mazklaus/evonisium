//! Individus échantillonnés (niveau 4 de l'échelle commune) : quelques
//! centaines d'individus d'une population, avec génomes complets,
//! reproduction réelle et développement.
//!
//! Règle du document Vision : « regarder ne doit pas changer l'histoire ».
//! Un échantillon se tire d'un `&World` (lecture seule) avec son propre flux
//! de hasard (graine de la partie, cellule, lignée, date), et rien de ce qu'il
//! fait ne remonte au monde. Ses règles sont calibrées pour reproduire les
//! taux de la population :
//!
//! - naissances : taux par tête b de la population ; un individu de valeur
//!   sélective w_i donne naissance au taux b·w_i/w̄, si bien que l'échantillon
//!   entier naît exactement au taux N·b, quelle que soit sa composition ;
//! - morts : taux d (mortalité totale de la population), dont la part due
//!   aux prédateurs ;
//! - déplacements : chaque individu quitte la cellule au taux de migration m
//!   du moteur, et des immigrants entrent au même taux (voisines
//!   supposées semblables).
//!
//! Les naissances sont réelles : les parents sont tirés selon leur valeur
//! sélective, le génome de l'enfant recombine ceux des parents (sexués) puis
//! mute selon le modèle de mutation du moteur, et chaque génome distinct est
//! développé une fois (phénotype, plan de construction, traits), puis mis en
//! cache. Sa valeur sélective vient des taux de croissance qu'il aurait dans
//! la cellule (s = (r_mutant − r_résident) × T, comme le moteur).
//!
//! [Simplification] Le taux de naissance du moteur est un taux de production
//! de biomasse par mole de carbone ; on le prend comme taux de naissance par
//! tête, ce qui vaut pour une structure de tailles stable. L'échantillon est
//! tenu entre un quart et deux fois sa taille nominale : au-delà il est
//! éclairci au hasard, en deçà il recrute des individus de la population ;
//! ni l'un ni l'autre ne compte comme naissance ou mort. La variation
//! présente au tirage est celle de quelques générations de mutations.

use crate::traits::Traits;
use evo_core::math::Det;
use evo_core::rng::{rng_for, SimRng, Stream};
use evo_genetics::{mutation::mutate, ChangedElement, Genome, GenomeChangeCause, MutationParams};
use evo_life::body::{body_plan, BodyPlan};
use evo_life::{growth_rates, selection_coefficient, CellContext, Conditions, GrowthRates, Phenotype, Physiology, Population};
use evo_planet::WaterChemistry;
use evo_sim::World;
use rand::Rng;
use rand_distr::{Distribution, Exp, Poisson};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

/// Taille nominale d'un échantillon (« quelques centaines d'individus par
/// espèce »).
pub const SAMPLE_SIZE: usize = 300;

/// Générations de mutations portées par un individu au tirage.
pub const STANDING_GENERATIONS: f64 = 4.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Sex {
    Female,
    Male,
    /// Organisme asexué.
    None,
}

/// Taux par tête de la population, an⁻¹.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PopRates {
    pub birth: f64,
    pub death: f64,
    /// Part de `death` due aux prédateurs.
    pub predation: f64,
    pub emigration: f64,
}

/// Un génome distinct et ce que son développement en fait.
#[derive(Clone, Debug)]
pub struct Genotype {
    pub genome: Arc<Genome>,
    pub phenotype: Arc<Phenotype>,
    pub plan: Arc<BodyPlan>,
    pub traits: Traits,
    /// Valeur sélective relative au génome de la population, ≥ 0.
    pub fitness: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Member {
    pub id: u32,
    pub genotype: u32,
    pub sex: Sex,
    pub born_years: f64,
    /// Parents dans l'échantillon (`u32::MAX` : tiré de la population).
    pub parents: [u32; 2],
    /// Position dans la cellule, km, depuis son centre (plan tangent).
    pub position_km: [f64; 2],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LifeEventKind {
    Birth { child: u32, mother: u32, father: u32 },
    Death { id: u32, predation: bool },
    Emigration { id: u32 },
    Immigration { id: u32 },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LifeEvent {
    pub years: f64,
    pub kind: LifeEventKind,
}

/// Compteurs de calibrage : événements et exposition (somme de N·dt).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Counters {
    pub births: u64,
    pub deaths: u64,
    pub predation_deaths: u64,
    pub emigrations: u64,
    pub immigrations: u64,
    /// Individus-années vécus dans l'échantillon.
    pub exposure: f64,
    /// Éclaircies et recrutements (hors démographie).
    pub thinned: u64,
    pub recruited: u64,
}

/// Ce qu'il faut pour développer et juger un nouveau génome.
#[derive(Clone, Debug)]
pub struct Development {
    pub physio: Physiology,
    pub mutation: MutationParams,
    pub conditions: Conditions,
    pub chemistry: WaterChemistry,
    pub resident: GrowthRates,
    pub aquatic: bool,
}

impl Development {
    /// Développe un génome : phénotype, plan, traits et valeur sélective.
    pub fn develop(&self, genome: Arc<Genome>) -> Genotype {
        let phenotype = Phenotype::from_genome(&genome, &self.physio);
        let rates = growth_rates(&phenotype, &self.conditions, &self.chemistry, &self.physio);
        let s = selection_coefficient(&rates, &self.resident, &self.physio);
        let fitness = if s.is_finite() { (1.0 + s).clamp(0.0, 2.0) } else { 0.0 };
        self.finish(genome, Arc::new(phenotype), &rates, fitness)
    }

    fn finish(&self, genome: Arc<Genome>, phenotype: Arc<Phenotype>, rates: &GrowthRates, fitness: f64) -> Genotype {
        let plan = Arc::new(body_plan(&genome, &phenotype, &self.physio));
        let traits = Traits::of(&phenotype, &plan, rates, self.aquatic);
        Genotype { genome, phenotype, plan, traits, fitness }
    }
}

/// Échantillon d'une population.
#[derive(Clone, Debug)]
pub struct Sample {
    pub bio_cell: u32,
    pub lineage: u32,
    pub signature: u32,
    /// Date de l'état publié dont il est tiré, années.
    pub date_years: f64,
    /// Horloge propre de l'échantillon (part de `date_years`), années.
    pub years: f64,
    /// Effectif de la population entière (individus).
    pub census: f64,
    pub rates: PopRates,
    /// Rayon du disque équivalent à la cellule, km.
    pub radius_km: f64,
    pub nominal: usize,
    pub genotypes: Vec<Genotype>,
    pub members: Vec<Member>,
    pub events: Vec<LifeEvent>,
    pub counters: Counters,
    /// Génome de la population voisine qui fournit les immigrants.
    immigrant_genome: Arc<Genome>,
    pub development: Development,
    cache: HashMap<u64, u32>,
    next_id: u32,
    rng: SimRng,
}

/// Empreinte d'un génome (clé du cache de développement).
pub fn genome_key(g: &Genome) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    g.genes.len().hash(&mut h);
    for gene in &g.genes {
        gene.domain.family.hash(&mut h);
        gene.functional.hash(&mut h);
        for x in [gene.domain.efficiency, gene.domain.affinity, gene.domain.t_opt_k, gene.domain.t_width_k, gene.domain.absorption_nm] {
            x.to_bits().hash(&mut h);
        }
    }
    g.marker.hash(&mut h);
    g.organelles.len().hash(&mut h);
    for o in &g.organelles {
        o.origin_lineage.hash(&mut h);
        o.genes.len().hash(&mut h);
        for gene in &o.genes {
            gene.domain.family.hash(&mut h);
            gene.functional.hash(&mut h);
            gene.domain.efficiency.to_bits().hash(&mut h);
        }
    }
    h.finish()
}

/// Effectif d'une population en individus (et non en cellules).
pub fn individuals_of(p: &Population, physio: &Physiology) -> f64 {
    let cells_per_body = (p.phenotype.cell_volume() * p.phenotype.cells()).max(1e-30);
    p.biomass / (physio.carbon_per_cell * cells_per_body)
}

/// Une population a-t-elle des individus au sens du niveau 4 ? Les microbes
/// restent des guildes (document Vision, « Vue microscope ») : seuls les
/// multicellulaires sont échantillonnés.
pub fn has_individuals(p: &Population) -> bool {
    p.phenotype.is_multicellular()
}

/// Taux par tête d'une population d'après sa dernière évaluation.
pub fn pop_rates(p: &Population, migration_rate: f64) -> PopRates {
    let r = &p.rates;
    let death = r.mortality.max(0.0);
    PopRates {
        birth: r.birth.max(0.0),
        death,
        predation: if death > 0.0 { (r.predation / death).clamp(0.0, 1.0) } else { 0.0 },
        emigration: migration_rate.max(0.0),
    }
}

impl Sample {
    /// Tire un échantillon de la population `pop` de la cellule du vivant
    /// `bio_cell`, à la date du monde. Lecture seule : le monde n'en sait
    /// rien.
    pub fn draw(world: &World, bio_cell: usize, pop: usize, size: usize) -> Option<Sample> {
        let p = world.communities.get(bio_cell)?.get(pop)?;
        let env = &world.bio.env[bio_cell];
        let cfg = &world.config;
        let ctx = CellContext { env, light_biomass_per_m2: cfg.light_biomass_per_m2 };
        let conditions = ctx.conditions_of(&world.communities[bio_cell], &cfg.physiology);
        let development = Development {
            physio: cfg.physiology.clone(),
            mutation: cfg.mutation.clone(),
            conditions,
            chemistry: world.chemistry[bio_cell],
            resident: p.rates,
            aquatic: env.is_ocean || env.water_volume_m3 > 0.0,
        };
        // Immigrants : la population de même espèce la plus abondante des
        // cellules voisines, sinon celle-ci.
        let signature = p.signature();
        let immigrant_genome = world
            .bio
            .grid
            .neighbours_of(bio_cell)
            .filter_map(|n| {
                world.communities[n].iter().filter(|q| q.signature() == signature).max_by(|a, b| a.biomass.total_cmp(&b.biomass))
            })
            .max_by(|a, b| a.biomass.total_cmp(&b.biomass))
            .map(|q| q.genome.clone())
            .unwrap_or_else(|| p.genome.clone());
        let years = world.years();
        let seed = world.config.seed;
        let rng = rng_for(seed, Stream::Individuals, &[bio_cell as u64, p.lineage as u64, years.to_bits()]);
        let resident = development.finish(p.genome.clone(), p.phenotype.clone(), &p.rates, 1.0);
        let radius_km = (env.area_m2 / std::f64::consts::PI).sqrt() / 1000.0;
        let mut s = Sample {
            bio_cell: bio_cell as u32,
            lineage: p.lineage,
            signature,
            date_years: years,
            years,
            census: individuals_of(p, &cfg.physiology),
            rates: pop_rates(p, cfg.migration_rate),
            radius_km,
            nominal: size.max(1),
            genotypes: Vec::new(),
            members: Vec::new(),
            events: Vec::new(),
            counters: Counters::default(),
            immigrant_genome,
            development,
            cache: HashMap::new(),
            next_id: 0,
            rng,
        };
        s.cache.insert(genome_key(&p.genome), 0);
        s.genotypes.push(resident);
        let n = (size as f64).min(s.census.max(1.0)).round().max(1.0) as usize;
        let ages = Exp::new(s.rates.birth.max(1e-9)).ok();
        for _ in 0..n {
            let genome = s.standing_variant(&p.genome.clone());
            let genotype = s.genotype_of(genome);
            let age = ages.map(|e| e.sample(&mut s.rng)).unwrap_or(0.0);
            let position_km = s.uniform_in_disk();
            let sex = s.draw_sex(genotype);
            let id = s.new_id();
            s.members.push(Member { id, genotype, sex, born_years: years - age, parents: [u32::MAX; 2], position_km });
        }
        Some(s)
    }

    /// Échantillon d'essai sans monde : un génome, ses taux, son milieu
    /// (tests, banc d'essai du client).
    pub fn synthetic(genome: Arc<Genome>, development: Development, rates: PopRates, size: usize, radius_km: f64, seed: u64) -> Sample {
        let phenotype = Arc::new(Phenotype::from_genome(&genome, &development.physio));
        let resident = development.resident;
        let g0 = development.finish(genome.clone(), phenotype, &resident, 1.0);
        let mut s = Sample {
            bio_cell: 0,
            lineage: 0,
            signature: g0.phenotype.signature,
            date_years: 0.0,
            years: 0.0,
            census: 1e9,
            rates,
            radius_km,
            nominal: size.max(1),
            genotypes: vec![g0],
            members: Vec::new(),
            events: Vec::new(),
            counters: Counters::default(),
            immigrant_genome: genome.clone(),
            development,
            cache: HashMap::new(),
            next_id: 0,
            rng: rng_for(seed, Stream::Individuals, &[0x5359_4E54]),
        };
        s.cache.insert(genome_key(&genome), 0);
        let ages = Exp::new(rates.birth.max(1e-9)).ok();
        for _ in 0..size {
            let genome = s.standing_variant(&genome);
            let genotype = s.genotype_of(genome);
            let age = ages.map(|e| e.sample(&mut s.rng)).unwrap_or(0.0);
            let position_km = s.uniform_in_disk();
            let sex = s.draw_sex(genotype);
            let id = s.new_id();
            s.members.push(Member { id, genotype, sex, born_years: -age, parents: [u32::MAX; 2], position_km });
        }
        s
    }

    fn new_id(&mut self) -> u32 {
        self.next_id += 1;
        self.next_id - 1
    }

    fn uniform_in_disk(&mut self) -> [f64; 2] {
        let r = self.radius_km * self.rng.random::<f64>().sqrt();
        let a = self.rng.random::<f64>() * std::f64::consts::TAU;
        [r * a.dcos(), r * a.dsin()]
    }

    fn draw_sex(&mut self, genotype: u32) -> Sex {
        if !self.genotypes[genotype as usize].traits.sexual {
            Sex::None
        } else if self.rng.random::<bool>() {
            Sex::Female
        } else {
            Sex::Male
        }
    }

    /// Génome porteur de la variation de quelques générations.
    fn standing_variant(&mut self, base: &Arc<Genome>) -> Arc<Genome> {
        let u = self.development.mutation.genomic_rate(base) * STANDING_GENERATIONS;
        self.mutated(base.clone(), u)
    }

    fn mutated(&mut self, genome: Arc<Genome>, expected: f64) -> Arc<Genome> {
        let k = if expected > 0.0 { Poisson::new(expected).map(|p| p.sample(&mut self.rng) as u32).unwrap_or(0) } else { 0 };
        if k == 0 {
            return genome;
        }
        let mut g = (*genome).clone();
        for _ in 0..k.min(16) {
            g = mutate(&g, &self.development.mutation, &mut self.rng).genome;
        }
        Arc::new(g)
    }

    /// Indice du génotype d'un génome, développé au besoin.
    fn genotype_of(&mut self, genome: Arc<Genome>) -> u32 {
        let key = genome_key(&genome);
        if let Some(&i) = self.cache.get(&key) {
            return i;
        }
        let g = self.development.develop(genome);
        self.genotypes.push(g);
        let i = (self.genotypes.len() - 1) as u32;
        self.cache.insert(key, i);
        i
    }

    pub fn genotype(&self, m: &Member) -> &Genotype {
        &self.genotypes[m.genotype as usize]
    }

    /// Valeur sélective moyenne de l'échantillon.
    pub fn mean_fitness(&self) -> f64 {
        if self.members.is_empty() {
            return 0.0;
        }
        self.members.iter().map(|m| self.genotypes[m.genotype as usize].fitness).sum::<f64>() / self.members.len() as f64
    }

    /// Avance l'échantillon de `dt` années (algorithme de Gillespie : chaque
    /// événement est tiré à son taux exact).
    pub fn advance(&mut self, dt: f64) {
        let end = self.years + dt.max(0.0);
        let r = self.rates;
        loop {
            let n = self.members.len();
            if n == 0 {
                self.recruit();
                if self.members.is_empty() {
                    self.years = end;
                    break;
                }
                continue;
            }
            let nf = n as f64;
            let w_sum: f64 = self.members.iter().map(|m| self.genotypes[m.genotype as usize].fitness).sum();
            let birth = if w_sum > 0.0 { nf * r.birth } else { 0.0 };
            let death = nf * r.death;
            let out = nf * r.emigration;
            let inn = nf * r.emigration;
            let total = birth + death + out + inn;
            if total <= 0.0 {
                self.counters.exposure += nf * (end - self.years);
                self.years = end;
                break;
            }
            let t = Exp::new(total).map(|e| e.sample(&mut self.rng)).unwrap_or(f64::INFINITY);
            if self.years + t > end {
                self.counters.exposure += nf * (end - self.years);
                self.years = end;
                break;
            }
            self.years += t;
            self.counters.exposure += nf * t;
            let u = self.rng.random::<f64>() * total;
            if u < birth {
                self.birth(w_sum);
            } else if u < birth + death {
                let i = self.rng.random_range(0..n);
                let m = self.members.swap_remove(i);
                let predation = self.rng.random::<f64>() < r.predation;
                self.counters.deaths += 1;
                self.counters.predation_deaths += predation as u64;
                self.events.push(LifeEvent { years: self.years, kind: LifeEventKind::Death { id: m.id, predation } });
            } else if u < birth + death + out {
                let i = self.rng.random_range(0..n);
                let m = self.members.swap_remove(i);
                self.counters.emigrations += 1;
                self.events.push(LifeEvent { years: self.years, kind: LifeEventKind::Emigration { id: m.id } });
            } else {
                let id = self.arrive();
                self.counters.immigrations += 1;
                self.events.push(LifeEvent { years: self.years, kind: LifeEventKind::Immigration { id } });
            }
            self.keep_bounds();
        }
        self.wander(dt);
    }

    /// Une naissance : mère selon la valeur sélective, père de même, enfant
    /// recombiné puis muté.
    fn birth(&mut self, w_sum: f64) {
        let mother = self.pick_weighted(w_sum, |_| true);
        let Some(mother) = mother else { return };
        let m = self.members[mother];
        let father = if m.sex == Sex::Female || m.sex == Sex::Male {
            let want = if m.sex == Sex::Female { Sex::Male } else { Sex::Female };
            let sum: f64 = self.members.iter().filter(|x| x.sex == want).map(|x| self.genotypes[x.genotype as usize].fitness).sum();
            if sum > 0.0 {
                self.pick_weighted(sum, |x| x.sex == want)
            } else {
                None
            }
        } else {
            None
        };
        let gm = self.genotypes[m.genotype as usize].genome.clone();
        let genome = match father {
            Some(f) => {
                let gf = self.genotypes[self.members[f].genotype as usize].genome.clone();
                Arc::new(self.recombine(&gm, &gf))
            }
            None => gm,
        };
        let u = self.development.mutation.genomic_rate(&genome);
        let genome = self.mutated(genome, u);
        let genotype = self.genotype_of(genome);
        let sex = self.draw_sex(genotype);
        let id = self.new_id();
        let father_id = father.map(|f| self.members[f].id).unwrap_or(u32::MAX);
        let near = [
            m.position_km[0] + 0.01 * self.radius_km * (self.rng.random::<f64>() - 0.5),
            m.position_km[1] + 0.01 * self.radius_km * (self.rng.random::<f64>() - 0.5),
        ];
        self.members.push(Member { id, genotype, sex, born_years: self.years, parents: [m.id, father_id], position_km: near });
        self.counters.births += 1;
        self.events.push(LifeEvent { years: self.years, kind: LifeEventKind::Birth { child: id, mother: m.id, father: father_id } });
    }

    fn pick_weighted(&mut self, sum: f64, keep: impl Fn(&Member) -> bool) -> Option<usize> {
        let mut u = self.rng.random::<f64>() * sum;
        let mut last = None;
        for (i, x) in self.members.iter().enumerate() {
            if !keep(x) {
                continue;
            }
            let w = self.genotypes[x.genotype as usize].fitness;
            if w <= 0.0 {
                continue;
            }
            last = Some(i);
            u -= w;
            if u < 0.0 {
                return Some(i);
            }
        }
        last
    }

    /// Recombinaison libre : chaque gène et chaque site du marqueur vient de
    /// l'un ou l'autre parent ; les organites viennent de la mère.
    /// [Simplification] Si les deux génomes n'ont pas le même nombre de
    /// gènes, un seul crossing-over.
    fn recombine(&mut self, mother: &Genome, father: &Genome) -> Genome {
        let picks: Vec<bool> = (0..mother.genes.len().max(father.genes.len())).map(|_| self.rng.random::<bool>()).collect();
        let marker: Vec<bool> = (0..mother.marker.len()).map(|_| self.rng.random::<bool>()).collect();
        let cut = self.rng.random_range(0..=mother.genes.len().min(father.genes.len()));
        mother
            .derive(GenomeChangeCause::Recombination, |g| {
                if mother.genes.len() == father.genes.len() {
                    for (i, gene) in g.genes.iter_mut().enumerate() {
                        if picks[i] {
                            *gene = father.genes[i];
                        }
                    }
                } else {
                    g.genes.truncate(cut);
                    g.genes.extend_from_slice(&father.genes[cut..]);
                }
                for (i, b) in g.marker.iter_mut().enumerate() {
                    if marker[i] {
                        *b = father.marker[i];
                    }
                }
                ChangedElement::Several
            })
            .genome
    }

    /// Un immigrant, au bord de la cellule.
    fn arrive(&mut self) -> u32 {
        let base = self.immigrant_genome.clone();
        let genome = self.standing_variant(&base);
        let genotype = self.genotype_of(genome);
        let sex = self.draw_sex(genotype);
        let a = self.rng.random::<f64>() * std::f64::consts::TAU;
        let r = 0.98 * self.radius_km;
        let id = self.new_id();
        let age = Exp::new(self.rates.birth.max(1e-9)).map(|e| e.sample(&mut self.rng)).unwrap_or(0.0);
        self.members.push(Member {
            id,
            genotype,
            sex,
            born_years: self.years - age,
            parents: [u32::MAX; 2],
            position_km: [r * a.dcos(), r * a.dsin()],
        });
        id
    }

    /// Recrute des individus de la population quand l'échantillon s'épuise
    /// (la population, elle, n'est pas éteinte).
    fn recruit(&mut self) {
        if self.census < 1.0 {
            return;
        }
        let want = (self.nominal / 2).max(1);
        while self.members.len() < want {
            self.arrive();
            self.counters.recruited += 1;
        }
    }

    fn keep_bounds(&mut self) {
        let n = self.members.len();
        if n > 2 * self.nominal {
            while self.members.len() > self.nominal {
                let i = self.rng.random_range(0..self.members.len());
                self.members.swap_remove(i);
                self.counters.thinned += 1;
            }
        } else if n < self.nominal / 4 && n > 0 {
            self.recruit();
        }
    }

    /// Marche aléatoire dans la cellule, réfléchie au bord : le coefficient
    /// de diffusion D = m·R²/8 donne au disque un temps moyen de sortie de
    /// 1/m (la sortie elle-même est l'événement d'émigration, tiré à part).
    fn wander(&mut self, dt: f64) {
        let d = self.rates.emigration * self.radius_km * self.radius_km / 8.0;
        let sigma = (2.0 * d * dt).sqrt().min(self.radius_km);
        if sigma <= 0.0 {
            return;
        }
        let r = self.radius_km;
        for i in 0..self.members.len() {
            let g1: f64 = self.normal();
            let g2: f64 = self.normal();
            let m = &mut self.members[i];
            let mut x = m.position_km[0] + sigma * g1;
            let mut y = m.position_km[1] + sigma * g2;
            let l = libm::hypot(x, y);
            if l > r {
                let k = (2.0 * r - l).max(0.0) / l;
                x *= k;
                y *= k;
            }
            m.position_km = [x, y];
        }
    }

    fn normal(&mut self) -> f64 {
        let u1: f64 = self.rng.random::<f64>().max(1e-300);
        let u2: f64 = self.rng.random();
        (-2.0 * u1.dln()).sqrt() * (std::f64::consts::TAU * u2).dcos()
    }

    /// Taux de la population tels que le moteur les a évalués.
    pub fn resident_rates(&self) -> &GrowthRates {
        &self.development.resident
    }

    /// Vide la liste des événements (lus par le niveau 5).
    pub fn take_events(&mut self) -> Vec<LifeEvent> {
        std::mem::take(&mut self.events)
    }
}
