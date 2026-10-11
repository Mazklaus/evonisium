//! Peuples et cultures vus depuis Godot (étape 6) : territoires sur le globe
//! (G5), fiches de peuple et de personnage, arbre des cultures, mythes, mode
//! historique, campements de la scène au sol.
//!
//! Le moteur ne publie pas encore d'état culturel. En attendant, le client
//! pose des peuples d'essai sur les terres de la planète en cours
//! (`evo_view::culture::bench_world`), dans une tâche de fond ; rien n'en
//! remonte vers le monde.

use super::*;
use evo_culture::CulturalState;
use evo_view::culture::{self as cv, BenchSpec};
use std::sync::Mutex;

/// L'état culturel d'essai : en calcul, ou prêt.
#[derive(Default)]
pub(crate) struct CultureSlot {
    state: Option<Arc<CulturalState>>,
    pending: Option<Arc<Mutex<Option<CulturalState>>>>,
    /// Partie pour laquelle il a été calculé (nom de la planète et graine).
    key: String,
}

impl EvoSession {
    fn culture_key(&self) -> String {
        self.game.as_ref().map(|g| format!("{}-{}", g.planet.name, g.planet.seed)).unwrap_or_default()
    }

    /// L'état culturel prêt, s'il est celui de la partie en cours.
    pub(super) fn culture(&mut self) -> Option<Arc<CulturalState>> {
        let key = self.culture_key();
        let slot = &mut self.culture;
        if slot.key != key {
            *slot = CultureSlot::default();
            return None;
        }
        let ready = slot.pending.as_ref().and_then(|p| p.lock().unwrap().take());
        if let Some(s) = ready {
            slot.state = Some(Arc::new(s));
            slot.pending = None;
        }
        slot.state.clone()
    }

    fn ago(&self, year: f64, now: f64) -> String {
        let d = format::duration((now - year).max(0.0), self.lang);
        match self.lang {
            Lang::Fr => format!("il y a {d}"),
            Lang::En => format!("{d} ago"),
        }
    }

    fn sheet_dict(&self, s: &cv::PeopleSheet, now: f64) -> VarDictionary {
        let mut d = VarDictionary::new();
        d.set("id", s.id as i64);
        d.set("name", s.name.as_str());
        d.set("species", s.species_name.as_str());
        d.set("summary", s.summary.as_str());
        d.set("parent", s.parent.as_ref().map_or(-1, |p| p.0 as i64));
        d.set("parent_name", s.parent.as_ref().map_or("", |p| p.1.as_str()));
        let mut children = VarArray::new();
        for (id, name) in &s.children {
            let mut c = VarDictionary::new();
            c.set("id", *id as i64);
            c.set("name", name.as_str());
            children.push(&c.to_variant());
        }
        d.set("children", &children);
        d.set("born", self.ago(s.born_year, now).as_str());
        d.set("ended", s.ended_year.map_or(String::new(), |y| self.ago(y, now)).as_str());
        d.set("population", format::number_in(self.lang, s.population.round() as f64, 0));
        d.set("groups", s.groups as i64);
        d.set("cell", s.core_cell as i64);
        d.set("colour", colour(s.colour));
        d.set("language", s.language.as_deref().unwrap_or(""));
        let mut words = VarArray::new();
        for (meaning, word) in &s.words {
            let mut w = VarDictionary::new();
            w.set("meaning", meaning.as_str());
            w.set("word", word.as_str());
            words.push(&w.to_variant());
        }
        d.set("words", &words);
        let lines = |v: &[cv::TraitLine]| {
            let mut a = VarArray::new();
            for t in v {
                let mut x = VarDictionary::new();
                x.set("id", t.id as i64);
                x.set("kind", t.kind.as_str());
                x.set("name", t.name.as_str());
                x.set("share", t.share as f64);
                x.set("mastery", t.mastery as f64);
                x.set("generations", t.generations.round() as i64);
                a.push(&x.to_variant());
            }
            a
        };
        d.set("techniques", &lines(&s.techniques));
        d.set("customs", &lines(&s.customs));
        let mut myths = VarArray::new();
        for (id, title) in &s.myths {
            let mut m = VarDictionary::new();
            m.set("id", *id as i64);
            m.set("title", title.as_str());
            myths.push(&m.to_variant());
        }
        d.set("myths", &myths);
        let mut notables = VarArray::new();
        for (id, name, alive) in &s.notables {
            let mut n = VarDictionary::new();
            n.set("id", *id as i64);
            n.set("name", name.as_str());
            n.set("alive", *alive);
            notables.push(&n.to_variant());
        }
        d.set("notables", &notables);
        d
    }
}

#[godot_api(secondary)]
impl EvoSession {
    /// Pose des peuples d'essai sur les terres de la partie en cours, dans
    /// une tâche de fond (une à deux secondes).
    #[func]
    fn culture_bench(&mut self, seed: i64) {
        let key = self.culture_key();
        let (Some(f), Some(grid)) = (self.frame(), self.grid.clone()) else { return };
        if self.culture.key == key && (self.culture.state.is_some() || self.culture.pending.is_some()) {
            return;
        }
        let out = Arc::new(Mutex::new(None));
        self.culture = CultureSlot { state: None, pending: Some(out.clone()), key };
        let spec = BenchSpec { seed: seed as u64, ..Default::default() };
        std::thread::spawn(move || {
            if let Some(w) = cv::bench_world(&grid, f.cells(), spec) {
                *out.lock().unwrap() = Some(CulturalState::of(&w));
            }
        });
    }

    /// 0 : pas de peuples ; 1 : en calcul ; 2 : prêts.
    #[func]
    fn culture_status(&mut self) -> i64 {
        if self.culture().is_some() {
            2
        } else if self.culture.pending.is_some() {
            1
        } else {
            0
        }
    }

    /// Texture des territoires (rang de couleur, log₁₀ de l'effectif,
    /// frontière, mode historique).
    #[func]
    fn culture_texture(&mut self) -> Option<Gd<Image>> {
        let st = self.culture()?;
        let grid = self.grid.clone()?;
        let cells = self.frame()?.cells().len();
        let (w, h) = layers::texture_size(cells);
        image_rgbaf(w, h, &cv::territory_texture(&st, &grid, cells))
    }

    /// Palette des peuples (une ligne de 16 texels ; le rang 0 est vide).
    #[func]
    fn people_palette(&self) -> Option<Gd<Image>> {
        let mut px = vec![0u8; 16 * 4];
        for (i, c) in cv::PEOPLE_COLOURS.iter().enumerate() {
            let k = (i + 1) * 4;
            px[k] = (c[0] * 255.0) as u8;
            px[k + 1] = (c[1] * 255.0) as u8;
            px[k + 2] = (c[2] * 255.0) as u8;
            px[k + 3] = 255;
        }
        image_rgba8(16, 1, &px)
    }

    /// Noms des peuples vivants, à poser sur le globe.
    #[func]
    fn culture_labels(&mut self) -> VarArray {
        let mut out = VarArray::new();
        let Some(st) = self.culture() else { return out };
        for l in cv::labels(&st) {
            let mut d = VarDictionary::new();
            d.set("id", l.id as i64);
            d.set("name", l.name.as_str());
            d.set("cell", l.cell as i64);
            d.set("position", self.cell_centre(l.cell as i64));
            d.set("population", l.population as f64);
            d.set("colour", colour(l.colour));
            out.push(&d.to_variant());
        }
        out
    }

    /// Fiche d'un peuple.
    #[func]
    fn people_info(&mut self, id: i64) -> VarDictionary {
        let Some(st) = self.culture() else { return VarDictionary::new() };
        match cv::people_sheet(&st, id as u32, self.lang) {
            Some(s) => self.sheet_dict(&s, st.year),
            None => VarDictionary::new(),
        }
    }

    /// Fiche d'un personnage notable.
    #[func]
    fn notable_info(&mut self, id: i64) -> VarDictionary {
        let mut d = VarDictionary::new();
        let Some(st) = self.culture() else { return d };
        let Some(n) = cv::notable_sheet(&st, id as u32, self.lang) else { return d };
        d.set("id", n.id as i64);
        d.set("name", n.name.as_str());
        d.set("role", n.role.as_str());
        d.set("people", n.people.0 as i64);
        d.set("people_name", n.people.1.as_str());
        d.set("species", n.species_name.as_str());
        d.set("cell", n.cell as i64);
        d.set("trait", n.trait_name.as_str());
        d.set("born", self.ago(n.born_year, st.year).as_str());
        d.set("death", self.ago(n.death_year, st.year).as_str());
        d.set("alive", n.alive);
        d.set("story", n.story.as_str());
        d
    }

    /// Arbre des cultures : peuples, ou langues (`languages`), au format de
    /// l'arbre du vivant, avec un nom et une couleur par noeud.
    #[func]
    fn culture_tree(&mut self, languages: bool, max_leaves: i64) -> VarDictionary {
        let mut d = VarDictionary::new();
        let Some(st) = self.culture() else { return d };
        let lineages = cv::tree_lineages(&st, languages);
        let t = evo_view::tree::build_keeping_parents(&lineages, st.year, max_leaves.max(2) as usize);
        let mut ids = PackedInt32Array::new();
        let mut parents = PackedInt32Array::new();
        let mut born = PackedFloat64Array::new();
        let mut end = PackedFloat64Array::new();
        let mut ys = PackedFloat32Array::new();
        let mut living = PackedByteArray::new();
        let mut names = PackedStringArray::new();
        let mut colours = PackedColorArray::new();
        for n in &t.nodes {
            ids.push(n.lineage as i32);
            parents.push(n.parent as i32);
            born.push(n.born_years);
            end.push(n.end_years);
            ys.push(n.y);
            living.push(n.living as u8);
            if languages {
                let l = st.languages.iter().find(|l| l.id == n.lineage);
                names.push(l.map_or("", |l| l.name.as_str()));
                // Une langue prend la couleur du peuple qui la parle.
                let p = st.peoples.iter().find(|p| p.language == Some(n.lineage));
                colours.push(p.map_or(Color::from_rgb(0.3, 0.25, 0.2), |p| colour(cv::people_colour(&st, p.id))));
            } else {
                names.push(st.peoples.iter().find(|p| p.id == n.lineage).map_or("", |p| p.name.as_str()));
                colours.push(colour(cv::people_colour(&st, n.lineage)));
            }
        }
        d.set("ids", &ids);
        d.set("parents", &parents);
        d.set("born", &born);
        d.set("end", &end);
        d.set("y", &ys);
        d.set("living", &living);
        d.set("names", &names);
        d.set("colours", &colours);
        d.set("leaves", t.leaves as i64);
        d.set("from_years", if t.from_years.is_finite() { t.from_years } else { 0.0 });
        d.set("to_years", t.to_years);
        d
    }

    /// Tous les mythes, du plus ancien au plus récent.
    #[func]
    fn myths(&mut self) -> VarArray {
        let mut out = VarArray::new();
        let Some(st) = self.culture() else { return out };
        for m in cv::myths(&st) {
            let mut d = VarDictionary::new();
            d.set("id", m.id as i64);
            d.set("title", m.title.as_str());
            d.set("text", m.text.as_str());
            d.set("people", m.people.as_str());
            d.set("date", self.ago(m.year, st.year).as_str());
            d.set("cell", m.cell as i64);
            d.set("intervention", m.intervention);
            d.set("theme", m.theme.as_str());
            d.set("figure", m.figure.as_str());
            out.push(&d.to_variant());
        }
        out
    }

    /// Le peuple d'une cellule, pour l'inspecteur : `people` et `text`.
    #[func]
    fn culture_at(&mut self, cell: i64) -> VarDictionary {
        let mut d = VarDictionary::new();
        let Some(st) = self.culture() else { return d };
        if let Some((id, text)) = cv::cell_line(&st, cell as u32, self.lang) {
            d.set("people", id as i64);
            d.set("text", text.as_str());
        }
        d
    }

    /// Ligne du mode culturel pour la barre du temps : `text`, `historical`
    /// et l'année du banc.
    #[func]
    fn culture_mode(&mut self) -> VarDictionary {
        let mut d = VarDictionary::new();
        let Some(st) = self.culture() else { return d };
        d.set("text", cv::mode_line(&st, self.lang).as_str());
        d.set("historical", st.historical);
        d.set("year", st.year);
        d
    }

    /// Campement d'une cellule pour la scène au sol : techniques portées
    /// (clés anglaises du catalogue), rites, effectif, couleur du peuple.
    #[func]
    fn camp_at(&mut self, cell: i64) -> VarDictionary {
        let mut d = VarDictionary::new();
        let Some(st) = self.culture() else { return d };
        let Some(c) = cv::camp(&st, cell as u32) else { return d };
        let mut t = PackedStringArray::new();
        for f in &c.techniques {
            t.push(format!("{f:?}").as_str());
        }
        d.set("techniques", &t);
        d.set("rites", c.rites);
        d.set("population", c.population as f64);
        d.set("cohesion", c.cohesion as f64);
        d.set("people", c.people as i64);
        d.set("colour", colour(cv::people_colour(&st, c.people)));
        d.set("name", st.peoples.iter().find(|p| p.id == c.people).map_or("", |p| p.name.as_str()));
        d
    }
}
