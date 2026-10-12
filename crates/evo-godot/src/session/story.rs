//! Narrateur de la première partie, conseiller, chronique en récit et
//! « Pendant votre absence », vus depuis Godot. La logique vit dans
//! `evo_view::guide` et `evo_view::story` ; ici, on assemble l'état et on
//! traduit.

use super::*;
use evo_view::chronicle::Level;
use evo_view::guide::{self, GuideState};
use evo_view::story;

impl EvoSession {
    fn guide_state(&self, opened: &PackedStringArray, cell_selected: bool) -> Option<GuideState> {
        let g = self.game.as_ref()?;
        let f = self.frame()?;
        let gl = &f.state.globals;
        let mut s = GuideState {
            years: f.state.years,
            lineages: gl.living_lineages as u32,
            photosynthesis_stage: gl.photosynthesis_stage,
            o2: gl.o2_mixing,
            opened: opened.as_slice().iter().map(|s| s.to_string()).collect(),
            cell_selected,
            // Les interventions du joueur seules, pas ses autres ordres
            // (ensemencement, pause, règles d'arrêt).
            interventions: g.interventions.len() as u32,
            ..Default::default()
        };
        for e in &g.events {
            match &e.kind {
                EventKind::NewLineage { .. } => s.speciations += 1,
                EventKind::Innovation { pathway, stage, .. } if pathway == "cellule complexe" => s.complexity = s.complexity.max(*stage),
                _ => {}
            }
            if chronicle::level(e, g.pace) >= Level::Notable && !matches!(e.kind, EventKind::OrderApplied { .. }) {
                s.notable += 1;
            }
        }
        Some(s)
    }

    fn events_by_id(&self, ids: &[u64]) -> VarArray {
        let mut out = VarArray::new();
        let Some(g) = &self.game else { return out };
        let lineages = self.lineages();
        for id in ids {
            if let Some(e) = g.events.iter().find(|e| e.id == *id) {
                out.push(&self.event_dict(e, &lineages, None, g.pace).to_variant());
            }
        }
        out
    }
}

#[godot_api(secondary)]
impl EvoSession {
    /// Prochaine phrase du guide : `id`, `tool` (l'outil à souligner) et
    /// `text` ; vide s'il n'y a rien à dire. `known` liste les phrases dont
    /// le joueur a déjà trouvé l'outil, à marquer vues.
    #[func]
    fn guide_next(&self, seen: PackedStringArray, opened: PackedStringArray, cell_selected: bool) -> VarDictionary {
        let mut d = VarDictionary::new();
        let Some(state) = self.guide_state(&opened, cell_selected) else { return d };
        let seen: Vec<String> = seen.as_slice().iter().map(|s| s.to_string()).collect();
        let (hint, known) = guide::next(&state, &seen);
        let known: Vec<GString> = known.iter().map(|s| GString::from(*s)).collect();
        d.set("known", &PackedStringArray::from(&known[..]));
        if let Some(h) = hint {
            d.set("id", h.id);
            d.set("tool", h.tool.key());
            d.set("text", h.text(self.lang));
        }
        d
    }

    /// Toutes les phrases du guide ont été vues.
    #[func]
    fn guide_finished(&self, seen: PackedStringArray) -> bool {
        let seen: Vec<String> = seen.as_slice().iter().map(|s| s.to_string()).collect();
        guide::finished(&seen)
    }

    /// Phrase du conseiller pour un événement (dictionnaire de
    /// `poll_events`), ou vide s'il ne vaut pas d'être signalé.
    #[func]
    fn advice_for(&self, event: VarDictionary) -> GString {
        let interest = event.get("interest").map_or(0.0, |v| v.to::<f64>());
        let level = event.get("level").map_or(0, |v| v.to::<i64>());
        let player = event.get("player").is_some_and(|v| v.to::<bool>());
        if player || level < 1 || !guide::worth_advice(interest) {
            return GString::new();
        }
        let text = event.get("text").map(|v| v.to::<GString>().to_string()).unwrap_or_default();
        GString::from(&guide::advice(&text, self.lang))
    }

    /// La chronique en récit : un chapitre par grand basculement, avec ses
    /// moments à aller voir (`events`).
    #[func]
    fn story(&self) -> VarArray {
        let mut out = VarArray::new();
        let (Some(g), Some(f)) = (&self.game, self.frame()) else { return out };
        let lineages = self.lineages();
        let name = |id: u32| self.name_of(&lineages, id);
        let orders: Vec<u64> = g.interventions.iter().map(|i| i.order).collect();
        for c in story::narrative(&g.events, f.state.years, &orders, self.lang, &name) {
            let mut d = VarDictionary::new();
            d.set("title", c.title.as_str());
            d.set("text", c.text.as_str());
            d.set("from_years", c.from_years);
            d.set("to_years", c.to_years);
            d.set("events", &self.events_by_id(&c.highlights));
            out.push(&d.to_variant());
        }
        out
    }

    /// « Pendant votre absence » depuis `since_years` : `text` et les
    /// moments à aller voir ; vide si rien ne vaut d'être dit.
    #[func]
    fn absence(&self, since_years: f64) -> VarDictionary {
        let mut d = VarDictionary::new();
        let (Some(g), Some(f)) = (&self.game, self.frame()) else { return d };
        let lineages = self.lineages();
        let name = |id: u32| self.name_of(&lineages, id);
        let orders: Vec<u64> = g.interventions.iter().map(|i| i.order).collect();
        if let Some((text, ids)) = story::absence(&g.events, since_years, f.state.years, &orders, self.lang, &name) {
            d.set("text", text.as_str());
            d.set("events", &self.events_by_id(&ids));
        }
        d
    }
}
