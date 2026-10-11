//! Réservoirs globaux et cycles géochimiques en boîtes (document Planète,
//! section 8, approche des modèles de type GEOCARB et COPSE).
//!
//! Atmosphère bien mélangée, océan profond, sédiments. Les cellules
//! océaniques de surface échangent avec ces réservoirs ; leurs flux annuels,
//! mesurés pendant l'écologie rapide, sont appliqués sur tout le pas en
//! sous-pas de 10 000 ans au plus (condition « sous-pas pour les cycles
//! chimiques globaux » du seuil climatique), avec un schéma implicite pour
//! les puits rapides.
//!
//! Budget de l'oxygène libre : sa seule source est la photosynthèse
//! oxygénique des cellules ; ses puits sont tenus par processus (respiration
//! profonde, gaz volcaniques et hydrothermaux réduits, fer et manganèse,
//! méthane, roches exposées). C'est ce budget qui prouve la porte de
//! l'étape 2 : l'oxygène ne s'accumule que si l'enfouissement de carbone
//! organique dépasse les puits.
//!
//! [Simplification] Le carbone inorganique de l'océan profond est confondu
//! avec celui de l'atmosphère ; la chimie atmosphérique est résumée en taux
//! (oxydation du méthane, titrage H₂-O₂, échappement de l'hydrogène limité
//! par la diffusion).
//!
//! Cycle du soufre (étape 5, type GEOCARBSULF, Berner, 2006) : le sulfate de
//! l'océan profond vient de l'oxydation de la pyrite des roches exposées ;
//! dans un océan profond sans oxygène, il oxyde la matière organique
//! (sulfato-réduction) et le méthane (oxydation anaérobie) avant les
//! méthanogènes ; une part du sulfure produit est enfouie en pyrite, source
//! nette d'oxygène, le reste est réoxydé. [Simplification] Pas de soufre
//! volcanique ; le fer de la pyrite n'est pas suivi (la pyrite compte comme
//! du sulfure) ; le gypse est enfoui à proportion du stock.

use crate::params::PlanetParams;
use crate::pools::{WaterPool, WATER_POOL_COUNT};
use evo_core::flux::{Element, FluxRegistry};
use evo_core::math::Det;

/// Gaz de l'atmosphère suivis.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Gas {
    N2 = 0,
    O2 = 1,
    Co2 = 2,
    Ch4 = 3,
    H2 = 4,
}

pub const GAS_COUNT: usize = 5;
pub const GASES: [Gas; GAS_COUNT] = [Gas::N2, Gas::O2, Gas::Co2, Gas::Ch4, Gas::H2];

impl Gas {
    /// Masse molaire, kg·mol⁻¹.
    pub fn molar_mass(self) -> f64 {
        match self {
            Gas::N2 => 0.028,
            Gas::O2 => 0.032,
            Gas::Co2 => 0.044,
            Gas::Ch4 => 0.016,
            Gas::H2 => 0.002,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Gas::N2 => "N2",
            Gas::O2 => "O2",
            Gas::Co2 => "CO2",
            Gas::Ch4 => "CH4",
            Gas::H2 => "H2",
        }
    }
}

/// Puits d'oxygène, cumulés depuis le début de la partie, mol d'O₂.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct OxygenBudget {
    /// O₂ produit par la photosynthèse oxygénique dans les cellules (brut).
    pub photosynthesis: f64,
    /// O₂ libéré net par la couche de surface vers l'atmosphère.
    pub surface_release: f64,
    /// O₂ repris par la couche de surface (respiration, oxydations locales).
    pub surface_uptake: f64,
    pub deep_respiration: f64,
    pub reduced_gases: f64,
    pub methane: f64,
    pub iron_manganese: f64,
    pub oxidative_weathering: f64,
    pub sulfide: f64,
    /// Oxydation de la croûte océanique jeune par l'eau de mer (étape 3).
    pub seafloor_oxidation: f64,
    /// O₂ laissé par la litière de la terre ferme enfouie (source, étape 5).
    #[serde(default)]
    pub land_burial: f64,
}

impl OxygenBudget {
    pub fn total_sinks(&self) -> f64 {
        self.surface_uptake
            + self.deep_respiration
            + self.reduced_gases
            + self.methane
            + self.iron_manganese
            + self.oxidative_weathering
            + self.sulfide
            + self.seafloor_oxidation
    }
}

/// Réservoirs globaux de la planète.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GlobalReservoirs {
    /// Atmosphère, mol par gaz.
    pub atmosphere: [f64; GAS_COUNT],
    /// Océan profond, mol.
    pub deep_fe2: f64,
    pub deep_mn2: f64,
    pub deep_po4: f64,
    /// Sédiments, mol.
    pub carbonate_c: f64,
    pub organic_c: f64,
    pub iron_oxides: f64,
    pub iron_reduced: f64,
    /// Manganèse réduit déposé (carbonates de manganèse), mol.
    pub manganese_reduced: f64,
    pub manganese_oxides: f64,
    pub sediment_p: f64,
    /// Sulfate de l'océan profond, mol.
    pub deep_so4: f64,
    /// Soufre réduit enfoui dans les sédiments (pyrite), mol de S.
    pub pyrite_s: f64,
    /// Sulfate enfoui en évaporites (gypse), mol de S.
    pub gypsum_s: f64,
    pub oxygen: OxygenBudget,
    /// Flux du dernier pas, mol·an⁻¹, pour les rapports.
    pub last: GlobalFluxes,
}

/// Flux globaux moyens du dernier pas, mol·an⁻¹.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GlobalFluxes {
    pub outgassing_co2: f64,
    pub weathering_co2: f64,
    pub organic_export: f64,
    pub organic_burial: f64,
    pub oxygen_release: f64,
    pub oxygen_sinks: f64,
    pub methane_release: f64,
    pub hydrogen_escape: f64,
    /// Pouvoir oxydant exporté par les couches de surface prolongées (leurs
    /// sources hydrothermales), mol d'équivalent O₂ par an.
    pub surface_redox: f64,
}

/// Ce que le reste de la planète fournit aux boîtes pour un pas.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoxContext {
    /// Aire des terres émergées, non englacées et arrosées par la pluie, m².
    pub land_area_m2: f64,
    /// Volume de l'océan profond, m³.
    pub deep_volume_m3: f64,
    /// Température moyenne de surface, K.
    pub mean_temperature_k: f64,
    /// Activité volcanique et hydrothermale relative au départ (expansion
    /// des dorsales × chaleur interne).
    pub activity: f64,
    /// Part de l'hydrothermalisme sous-marin active (dorsales immergées).
    pub hydrothermal_share: f64,
    /// Part des sédiments marins entraînée en subduction par an.
    pub subduction_per_year: f64,
    /// Gravité et aire de la planète, pour la pression.
    pub gravity: f64,
    pub area_m2: f64,
    /// Accélération de l'altération des silicates et du phosphore par la vie
    /// de la terre ferme, 1 sans elle (étape 5).
    pub biotic_weathering: f64,
    /// Phosphore de l'altération des terres livré directement aux sols des
    /// cellules de la terre ferme, mol·an⁻¹ : il rejoint l'océan profond par
    /// les eaux de ces cellules au lieu d'y entrer d'un coup.
    pub land_phosphorus_routed: f64,
    /// Carbone de la litière de la terre ferme enfoui, mol·an⁻¹ : pris au CO₂
    /// de l'air par la photosynthèse, il laisse son O₂ (étape 5).
    pub land_burial: f64,
}

impl GlobalReservoirs {
    /// Atmosphère initiale à partir des pressions partielles du préréglage.
    pub fn new(params: &PlanetParams, deep_volume_m3: f64) -> Self {
        let g = params.gravity();
        let area = params.surface_area();
        let pressures = [params.n2_pa, 0.0, params.co2_pa, params.ch4_pa, params.h2_pa];
        let mean_mass: f64 = {
            let total: f64 = pressures.iter().sum();
            GASES.iter().map(|&gas| pressures[gas as usize] / total * gas.molar_mass()).sum()
        };
        // p_i = x_i · P, P = g · Σ(n_i·M_i) / A ⇒ n_i = p_i · A / (g · M̄).
        let mut atmosphere = pressures.map(|p| p * area / (g * mean_mass));
        if params.h2_pa <= 0.0 {
            // Sans valeur imposée, l'hydrogène part de l'équilibre entre les
            // sources abiotiques (volcans, sources hydrothermales) et
            // l'échappement vers l'espace, atteint en quelques dizaines de
            // milliers d'années.
            let sources = params.outgassing_co2 * params.reduced_outgassing_ratio() + params.vent_h2_flux;
            let esc_per_mixing = params.hydrogen_escape * area * evo_core::units::SECONDS_PER_YEAR / 6.022_140_76e23;
            let total: f64 = atmosphere.iter().sum();
            let f = sources / esc_per_mixing;
            atmosphere[Gas::H2 as usize] = f / (1.0 - f) * total;
        }
        // Océan profond de départ : un peu de fer, de manganèse et de phosphate.
        Self {
            atmosphere,
            deep_fe2: 0.05 * deep_volume_m3,
            deep_mn2: 0.005 * deep_volume_m3,
            deep_po4: 1e-3 * deep_volume_m3,
            carbonate_c: 0.0,
            organic_c: 0.0,
            iron_oxides: 0.0,
            iron_reduced: 0.0,
            manganese_reduced: 0.0,
            manganese_oxides: 0.0,
            sediment_p: 0.0,
            deep_so4: params.sulfate_equilibrium * deep_volume_m3,
            pyrite_s: 0.0,
            gypsum_s: 0.0,
            oxygen: OxygenBudget::default(),
            last: GlobalFluxes::default(),
        }
    }

    pub fn total_moles(&self) -> f64 {
        self.atmosphere.iter().sum()
    }

    pub fn mixing_ratio(&self, gas: Gas) -> f64 {
        self.atmosphere[gas as usize] / self.total_moles().max(1e-300)
    }

    /// Pression de surface, Pa.
    pub fn pressure_pa(&self, gravity: f64, area_m2: f64) -> f64 {
        let mass: f64 = GASES.iter().map(|&g| self.atmosphere[g as usize] * g.molar_mass()).sum();
        mass * gravity / area_m2
    }

    pub fn partial_pressure(&self, gas: Gas, gravity: f64, area_m2: f64) -> f64 {
        self.mixing_ratio(gas) * self.pressure_pa(gravity, area_m2)
    }

    /// Carbone des réservoirs globaux, mol.
    pub fn carbon(&self) -> f64 {
        self.atmosphere[Gas::Co2 as usize] + self.atmosphere[Gas::Ch4 as usize] + self.carbonate_c + self.organic_c
    }

    /// Pouvoir oxydant des réservoirs globaux, mol d'équivalent O₂ (voir
    /// [`evo_core::flux::Element::Electrons`]).
    pub fn electrons(&self) -> f64 {
        let a = &self.atmosphere;
        a[Gas::O2 as usize]
            - 2.0 * a[Gas::Ch4 as usize]
            - 0.5 * a[Gas::H2 as usize]
            - 0.25 * self.deep_fe2
            - 0.5 * self.deep_mn2
            - self.organic_c
            - 0.25 * self.iron_reduced
            - 0.5 * self.manganese_reduced
            - 2.0 * self.pyrite_s
    }

    /// Phosphore des réservoirs globaux, mol.
    pub fn phosphorus(&self) -> f64 {
        self.deep_po4 + self.sediment_p
    }

    /// Réservoir global qui fait face à un pool de la couche de surface,
    /// quand il existe (les autres échanges sortent du système suivi ou vont
    /// à la reminéralisation).
    fn counterpart(&mut self, pool: WaterPool) -> Option<&mut f64> {
        match pool {
            WaterPool::Dic => Some(&mut self.atmosphere[Gas::Co2 as usize]),
            WaterPool::Ch4 => Some(&mut self.atmosphere[Gas::Ch4 as usize]),
            WaterPool::O2 => Some(&mut self.atmosphere[Gas::O2 as usize]),
            WaterPool::H2 => Some(&mut self.atmosphere[Gas::H2 as usize]),
            WaterPool::Fe2 => Some(&mut self.deep_fe2),
            WaterPool::Mn2 => Some(&mut self.deep_mn2),
            WaterPool::Po4 => Some(&mut self.deep_po4),
            WaterPool::FeOx => Some(&mut self.iron_oxides),
            WaterPool::MnOx => Some(&mut self.manganese_oxides),
            WaterPool::Sulfate => Some(&mut self.deep_so4),
            // Le carbone organique exporté est traité par `remineralise`.
            WaterPool::Doc | WaterPool::H2s => None,
        }
    }

    fn counterpart_ref(&self, pool: WaterPool) -> Option<&f64> {
        match pool {
            WaterPool::Dic => Some(&self.atmosphere[Gas::Co2 as usize]),
            WaterPool::Ch4 => Some(&self.atmosphere[Gas::Ch4 as usize]),
            WaterPool::O2 => Some(&self.atmosphere[Gas::O2 as usize]),
            WaterPool::H2 => Some(&self.atmosphere[Gas::H2 as usize]),
            WaterPool::Fe2 => Some(&self.deep_fe2),
            WaterPool::Mn2 => Some(&self.deep_mn2),
            WaterPool::Po4 => Some(&self.deep_po4),
            WaterPool::FeOx => Some(&self.iron_oxides),
            WaterPool::MnOx => Some(&self.manganese_oxides),
            WaterPool::Sulfate => Some(&self.deep_so4),
            WaterPool::Doc | WaterPool::H2s => None,
        }
    }

    /// Applique des échanges de surface `moles` (sortie des cellules
    /// positive), sur une durée `years`, et renvoie les moles de carbone
    /// organique exporté, enfoui, et d'H₂S dégazé.
    fn apply_surface(
        &mut self,
        params: &PlanetParams,
        moles: &[f64; WATER_POOL_COUNT],
        deep_oxic: f64,
        deep_volume_m3: f64,
    ) -> (f64, f64, f64) {
        for (i, &m) in moles.iter().enumerate() {
            let pool = crate::pools::WATER_POOLS[i];
            if let Some(r) = self.counterpart(pool) {
                *r += m;
            }
        }
        let o2 = moles[WaterPool::O2 as usize];
        if o2 >= 0.0 {
            self.oxygen.surface_release += o2;
        } else {
            self.oxygen.surface_uptake -= o2;
        }
        // Les oxydes qui sédimentent piègent du phosphate de l'océan profond.
        self.scavenge_phosphorus(params, moles[WaterPool::FeOx as usize].max(0.0));
        let export = moles[WaterPool::Doc as usize];
        let buried = self.remineralise(params, export, deep_oxic, deep_volume_m3);
        (export, buried, moles[WaterPool::H2s as usize])
    }

    fn scavenge_phosphorus(&mut self, params: &PlanetParams, iron_oxides: f64) {
        let p = (iron_oxides * params.iron_oxide_phosphorus).min(self.deep_po4).max(0.0);
        self.deep_po4 -= p;
        self.sediment_p += p;
    }

    /// Devenir du carbone organique exporté : une part est enfouie (avec son
    /// phosphore), le reste est reminéralisé, par l'O₂ s'il y en a, sinon par
    /// méthanogenèse (moitié CH₄, moitié CO₂). L'enfouissement emporte du
    /// phosphore au rapport C/P des sédiments : quand l'océan profond n'en a
    /// plus, la matière organique est reminéralisée au lieu d'être enfouie.
    /// Renvoie le carbone enfoui.
    fn remineralise(&mut self, params: &PlanetParams, export: f64, deep_oxic: f64, deep_volume_m3: f64) -> f64 {
        if export <= 0.0 {
            // Import net de carbone organique : impossible à cette échelle.
            return 0.0;
        }
        let cp = params.burial_carbon_to_phosphorus(deep_oxic);
        let buried = (export * params.burial_efficiency(deep_oxic)).min(self.deep_po4.max(0.0) * cp);
        self.organic_c += buried;
        let p = buried / cp;
        self.deep_po4 -= p;
        self.sediment_p += p;
        let rest = export - buried;
        let o2 = &mut self.atmosphere[Gas::O2 as usize];
        let aerobic = (rest * deep_oxic).min(*o2).max(0.0);
        *o2 -= aerobic;
        self.oxygen.deep_respiration += aerobic;
        let anaerobic = rest - aerobic;
        // Sulfato-réduction avant la méthanogenèse quand il y a du sulfate
        // (2 C oxydés par sulfate réduit), puis oxydation anaérobie du
        // méthane par le sulfate (CH₄ + SO₄ → H₂S + CO₂).
        let so4 = self.deep_so4.max(0.0);
        let c = so4 / deep_volume_m3.max(1.0);
        let reduced_c = (anaerobic * c / (c + params.sulfate_reduction_half)).min(2.0 * so4);
        let methanogenic = anaerobic - reduced_c;
        let mut h2s = 0.5 * reduced_c;
        let methane = 0.5 * methanogenic;
        let aom = (methane * c / (c + params.methane_sulfate_half)).min(so4 - h2s).max(0.0);
        h2s += aom;
        self.deep_so4 -= h2s;
        self.atmosphere[Gas::Co2 as usize] += aerobic + reduced_c + 0.5 * methanogenic + aom;
        self.atmosphere[Gas::Ch4 as usize] += methane - aom;
        // Le sulfure : une part enfouie en pyrite, le reste réoxydé en
        // sulfate par l'oxygène qui reste (ou enfoui lui aussi, faute d'O₂).
        if h2s > 0.0 {
            let o2 = &mut self.atmosphere[Gas::O2 as usize];
            let reoxidised = (h2s * (1.0 - params.pyrite_burial_share)).min(0.5 * o2.max(0.0));
            *o2 -= 2.0 * reoxidised;
            self.oxygen.sulfide += 2.0 * reoxidised;
            self.deep_so4 += reoxidised;
            self.pyrite_s += h2s - reoxidised;
        }
        buried
    }

    /// Reçoit le contenu d'une couche d'eau qui disparaît (cellule devenue
    /// terre) : chaque pool rejoint sa boîte, la matière organique dissoute et
    /// la biomasse (carbone `organic_c`, phosphore `organic_p`) vont aux
    /// sédiments. Le soufre n'est pas suivi.
    pub fn absorb_layer(&mut self, moles: &[f64; WATER_POOL_COUNT], organic_c: f64, organic_p: f64, flux: &mut FluxRegistry) {
        // Le sulfure n'a pas de boîte : il quitte le système suivi.
        flux.exchange(Element::Electrons, 2.0 * moles[WaterPool::H2s as usize]);
        for (i, &m) in moles.iter().enumerate() {
            if let Some(r) = self.counterpart(crate::pools::WATER_POOLS[i]) {
                *r += m;
            }
        }
        self.organic_c += moles[WaterPool::Doc as usize] + organic_c;
        self.sediment_p += organic_p;
    }

    /// Fournit le contenu d'une couche d'eau nouvelle (cellule devenue océan)
    /// à partir des boîtes, sans en prendre plus que les neuf dixièmes.
    /// Renvoie les moles réellement fournies.
    pub fn fill_layer(&mut self, wanted: &[f64; WATER_POOL_COUNT], flux: &mut FluxRegistry) -> [f64; WATER_POOL_COUNT] {
        let mut got = [0.0; WATER_POOL_COUNT];
        for (i, &m) in wanted.iter().enumerate() {
            let pool = crate::pools::WATER_POOLS[i];
            match self.counterpart(pool) {
                Some(r) => {
                    let take = m.max(0.0).min(0.9 * r.max(0.0));
                    *r -= take;
                    got[i] = take;
                }
                // Sans boîte suivie (soufre), la couche reçoit sa valeur
                // d'équilibre ; la matière organique dissoute part de zéro.
                None => got[i] = if pool == WaterPool::Doc { 0.0 } else { m.max(0.0) },
            }
        }
        flux.exchange(Element::Electrons, -2.0 * got[WaterPool::H2s as usize]);
        got
    }

    /// Applique exactement les échanges mesurés pendant l'écologie rapide
    /// (`years` années).
    pub fn apply_exact(
        &mut self,
        params: &PlanetParams,
        ctx: &BoxContext,
        moles: &[f64; WATER_POOL_COUNT],
        years: f64,
        flux: &mut FluxRegistry,
    ) {
        let deep_oxic = self.deep_oxic(params, ctx, moles[WaterPool::Doc as usize] / years.max(1e-12));
        let (_, _, h2s) = self.apply_surface(params, moles, deep_oxic, ctx.deep_volume_m3);
        // Le sulfure dégazé quitte le système suivi ; il y reprend de l'O₂
        // en s'oxydant (puits « sulfure »).
        flux.exchange(Element::Electrons, 2.0 * h2s);
        let s = (2.0 * h2s.max(0.0)).min(self.atmosphere[Gas::O2 as usize]);
        self.atmosphere[Gas::O2 as usize] -= s;
        self.oxygen.sulfide += s;
        self.deep_so4 += 0.5 * s;
        flux.exchange(Element::Electrons, -s);
    }

    /// Part oxygénée de l'océan profond, quand la surface exporte
    /// `export_rate` mol de carbone organique par an. La ventilation lui
    /// apporte de l'eau de surface saturée en O₂ (loi de Henry) ; la matière
    /// organique exportée y consomme une mole d'O₂ par mole de carbone.
    /// L'océan profond est oxygéné quand l'apport l'emporte sur la demande,
    /// anoxique sinon (loi de Hill de raideur `anoxia_steepness`). Le seuil
    /// suit donc la productivité, comme dans les modèles de type COPSE
    /// (Lenton et Watson, 2000), au lieu d'un niveau d'O₂ fixé.
    pub fn deep_oxic(&self, params: &PlanetParams, ctx: &BoxContext, export_rate: f64) -> f64 {
        let p_o2 = self.partial_pressure(Gas::O2, ctx.gravity, ctx.area_m2).max(0.0);
        let saturated = WaterPool::O2.henry().unwrap_or(0.0) * p_o2;
        let supply = ctx.deep_volume_m3 / params.deep_ventilation_years * saturated;
        let demand = export_rate.max(0.0);
        if supply <= 0.0 {
            return 0.0;
        }
        if demand <= 0.0 {
            return 1.0;
        }
        let x = (supply / demand).dpowf(params.anoxia_steepness);
        x / (1.0 + x)
    }

    /// Fait avancer les boîtes de `dt` années. `surface_rates` : flux
    /// annuels nets des cellules vers l'extérieur (équilibrés en carbone et
    /// en phosphore cellule par cellule), à appliquer pendant `dt`. Renvoie
    /// le pouvoir oxydant déplacé quand une boîte vide freine un prélèvement,
    /// et celui qui n'a pu être repris (voir [`pair_throttled`]), mol
    /// d'équivalent O₂.
    fn co2_factor(&self, params: &PlanetParams, ctx: &BoxContext) -> f64 {
        let p_co2 = self.mixing_ratio(Gas::Co2) * self.pressure_pa(ctx.gravity, ctx.area_m2);
        (p_co2 / params.co2_pa).max(0.0)
    }

    /// Altération des silicates des terres, mol de CO₂ par an : sans la vie,
    /// puis avec elle. Les racines, les acides organiques et les sols de la
    /// vie terrestre accélèrent l'altération des silicates et la libération
    /// du phosphore ; l'oxydation des roches suit l'érosion, pas la vie.
    pub fn land_weathering(&self, params: &PlanetParams, ctx: &BoxContext) -> (f64, f64) {
        let bare = params.weathering_per_m2
            * ctx.land_area_m2
            * ((ctx.mean_temperature_k - params.weathering_reference_k) / params.weathering_activation_k).dexp()
            * self.co2_factor(params, ctx).dpowf(params.weathering_co2_exponent);
        (bare, bare * ctx.biotic_weathering.max(1.0))
    }

    /// Phosphore libéré par l'altération des terres, mol·an⁻¹.
    pub fn land_phosphorus(&self, params: &PlanetParams, ctx: &BoxContext) -> f64 {
        self.land_weathering(params, ctx).1 * params.weathering_phosphorus_ratio
    }

    pub fn integrate(
        &mut self,
        params: &PlanetParams,
        ctx: &BoxContext,
        surface_rates: &[f64; WATER_POOL_COUNT],
        dt: f64,
        flux: &mut FluxRegistry,
    ) -> (f64, f64) {
        // Sous-pas de 10 000 ans au plus, raccourcis pour qu'un sous-pas ne
        // prélève pas plus de 2 % d'une boîte (la photosynthèse renouvelle le
        // CO₂ de l'air en quelques siècles à quelques millénaires).
        let mut steps = (dt / 10_000.0).ceil();
        for (i, &r) in surface_rates.iter().enumerate() {
            if r < 0.0 {
                if let Some(&stock) = self.counterpart_ref(crate::pools::WATER_POOLS[i]) {
                    if stock > 0.0 {
                        steps = steps.max((dt * -r / (0.02 * stock)).ceil());
                    }
                }
            }
        }
        let n = steps.clamp(1.0, 20_000.0) as usize;
        let h = dt / n as f64;
        // Les prélèvements des cellules suivent la disponibilité de ce
        // qu'elles prélèvent (cinétique d'ordre un, schéma implicite) : une
        // boîte qui se vide les ralentit au lieu d'être vidée d'un coup.
        let initial: [f64; WATER_POOL_COUNT] =
            std::array::from_fn(|i| self.counterpart_ref(crate::pools::WATER_POOLS[i]).copied().unwrap_or(0.0).max(0.0));
        let mut acc = GlobalFluxes::default();
        let mut throttled = (0.0, 0.0);
        let _ = ctx.deep_volume_m3;
        let sinks_before = self.oxygen.total_sinks();
        let release_before = self.oxygen.surface_release;
        let h2_ratio = params.reduced_outgassing_ratio();
        let esc_per_mixing = params.hydrogen_escape * ctx.area_m2 * evo_core::units::SECONDS_PER_YEAR / 6.022_140_76e23;
        for _ in 0..n {
            // 1. Échanges de surface. Les pools carbonés partagent un même
            //    facteur (le carbone reste équilibré cellule par cellule), que
            //    suit l'O₂ libéré par la fixation ; les autres pools ont le leur.
            let mut moles = surface_rates.map(|r| r * h);
            let mut phi = [1.0f64; WATER_POOL_COUNT];
            let mut phi_carbon: f64 = 1.0;
            for (i, &m) in moles.iter().enumerate() {
                let pool = crate::pools::WATER_POOLS[i];
                if m < 0.0 {
                    if let Some(&r) = self.counterpart_ref(pool) {
                        let f = if initial[i] > 0.0 { (r.max(0.0) / (initial[i] - m)).min(1.0) } else { 0.0 };
                        if pool.carbon_atoms() > 0.0 {
                            phi_carbon = phi_carbon.min(f);
                        } else {
                            phi[i] = f;
                        }
                    }
                }
            }
            let planned = moles;
            for (i, m) in moles.iter_mut().enumerate() {
                let pool = crate::pools::WATER_POOLS[i];
                let carbon_linked = pool.carbon_atoms() > 0.0 || (pool == WaterPool::O2 && *m > 0.0);
                *m *= if carbon_linked { phi_carbon.min(phi[i]) } else { phi[i] };
            }
            let (moved, unpaired) = pair_throttled(&planned, &mut moles);
            throttled.0 += moved;
            throttled.1 += unpaired;
            let export_rate = moles[WaterPool::Doc as usize] / h;
            let deep_oxic = self.deep_oxic(params, ctx, export_rate);
            // Les couches de surface sont à l'équilibre : ce qu'elles exportent
            // de pouvoir oxydant (ou réducteur) vient de leurs sources
            // hydrothermales, hors du système suivi. On inscrit exactement ce
            // qui est appliqué, y compris quand une boîte vide freine un flux.
            let surface_ox: f64 = moles.iter().enumerate().map(|(i, m)| m * crate::pools::WATER_POOLS[i].oxidant_equivalents()).sum();
            flux.exchange(Element::Electrons, surface_ox);
            acc.surface_redox += surface_ox;
            let (export, buried, h2s) = self.apply_surface(params, &moles, deep_oxic, ctx.deep_volume_m3);
            flux.exchange(Element::Electrons, 2.0 * h2s);
            acc.organic_export += export;
            acc.organic_burial += buried;

            // 2. Chimie rapide de l'atmosphère (juste après les apports de surface,
            //    pour que les puits lents voient l'O₂ qui reste) : titrage H₂-O₂, oxydation et photolyse
            //    du méthane, échappement de l'hydrogène vers l'espace.
            let atm = &mut self.atmosphere;
            let r = (atm[Gas::H2 as usize] / 2.0).min(atm[Gas::O2 as usize]).max(0.0);
            atm[Gas::H2 as usize] -= 2.0 * r;
            atm[Gas::O2 as usize] -= r;
            self.oxygen.reduced_gases += r;
            let total: f64 = atm.iter().sum();
            let k_esc = esc_per_mixing / total.max(1e-300);
            let f_o2 = atm[Gas::O2 as usize] / total;
            let k_ox = params.ch4_oxidation * f_o2.max(0.0).sqrt();
            let ch4_0 = atm[Gas::Ch4 as usize];
            atm[Gas::Ch4 as usize] = ch4_0 / (1.0 + h * (k_ox + k_esc));
            let destroyed = ch4_0 - atm[Gas::Ch4 as usize];
            let oxidised = (destroyed * k_ox / (k_ox + k_esc).max(1e-300)).min(atm[Gas::O2 as usize] / 2.0);
            atm[Gas::O2 as usize] -= 2.0 * oxidised;
            self.oxygen.methane += 2.0 * oxidised;
            // Le carbone du méthane détruit revient en CO₂.
            atm[Gas::Co2 as usize] += destroyed;
            let h2_0 = atm[Gas::H2 as usize];
            atm[Gas::H2 as usize] = h2_0 / (1.0 + h * k_esc);
            let escaped_h2 = h2_0 - atm[Gas::H2 as usize] + 2.0 * (destroyed - oxidised);
            acc.hydrogen_escape += escaped_h2;
            // L'hydrogène qui s'échappe laisse la planète plus oxydée : ½ par
            // H₂, 2 par CH₄ photolysé dont l'hydrogène part (CH₄ + 2 H₂O →
            // CO₂ + 4 H₂).
            flux.exchange(Element::Electrons, 0.5 * (h2_0 - atm[Gas::H2 as usize]) + 2.0 * (destroyed - oxidised));
            acc.methane_release += moles[WaterPool::Ch4 as usize];

            // 3. Volcanisme et hydrothermalisme profond.
            let v = params.outgassing_co2 * ctx.activity * h;
            self.atmosphere[Gas::Co2 as usize] += v;
            self.atmosphere[Gas::H2 as usize] += v * h2_ratio;
            flux.exchange(Element::Carbon, v);
            flux.exchange(Element::Electrons, -0.5 * v * h2_ratio);
            acc.outgassing_co2 += v;
            let deep_share = (1.0 - params.vent_local_share) * ctx.activity * ctx.hydrothermal_share * h;
            self.deep_fe2 += params.vent_fe_flux * deep_share;
            self.deep_mn2 += params.vent_mn_flux * deep_share;
            flux.exchange(Element::Electrons, -(0.25 * params.vent_fe_flux + 0.5 * params.vent_mn_flux) * deep_share);

            // 4. Altération des silicates (thermostat) et des fonds.
            let co2_factor = self.co2_factor(params, ctx);
            let (bare, land) = self.land_weathering(params, ctx);
            let seafloor = params.seafloor_weathering_share
                * params.outgassing_co2
                * ctx.activity
                * co2_factor.dpowf(0.23)
                * ((ctx.mean_temperature_k - params.weathering_reference_k) / params.seafloor_weathering_activation_k).dexp();
            let w = ((land + seafloor) * h).min(0.9 * self.atmosphere[Gas::Co2 as usize]);
            self.atmosphere[Gas::Co2 as usize] -= w;
            self.carbonate_c += w;
            acc.weathering_co2 += w;
            let p_in = (land + params.seafloor_phosphorus_share * seafloor) * h * params.weathering_phosphorus_ratio;
            // Litière enfouie (tourbes, deltas) : sans phosphore, il reste au
            // sol.
            let lb = (ctx.land_burial.max(0.0) * h).min(0.9 * self.atmosphere[Gas::Co2 as usize].max(0.0));
            self.atmosphere[Gas::Co2 as usize] -= lb;
            self.atmosphere[Gas::O2 as usize] += lb;
            self.organic_c += lb;
            self.oxygen.land_burial += lb;
            // La part livrée aux sols arrive par les couches des cellules.
            let p_in = p_in - ctx.land_phosphorus_routed * h;
            self.deep_po4 += p_in;
            flux.exchange(Element::Phosphorus, p_in);
            // Phosphore authigène (apatite, fluorapatite carbonatée) : puits
            // indépendant de l'oxygène, proportionnel au stock dissous.
            let apatite = self.deep_po4.max(0.0) * (1.0 - (-h / params.apatite_burial_years).dexp());
            self.deep_po4 -= apatite;
            self.sediment_p += apatite;

            // 5. Puits d'oxygène lents.
            let f_o2 = self.mixing_ratio(Gas::O2);
            let o2 = self.atmosphere[Gas::O2 as usize];
            let ow = (params.oxidative_weathering_per_weathered_c
                * bare
                * (f_o2.max(0.0) / params.oxidative_weathering_reference).dpowf(params.oxidative_weathering_exponent)
                * h)
                .min(o2);
            self.atmosphere[Gas::O2 as usize] -= ow;
            self.oxygen.oxidative_weathering += ow;
            flux.exchange(Element::Electrons, -ow);
            // Sulfure dégazé : oxydé en sulfate s'il y a de l'O₂.
            let s = (2.0 * h2s.max(0.0)).min(self.atmosphere[Gas::O2 as usize]);
            self.atmosphere[Gas::O2 as usize] -= s;
            self.oxygen.sulfide += s;
            self.deep_so4 += 0.5 * s;
            flux.exchange(Element::Electrons, -s);
            // Sulfate des roches : la pyrite exposée s'oxyde avec le reste
            // des roches (une part `pyrite_weathering_share` de l'O₂ qu'elles
            // prennent, 2 O₂ par soufre) ; enfouissement en gypse.
            self.deep_so4 += 0.5 * params.pyrite_weathering_share * ow;
            let gypsum = self.deep_so4.max(0.0) * (1.0 - (-h / params.gypsum_burial_years).dexp());
            self.deep_so4 -= gypsum;
            self.gypsum_s += gypsum;
            // Oxydation de la croûte océanique jeune (fer et soufre du
            // basalte) par une eau de mer oxygénée : proportionnelle à la
            // production de croûte et à l'oxygénation de l'océan profond.
            let deep_oxic = self.deep_oxic(params, ctx, export_rate);
            let so = (params.seafloor_oxidation_o2 * ctx.activity * ctx.hydrothermal_share * deep_oxic * h)
                .min(self.atmosphere[Gas::O2 as usize]);
            self.atmosphere[Gas::O2 as usize] -= so;
            self.oxygen.seafloor_oxidation += so;
            flux.exchange(Element::Electrons, -so);
            // Fer et manganèse de l'océan profond : oxydés par l'O₂ (ventilation
            // millénaire), ou déposés lentement en milieu anoxique.
            let deep_oxic = self.deep_oxic(params, ctx, export_rate);
            let k_res = 1.0 / params.deep_metal_residence_years;
            let k_fe = deep_oxic / 1_000.0;
            let k_mn = self.mixing_ratio(Gas::O2) / (self.mixing_ratio(Gas::O2) + 10.0 * params.deep_oxic_half_mixing) / 1_000.0;
            let fe0 = self.deep_fe2;
            self.deep_fe2 = fe0 / (1.0 + h * (k_fe + k_res));
            let fe_lost = fe0 - self.deep_fe2;
            let fe_ox = (fe_lost * k_fe / (k_fe + k_res)).min(4.0 * self.atmosphere[Gas::O2 as usize]);
            self.atmosphere[Gas::O2 as usize] -= fe_ox / 4.0;
            self.iron_oxides += fe_ox;
            self.iron_reduced += fe_lost - fe_ox;
            self.scavenge_phosphorus(params, fe_ox);
            let mn0 = self.deep_mn2;
            self.deep_mn2 = mn0 / (1.0 + h * (k_mn + k_res));
            let mn_lost = mn0 - self.deep_mn2;
            let mn_ox = (mn_lost * k_mn / (k_mn + k_res)).min(2.0 * self.atmosphere[Gas::O2 as usize]);
            self.atmosphere[Gas::O2 as usize] -= mn_ox / 2.0;
            self.manganese_oxides += mn_ox;
            self.manganese_reduced += mn_lost - mn_ox;
            self.oxygen.iron_manganese += fe_ox / 4.0 + mn_ox / 2.0;

            // 6. Subduction des sédiments.
            let sub = (ctx.subduction_per_year * h).min(1.0);
            let c_out = sub * (self.carbonate_c + self.organic_c);
            flux.exchange(Element::Electrons, sub * (self.organic_c + 2.0 * self.pyrite_s));
            self.pyrite_s *= 1.0 - sub;
            self.gypsum_s *= 1.0 - sub;
            self.carbonate_c *= 1.0 - sub;
            self.organic_c *= 1.0 - sub;
            flux.exchange(Element::Carbon, -c_out);
            let p_out = sub * self.sediment_p;
            self.sediment_p -= p_out;
            flux.exchange(Element::Phosphorus, -p_out);
        }
        acc.oxygen_release = self.oxygen.surface_release - release_before;
        acc.oxygen_sinks = self.oxygen.total_sinks() - sinks_before;
        self.last = GlobalFluxes {
            outgassing_co2: acc.outgassing_co2 / dt,
            weathering_co2: acc.weathering_co2 / dt,
            organic_export: acc.organic_export / dt,
            organic_burial: acc.organic_burial / dt,
            oxygen_release: acc.oxygen_release / dt,
            oxygen_sinks: acc.oxygen_sinks / dt,
            methane_release: acc.methane_release / dt,
            hydrogen_escape: acc.hydrogen_escape / dt,
            surface_redox: acc.surface_redox / dt,
        };
        throttled
    }
}

/// Retire `need` équivalents d'O₂ de pouvoir réducteur exporté, en puisant
/// dans les pools `order` dans cet ordre (le carbone retiré revient en CO₂).
/// Renvoie ce qui n'a pu être retiré.
fn withdraw_reductant(moles: &mut [f64; WATER_POOL_COUNT], mut need: f64, order: &[WaterPool]) -> f64 {
    for &pool in order {
        if need <= 0.0 {
            break;
        }
        let i = pool as usize;
        let eq = -pool.oxidant_equivalents();
        let x = (need / eq).min(moles[i].max(0.0));
        moles[i] -= x;
        if pool.carbon_atoms() > 0.0 {
            moles[WaterPool::Dic as usize] += x;
        }
        need -= eq * x;
    }
    need.max(0.0)
}

/// Un prélèvement des couches de surface freiné par une boîte vide freine
/// aussi ce qu'il alimentait, pour que le pouvoir oxydant appliqué reste
/// exactement celui prévu (`planned`, dont le bilan d'électrons est celui des
/// sources hydrothermales) :
/// - le fer et le manganèse importés qui manquent ne sont pas oxydés
///   (l'oxyde exporté baisse d'autant) et ne fixent pas de carbone (moins
///   de matière organique exportée) ;
/// - l'hydrogène importé qui manque ne fait pas de méthane ;
/// - l'O₂ importé qui manque ne respire pas de matière organique, qui sort
///   non respirée.
///
/// Un reste éventuel (pools carbonés freinés ensemble) est retiré de la
/// matière organique, du méthane, du sulfure et de l'hydrogène exportés,
/// puis de l'O₂ importé. Renvoie le pouvoir oxydant ainsi déplacé, et celui
/// qu'aucun de ces flux ne suffit à reprendre (non appliqué, il sort du
/// système suivi), mol d'équivalent O₂.
pub fn pair_throttled(planned: &[f64; WATER_POOL_COUNT], moles: &mut [f64; WATER_POOL_COUNT]) -> (f64, f64) {
    use crate::pools::WATER_POOLS;
    use WaterPool::*;
    let ox = |m: &[f64; WATER_POOL_COUNT]| WATER_POOLS.iter().map(|&p| m[p as usize] * p.oxidant_equivalents()).sum::<f64>();
    let moved = (ox(planned) - ox(moles)).abs();
    let short = |m: &[f64; WATER_POOL_COUNT], p: WaterPool| (m[p as usize] - planned[p as usize]).max(0.0);
    let mut rest = 0.0;
    for (metal, oxide) in [(Fe2, FeOx), (Mn2, MnOx)] {
        let x = short(moles, metal);
        if x > 0.0 {
            let o = &mut moles[oxide as usize];
            *o -= x.min(o.max(0.0));
            rest += withdraw_reductant(moles, -metal.oxidant_equivalents() * x, &[Doc, Ch4, H2s, H2]);
        }
    }
    let x = short(moles, H2);
    if x > 0.0 {
        rest += withdraw_reductant(moles, 0.5 * x, &[Ch4, Doc, H2s]);
    }
    let x = short(moles, O2);
    if x > 0.0 && planned[O2 as usize] < 0.0 {
        moles[Doc as usize] += x;
        moles[Dic as usize] -= x;
    }
    // Reste : pools carbonés freinés ensemble, reliquats des étapes
    // précédentes.
    let mut delta = ox(planned) - ox(moles);
    if delta > 0.0 {
        delta = withdraw_reductant(moles, delta, &[Doc, Ch4, H2s, H2]);
        let o2 = O2 as usize;
        let y = delta.min((-moles[o2]).max(0.0));
        moles[o2] += y;
        delta -= y;
    } else if delta < 0.0 {
        moles[Doc as usize] -= delta;
        moles[Dic as usize] += delta;
        delta = 0.0;
    }
    let _ = rest;
    (moved, delta.max(0.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn throttled_imports_are_paired_with_what_they_fed() {
        use crate::pools::WATER_POOLS;
        let sum = |m: &[f64; WATER_POOL_COUNT], f: fn(WaterPool) -> f64| WATER_POOLS.iter().map(|&p| m[p as usize] * f(p)).sum::<f64>();
        // Photoferrotrophes et méthanogènes : 4 Fe²⁺ importés par carbone
        // exporté, 4 H₂ importés par CH₄ exporté, et des respirateurs.
        let mut planned = [0.0; WATER_POOL_COUNT];
        planned[WaterPool::Fe2 as usize] = -400.0;
        planned[WaterPool::FeOx as usize] = 400.0;
        planned[WaterPool::Doc as usize] = 100.0 - 30.0;
        planned[WaterPool::H2 as usize] = -80.0;
        planned[WaterPool::Ch4 as usize] = 20.0;
        planned[WaterPool::O2 as usize] = -30.0;
        planned[WaterPool::Dic as usize] = -100.0 - 20.0 + 30.0;
        // Les boîtes ne fournissent qu'une part du fer, de l'H₂ et de l'O₂.
        let mut moles = planned;
        moles[WaterPool::Fe2 as usize] *= 0.3;
        moles[WaterPool::H2 as usize] *= 0.5;
        moles[WaterPool::O2 as usize] *= 0.1;
        let (moved, unpaired) = pair_throttled(&planned, &mut moles);
        assert!(moved > 0.0);
        assert_eq!(unpaired, 0.0);
        let e = |m: &[f64; WATER_POOL_COUNT]| sum(m, WaterPool::oxidant_equivalents);
        assert!((e(&moles) - e(&planned)).abs() < 1e-9, "électrons {moles:?}");
        assert!((sum(&moles, WaterPool::carbon_atoms) - sum(&planned, WaterPool::carbon_atoms)).abs() < 1e-9);
        assert!((moles[WaterPool::FeOx as usize] - 120.0).abs() < 1e-9, "le fer non importé n'est pas oxydé");
        assert!((moles[WaterPool::Ch4 as usize] - 10.0).abs() < 1e-9, "l'H₂ qui manque ne fait pas de méthane");
    }

    fn ctx(params: &PlanetParams, t: f64) -> BoxContext {
        BoxContext {
            land_area_m2: 0.25 * params.surface_area(),
            deep_volume_m3: 1.3e18,
            mean_temperature_k: t,
            activity: 1.0,
            hydrothermal_share: 1.0,
            subduction_per_year: 0.0,
            gravity: params.gravity(),
            area_m2: params.surface_area(),
            biotic_weathering: 1.0,
            land_phosphorus_routed: 0.0,
            land_burial: 0.0,
        }
    }

    #[test]
    fn initial_pressure_matches_preset() {
        let p = PlanetParams::earth_archean();
        let r = GlobalReservoirs::new(&p, 1.3e18);
        let pressure = r.pressure_pa(p.gravity(), p.surface_area());
        // L'hydrogène d'équilibre ajoute moins d'un millième.
        assert!((pressure - (p.n2_pa + p.co2_pa)).abs() / pressure < 1e-3, "{pressure}");
        assert!((r.partial_pressure(Gas::Co2, p.gravity(), p.surface_area()) - p.co2_pa).abs() < 1e-3 * p.co2_pa);
    }

    #[test]
    fn silicate_weathering_is_a_thermostat_and_carbon_is_conserved() {
        let p = PlanetParams::earth_archean();
        let mut hot = GlobalReservoirs::new(&p, 1.3e18);
        let mut cold = hot.clone();
        let mut flux = FluxRegistry::default();
        flux.set_initial(Element::Carbon, hot.carbon());
        hot.integrate(&p, &ctx(&p, 300.0), &[0.0; WATER_POOL_COUNT], 1e6, &mut flux);
        let mut f2 = FluxRegistry::default();
        cold.integrate(&p, &ctx(&p, 275.0), &[0.0; WATER_POOL_COUNT], 1e6, &mut f2);
        assert!(hot.atmosphere[Gas::Co2 as usize] < cold.atmosphere[Gas::Co2 as usize]);
        assert!(flux.relative_error(Element::Carbon, hot.carbon()) < 1e-12);
    }

    #[test]
    fn electrons_are_conserved_by_the_boxes() {
        let p = PlanetParams::earth_archean();
        let mut r = GlobalReservoirs::new(&p, 1.3e18);
        let mut flux = FluxRegistry::default();
        flux.set_initial(Element::Electrons, r.electrons());
        // Photosynthèse de surface, méthane et sulfure exportés.
        let mut rates = [0.0; WATER_POOL_COUNT];
        rates[WaterPool::O2 as usize] = 3e12;
        rates[WaterPool::Doc as usize] = 2e12;
        rates[WaterPool::Ch4 as usize] = 1e11;
        rates[WaterPool::Dic as usize] = -2.1e12;
        rates[WaterPool::H2s as usize] = 1e10;
        for _ in 0..30 {
            r.integrate(&p, &ctx(&p, 290.0), &rates, 1e5, &mut flux);
            let e = flux.relative_error(Element::Electrons, r.electrons());
            assert!(e < 1e-9, "écart {e}");
        }
    }

    #[test]
    fn oxygen_accumulates_only_when_sources_beat_sinks() {
        let p = PlanetParams::earth_archean();
        let base = GlobalReservoirs::new(&p, 1.3e18);
        let mut flux = FluxRegistry::default();
        // Photosynthèse oxygénique de surface : chaque mole d'O₂ libérée
        // accompagne une mole de carbone organique exportée ; seule la part
        // enfouie (5 %) est une source nette, face aux gaz réduits, au fer et
        // aux roches exposées.
        let run = |production: f64| {
            let mut r = base.clone();
            let mut rates = [0.0; WATER_POOL_COUNT];
            rates[WaterPool::O2 as usize] = production;
            rates[WaterPool::Doc as usize] = production;
            rates[WaterPool::Dic as usize] = -production;
            let mut f = FluxRegistry::default();
            for _ in 0..50 {
                r.integrate(&p, &ctx(&p, 290.0), &rates, 1e5, &mut f);
            }
            r.mixing_ratio(Gas::O2)
        };
        assert!(run(1e11) < 1e-6, "faible production : l'oxygène ne s'accumule pas");
        let high = run(2e13);
        assert!(high > 1e-5, "forte production : O₂ {high}");
        flux.set_initial(Element::Carbon, 0.0);
    }
}
