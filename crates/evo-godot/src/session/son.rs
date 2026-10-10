//! Ce que le son lit de la session : le monde publié et les événements de
//! la chronique, avec son propre curseur (la chronique du client garde le
//! sien). Lecture seule.

use super::*;

impl EvoSession {
    /// Monde publié et familles des événements arrivés depuis `cursor`
    /// (avec leur score d'intérêt). Au premier appel d'une partie, l'histoire
    /// déjà écrite est sautée : seuls les nouveaux événements sonnent.
    pub(crate) fn sound_state(&self, cursor: &mut Option<usize>) -> Option<(evo_son::Monde, Vec<(&'static str, f64)>)> {
        let g = self.game.as_ref()?;
        let f = self.frame()?;
        let st = &f.state;
        let gl = &st.globals;
        let monde = evo_son::Monde {
            o2: gl.o2_mixing,
            temperature_k: gl.mean_temperature_k,
            ice: gl.ice_fraction,
            ocean: gl.ocean_fraction,
            lineages: gl.living_lineages as u32,
            paused: st.paused,
        };
        let n = g.events.len();
        let from = match *cursor {
            Some(c) if c <= n => c,
            _ => n,
        };
        // Au plus 50 par image : un rattrapage ne doit pas tout faire sonner.
        let events = g.events[from.max(n.saturating_sub(50))..].iter().map(|e| (Family::of(e).key(), e.interest)).collect();
        *cursor = Some(n);
        Some((monde, events))
    }
}
