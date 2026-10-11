//! Narrateur discret de la première partie (document Fonctionnalités,
//! « Prise en main » ; décision de l'utilisateur du 8 octobre 2026) : il
//! présente un outil à la fois, quand il devient utile (la frise au premier
//! jalon, l'arbre du vivant à la première spéciation…), sans jamais bloquer
//! le jeu. Une fois le guide épuisé, il devient le conseiller : il signale
//! ce qui vaut d'être regardé, d'après le score d'intérêt des événements, et
//! ne dit jamais quoi faire.
//!
//! Ce module choisit la phrase ; le client l'affiche en sous-titre, la
//! souligne sur l'outil visé et la confie à la voix du module son.

use crate::format::Lang;

/// Ce que le guide sait de la partie.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GuideState {
    pub years: f64,
    /// Lignées vivantes.
    pub lineages: u32,
    /// Nouvelles lignées depuis le début.
    pub speciations: u32,
    /// Événements au moins notables depuis le début.
    pub notable: u32,
    pub photosynthesis_stage: u8,
    pub o2: f64,
    /// Plus haute étape du chemin vers la cellule complexe (0 : aucune ;
    /// 2 : eucaryote ; 5 : colonie ; 7 : multicellulaire à deux types ;
    /// 8 : hors de l'eau).
    pub complexity: u8,
    /// Interventions du joueur appliquées.
    pub interventions: u32,
    /// Outils que le joueur a déjà ouverts (clés de [`Tool`]).
    pub opened: Vec<String>,
    /// Une cellule est sélectionnée sur le globe.
    pub cell_selected: bool,
}

impl GuideState {
    fn has_opened(&self, tool: Tool) -> bool {
        self.opened.iter().any(|o| o == tool.key())
    }
}

/// Outil de l'interface que le guide présente : le client le souligne.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tool {
    TimeBar,
    Globe,
    Inspector,
    Magnifier,
    Timeline,
    Tree,
    Layers,
    Chronicle,
    OxygenGauge,
    Interventions,
    WithWithout,
    FoodWeb,
    Strata,
    SpeciesSheet,
    Anatomy,
    Ground,
}

impl Tool {
    pub fn key(self) -> &'static str {
        match self {
            Tool::TimeBar => "temps",
            Tool::Globe => "globe",
            Tool::Inspector => "inspecteur",
            Tool::Magnifier => "loupe",
            Tool::Timeline => "frise",
            Tool::Tree => "arbre",
            Tool::Layers => "calques",
            Tool::Chronicle => "chronique",
            Tool::OxygenGauge => "oxygene",
            Tool::Interventions => "interventions",
            Tool::WithWithout => "avec_sans",
            Tool::FoodWeb => "reseau",
            Tool::Strata => "strates",
            Tool::SpeciesSheet => "fiche",
            Tool::Anatomy => "anatomie",
            Tool::Ground => "sol",
        }
    }
}

/// Une phrase du guide.
pub struct Hint {
    pub id: &'static str,
    pub tool: Tool,
    when: fn(&GuideState) -> bool,
    /// Le joueur s'en sert déjà : inutile de le présenter.
    known: fn(&GuideState) -> bool,
    pub fr: &'static str,
    pub en: &'static str,
}

impl Hint {
    pub fn text(&self, lang: Lang) -> &'static str {
        match lang {
            Lang::Fr => self.fr,
            Lang::En => self.en,
        }
    }
}

fn never(_: &GuideState) -> bool {
    false
}

/// Les phrases, dans l'ordre où elles peuvent venir. Chacune ne vient
/// qu'une fois, quand sa condition est vraie et que l'outil n'a pas déjà
/// été trouvé par le joueur.
pub const HINTS: &[Hint] = &[
    Hint {
        id: "temps",
        tool: Tool::TimeBar,
        when: |_| true,
        known: never,
        fr: "La vie est déposée près des sources chaudes. Le temps file : la barre du haut l'accélère, l'arrête, ou saute au prochain événement (touche N).",
        en: "Life is seeded near the hot springs. Time is running: the top bar speeds it up, stops it, or jumps to the next event (N key).",
    },
    Hint {
        id: "inspecteur",
        tool: Tool::Globe,
        when: |s| s.years > 2.0e6,
        known: |s| s.cell_selected || s.has_opened(Tool::Inspector),
        fr: "Cliquez sur la mer : l'inspecteur décrit le milieu de ce lieu et les populations qui y vivent.",
        en: "Click the sea: the inspector describes the environment there and the populations living in it.",
    },
    Hint {
        id: "loupe",
        tool: Tool::Magnifier,
        when: |s| s.cell_selected,
        known: |s| s.has_opened(Tool::Magnifier),
        fr: "Sous la liste des populations, la loupe montre les cellules elles-mêmes, dessinées d'après leurs gènes.",
        en: "Below the populations, the magnifier shows the cells themselves, drawn from their genes.",
    },
    Hint {
        id: "arbre",
        tool: Tool::Tree,
        when: |s| s.speciations >= 1,
        known: |s| s.has_opened(Tool::Tree),
        fr: "Une nouvelle lignée s'est détachée. L'arbre du vivant (touche T) montre d'où elle vient.",
        en: "A new lineage has branched off. The tree of life (T key) shows where it comes from.",
    },
    Hint {
        id: "frise",
        tool: Tool::Timeline,
        when: |s| s.notable >= 1,
        known: never,
        fr: "Ce repère sur la frise marque un premier jalon. Un clic dessus ramène la caméra au lieu et à la date.",
        en: "This mark on the timeline is a first milestone. Clicking it takes the camera back to the place and date.",
    },
    Hint {
        id: "calques",
        tool: Tool::Layers,
        when: |s| s.lineages >= 3 && s.years > 2.0e7,
        known: |s| s.has_opened(Tool::Layers),
        fr: "Les calques posent une donnée en lavis sur le globe. Essayez « Biomasse », puis revenez à la vue naturelle.",
        en: "Layers lay one quantity as a wash on the globe. Try “Biomass”, then return to the natural view.",
    },
    Hint {
        id: "pigment",
        tool: Tool::OxygenGauge,
        when: |s| s.photosynthesis_stage >= 1,
        known: never,
        fr: "Des cellules captent la lumière avec un pigment. Si l'une d'elles apprend à casser l'eau, l'oxygène montera : il se lit en haut de l'écran.",
        en: "Cells now harvest light with a pigment. If one learns to split water, oxygen will rise: it reads at the top of the screen.",
    },
    Hint {
        id: "chronique",
        tool: Tool::Chronicle,
        when: |s| s.notable >= 3,
        known: |s| s.has_opened(Tool::Chronicle),
        fr: "Tout ce qui s'est passé est écrit dans la chronique (touche C), événement par événement ou en récit.",
        en: "Everything that has happened is written in the chronicle (C key), event by event or as a story.",
    },
    Hint {
        id: "reseau",
        tool: Tool::FoodWeb,
        when: |s| s.lineages >= 6 && s.cell_selected,
        known: |s| s.has_opened(Tool::FoodWeb),
        fr: "Plusieurs espèces partagent maintenant ce lieu. Le réseau trophique, dans l'inspecteur, montre qui vit de qui.",
        en: "Several species now share this place. The food web, in the inspector, shows who lives off whom.",
    },
    Hint {
        id: "oxygene",
        tool: Tool::Interventions,
        when: |s| s.o2 > 1.0e-4 || s.years > 4.0e8,
        known: |s| s.has_opened(Tool::Interventions),
        fr: "Vous pouvez aider ou gêner la vie : les interventions agissent sur l'environnement, jamais sur les gènes, et puisent dans une réserve qui se recharge.",
        en: "You can help or hinder life: interventions act on the environment, never on genes, and draw on a reserve that refills.",
    },
    Hint {
        id: "avec_sans",
        tool: Tool::WithWithout,
        when: |s| s.interventions >= 1,
        known: |s| s.has_opened(Tool::WithWithout),
        fr: "Et sans votre intervention ? La touche A rejoue la planète sans elle, pour comparer les deux histoires.",
        en: "And without your intervention? The A key replays the planet without it, to compare both histories.",
    },
    Hint {
        id: "strates",
        tool: Tool::Strata,
        when: |s| s.years > 5.0e8 && s.cell_selected,
        known: |s| s.has_opened(Tool::Strata),
        fr: "Les roches gardent la mémoire de la partie : la colonne stratigraphique, dans l'inspecteur, se lit de bas en haut.",
        en: "Rocks keep the memory of the game: the stratigraphic column, in the inspector, reads from bottom to top.",
    },
    Hint {
        id: "eucaryote",
        tool: Tool::SpeciesSheet,
        when: |s| s.complexity >= 2,
        known: never,
        fr: "Une cellule en a gardé une autre en elle : voici une cellule complexe. Ouvrez sa fiche pour voir ses organites.",
        en: "One cell has kept another inside it: here is a complex cell. Open its sheet to see its organelles.",
    },
    Hint {
        id: "corps",
        tool: Tool::Anatomy,
        when: |s| s.complexity >= 5,
        known: |s| s.has_opened(Tool::Anatomy),
        fr: "Des cellules restent ensemble après s'être divisées : un premier corps. L'anatomie le montre en volume.",
        en: "Cells now stay together after dividing: a first body. The anatomy view shows it in the round.",
    },
    Hint {
        id: "sol",
        tool: Tool::Ground,
        when: |s| s.complexity >= 8,
        known: |s| s.has_opened(Tool::Ground),
        fr: "La vie complexe a gagné la terre ferme. Depuis l'inspecteur, « Descendre au sol » mène au milieu des êtres vivants.",
        en: "Complex life has reached dry land. From the inspector, “Go down to the ground” takes you among living things.",
    },
];

/// Prochaine phrase du guide : la première non vue dont la condition est
/// vraie et dont l'outil n'est pas déjà connu du joueur. Les phrases dont
/// l'outil est connu sont rendues à part, pour être marquées vues.
pub fn next<'a>(state: &GuideState, seen: &[String]) -> (Option<&'a Hint>, Vec<&'static str>) {
    let mut known = Vec::new();
    let mut found = None;
    for h in HINTS {
        if seen.iter().any(|s| s == h.id) {
            continue;
        }
        if (h.known)(state) {
            known.push(h.id);
            continue;
        }
        if found.is_none() && (h.when)(state) {
            found = Some(h);
        }
    }
    (found, known)
}

/// Le guide est épuisé : toutes ses phrases ont été vues.
pub fn finished(seen: &[String]) -> bool {
    HINTS.iter().all(|h| seen.iter().any(|s| s == h.id))
}

/// Phrase du conseiller pour un événement : ce qui vaut d'être regardé, sans
/// dire quoi faire.
pub fn advice(sentence: &str, lang: Lang) -> String {
    match lang {
        Lang::Fr => format!("À regarder : {sentence}"),
        Lang::En => format!("Worth a look: {sentence}"),
    }
}

/// Un événement mérite le conseiller s'il est au moins notable et assez
/// intéressant ; le client espace ensuite les conseils.
pub fn worth_advice(interest: f64) -> bool {
    interest >= 0.6
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seen(ids: &[&str]) -> Vec<String> {
        ids.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn the_guide_opens_with_time_then_follows_the_game() {
        let mut s = GuideState::default();
        let (h, _) = next(&s, &[]);
        assert_eq!(h.unwrap().id, "temps");
        // Rien d'autre tant que la partie ne fait que commencer.
        let (h, _) = next(&s, &seen(&["temps"]));
        assert!(h.is_none());
        s.years = 3.0e6;
        assert_eq!(next(&s, &seen(&["temps"])).0.unwrap().id, "inspecteur");
        // Première spéciation : l'arbre, une seule fois.
        s.speciations = 1;
        let v = seen(&["temps", "inspecteur"]);
        assert_eq!(next(&s, &v).0.unwrap().id, "arbre");
        let v = seen(&["temps", "inspecteur", "arbre"]);
        assert!(next(&s, &v).0.is_none());
        // Premier jalon : la frise.
        s.notable = 1;
        assert_eq!(next(&s, &v).0.unwrap().id, "frise");
    }

    #[test]
    fn a_tool_the_player_already_found_is_not_presented() {
        let s = GuideState { years: 3.0e6, cell_selected: true, opened: vec!["loupe".into()], ..Default::default() };
        let (h, known) = next(&s, &seen(&["temps"]));
        assert!(h.is_none(), "{:?}", h.map(|h| h.id));
        assert!(known.contains(&"inspecteur") && known.contains(&"loupe"));
    }

    #[test]
    fn every_hint_has_both_languages_and_the_guide_ends() {
        let mut ids = Vec::new();
        for h in HINTS {
            assert!(!h.fr.is_empty() && !h.en.is_empty(), "{}", h.id);
            assert!(!ids.contains(&h.id), "{} en double", h.id);
            ids.push(h.id);
        }
        let all: Vec<String> = ids.iter().map(|s| s.to_string()).collect();
        assert!(finished(&all));
        assert!(!finished(&all[1..]));
        // Une partie qui a tout vu : rien à dire.
        let s = GuideState { years: 1.0e9, complexity: 8, o2: 0.2, lineages: 50, speciations: 50, notable: 9, ..Default::default() };
        assert!(next(&s, &all).0.is_none());
    }
}
