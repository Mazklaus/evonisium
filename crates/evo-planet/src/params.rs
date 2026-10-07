//! Paramètres de planète. La Terre n'est qu'un préréglage : aucune valeur
//! terrestre n'est utilisée ailleurs dans le code.

use evo_core::units::{GRAVITATIONAL_CONSTANT, STEFAN_BOLTZMANN};

#[derive(Clone, Debug, PartialEq)]
pub struct PlanetParams {
    /// Nom affiché du préréglage.
    pub name: String,
    /// Luminosité de l'étoile, W.
    pub star_luminosity_w: f64,
    /// Demi-grand axe de l'orbite, m.
    pub orbit_m: f64,
    /// Masse de la planète, kg.
    pub mass_kg: f64,
    /// Rayon de la planète, m.
    pub radius_m: f64,
    /// Inclinaison de l'axe, rad.
    pub obliquity_rad: f64,
    /// Albédo de Bond moyen.
    pub albedo: f64,
    /// Réchauffement par effet de serre, K (l'étape 2 le calculera depuis
    /// l'atmosphère ; ici c'est un paramètre).
    pub greenhouse_k: f64,
    /// Part de la surface couverte d'océans.
    pub ocean_fraction: f64,
    /// Coefficient du bilan d'énergie latitudinal (type Budyko), W·m⁻²·K⁻¹ :
    /// plus il est faible, plus l'écart pôle-équateur est grand.
    pub heat_transport: f64,
    /// Capacité calorifique massique de l'atmosphère, J·kg⁻¹·K⁻¹.
    pub air_heat_capacity: f64,
    /// Profondeur de la couche de mélange océanique, m.
    pub mixed_layer_m: f64,
    /// Coefficient d'atténuation de la lumière dans l'eau, m⁻¹.
    pub water_light_attenuation: f64,
    /// Pression atmosphérique de surface, Pa.
    pub surface_pressure_pa: f64,
    /// Masse volumique de l'eau de mer, kg·m⁻³.
    pub seawater_density: f64,
    /// Salinité moyenne, g·kg⁻¹.
    pub salinity: f64,
    /// pH moyen de l'océan.
    pub ocean_ph: f64,
    /// Part des cellules océaniques portant des sources hydrothermales.
    pub vent_fraction: f64,
    /// Flux hydrothermal global de H₂, mol·an⁻¹.
    pub vent_h2_flux: f64,
    /// Flux hydrothermal global de H₂S, mol·an⁻¹.
    pub vent_h2s_flux: f64,
    /// Concentrations de référence de la couche de surface, mol·m⁻³ :
    /// carbone inorganique dissous, sulfate.
    pub dic_equilibrium: f64,
    pub sulfate_equilibrium: f64,
    /// H₂ dissous en équilibre avec l'atmosphère, mol·m⁻³ (l'atmosphère
    /// archéenne contenait de l'ordre de 0,1 % de H₂).
    pub h2_equilibrium: f64,
}

impl PlanetParams {
    /// Préréglage Terre, à l'Archéen : océans formés, atmosphère sans
    /// oxygène, beaucoup de CO₂, étoile plus faible (environ 80 %).
    pub fn earth_archean() -> Self {
        Self {
            name: "Terre (Archéen)".into(),
            star_luminosity_w: 0.8 * 3.828e26,
            orbit_m: 1.496e11,
            mass_kg: 5.972e24,
            radius_m: 6.371e6,
            obliquity_rad: 23.44_f64.to_radians(),
            albedo: 0.3,
            greenhouse_k: 55.0,
            ocean_fraction: 0.71,
            heat_transport: 3.0,
            air_heat_capacity: 1004.0,
            mixed_layer_m: 100.0,
            water_light_attenuation: 0.04,
            surface_pressure_pa: 101_325.0,
            seawater_density: 1025.0,
            salinity: 35.0,
            ocean_ph: 7.2,
            vent_fraction: 0.02,
            vent_h2_flux: 1.0e12,
            vent_h2s_flux: 2.0e11,
            dic_equilibrium: 8.0,
            sulfate_equilibrium: 0.2,
            h2_equilibrium: 8.0e-4,
        }
    }

    /// Gravité de surface, m·s⁻², déduite de la masse et du rayon.
    pub fn gravity(&self) -> f64 {
        GRAVITATIONAL_CONSTANT * self.mass_kg / (self.radius_m * self.radius_m)
    }

    /// Flux stellaire au sommet de l'atmosphère, W·m⁻².
    pub fn stellar_flux(&self) -> f64 {
        self.star_luminosity_w / (4.0 * std::f64::consts::PI * self.orbit_m * self.orbit_m)
    }

    /// Température d'équilibre radiatif moyenne, K, effet de serre compris.
    pub fn mean_surface_temperature(&self) -> f64 {
        let absorbed = self.stellar_flux() * (1.0 - self.albedo) / 4.0;
        (absorbed / STEFAN_BOLTZMANN).powf(0.25) + self.greenhouse_k
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn earth_preset_derived_values_are_plausible() {
        let p = PlanetParams::earth_archean();
        assert!((p.gravity() - 9.82).abs() < 0.05);
        assert!((p.stellar_flux() - 0.8 * 1361.0).abs() < 5.0);
        let t = p.mean_surface_temperature();
        assert!((270.0..310.0).contains(&t), "température moyenne {t}");
        assert!((p.lapse_rate() * 1000.0 - 6.45).abs() < 0.1);
    }
}
