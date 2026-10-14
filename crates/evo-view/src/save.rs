//! Description d'une partie à créer et fiche d'un point de sauvegarde.
//!
//! Le moteur écrit l'état complet (`Engine::save`) et le recharge tel quel.
//! Le client range à côté une petite fiche lisible (`<sauvegarde>.fiche`) :
//! nom, planète, date atteinte, mode. Elle sert à la liste des sauvegardes
//! sans ouvrir l'état, et à retrouver le code de planète.

use evo_core::math::Det;
use evo_planet::{Gas, PlanetParams};
use evo_sim::orders::OrderKind;
use evo_sim::Seeding;

pub const FORMAT: &str = "evonisium-point-de-sauvegarde";
pub const FORMAT_VERSION: u32 = 3;

/// Extension de la fiche rangée à côté de l'état sauvegardé.
pub const META_SUFFIX: &str = ".fiche";

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
            // Calcul déterministe : la planète d'un code partagé est la même
            // sur toutes les plateformes.
            p.star_luminosity_w *= (t / p.star_temperature_k).dpowf(5.6);
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
    /// Point de sauvegarde d'où la partie avait repris (chemin), vide pour
    /// une partie neuve : revenir à un point puis agir autrement crée une
    /// branche au lieu d'écraser la suite.
    pub branch_of: String,
    /// Interventions du joueur et le point de sauvegarde écrit juste avant
    /// chacune (outil « avec et sans »).
    pub interventions: Vec<SavedIntervention>,
    /// Jalons de l'histoire (début de chaque chapitre du récit) : date et
    /// titre, dans la langue de l'écran au moment de la sauvegarde.
    pub milestones: Vec<(f64, String)>,
}

/// Une intervention passée et son point de sauvegarde.
#[derive(Clone, Debug, PartialEq)]
pub struct SavedIntervention {
    /// Identifiant de l'ordre dans le registre du moteur.
    pub order: u64,
    pub years: f64,
    /// Point de sauvegarde écrit juste avant.
    pub save: String,
    pub label: String,
}

impl SaveFile {
    /// Ensemencement du moteur correspondant au choix du joueur.
    pub fn seeding_mode(&self) -> Seeding {
        seeding_of(&self.seeding)
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
        if !self.branch_of.is_empty() {
            s += &format!("branche_de\t{}\n", self.branch_of.replace(['\t', '\n'], " "));
        }
        for i in &self.interventions {
            s += &format!(
                "intervention\t{}\t{:e}\t{}\t{}\n",
                i.order,
                i.years,
                i.save.replace(['\t', '\n'], " "),
                i.label.replace(['\t', '\n'], " ")
            );
        }
        s += &milestone_lines(&self.milestones);
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
            branch_of: String::new(),
            interventions: Vec::new(),
            milestones: Vec::new(),
        };
        for line in lines {
            if line.trim().is_empty() {
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
                "branche_de" => save.branch_of = v.into(),
                "jalon" => save.milestones.extend(milestone_of(v)),
                "intervention" => {
                    let f: Vec<&str> = v.splitn(4, '\t').collect();
                    if let [order, years, path, label] = f[..] {
                        save.interventions.push(SavedIntervention {
                            order: order.parse().map_err(|_| "intervention illisible")?,
                            years: years.parse().map_err(|_| "intervention illisible")?,
                            save: path.into(),
                            label: label.into(),
                        });
                    }
                }
                _ => {}
            }
        }
        Ok(save)
    }
}

fn milestone_lines(list: &[(f64, String)]) -> String {
    list.iter().map(|(y, t)| format!("jalon\t{y:e}\t{}\n", t.replace(['\t', '\n'], " "))).collect()
}

fn milestone_of(v: &str) -> Option<(f64, String)> {
    let (y, t) = v.split_once('\t')?;
    Some((y.parse().ok()?, t.to_string()))
}

pub const REPLAY_FORMAT: &str = "evonisium-rejeu";
pub const REPLAY_FORMAT_VERSION: u32 = 1;
/// Extension des fichiers de rejeu.
pub const REPLAY_SUFFIX: &str = ".rejeu";

/// Un rejeu à partager : la planète (code), l'ensemencement, le mode et le
/// registre des ordres. La graine et ce registre suffisent à refaire la même
/// histoire ; le fichier tient en quelques kilo-octets. Les pauses et les
/// reprises n'en font pas partie : elles ne changent pas l'histoire.
#[derive(Clone, Debug, PartialEq)]
pub struct ReplayFile {
    pub name: String,
    pub engine_version: String,
    pub spec: PlanetSpec,
    pub seed: u64,
    pub level: u32,
    pub seeding: String,
    pub mode: String,
    /// Date atteinte quand le rejeu a été exporté.
    pub years: f64,
    pub milestones: Vec<(f64, String)>,
    /// Ordres : date demandée et ordre.
    pub orders: Vec<(f64, OrderKind)>,
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn unhex(s: &str) -> Option<Vec<u8>> {
    if !s.len().is_multiple_of(2) {
        return None;
    }
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(s.get(i..i + 2)?, 16).ok()).collect()
}

impl ReplayFile {
    /// Garde les ordres qui changent l'histoire (sans pauses ni reprises).
    pub fn keep(kind: &OrderKind) -> bool {
        !matches!(kind, OrderKind::Pause | OrderKind::Resume)
    }

    /// Interventions du joueur dans le registre.
    pub fn interventions(&self) -> usize {
        self.orders.iter().filter(|(_, k)| matches!(k, OrderKind::Intervene(_))).count()
    }

    pub fn to_text(&self) -> String {
        let mut s = format!("{REPLAY_FORMAT}\t{REPLAY_FORMAT_VERSION}\n");
        s += &format!("nom\t{}\n", self.name.replace(['\t', '\n'], " "));
        s += &format!("version_moteur\t{}\n", self.engine_version);
        s += &format!("mode\t{}\n", self.mode);
        s += &format!("planete\t{}\n", self.spec.code(self.seed, self.level));
        s += &format!("ensemencement\t{}\n", self.seeding);
        s += &format!("annees\t{:e}\n", self.years);
        s += &milestone_lines(&self.milestones);
        for (due, kind) in &self.orders {
            // L'ordre en binaire (hexadécimal), suivi de son libellé lisible.
            let bytes = bincode::serialize(kind).unwrap_or_default();
            s += &format!("ordre\t{due:e}\t{}\t{}\n", hex(&bytes), kind.label().replace(['\t', '\n'], " "));
        }
        s
    }

    pub fn from_text(text: &str) -> Result<ReplayFile, String> {
        let mut lines = text.lines();
        let head = lines.next().ok_or("fichier vide")?;
        let (fmt, ver) = head.split_once('\t').ok_or("en-tête illisible")?;
        if fmt != REPLAY_FORMAT {
            return Err("ce fichier n'est pas un rejeu d'Evonisium".into());
        }
        if ver.trim().parse::<u32>().map_err(|e| e.to_string())? > REPLAY_FORMAT_VERSION {
            return Err("rejeu d'une version plus récente".into());
        }
        let mut r = ReplayFile {
            name: String::new(),
            engine_version: String::new(),
            spec: PlanetSpec::default(),
            seed: 0,
            level: 4,
            seeding: "sources".into(),
            mode: "observateur".into(),
            years: 0.0,
            milestones: Vec::new(),
            orders: Vec::new(),
        };
        for line in lines {
            if line.trim().is_empty() {
                continue;
            }
            let (k, v) = line.split_once('\t').ok_or("ligne illisible")?;
            match k {
                "nom" => r.name = v.into(),
                "version_moteur" => r.engine_version = v.into(),
                "mode" => r.mode = v.into(),
                "planete" => {
                    let (spec, seed, level) = PlanetSpec::from_code(v).ok_or("code de planète illisible")?;
                    r.spec = spec;
                    r.seed = seed;
                    r.level = level;
                }
                "ensemencement" => r.seeding = v.into(),
                "annees" => r.years = v.parse().map_err(|_| "date illisible")?,
                "jalon" => r.milestones.extend(milestone_of(v)),
                "ordre" => {
                    let f: Vec<&str> = v.splitn(3, '\t').collect();
                    let due: f64 = f.first().and_then(|x| x.parse().ok()).ok_or("ordre illisible")?;
                    let bytes = f.get(1).and_then(|x| unhex(x)).ok_or("ordre illisible")?;
                    let kind: OrderKind = bincode::deserialize(&bytes).map_err(|_| "ordre illisible")?;
                    r.orders.push((due, kind));
                }
                _ => {}
            }
        }
        Ok(r)
    }
}

/// « sources » : près des sources hydrothermales ; « mers » : dans toutes
/// les mers.
pub fn seeding_of(key: &str) -> Seeding {
    if key == "mers" {
        Seeding::AllOcean
    } else {
        Seeding::Vents
    }
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
            branch_of: "/tmp/avant.evo".into(),
            interventions: vec![SavedIntervention { order: 4, years: 1e6, save: "/tmp/i.evo".into(), label: "impact".into() }],
            milestones: vec![(3.2e8, "Les premières cellules".into()), (9e8, "L'air change".into())],
        };
        let back = SaveFile::from_text(&save.to_text()).unwrap();
        assert_eq!(back, save);
        assert!(SaveFile::from_text("autre chose").is_err());
        assert_eq!(back.seeding_mode(), Seeding::AllOcean);
    }

    #[test]
    fn replay_round_trips() {
        use evo_sim::orders::Intervention;
        let r = ReplayFile {
            name: "Une Terre".into(),
            engine_version: "0.1.0".into(),
            spec: PlanetSpec::default(),
            seed: 2026,
            level: 4,
            seeding: "sources".into(),
            mode: "bac-a-sable".into(),
            years: 2.5e9,
            milestones: vec![(4e8, "La vie".into())],
            orders: vec![(0.0, OrderKind::SeedLife), (1.5e9, OrderKind::Intervene(Intervention::Impact { cell: 12, diameter_km: 10.0 }))],
        };
        let back = ReplayFile::from_text(&r.to_text()).unwrap();
        assert_eq!(back, r);
        assert_eq!(back.interventions(), 1);
        assert!(ReplayFile::from_text("evonisium-point-de-sauvegarde\t3").is_err());
        assert!(!ReplayFile::keep(&OrderKind::Pause) && ReplayFile::keep(&OrderKind::SeedLife));
    }
}
