//! Lame de microscope en 3D : ouverture depuis la session (voir
//! `crate::slide`).

use super::*;
use crate::slide::EvoSlide;
use evo_view::slide::{Slide, SlideSpecies};

#[godot_api(secondary)]
impl EvoSession {
    /// Lame de microscope en 3D sur `cell` : les populations de la cellule,
    /// construite en tâche de fond (`EvoSlide.is_ready()`).
    #[func]
    fn open_slide(&mut self, cell: i64) -> Option<Gd<EvoSlide>> {
        let (Some(g), Some(f)) = (&self.game, self.frame()) else { return None };
        if cell < 0 || cell as usize >= f.cells().len() {
            return None;
        }
        let rx = g.engine.query(Query::Cell { cell: cell as u32 });
        let game_seed = f.planet.seed;
        let seed = game_seed ^ ((cell as u64) << 24) ^ 0x534C;
        let slide = Gd::<EvoSlide>::default();
        let slot = slide.bind().slot.clone();
        std::thread::spawn(move || {
            // Sans réponse du moteur, une lame vide : l'écran ne reste pas en attente.
            let pops = match rx.recv() {
                Ok(Answer::Cell(Some(detail))) => detail.populations,
                _ => Vec::new(),
            };
            let members = evo_view::species::microscope_members(&pops, game_seed);
            let species = members
                .into_iter()
                .zip(&pops)
                .map(|(m, p)| SlideSpecies { form: m.form, share: m.share, species: p.species, lineage: p.lineage })
                .collect();
            *slot.lock().unwrap() = Some(Slide::new(species, seed));
        });
        Some(slide)
    }
}
