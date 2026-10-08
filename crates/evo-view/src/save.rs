//! Description d'une partie à créer et points de sauvegarde provisoires.
//!
//! [Simplification] Les points de sauvegarde complets (état sur disque et
//! reprise directe) sont un service du volet moteur de l'étape 3. En
//! attendant, le client sauvegarde un rejeu : la description de la planète,
//! la graine et le registre des ordres. Le moteur étant déterministe, le
//! recharger rejoue la partie jusqu'au même pas et redonne exactement le même
//! état ; c'est aussi le format de partage « rejeu » du document
//! Fonctionnalités. Le chargement prend le temps de rejouer.

use evo_planet::{Gas, PlanetParams, GASES};
use evo_sim::{Order, OrderKind, WorldConfig};

pub const FORMAT: &str = "evonisium-point-de-sauvegarde";
pub const FORMAT_VERSION: u32 = 1;

/// Planète demandée sur l'écran de création.
#[derive(Clone, Debug, PartialEq)]
pub struct PlanetSpec {
    /// Préréglage (`PlanetParams::KEYS`).
    pub preset: String,
    /// Distance à l'étoile, en multiple de celle du préréglage.
    pub orbit_factor: f64,
    /// Inventaire d'eau, en multiple de celui du préréglage.
    pub water_factor: f64,
    /// Température de l'étoile, K (`None` : celle du préréglage).
    pub star_temperature_k: Option<f64>,
}

impl Default for PlanetSpec {
    fn default() -> Self {
        Self { preset: "terre".into(), orbit_factor: 1.0, water_factor: 1.0, star_temperature_k: None }
    }
}

impl PlanetSpec {
    /// Paramètres de la planète.
    ///
    /// [Simplification] Changer le type d'étoile suit une séquence principale
    /// grossière : rayon ∝ T^0,8, donc luminosité ∝ T^5,6 (loi de
    /// Stefan-Boltzmann avec ce rayon).
    pub fn to_params(&self) -> PlanetParams {
        let mut p = PlanetParams::by_key(&self.preset).unwrap_or_else(PlanetParams::earth_archean);
        p.orbit_m *= self.orbit_factor.clamp(0.5, 2.0);
        p.water_inventory_m *= self.water_factor.clamp(0.05, 10.0);
        if let Some(t) = self.star_temperature_k {
            let t = t.clamp(3000.0, 8000.0);
            p.star_luminosity_w *= (t / p.star_temperature_k).powf(5.6);
            p.star_temperature_k = t;
        }
        p
    }

    /// Code de planète à partager : quelques dizaines de caractères.
    pub fn code(&self, seed: u64, level: u32) -> String {
        format!(
            "{}-{}-{}-{:.2}-{:.2}-{}",
            self.preset,
            seed,
            level,
            self.orbit_factor,
            self.water_factor,
            self.star_temperature_k.map_or("0".into(), |t| format!("{t:.0}"))
        )
    }

    pub fn from_code(code: &str) -> Option<(PlanetSpec, u64, u32)> {
        let parts: Vec<&str> = code.trim().rsplitn(6, '-').collect();
        if parts.len() != 6 {
            return None;
        }
        let star: f64 = parts[0].parse().ok()?;
        Some((
            PlanetSpec {
                preset: parts[5].to_string(),
                orbit_factor: parts[2].parse().ok()?,
                water_factor: parts[1].parse().ok()?,
                star_temperature_k: (star > 0.0).then_some(star),
            },
            parts[4].parse().ok()?,
            parts[3].parse().ok()?,
        ))
    }
}

/// Une partie sauvegardée.
#[derive(Clone, Debug, PartialEq)]
pub struct SaveFile {
    pub name: String,
    pub engine_version: String,
    pub spec: PlanetSpec,
    pub seed: u64,
    pub level: u32,
    /// Ensemencement choisi : « sources » ou « mers ».
    pub seeding: String,
    /// Pas et date atteints.
    pub steps: u64,
    pub years: f64,
    /// Mode de partie (« observateur », « bac-a-sable »).
    pub mode: String,
    pub orders: Vec<Order>,
}

impl SaveFile {
    pub fn config(&self) -> WorldConfig {
        WorldConfig::with_planet(self.spec.to_params(), self.seed, self.level)
    }

    pub fn to_text(&self) -> String {
        let mut s = format!("{FORMAT}\t{FORMAT_VERSION}\n");
        s += &format!("nom\t{}\n", self.name.replace(['\t', '\n'], " "));
        s += &format!("version_moteur\t{}\n", self.engine_version);
        s += &format!("mode\t{}\n", self.mode);
        s += &format!("planete\t{}\n", self.spec.code(self.seed, self.level));
        s += &format!("ensemencement\t{}\n", self.seeding);
        s += &format!("pas\t{}\n", self.steps);
        s += &format!("annees\t{:e}\n", self.years);
        s += "ordres\n";
        for o in &self.orders {
            s += &format!("{}\t{:e}\t{}\n", o.id, o.due_years, encode(&o.kind));
        }
        s
    }

    pub fn from_text(text: &str) -> Result<SaveFile, String> {
        let mut lines = text.lines();
        let head = lines.next().ok_or("fichier vide")?;
        let (fmt, ver) = head.split_once('\t').ok_or("en-tête illisible")?;
        if fmt != FORMAT {
            return Err("ce fichier n'est pas un point de sauvegarde d'Evonisium".into());
        }
        if ver.trim().parse::<u32>().map_err(|e| e.to_string())? > FORMAT_VERSION {
            return Err("point de sauvegarde d'une version plus récente".into());
        }
        let mut save = SaveFile {
            name: String::new(),
            engine_version: String::new(),
            spec: PlanetSpec::default(),
            seed: 0,
            level: 4,
            seeding: "sources".into(),
            steps: 0,
            years: 0.0,
            mode: "observateur".into(),
            orders: Vec::new(),
        };
        let mut in_orders = false;
        for line in lines {
            if line.trim().is_empty() {
                continue;
            }
            if in_orders {
                let mut f = line.splitn(3, '\t');
                let id = f.next().and_then(|x| x.parse().ok()).ok_or("ordre illisible")?;
                let due = f.next().and_then(|x| x.parse().ok()).ok_or("date d'ordre illisible")?;
                let kind = decode(f.next().unwrap_or("")).ok_or_else(|| format!("ordre inconnu : {line}"))?;
                save.orders.push(Order { id, due_years: due, kind });
                continue;
            }
            if line == "ordres" {
                in_orders = true;
                continue;
            }
            let (k, v) = line.split_once('\t').ok_or("ligne illisible")?;
            match k {
                "nom" => save.name = v.into(),
                "version_moteur" => save.engine_version = v.into(),
                "mode" => save.mode = v.into(),
                "planete" => {
                    let (spec, seed, level) = PlanetSpec::from_code(v).ok_or("code de planète illisible")?;
                    save.spec = spec;
                    save.seed = seed;
                    save.level = level;
                }
                "ensemencement" => save.seeding = v.into(),
                "pas" => save.steps = v.parse().map_err(|_| "pas illisible")?,
                "annees" => save.years = v.parse().map_err(|_| "date illisible")?,
                _ => {}
            }
        }
        Ok(save)
    }
}

fn encode(k: &OrderKind) -> String {
    match k {
        OrderKind::SetStepYears(y) => format!("vitesse\t{y:e}"),
        OrderKind::Pause => "pause".into(),
        OrderKind::Resume => "reprise".into(),
        OrderKind::SeedLife => "ensemencer".into(),
        OrderKind::AddPhosphate { moles } => format!("phosphate\t{moles:e}"),
        OrderKind::InjectGas { gas, moles } => format!("gaz\t{}\t{moles:e}", *gas as usize),
        OrderKind::MarkLineage { lineage } => format!("suivi\t{lineage}"),
    }
}

fn decode(s: &str) -> Option<OrderKind> {
    let f: Vec<&str> = s.split('\t').collect();
    let num = |i: usize| f.get(i).and_then(|x| x.parse::<f64>().ok());
    Some(match f[0] {
        "vitesse" => OrderKind::SetStepYears(num(1)?),
        "pause" => OrderKind::Pause,
        "reprise" => OrderKind::Resume,
        "ensemencer" => OrderKind::SeedLife,
        "phosphate" => OrderKind::AddPhosphate { moles: num(1)? },
        "gaz" => OrderKind::InjectGas { gas: *GASES.get(f.get(1)?.parse::<usize>().ok()?)?, moles: num(2)? },
        "suivi" => OrderKind::MarkLineage { lineage: f.get(1)?.parse().ok()? },
        _ => return None,
    })
}

/// Gaz par clé courte (palette de commandes et interface).
pub fn gas_by_key(k: &str) -> Option<Gas> {
    match k {
        "co2" => Some(Gas::Co2),
        "ch4" => Some(Gas::Ch4),
        "h2" => Some(Gas::H2),
        "o2" => Some(Gas::O2),
        "n2" => Some(Gas::N2),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn planet_code_round_trips() {
        let spec = PlanetSpec { preset: "super-terre".into(), orbit_factor: 1.1, water_factor: 0.5, star_temperature_k: Some(5200.0) };
        let code = spec.code(123, 5);
        assert_eq!(PlanetSpec::from_code(&code), Some((spec.clone(), 123, 5)));
        let p = spec.to_params();
        assert!(p.star_luminosity_w < PlanetParams::super_earth().star_luminosity_w);
    }

    #[test]
    fn save_round_trips() {
        let save = SaveFile {
            name: "Essai".into(),
            engine_version: "0.1.0".into(),
            spec: PlanetSpec::default(),
            seed: 9,
            level: 4,
            seeding: "mers".into(),
            steps: 12,
            years: 1.2e6,
            mode: "observateur".into(),
            orders: vec![
                Order { id: 0, due_years: 0.0, kind: OrderKind::SeedLife },
                Order { id: 1, due_years: 1e5, kind: OrderKind::SetStepYears(5e4) },
                Order { id: 2, due_years: 2e5, kind: OrderKind::InjectGas { gas: Gas::Co2, moles: 1e15 } },
                Order { id: 3, due_years: 3e5, kind: OrderKind::MarkLineage { lineage: 4 } },
                Order { id: 4, due_years: 3e5, kind: OrderKind::AddPhosphate { moles: 2.5e12 } },
            ],
        };
        let back = SaveFile::from_text(&save.to_text()).unwrap();
        assert_eq!(back, save);
        assert!(SaveFile::from_text("autre chose").is_err());
    }

    #[test]
    fn replaying_a_save_gives_the_same_world() {
        use evo_sim::World;
        let mut w = World::new(WorldConfig::new(4, 3));
        w.orders.submit(0.0, OrderKind::SeedLife);
        w.orders.submit(2e5, OrderKind::AddPhosphate { moles: 1e12 });
        for _ in 0..5 {
            w.step();
        }
        let save = SaveFile {
            name: String::new(),
            engine_version: String::new(),
            spec: PlanetSpec::default(),
            seed: 4,
            level: 3,
            seeding: "sources".into(),
            steps: w.stats.steps,
            years: w.years,
            mode: "observateur".into(),
            orders: w.orders.log(),
        };
        let back = SaveFile::from_text(&save.to_text()).unwrap();
        let mut r = World::replay(back.config(), &back.orders);
        while r.stats.steps < back.steps {
            r.step();
        }
        assert_eq!(r.state_hash(), w.state_hash());
    }
}
