//! Vitesse réellement tenue, mesurée à l'horloge murale du client.
//!
//! Le moteur publie la vitesse de calcul d'un pas (années avancées divisées
//! par la durée du calcul), sans l'attente du frein : à 1 ka/s comme à
//! 10 Ma/s, elle donne ce que la machine pourrait tenir, pas ce qu'elle
//! tient. Le client compte donc les années gagnées entre les pas publiés sur
//! une fenêtre de quelques secondes, attente comprise.

use std::collections::VecDeque;

/// Fenêtre de mesure, s.
const WINDOW: f64 = 8.0;

#[derive(Clone, Debug, Default)]
pub struct Meter {
    /// (instant d'arrivée du pas, s ; années de jeu)
    samples: VecDeque<(f64, f64)>,
}

impl Meter {
    /// Note l'état publié à l'instant `now` (s). Une pause ou un retour en
    /// arrière (chargement, rejeu) vide la fenêtre.
    pub fn observe(&mut self, now: f64, years: f64, paused: bool) {
        if paused {
            self.samples.clear();
            return;
        }
        match self.samples.back() {
            Some(&(_, y)) if years < y => self.samples.clear(),
            Some(&(_, y)) if years == y => return,
            _ => {}
        }
        self.samples.push_back((now, years));
        // Garde au moins deux pas, même lents, pour la mesure.
        while self.samples.len() > 2 && now - self.samples[1].0 > WINDOW {
            self.samples.pop_front();
        }
    }

    /// Vitesse tenue à l'instant `now`, années par seconde ; `None` tant que
    /// deux pas ne sont pas arrivés. Quand le pas suivant tarde plus que
    /// l'intervalle moyen, l'attente en cours compte : la vitesse affichée
    /// baisse pendant qu'un pas lourd se calcule.
    pub fn rate(&self, now: f64) -> Option<f64> {
        let (&(t0, y0), &(t1, y1)) = (self.samples.front()?, self.samples.back()?);
        let n = self.samples.len();
        if n < 2 || t1 <= t0 {
            return None;
        }
        let mean_gap = (t1 - t0) / (n - 1) as f64;
        let late = (now - t1 - mean_gap).max(0.0);
        Some((y1 - y0) / (t1 - t0 + late))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn freine_mesure_la_vitesse_tenue() {
        // 300 ka par pas, un pas toutes les 3 s : 100 ka/s tenus, même si
        // chaque pas se calcule en 30 ms.
        let mut m = Meter::default();
        for k in 0..6 {
            m.observe(k as f64 * 3.0, k as f64 * 3.0e5, false);
        }
        let r = m.rate(15.0).unwrap();
        assert!((r - 1.0e5).abs() < 1.0, "{r}");
    }

    #[test]
    fn un_pas_qui_tarde_fait_baisser_la_mesure() {
        let mut m = Meter::default();
        for k in 0..5 {
            m.observe(k as f64, k as f64 * 1.0e6, false);
        }
        let now = m.rate(4.0).unwrap();
        let late = m.rate(10.0).unwrap();
        assert!((now - 1.0e6).abs() < 1.0);
        assert!(late < now * 0.6, "{late}");
    }

    #[test]
    fn pause_et_retour_en_arriere_vident_la_fenetre() {
        let mut m = Meter::default();
        m.observe(0.0, 0.0, false);
        m.observe(1.0, 1.0e5, false);
        assert!(m.rate(1.0).is_some());
        m.observe(2.0, 1.0e5, true);
        assert!(m.rate(2.0).is_none());
        m.observe(3.0, 2.0e5, false);
        m.observe(4.0, 1.0e5, false);
        assert!(m.rate(4.0).is_none());
    }

    #[test]
    fn fenetre_glissante() {
        let mut m = Meter::default();
        // 10 s à 1 Ma/s, puis 10 s à 100 ka/s : la mesure suit la seconde.
        let mut y = 0.0;
        for k in 0..=10 {
            m.observe(k as f64, y, false);
            y += 1.0e6;
        }
        y -= 1.0e6;
        for k in 11..=20 {
            y += 1.0e5;
            m.observe(k as f64, y, false);
        }
        let r = m.rate(20.0).unwrap();
        assert!((r - 1.0e5).abs() < 1.0e3, "{r}");
    }
}
