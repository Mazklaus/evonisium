//! Registre de flux : comptabilité des éléments échangés entre la planète,
//! le vivant et l'extérieur du système simulé.
//!
//! Règle : la quantité d'un élément dans le système vaut toujours le stock
//! initial plus les entrées moins les sorties déclarées. Un écart signale une
//! fuite de matière, donc un bug.

/// Éléments suivis : le carbone depuis l'étape 1, le phosphore depuis
/// l'étape 2 (nutriment limitant), et depuis l'étape 3 les électrons : le
/// pouvoir oxydant total du système, en équivalents d'O₂ (CO₂, eau, sulfate
/// et oxydes à zéro ; O₂ +1, CH₂O −1, CH₄ −2, H₂ −½, Fe²⁺ −¼…). La vie ne
/// fait que le déplacer ; seuls les échanges avec l'extérieur (gaz
/// volcaniques réduits, sources hydrothermales, échappement de l'hydrogène,
/// roches réduites altérées, subduction) le changent. Le budget de
/// l'oxygène libre, par source et par puits, est tenu à part dans les
/// réservoirs globaux de la planète.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Element {
    Carbon = 0,
    Phosphorus = 1,
    Electrons = 2,
}

pub const ELEMENT_COUNT: usize = 3;
pub const ELEMENTS: [Element; ELEMENT_COUNT] = [Element::Carbon, Element::Phosphorus, Element::Electrons];

/// Entrées et sorties cumulées, en moles, par élément.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FluxRegistry {
    pub initial: [f64; ELEMENT_COUNT],
    pub inflow: [f64; ELEMENT_COUNT],
    pub outflow: [f64; ELEMENT_COUNT],
}

impl FluxRegistry {
    pub fn set_initial(&mut self, e: Element, moles: f64) {
        self.initial[e as usize] = moles;
    }

    /// Échange signé avec l'extérieur : positif = entrée, négatif = sortie.
    #[inline]
    pub fn exchange(&mut self, e: Element, moles: f64) {
        if moles >= 0.0 {
            self.inflow[e as usize] += moles;
        } else {
            self.outflow[e as usize] -= moles;
        }
    }

    pub fn merge(&mut self, other: &FluxRegistry) {
        for i in 0..ELEMENT_COUNT {
            self.inflow[i] += other.inflow[i];
            self.outflow[i] += other.outflow[i];
        }
    }

    /// Quantité attendue dans le système d'après la comptabilité.
    pub fn expected(&self, e: Element) -> f64 {
        let i = e as usize;
        self.initial[i] + self.inflow[i] - self.outflow[i]
    }

    /// Écart relatif entre la quantité mesurée et la quantité attendue,
    /// rapporté au plus grand des volumes en jeu.
    pub fn relative_error(&self, e: Element, measured: f64) -> f64 {
        let i = e as usize;
        let scale = self.initial[i].abs().max(self.inflow[i]).max(self.outflow[i]).max(1e-300);
        (measured - self.expected(e)).abs() / scale
    }
}
