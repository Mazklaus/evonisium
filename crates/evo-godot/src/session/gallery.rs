//! Galerie de rejeux (étape 7, document Fonctionnalités) vue depuis Godot :
//! jalons du récit rangés dans les fiches des sauvegardes, export d'un rejeu
//! (planète et registre des ordres) et sa relecture, depuis la graine.

use super::*;
use evo_view::save::ReplayFile;
use evo_view::story;

/// Jalons d'une fiche pour l'écran : date lisible et titre.
fn milestones_array(list: &[(f64, String)], lang: Lang) -> VarArray {
    let mut out = VarArray::new();
    for (y, t) in list {
        let mut d = VarDictionary::new();
        d.set("years", *y);
        d.set("date", format::game_date(*y, lang).as_str());
        d.set("title", t.as_str());
        out.push(&d.to_variant());
    }
    out
}

impl EvoSession {
    /// Rafraîchit les jalons de la partie : le début de chaque chapitre du
    /// récit.
    pub(super) fn refresh_milestones(&mut self) {
        let Some(f) = self.frame() else { return };
        let lineages = self.lineages();
        let Some(g) = &self.game else { return };
        let name = |id: u32| self.name_of(&lineages, id);
        let orders: Vec<u64> = g.interventions.iter().map(|i| i.order).collect();
        // Le titre seul, sans sa période (« — de … à … ») : la fin du
        // dernier chapitre est la date de la partie.
        let list: Vec<(f64, String)> = story::narrative(&g.events, f.state.years, &orders, self.lang, &name)
            .into_iter()
            .map(|c| (c.from_years, c.title.split(" — ").next().unwrap_or_default().to_string()))
            .collect();
        if let Some(g) = &mut self.game {
            g.milestones = list;
        }
    }

    pub(super) fn save_milestones(&self, path: &str) -> VarArray {
        crate::runner::read_meta(&PathBuf::from(path)).map_or_else(|_| VarArray::new(), |s| milestones_array(&s.milestones, self.lang))
    }
}

#[godot_api(secondary)]
impl EvoSession {
    /// Exporte le rejeu de la partie (`.rejeu`) ; il s'écrit dès que le
    /// moteur rend son registre, puis une notice « ok » arrive.
    #[func]
    fn export_replay(&mut self, path: GString, name: GString) {
        self.refresh_milestones();
        if let Some(g) = &mut self.game {
            g.export_replay(&path.to_string(), &name.to_string());
        }
    }

    /// Nom, planète, code, date atteinte, interventions et jalons d'un
    /// rejeu, sans le jouer ; `error` s'il est illisible.
    #[func]
    fn describe_replay(&self, path: GString) -> VarDictionary {
        let mut d = VarDictionary::new();
        let r = std::fs::read_to_string(path.to_string()).map_err(|e| e.to_string()).and_then(|t| ReplayFile::from_text(&t));
        match r {
            Ok(r) => {
                d.set("name", r.name.as_str());
                d.set("date", format::game_date(r.years, self.lang));
                d.set("years", r.years);
                d.set("planet", r.spec.to_params().name.as_str());
                d.set("code", r.spec.code(r.seed, r.level).as_str());
                d.set("mode", r.mode.as_str());
                d.set("orders", r.orders.len() as i64);
                d.set("interventions", r.interventions() as i64);
                d.set("milestones", &milestones_array(&r.milestones, self.lang));
            }
            Err(e) => d.set("error", e.as_str()),
        }
        d
    }

    /// Rejoue un fichier : partie neuve sur la même planète, mêmes ordres,
    /// en pause à la date où il s'arrêtait.
    #[func]
    fn start_replay(&mut self, path: GString) -> bool {
        let r = match std::fs::read_to_string(path.to_string()).map_err(|e| e.to_string()).and_then(|t| ReplayFile::from_text(&t)) {
            Ok(r) => r,
            Err(e) => {
                godot_error!("rejeu illisible : {e}");
                return false;
            }
        };
        if evo_planet::PlanetParams::by_key(&r.spec.preset).is_none() {
            return false;
        }
        self.game = None;
        self.loading = None;
        self.set_grid(r.level);
        self.reset_view();
        match Game::replay(&r) {
            Ok(g) => {
                self.game = Some(g);
                true
            }
            Err(e) => {
                godot_error!("rejeu impossible : {e}");
                false
            }
        }
    }

    /// La partie rejoue-t-elle un fichier ? Son nom et la date où il
    /// s'arrêtait ; vide sinon.
    #[func]
    fn replay_info(&self) -> VarDictionary {
        let mut d = VarDictionary::new();
        if let Some((name, years)) = self.game.as_ref().and_then(|g| g.replay_of.as_ref()) {
            d.set("name", name.as_str());
            d.set("date", format::game_date(*years, self.lang));
        }
        d
    }

    /// Jalons de la partie en cours, tels qu'ils seraient sauvegardés.
    #[func]
    fn milestones(&mut self) -> VarArray {
        self.refresh_milestones();
        self.game.as_ref().map_or_else(VarArray::new, |g| milestones_array(&g.milestones, self.lang))
    }
}
