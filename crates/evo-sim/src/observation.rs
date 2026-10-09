//! Canal d'observation (document Vision, « Cohérence des interfaces ») : la
//! zone d'intérêt de la caméra. Il ne change jamais l'histoire : la zone
//! décide seulement de ce que l'état publié détaille (populations des
//! cellules regardées, pour l'inspecteur). Le test
//! `camera_paths_do_not_change_history` le vérifie bit pour bit.

/// Zone que la caméra regarde.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct InterestZone {
    /// Cellule physique au centre de la vue.
    pub center_cell: u32,
    /// Rayon de la zone au sol, km.
    pub radius_km: f64,
    /// Bande de zoom du globe (1 : orbite ... 6 : loupe).
    pub zoom_band: u8,
}

/// Population d'une cellule regardée, pour l'inspecteur.
#[derive(Clone, Debug, PartialEq)]
pub struct PopulationView {
    pub lineage: u32,
    /// Espèce (guilde métabolique) : signature des réactions catalysées.
    pub species: u32,
    pub biomass: f32,
    /// Taux de croissance net, an⁻¹.
    pub growth_per_year: f32,
    /// Taux de production brut (naissances), an⁻¹ : biomasse × ce taux
    /// donne la production de la population (réseau trophique).
    pub birth_per_year: f32,
    pub genes: u16,
    pub pigment_rgb: Option<[u8; 3]>,
    /// Longueur d'onde d'absorption du pigment, nm.
    pub pigment_nm: Option<f32>,
    pub phototroph: bool,
    pub photosynthesis_stage: u8,
}

/// Détail d'une cellule de la grille du vivant dans la zone d'intérêt.
#[derive(Clone, Debug, PartialEq)]
pub struct FocusCell {
    /// Cellule de la grille du vivant.
    pub bio_cell: u32,
    pub populations: Vec<PopulationView>,
    /// Concentrations de la couche d'eau, mol·m⁻³ (ordre de `WATER_POOLS`).
    pub chemistry: Vec<f32>,
}

/// Ce que la zone d'intérêt ajoute à l'état publié.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Focus {
    pub zone: Option<InterestZone>,
    pub cells: Vec<FocusCell>,
}

/// Nombre maximal de cellules détaillées par état publié.
pub const MAX_FOCUS_CELLS: usize = 64;
