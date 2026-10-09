//! Plan de construction, côté affichage (document Rendu du vivant, « Champs
//! à demander dans le plan de construction ») : un graphe de modules, chacun
//! avec son parent et son point d'attache, son axe, ses dimensions, sa
//! symétrie et son nombre de répétitions, son articulation, son tissu de
//! revêtement et ses pigments.
//!
//! [Simplification] Ce type est la lecture qu'en fait le générateur
//! d'apparence. Le plan simulé appartient au moteur (fil « cellules
//! complexes », `evo_life::BodyPlan`) ; evo-view le traduit ici. Les
//! procaryotes passent par [`super::from_microbe`], qui garde leurs formes
//! du palier 1.

/// Version du format lu par le générateur.
pub const PLAN_VERSION: u16 = 1;

/// Nature d'un module : elle choisit la primitive qui le dessine.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ModuleKind {
    /// Tronc ou corps cellulaire : tube à section variable.
    Trunk,
    /// Segment répété d'un tronc.
    Segment,
    /// Membre articulé.
    Limb,
    /// Nageoire, aile : lame.
    Fin,
    /// Tête : ellipsoïde.
    Head,
    /// Tige d'un organisme fixé.
    Stalk,
    /// Feuille, fronde : lame.
    Leaf,
    /// Flagelle, cil, hyphe : filament.
    Flagellum,
    /// Antenne, tentacule : tube fin.
    Antenna,
    /// Pièce rigide (carapace, coquille, plaque).
    Shell,
    /// Œil, selon son stade.
    Eye(EyeStage),
    /// Bouche et mâchoires.
    Mouth,
    /// Branchie : lame fine.
    Gill,
    /// Organe interne, montré seulement dans la vue anatomie.
    Organ(System),
}

/// Stades de l'œil (Nilsson et Pelger, 1994).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EyeStage {
    Spot,
    Cup,
    Pinhole,
    Lens,
    Compound,
}

/// Appareil d'un organe interne : il donne sa couleur dans la vue anatomie.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum System {
    Digestive,
    Circulatory,
    Nervous,
    Respiratory,
    Reproductive,
    Skeletal,
    /// Membranes photosynthétiques, chloroplastes.
    Photosynthetic,
    /// Réserves (granules, graisses).
    Storage,
    /// Noyau d'une cellule eucaryote.
    Nucleus,
}

impl System {
    pub const ALL: [System; 9] = [
        System::Digestive,
        System::Circulatory,
        System::Nervous,
        System::Respiratory,
        System::Reproductive,
        System::Skeletal,
        System::Photosynthetic,
        System::Storage,
        System::Nucleus,
    ];

    /// Couleur conventionnelle des planches d'anatomie, en lavis.
    pub fn colour(self) -> [f32; 3] {
        match self {
            System::Digestive => [0.80, 0.62, 0.30],
            System::Circulatory => [0.66, 0.20, 0.16],
            System::Nervous => [0.93, 0.86, 0.55],
            System::Respiratory => [0.55, 0.66, 0.78],
            System::Reproductive => [0.78, 0.50, 0.62],
            System::Skeletal => [0.93, 0.91, 0.84],
            System::Photosynthetic => [0.36, 0.55, 0.28],
            System::Storage => [0.62, 0.52, 0.40],
            // Violet de l'hématoxyline, comme sur les coupes colorées.
            System::Nucleus => [0.42, 0.36, 0.62],
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            System::Digestive => "digestif",
            System::Circulatory => "circulatoire",
            System::Nervous => "nerveux",
            System::Respiratory => "respiratoire",
            System::Reproductive => "reproducteur",
            System::Skeletal => "squelette",
            System::Photosynthetic => "photosynthese",
            System::Storage => "reserves",
            System::Nucleus => "noyau",
        }
    }
}

/// Symétrie d'un module et de ses descendants, avec le nombre de copies.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Symmetry {
    None,
    /// Miroir par le plan sagittal (une paire).
    Bilateral,
    /// Répétition d'ordre n autour de l'axe du parent.
    Radial(u8),
    /// n copies réparties le long de l'axe du parent.
    Segmental(u8),
}

/// Tissu de revêtement : il donne le matériau.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Covering {
    /// Membrane d'une cellule nue.
    Membrane,
    /// Paroi cellulaire.
    Wall,
    Naked,
    Mucus,
    Scales,
    Hair,
    Feathers,
    Cuticle,
    Bark,
    Wax,
}

impl Covering {
    /// Brillance du matériau, 0 (mat) à 1 (luisant).
    pub fn gloss(self) -> f32 {
        match self {
            Covering::Mucus => 0.8,
            Covering::Membrane => 0.5,
            Covering::Scales | Covering::Cuticle | Covering::Wax => 0.6,
            Covering::Naked => 0.35,
            Covering::Wall => 0.3,
            Covering::Hair | Covering::Feathers | Covering::Bark => 0.1,
        }
    }
}

/// Articulation avec le parent.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Joint {
    /// Degrés de liberté (1 charnière, 2 cardan, 3 rotule).
    pub dof: u8,
    pub range_deg: f32,
}

/// Un module du plan.
#[derive(Clone, Debug, PartialEq)]
pub struct Module {
    pub kind: ModuleKind,
    /// Index du parent dans le plan (toujours plus petit que le sien).
    pub parent: Option<u16>,
    /// Point d'attache le long de l'axe du parent, 0 (base) à 1 (bout).
    pub attach: f32,
    /// Direction de départ autour de l'axe du parent, en degrés : 0 vers
    /// le dos, 90 vers la droite, 180 vers le ventre.
    pub around_deg: f32,
    /// Angle entre l'axe du module et celui du parent, en degrés.
    pub elevation_deg: f32,
    /// Longueur le long de l'axe et rayons à la base et au bout, en mètres.
    pub length_m: f32,
    pub radius_m: [f32; 2],
    /// Largeur d'une lame (nageoire, feuille), en mètres.
    pub width_m: f32,
    /// Courbure : déviation du bout vers le dos, en part de la longueur.
    pub curvature: f32,
    pub symmetry: Symmetry,
    pub joint: Option<Joint>,
    pub covering: Covering,
    /// Couleurs de réflectance des pigments, le premier dominant.
    pub pigments: Vec<[u8; 3]>,
    /// Couleur structurale (irisation), 0 à 1, distincte des pigments.
    pub iridescence: f32,
}

impl Module {
    /// Module sans parent, aux réglages neutres.
    pub fn new(kind: ModuleKind, length_m: f32, radius_m: f32) -> Module {
        Module {
            kind,
            parent: None,
            attach: 0.0,
            around_deg: 0.0,
            elevation_deg: 0.0,
            length_m,
            radius_m: [radius_m, radius_m],
            width_m: 0.0,
            curvature: 0.0,
            symmetry: Symmetry::None,
            joint: None,
            covering: Covering::Naked,
            pigments: Vec::new(),
            iridescence: 0.0,
        }
    }

    pub fn internal(&self) -> bool {
        matches!(self.kind, ModuleKind::Organ(_))
    }
}

/// Paramètres génétiques du motif de Turing (Gray-Scott) : taux
/// d'alimentation et de disparition, contraste.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pattern {
    pub feed: f32,
    pub kill: f32,
    /// 0 : pas de motif ; 1 : motif franc.
    pub contrast: f32,
    /// Échelle du motif : nombre de motifs le long du corps.
    pub scale: f32,
}

impl Pattern {
    pub const NONE: Pattern = Pattern { feed: 0.037, kill: 0.06, contrast: 0.0, scale: 1.0 };
}

/// Plan de construction d'une espèce, tel que l'affichage le lit.
#[derive(Clone, Debug, PartialEq)]
pub struct BodyPlan {
    pub version: u16,
    pub modules: Vec<Module>,
    /// Types cellulaires (1 pour un microbe).
    pub cell_types: u8,
    pub pattern: Pattern,
    /// Graine de l'apparence : même lignée, même corps.
    pub seed: u64,
}

impl BodyPlan {
    /// Vérifie que le graphe est un arbre enraciné au module 0.
    pub fn is_valid(&self) -> bool {
        !self.modules.is_empty()
            && self.modules[0].parent.is_none()
            && self.modules.iter().enumerate().skip(1).all(|(i, m)| m.parent.is_some_and(|p| (p as usize) < i))
            && self.modules.iter().all(|m| m.length_m.is_finite() && m.length_m >= 0.0 && m.radius_m.iter().all(|r| *r > 0.0))
    }

    /// Nombre de modules par nature, pour le comparateur.
    pub fn count(&self, f: impl Fn(&ModuleKind) -> bool) -> usize {
        self.modules.iter().filter(|m| f(&m.kind)).count()
    }
}
