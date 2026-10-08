//! Réserve d'influence du joueur (décision du 8 octobre 2026).
//!
//! Les interventions sur l'environnement coûtent des points d'influence qui
//! se rechargent avec le temps simulé. La réserve fait partie de l'état
//! simulé : elle est sauvegardée et rejouée avec lui, et le moteur valide
//! chaque ordre d'intervention contre elle au moment de l'appliquer. Le mode
//! « bac à sable » lève la limite.

/// Réglages de la réserve.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct InfluenceParams {
    pub initial: f64,
    pub max: f64,
    /// Points regagnés par million d'années de jeu.
    pub recharge_per_myr: f64,
    /// Bac à sable : interventions sans limite.
    pub sandbox: bool,
}

impl Default for InfluenceParams {
    /// Une réserve pleine permet deux ou trois interventions fortes ; à la
    /// vitesse visée du monde microbien (1 Ma/s), elle se remplit en moins
    /// d'une minute de jeu, et en un siècle de secondes au rythme des
    /// premières sociétés.
    fn default() -> Self {
        Self { initial: 60.0, max: 100.0, recharge_per_myr: 2.0, sandbox: false }
    }
}

/// Compteur d'influence.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct InfluenceReserve {
    pub points: f64,
    /// Points dépensés depuis le début de la partie.
    pub spent: f64,
    /// Interventions refusées faute de points.
    pub refused: u32,
}

impl InfluenceReserve {
    pub fn new(p: &InfluenceParams) -> Self {
        Self { points: p.initial.min(p.max), spent: 0.0, refused: 0 }
    }

    pub fn recharge(&mut self, p: &InfluenceParams, years: f64) {
        self.points = (self.points + p.recharge_per_myr * years / 1e6).min(p.max);
    }

    /// Dépense `cost` points si la réserve les contient ; renvoie le verdict.
    pub fn try_spend(&mut self, p: &InfluenceParams, cost: f64) -> bool {
        if p.sandbox {
            self.spent += cost;
            return true;
        }
        if cost <= self.points {
            self.points -= cost;
            self.spent += cost;
            true
        } else {
            self.refused += 1;
            false
        }
    }
}

/// Ce que le client affiche de la réserve.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct InfluenceView {
    pub points: f32,
    pub max: f32,
    pub recharge_per_myr: f32,
    pub sandbox: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spends_recharges_and_refuses() {
        let p = InfluenceParams { initial: 10.0, max: 20.0, recharge_per_myr: 5.0, sandbox: false };
        let mut r = InfluenceReserve::new(&p);
        assert!(r.try_spend(&p, 8.0));
        assert!(!r.try_spend(&p, 8.0));
        assert_eq!(r.refused, 1);
        r.recharge(&p, 2e6);
        assert!((r.points - 12.0).abs() < 1e-12);
        r.recharge(&p, 1e9);
        assert_eq!(r.points, 20.0);
        let sandbox = InfluenceParams { sandbox: true, ..p };
        assert!(r.try_spend(&sandbox, 1e6));
    }
}
