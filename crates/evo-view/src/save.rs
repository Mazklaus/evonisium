//! Description d'une partie à créer et fiche d'un point de sauvegarde.
//!
//! Le moteur écrit l'état complet (`Engine::save`) et le recharge tel quel.
//! Le client range à côté une petite fiche lisible (`<sauvegarde>.fiche`) :
//! nom, planète, date atteinte, mode. Elle sert à la liste des sauvegardes
//! sans ouvrir l'état, et à retrouver le code de planète.

use evo_planet::{Gas, PlanetParams};
use evo_sim::Seeding;

pub const FORMAT: &str = "evonisium-point-de-sauvegarde";
pub const FORMAT_VERSION: u32 = 2;

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
                _ => {}
            }
        }
        Ok(save)
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
        };
        let back = SaveFile::from_text(&save.to_text()).unwrap();
        assert_eq!(back, save);
        assert!(SaveFile::from_text("autre chose").is_err());
        assert_eq!(back.seeding_mode(), Seeding::AllOcean);
    }
}
