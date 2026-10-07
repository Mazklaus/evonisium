//! Horloge maître et horloges de système.
//!
//! L'horloge maître compte le temps géologique ; chaque système s'abonne avec
//! son pas naturel et n'est réveillé que lorsque ce pas est écoulé (section
//! « Gestion du temps » du document Vision).

/// Temps géologique, en années depuis la formation de la planète.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MasterClock {
    pub years: f64,
}

/// Abonnement d'un système à l'horloge maître.
#[derive(Clone, Debug)]
pub struct SystemClock {
    pub name: &'static str,
    pub step_years: f64,
    pub next_due: f64,
}

/// Ordonnanceur : renvoie, pour un intervalle de temps, les systèmes à réveiller.
#[derive(Clone, Debug, Default)]
pub struct Scheduler {
    pub clock: MasterClock,
    pub systems: Vec<SystemClock>,
}

impl Scheduler {
    pub fn new(start_years: f64) -> Self {
        Self { clock: MasterClock { years: start_years }, systems: Vec::new() }
    }

    /// Abonne un système ; il sera dû dès le premier appel.
    pub fn subscribe(&mut self, name: &'static str, step_years: f64) {
        assert!(step_years > 0.0, "pas de temps nul pour {name}");
        self.systems.push(SystemClock { name, step_years, next_due: self.clock.years });
    }

    /// Change le pas d'un système (la vitesse du jeu varie selon les ères).
    pub fn set_step(&mut self, name: &str, step_years: f64) {
        if let Some(s) = self.systems.iter_mut().find(|s| s.name == name) {
            s.step_years = step_years;
        }
    }

    /// Avance l'horloge jusqu'au prochain réveil et renvoie les systèmes dus,
    /// avec le temps écoulé depuis leur dernier réveil.
    pub fn next_tick(&mut self) -> Vec<(&'static str, f64)> {
        let Some(t) = self.systems.iter().map(|s| s.next_due).reduce(f64::min) else {
            return Vec::new();
        };
        self.clock.years = t;
        let mut due = Vec::new();
        for s in &mut self.systems {
            if s.next_due <= t {
                due.push((s.name, s.step_years));
                s.next_due = t + s.step_years;
            }
        }
        due
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slow_systems_wake_less_often() {
        let mut s = Scheduler::new(0.0);
        s.subscribe("rapide", 10.0);
        s.subscribe("lent", 100.0);
        let mut counts = (0, 0);
        while s.clock.years < 1000.0 {
            for (name, _) in s.next_tick() {
                match name {
                    "rapide" => counts.0 += 1,
                    _ => counts.1 += 1,
                }
            }
        }
        assert_eq!(counts, (101, 11));
    }
}
