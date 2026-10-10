//! Traits de comportement d'un génotype : ce que son corps lui permet de
//! percevoir, de décider et de faire.
//!
//! Tout vient du phénotype, du plan de construction et des taux de sa
//! population ; rien n'est attaché à une espèce. Le document Vision demande
//! des comportements « paramétrés par le phénotype (sens, système nerveux,
//! locomotion) ».
//!
//! [Simplification] À ce jour, le phénotype ne porte ni organe des sens, ni
//! système nerveux, ni muscle : ces familles arrivent avec le fil « Animaux
//! et écosystèmes ». En attendant, la perception, le système nerveux et la
//! sociabilité sont tirés de mesures voisines (signal entre cellules, nombre
//! de cellules, types cellulaires) ; chaque trait ainsi deviné est marqué
//! dans [`Traits::proxies`]. Les vitesses suivent des lois d'échelle
//! simples (en √longueur), du bon ordre de grandeur pour la marche, la nage
//! et le vol, sans prétendre à mieux.

use evo_core::math::Det;
use evo_life::body::{BodyPlan, Covering, JointKind, Material, ModuleKind, Profile};
use evo_life::{GrowthRates, Phenotype};

/// Façon de se déplacer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Locomotion {
    /// Fixé (algues, tapis, plantes).
    Sessile,
    /// Porté par l'eau, sans propulsion propre.
    Drift,
    /// Battement de cils ou glissement sur un mucus : moins d'un millimètre
    /// par seconde.
    Cilia,
    /// Ondulation ou péristaltisme d'un corps mou.
    Undulate,
    /// Nage à l'aide d'appendices.
    Swim,
    /// Marche sur des appendices articulés.
    Walk,
    /// Vol sur des appendices aplatis.
    Fly,
}

/// Ce qu'il mange.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Diet {
    /// Tire son énergie de la lumière ou de la chimie, sans proie.
    Autotroph,
    /// Matière organique dissoute, débris, tapis microbiens.
    Grazer,
    /// Proies vivantes.
    Predator,
}

/// Traits devinés faute de famille de gènes dédiée (bits de
/// [`Traits::proxies`]).
pub mod proxy {
    pub const PERCEPTION: u8 = 1;
    pub const NERVOUS: u8 = 2;
    pub const SOCIALITY: u8 = 4;
    pub const CARE: u8 = 8;
    pub const MUSCLE: u8 = 16;
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Traits {
    /// Plus grande longueur du corps, m.
    pub length_m: f64,
    /// Masse fraîche (densité de l'eau), kg.
    pub mass_kg: f64,
    pub locomotion: Locomotion,
    /// Vitesse de croisière et vitesse de pointe, m/s.
    pub cruise_m_s: f64,
    pub sprint_m_s: f64,
    /// Portée de détection d'une proie, d'un prédateur ou d'un partenaire, m.
    pub perception_m: f64,
    /// Intervalle entre deux décisions, s.
    pub reaction_s: f64,
    /// Complexité du système nerveux, de 0 (aucun) à 1.
    pub nervous: f64,
    pub diet: Diet,
    /// Protection du revêtement et du squelette, de 0 à 1.
    pub defence: f64,
    /// Tendance à rester en groupe, de 0 (solitaire) à 1 (troupeau serré).
    pub sociality: f64,
    /// Soin des petits, de 0 (aucun) à 1.
    pub care: f64,
    pub sexual: bool,
    pub aquatic: bool,
    /// Traits devinés (voir [`proxy`]).
    pub proxies: u8,
}

impl Traits {
    /// Traits d'un génotype vivant dans un milieu aquatique ou non.
    pub fn of(p: &Phenotype, plan: &BodyPlan, rates: &GrowthRates, aquatic: bool) -> Traits {
        let mut proxies = 0u8;
        // Dimensions : le plus grand module (le tronc, à ce jour).
        let main = plan.modules.iter().max_by(|a, b| a.dimensions.length_m.total_cmp(&b.dimensions.length_m));
        let (length_m, width_m, thick_m, profile) = main
            .map(|m| (m.dimensions.length_m, m.dimensions.width_m, m.dimensions.thickness_m, m.dimensions.profile))
            .unwrap_or((1e-6, 1e-6, 1e-6, Profile::Round));
        let volume: f64 = plan
            .modules
            .iter()
            .map(|m| {
                let d = &m.dimensions;
                std::f64::consts::PI / 6.0 * d.length_m * d.width_m * d.thickness_m * m.symmetry.copies.max(1) as f64
            })
            .sum::<f64>()
            .max(std::f64::consts::PI / 6.0 * length_m * width_m * thick_m);
        let mass_kg = 1000.0 * volume;

        // Appendices articulés et muscles.
        let mut limbs = 0u32;
        let mut flat_limbs = 0u32;
        let mut muscle = 0.0;
        let mut fast = 0.0;
        for m in &plan.modules {
            let moving: Vec<_> = m.joints.iter().filter(|j| j.kind != JointKind::Rigid).collect();
            if moving.is_empty() {
                continue;
            }
            for j in &moving {
                muscle += j.muscle_section_m2 * m.symmetry.copies.max(1) as f64;
                fast = f64::max(fast, j.fast_fibres);
            }
            if m.kind == ModuleKind::Appendage {
                limbs += m.symmetry.copies.max(1);
                if m.dimensions.profile == Profile::Flattened {
                    flat_limbs += m.symmetry.copies.max(1);
                }
            }
        }
        let autotroph = rates.heterotroph_share < 0.5 && rates.prey_share < 0.1;
        let locomotion = if limbs >= 2 && flat_limbs >= 2 && !aquatic {
            Locomotion::Fly
        } else if limbs >= 2 {
            if aquatic {
                Locomotion::Swim
            } else {
                Locomotion::Walk
            }
        } else if autotroph {
            if aquatic && p.body.as_ref().is_none_or(|b| b.cells < 1e4) {
                Locomotion::Drift
            } else {
                Locomotion::Sessile
            }
        } else if profile == Profile::Elongated && length_m >= 2e-3 {
            Locomotion::Undulate
        } else if length_m < 2e-3 {
            Locomotion::Cilia
        } else {
            Locomotion::Undulate
        };

        // Vitesse de pointe ∝ √L (m/s pour L en m), selon le mode ; les
        // muscles, quand le plan en porte, la modulent.
        let root_l = length_m.max(1e-9).sqrt();
        let base = match locomotion {
            Locomotion::Sessile | Locomotion::Drift => 0.0,
            Locomotion::Cilia => f64::min(1e-3, 10.0 * length_m),
            Locomotion::Undulate => 0.6 * root_l,
            Locomotion::Swim => 3.0 * root_l,
            Locomotion::Walk => 8.0 * root_l,
            Locomotion::Fly => 12.0 * root_l,
        };
        let muscle_factor = if muscle > 0.0 {
            ((muscle / (0.05 * length_m * length_m)).sqrt().clamp(0.5, 1.5)) * (0.8 + 0.4 * fast)
        } else {
            if matches!(locomotion, Locomotion::Swim | Locomotion::Walk | Locomotion::Fly) {
                proxies |= proxy::MUSCLE;
            }
            1.0
        };
        let sprint_m_s = base * muscle_factor;
        let cruise_m_s = 0.3 * sprint_m_s;

        // Système nerveux deviné : taille du corps (en cellules), diversité
        // des types cellulaires et signal entre cellules.
        let cells = p.cells().max(1.0);
        let types = p.cell_types().max(1) as f64;
        let signal = p.signalling.clamp(0.0, 1.0);
        let nervous = if p.is_multicellular() {
            proxies |= proxy::NERVOUS;
            (0.45 * (libm::log10(cells) / 10.0).clamp(0.0, 1.0) + 0.35 * signal + 0.2 * ((types - 1.0) / 6.0).clamp(0.0, 1.0))
                .clamp(0.0, 1.0)
        } else {
            0.0
        };
        let reaction_s = (1.0 / (0.5 + 9.5 * nervous)) * (length_m / 0.1).clamp(1e-3, 100.0).dpowf(0.2);

        // Perception chimique : quelques longueurs, plus loin avec le signal
        // et le système nerveux.
        proxies |= proxy::PERCEPTION;
        let perception_m = length_m * (5.0 + 25.0 * signal + 60.0 * nervous);

        let diet = if rates.prey_share >= 0.3 || (p.engulfment > 0.0 && rates.prey_uptake > 0.0) {
            Diet::Predator
        } else if autotroph {
            Diet::Autotroph
        } else {
            Diet::Grazer
        };

        let mut defence: f64 = 0.0;
        for m in &plan.modules {
            let c: f64 = match m.covering {
                Covering::Membrane => 0.0,
                Covering::Mucus | Covering::Wax => 0.1,
                Covering::Hair | Covering::Feathers => 0.2,
                Covering::Cuticle => 0.5,
                Covering::Scales => 0.6,
                Covering::Bark => 0.8,
            };
            let s = match m.material {
                Material::Bone | Material::Chitin | Material::Wood => 0.2,
                Material::Cartilage => 0.1,
                _ => 0.0,
            };
            defence = defence.max((c + s).min(1.0));
        }

        proxies |= proxy::SOCIALITY | proxy::CARE;
        let sociality = (nervous * (0.3 + 0.7 * signal)).clamp(0.0, 1.0);
        let care = (nervous * nervous * 1.5).clamp(0.0, 1.0);

        Traits {
            length_m,
            mass_kg,
            locomotion,
            cruise_m_s,
            sprint_m_s,
            perception_m,
            reaction_s,
            nervous,
            diet,
            defence,
            sociality,
            care,
            sexual: p.sexual,
            aquatic,
            proxies,
        }
    }

    /// Peut-il se déplacer par lui-même ?
    pub fn moves(&self) -> bool {
        self.sprint_m_s > 0.0
    }

    /// Un prédateur de ces traits peut-il s'attaquer à une proie de longueur
    /// `prey_m` ? [Simplification] Fenêtre de taille fixe (de 1/50 à 4/5 de
    /// sa longueur) ; la prédation selon la taille, la vitesse, les sens et
    /// les défenses appartient au fil « Animaux et écosystèmes ».
    pub fn can_eat(&self, prey_m: f64) -> bool {
        self.diet == Diet::Predator && prey_m >= self.length_m / 50.0 && prey_m <= 0.8 * self.length_m
    }
}
