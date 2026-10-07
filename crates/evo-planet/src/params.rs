//! Paramètres de planète. La Terre n'est qu'un préréglage : aucune valeur
//! terrestre n'est utilisée ailleurs dans le code. Les cinq autres
//! préréglages sont les mondes exotiques de la première vague (document
//! Planète, « Mondes exotiques ») : ils ne changent que des paramètres.

use evo_core::units::{GRAVITATIONAL_CONSTANT, STEFAN_BOLTZMANN};

#[derive(Clone, Debug, PartialEq)]
pub struct PlanetParams {
    /// Nom affiché du préréglage.
    pub name: String,

    // — Étoile et orbite —
    /// Luminosité de l'étoile au début de la partie, W.
    pub star_luminosity_w: f64,
    /// Croissance relative de la luminosité par milliard d'années.
    pub luminosity_growth_per_gyr: f64,
    /// Température de surface de l'étoile, K (fixe son spectre).
    pub star_temperature_k: f64,
    /// Demi-grand axe de l'orbite, m.
    pub orbit_m: f64,
    /// Inclinaison de l'axe au départ, rad.
    pub obliquity_rad: f64,
    /// Marche aléatoire de l'obliquité, rad par √Ma (0 quand une grosse lune
    /// la stabilise) ; elle reste entre 0 et `obliquity_max_rad`.
    pub obliquity_chaos_rad_per_sqrt_myr: f64,
    pub obliquity_max_rad: f64,

    // — Corps de la planète —
    /// Masse de la planète, kg.
    pub mass_kg: f64,
    /// Rayon de la planète, m.
    pub radius_m: f64,
    /// Temps caractéristique de décroissance de la chaleur interne, Ga (plus
    /// court pour une petite planète, qui refroidit vite).
    pub internal_heat_decay_gyr: f64,

    // — Climat (bilan d'énergie de type Budyko-Sellers, à l'équilibre) —
    /// Albédo planétaire d'une surface libre de glace (nuages compris), et
    /// d'une surface englacée.
    pub albedo: f64,
    pub albedo_ice: f64,
    /// Température moyenne annuelle sous laquelle une cellule est englacée, K.
    pub ice_temperature_k: f64,
    /// Rayonnement sortant OLR = A + B·(T − 273,15), W·m⁻² ; A est abaissé
    /// par l'effet de serre.
    pub olr_a: f64,
    pub olr_b: f64,
    /// Transport de chaleur méridien, W·m⁻²·K⁻¹ : plus il est faible, plus
    /// l'écart pôle-équateur est grand.
    pub heat_transport: f64,
    /// Forçage du CO₂ : coefficient × ln(p/p_réf), W·m⁻² ; du CH₄ :
    /// coefficient × (√ppb − √ppb_réf). Les références sont celles de
    /// l'ajustement de l'OLR.
    pub co2_forcing: f64,
    pub co2_reference_pa: f64,
    /// Terme d'élargissement par la pression aux fortes teneurs en CO₂,
    /// W·m⁻² pour 1 bar de CO₂ (croît comme la racine carrée de la pression).
    /// [Simplification] Remplace le calcul radiatif qui, au-delà de quelques
    /// dixièmes de bar, dépasse la loi logarithmique.
    pub co2_broadening_w_m2: f64,
    pub ch4_forcing: f64,
    pub ch4_reference_ppb: f64,
    /// Au-delà de ce rapport CH₄/CO₂, une brume organique se forme et le
    /// méthane ne réchauffe plus davantage.
    pub haze_ch4_co2_ratio: f64,
    /// Capacité calorifique massique de l'atmosphère, J·kg⁻¹·K⁻¹.
    pub air_heat_capacity: f64,
    /// Ozone : écran contre les UV. Facteur UV = 1 / (1 + force·√(fO₂/réf)).
    pub ozone_reference_mixing: f64,
    pub ozone_strength: f64,

    // — Atmosphère de départ —
    /// Pressions partielles initiales, Pa.
    pub n2_pa: f64,
    pub co2_pa: f64,
    pub ch4_pa: f64,
    pub h2_pa: f64,
    /// Échappement de l'hydrogène limité par la diffusion :
    /// molécules·m⁻²·s⁻¹ par unité de fraction molaire d'hydrogène total
    /// (proportionnel à la gravité).
    pub hydrogen_escape: f64,
    /// Oxydation photochimique du CH₄ par l'O₂ : taux, an⁻¹, par √fO₂.
    pub ch4_oxidation: f64,

    // — Eau et océan —
    /// Inventaire d'eau, en épaisseur de couche répartie sur toute la surface, m.
    pub water_inventory_m: f64,
    /// Profondeur de la couche de mélange océanique, m.
    pub mixed_layer_m: f64,
    /// Coefficient d'atténuation de la lumière dans l'eau, m⁻¹.
    pub water_light_attenuation: f64,
    /// Masse volumique de l'eau de mer, kg·m⁻³.
    pub seawater_density: f64,
    /// Salinité moyenne, g·kg⁻¹.
    pub salinity: f64,
    /// pH moyen de l'océan.
    pub ocean_ph: f64,
    /// Carbone inorganique dissous à l'équilibre avec la pression de CO₂ de
    /// départ, mol·m⁻³, et exposant de sa dépendance à la pression (effet
    /// tampon des carbonates).
    pub dic_equilibrium: f64,
    pub dic_co2_exponent: f64,
    /// Sulfate de la couche de surface, mol·m⁻³.
    pub sulfate_equilibrium: f64,
    /// Échange entre la couche de surface et l'océan profond, an⁻¹.
    pub upwelling_rate: f64,

    // — Tectonique —
    pub plate_count: u32,
    /// Vitesse typique des plaques au départ, cm·an⁻¹.
    pub plate_speed_cm_per_yr: f64,
    /// Part de la surface en croûte continentale au départ.
    pub continental_fraction: f64,
    /// Épaisseurs de croûte de référence, km, et masses volumiques.
    pub oceanic_crust_km: f64,
    pub continental_crust_km: f64,
    pub oceanic_crust_density: f64,
    pub continental_crust_density: f64,
    pub mantle_density: f64,
    /// Profondeur des dorsales sous le niveau de référence, km, et
    /// subsidence thermique de la croûte océanique, km·Ma^(−1/2).
    pub ridge_depth_km: f64,
    pub subsidence_km_per_sqrt_myr: f64,
    /// Période des réorganisations de plaques, Ma.
    pub plate_reorganisation_myr: f64,
    /// Temps caractéristique de l'érosion des reliefs épaissis, Ma.
    pub erosion_myr: f64,
    /// Âge de croûte sous lequel une cellule océanique porte des sources
    /// hydrothermales (axe des dorsales), Ma.
    pub vent_crust_age_myr: f64,

    // — Volcanisme et hydrothermalisme (au départ, proportionnels à
    //   l'activité tectonique et à la chaleur interne) —
    /// Dégazage volcanique de CO₂, mol·an⁻¹, et part de gaz réduits (H₂)
    /// par mole de CO₂ (état d'oxydation du manteau).
    pub outgassing_co2: f64,
    pub outgassing_h2_ratio: f64,
    /// Flux hydrothermaux globaux, mol·an⁻¹ ; une partie sort aux sources de
    /// la couche de surface, le reste va à l'océan profond.
    pub vent_h2_flux: f64,
    pub vent_h2s_flux: f64,
    pub vent_fe_flux: f64,
    pub vent_mn_flux: f64,
    pub vent_local_share: f64,

    // — Cycles géochimiques —
    /// Altération des silicates des terres, mol de CO₂·m⁻²·an⁻¹ à la
    /// température et à la pression de CO₂ de référence ; sensibilité à la
    /// température (K) et exposant de pression (Walker et coll., 1981).
    pub weathering_per_m2: f64,
    pub weathering_reference_k: f64,
    pub weathering_activation_k: f64,
    pub weathering_co2_exponent: f64,
    /// Altération des fonds océaniques, en part du dégazage de référence.
    pub seafloor_weathering_share: f64,
    /// Phosphore libéré par mole de CO₂ consommée par l'altération.
    pub weathering_phosphorus_ratio: f64,
    /// Part du carbone organique exporté qui est enfoui dans les sédiments.
    pub organic_burial_efficiency: f64,
    /// Rapport C/P du carbone organique enfoui, et P piégé par mole
    /// d'oxydes de fer déposés.
    pub burial_carbon_to_phosphorus: f64,
    pub iron_oxide_phosphorus: f64,
    /// Fraction molaire d'O₂ qui rend aérobie la moitié de la
    /// reminéralisation profonde, et l'oxydation du fer profond.
    pub deep_oxic_half_mixing: f64,
    /// Oxydation des roches réduites exposées (sulfures, fer, kérogène) :
    /// maximum en mol d'O₂·m⁻²·an⁻¹ et fraction de demi-effet.
    pub oxidative_weathering_per_m2: f64,
    pub oxidative_weathering_half: f64,
    /// Temps de résidence du fer et du manganèse dans un océan profond
    /// anoxique (dépôt de sidérite, pyrite), ans.
    pub deep_metal_residence_years: f64,
}

impl PlanetParams {
    /// Préréglage Terre, à l'Archéen : océans formés, atmosphère sans
    /// oxygène, beaucoup de CO₂, étoile plus faible (environ 80 %).
    pub fn earth_archean() -> Self {
        Self {
            name: "Terre (Archéen)".into(),
            star_luminosity_w: 0.8 * 3.828e26,
            luminosity_growth_per_gyr: 0.066,
            star_temperature_k: 5600.0,
            orbit_m: 1.496e11,
            obliquity_rad: 23.44_f64.to_radians(),
            obliquity_chaos_rad_per_sqrt_myr: 0.0,
            obliquity_max_rad: 85_f64.to_radians(),
            mass_kg: 5.972e24,
            radius_m: 6.371e6,
            internal_heat_decay_gyr: 3.0,
            albedo: 0.3,
            albedo_ice: 0.62,
            ice_temperature_k: 263.15,
            olr_a: 203.3,
            olr_b: 2.09,
            heat_transport: 3.04,
            co2_forcing: 5.35,
            co2_reference_pa: 28.4,
            co2_broadening_w_m2: 30.0,
            ch4_forcing: 0.036,
            ch4_reference_ppb: 700.0,
            haze_ch4_co2_ratio: 0.1,
            air_heat_capacity: 1004.0,
            ozone_reference_mixing: 2e-3,
            ozone_strength: 5.0,
            n2_pa: 8.0e4,
            co2_pa: 1.0e4,
            ch4_pa: 0.0,
            h2_pa: 0.0,
            hydrogen_escape: 2.5e17,
            ch4_oxidation: 0.22,
            water_inventory_m: 2630.0,
            mixed_layer_m: 100.0,
            water_light_attenuation: 0.04,
            seawater_density: 1025.0,
            salinity: 35.0,
            ocean_ph: 7.2,
            dic_equilibrium: 8.0,
            dic_co2_exponent: 0.3,
            sulfate_equilibrium: 0.2,
            upwelling_rate: 0.01,
            plate_count: 14,
            plate_speed_cm_per_yr: 6.0,
            continental_fraction: 0.25,
            oceanic_crust_km: 7.0,
            continental_crust_km: 35.0,
            oceanic_crust_density: 2.9,
            continental_crust_density: 2.75,
            mantle_density: 3.3,
            ridge_depth_km: 2.6,
            subsidence_km_per_sqrt_myr: 0.35,
            plate_reorganisation_myr: 150.0,
            erosion_myr: 100.0,
            vent_crust_age_myr: 4.0,
            outgassing_co2: 1.5e13,
            outgassing_h2_ratio: 0.03,
            vent_h2_flux: 1.0e12,
            vent_h2s_flux: 2.0e11,
            vent_fe_flux: 5.0e11,
            vent_mn_flux: 5.0e10,
            vent_local_share: 0.3,
            weathering_per_m2: 0.12,
            weathering_reference_k: 288.0,
            weathering_activation_k: 13.7,
            weathering_co2_exponent: 0.3,
            seafloor_weathering_share: 0.15,
            weathering_phosphorus_ratio: 0.004,
            organic_burial_efficiency: 0.05,
            burial_carbon_to_phosphorus: 250.0,
            iron_oxide_phosphorus: 0.02,
            deep_oxic_half_mixing: 1e-3,
            oxidative_weathering_per_m2: 0.08,
            oxidative_weathering_half: 1e-3,
            deep_metal_residence_years: 2.0e5,
        }
    }

    /// Monde océan : dix fois l'inventaire d'eau terrestre, aucune terre
    /// émergée ; seule l'altération des fonds règle le CO₂.
    pub fn ocean_world() -> Self {
        Self { name: "Monde océan".into(), water_inventory_m: 26_300.0, ..Self::earth_archean() }
    }

    /// Monde désertique : un dixième de l'eau terrestre, des mers fermées
    /// dans les bassins les plus profonds.
    pub fn desert_world() -> Self {
        Self { name: "Monde désertique".into(), water_inventory_m: 300.0, ..Self::earth_archean() }
    }

    /// Super-Terre : quatre masses terrestres, 1,5 rayon. Gravité forte,
    /// atmosphère plus épaisse, chaleur interne et dégazage plus forts,
    /// échappement de l'hydrogène plus lent… et plus d'eau.
    pub fn super_earth() -> Self {
        let e = Self::earth_archean();
        Self {
            name: "Super-Terre".into(),
            mass_kg: 4.0 * e.mass_kg,
            radius_m: 1.5 * e.radius_m,
            internal_heat_decay_gyr: 5.0,
            n2_pa: 2.0e5,
            co2_pa: 1.5e4,
            hydrogen_escape: e.hydrogen_escape * 4.0 / 2.25,
            water_inventory_m: e.water_inventory_m * 4.0 / 2.25,
            outgassing_co2: e.outgassing_co2 * 4.0,
            vent_h2_flux: e.vent_h2_flux * 4.0,
            vent_h2s_flux: e.vent_h2s_flux * 4.0,
            vent_fe_flux: e.vent_fe_flux * 4.0,
            vent_mn_flux: e.vent_mn_flux * 4.0,
            ..e
        }
    }

    /// Petite planète : 0,3 masse terrestre, 0,7 rayon. Elle refroidit vite :
    /// sa tectonique et son dégazage s'essoufflent ; atmosphère plus mince.
    pub fn small_planet() -> Self {
        let e = Self::earth_archean();
        Self {
            name: "Petite planète".into(),
            mass_kg: 0.3 * e.mass_kg,
            radius_m: 0.7 * e.radius_m,
            internal_heat_decay_gyr: 1.5,
            n2_pa: 4.0e4,
            co2_pa: 8.0e3,
            hydrogen_escape: e.hydrogen_escape * 0.3 / 0.49,
            water_inventory_m: e.water_inventory_m * 0.3 / 0.49,
            outgassing_co2: e.outgassing_co2 * 0.3,
            vent_h2_flux: e.vent_h2_flux * 0.3,
            vent_h2s_flux: e.vent_h2s_flux * 0.3,
            vent_fe_flux: e.vent_fe_flux * 0.3,
            vent_mn_flux: e.vent_mn_flux * 0.3,
            ..e
        }
    }

    /// Monde sans lune massive : l'obliquité n'est plus stabilisée et dérive
    /// de façon chaotique entre 0 et 85° (Laskar et coll., 1993).
    pub fn moonless() -> Self {
        Self { name: "Monde sans lune".into(), obliquity_chaos_rad_per_sqrt_myr: 0.06, ..Self::earth_archean() }
    }

    /// Les six mondes de la première vague, sur lesquels la porte de
    /// l'étape 2 est vérifiée.
    pub fn wave_one() -> Vec<Self> {
        vec![Self::earth_archean(), Self::ocean_world(), Self::desert_world(), Self::super_earth(), Self::small_planet(), Self::moonless()]
    }

    /// Préréglage par nom court (outil en ligne de commande).
    pub fn by_key(key: &str) -> Option<Self> {
        match key {
            "terre" => Some(Self::earth_archean()),
            "ocean" => Some(Self::ocean_world()),
            "desert" => Some(Self::desert_world()),
            "super-terre" => Some(Self::super_earth()),
            "petite" => Some(Self::small_planet()),
            "sans-lune" => Some(Self::moonless()),
            _ => None,
        }
    }

    pub const KEYS: [&'static str; 6] = ["terre", "ocean", "desert", "super-terre", "petite", "sans-lune"];

    /// Gravité de surface, m·s⁻², déduite de la masse et du rayon.
    pub fn gravity(&self) -> f64 {
        GRAVITATIONAL_CONSTANT * self.mass_kg / (self.radius_m * self.radius_m)
    }

    /// Luminosité de l'étoile après `years` années de partie, W.
    pub fn luminosity_at(&self, years: f64) -> f64 {
        self.star_luminosity_w * (1.0 + self.luminosity_growth_per_gyr * years / 1e9)
    }

    /// Flux stellaire au sommet de l'atmosphère au départ, W·m⁻².
    pub fn stellar_flux(&self) -> f64 {
        self.star_luminosity_w / (4.0 * std::f64::consts::PI * self.orbit_m * self.orbit_m)
    }

    /// Température d'équilibre radiatif sans effet de serre, K.
    pub fn equilibrium_temperature(&self) -> f64 {
        let absorbed = self.stellar_flux() * (1.0 - self.albedo) / 4.0;
        (absorbed / STEFAN_BOLTZMANN).powf(0.25)
    }

    /// Gradient thermique vertical, K·m⁻¹ : adiabatique sèche g/cp corrigée
    /// par un facteur humide fixe (environ 6,4 K/km sur Terre).
    pub fn lapse_rate(&self) -> f64 {
        0.66 * self.gravity() / self.air_heat_capacity
    }

    /// Surface totale, m².
    pub fn surface_area(&self) -> f64 {
        4.0 * std::f64::consts::PI * self.radius_m * self.radius_m
    }

    /// Chaleur interne relative à celle du départ.
    pub fn internal_heat(&self, years: f64) -> f64 {
        (-years / (self.internal_heat_decay_gyr * 1e9)).exp()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn earth_preset_derived_values_are_plausible() {
        let p = PlanetParams::earth_archean();
        assert!((p.gravity() - 9.82).abs() < 0.05);
        assert!((p.stellar_flux() - 0.8 * 1361.0).abs() < 5.0);
        assert!((p.lapse_rate() * 1000.0 - 6.45).abs() < 0.1);
        // Le Soleil revient à sa luminosité actuelle en environ 3,8 Ga.
        assert!((p.luminosity_at(3.8e9) / p.star_luminosity_w - 1.25).abs() < 0.01);
    }

    #[test]
    fn wave_one_has_six_distinct_worlds() {
        let worlds = PlanetParams::wave_one();
        assert_eq!(worlds.len(), 6);
        let g: Vec<f64> = worlds.iter().map(|w| w.gravity()).collect();
        assert!(g[3] > 1.5 * g[0] && g[4] < 0.7 * g[0]);
        for key in PlanetParams::KEYS {
            assert!(PlanetParams::by_key(key).is_some());
        }
    }
}
