//! Chronique en récit et « Pendant votre absence » (document Fonctionnalités,
//! « Alertes et chronique ») : la partie racontée par chapitres, comme une
//! histoire naturelle, et le résumé de ce qui a changé pendant une longue
//! avance sans regard du joueur.
//!
//! Les chapitres s'ouvrent aux grands basculements de la partie (première
//! lumière captée, oxygène dans l'air, cellule complexe, premiers corps,
//! terre ferme), jamais à des dates fixées d'avance : chaque planète a son
//! propre calendrier.

use crate::chronicle::{body, level, Level};
use crate::format::{duration, number_in, Lang};
use evo_core::events::{Event, EventKind, Origin};

/// Un chapitre du récit.
#[derive(Clone, Debug, PartialEq)]
pub struct Chapter {
    pub title: String,
    pub from_years: f64,
    /// Fin du chapitre (le suivant, ou la date de la partie).
    pub to_years: f64,
    pub text: String,
    /// Événements à aller voir, du plus intéressant au moins intéressant.
    pub highlights: Vec<u64>,
}

/// Basculement qui ouvre un chapitre, et son rang. Chaque chapitre s'ouvre
/// une fois, au premier de ses basculements ; les chapitres se suivent dans
/// l'ordre où la planète les a vécus, qui n'est pas toujours celui de la
/// Terre (une colonie peut précéder la cellule complexe).
fn opening(e: &Event) -> Option<usize> {
    match &e.kind {
        EventKind::LifeSeeded { .. } => Some(0),
        EventKind::Innovation { pathway, stage, .. } if pathway == "photosynthèse" && *stage >= 1 => Some(1),
        EventKind::Innovation { pathway, .. } if pathway == "rhodopsine" => Some(1),
        EventKind::OxygenThreshold { rising: true, .. } => Some(2),
        EventKind::Innovation { pathway, stage, .. } if pathway == "cellule complexe" && *stage >= 2 && *stage < 5 => Some(3),
        EventKind::Innovation { pathway, stage, .. } if pathway == "cellule complexe" && (5..8).contains(stage) => Some(4),
        EventKind::Innovation { pathway, stage, .. } if pathway == "cellule complexe" && *stage >= 8 => Some(5),
        _ => None,
    }
}

const TITLES: [[&str; 2]; 6] = [
    ["Les premières cellules", "The first cells"],
    ["La lumière captée", "Catching light"],
    ["L'air change", "The air changes"],
    ["La cellule complexe", "The complex cell"],
    ["Les premiers corps", "The first bodies"],
    ["La conquête des terres", "Onto dry land"],
];

fn title(rank: usize, lang: Lang) -> &'static str {
    TITLES[rank][if lang == Lang::Fr { 0 } else { 1 }]
}

/// Compte des événements d'une période.
#[derive(Default)]
struct Tally {
    new: usize,
    extinct: usize,
    player: usize,
    refused: usize,
    accelerated: bool,
}

fn tally(events: &[&Event], interventions: &[u64]) -> Tally {
    let mut t = Tally::default();
    for e in events {
        match &e.kind {
            EventKind::NewLineage { .. } => t.new += 1,
            EventKind::LineageExtinct { .. } => t.extinct += 1,
            EventKind::OrderApplied { order, .. } if interventions.contains(order) => t.player += 1,
            EventKind::OrderRefused { .. } => t.refused += 1,
            _ => {}
        }
        if e.origin == Origin::Accelerator || matches!(e.kind, EventKind::AcceleratorOn { .. }) {
            t.accelerated = true;
        }
    }
    t
}

fn plural(n: usize, one: &str, many: &str) -> String {
    if n > 1 {
        many.to_string()
    } else {
        one.to_string()
    }
}

/// Phrase des naissances et des extinctions.
fn turnover(t: &Tally, lang: Lang) -> Option<String> {
    let n = |x: usize| number_in(lang, x as f64, 0);
    match (t.new, t.extinct, lang) {
        (0, 0, _) => None,
        (a, 0, Lang::Fr) => {
            Some(format!("{} {} {}.", n(a), plural(a, "nouvelle lignée", "nouvelles lignées"), plural(a, "apparaît", "apparaissent")))
        }
        (a, 0, Lang::En) => Some(format!("{} new {} {}.", n(a), plural(a, "lineage", "lineages"), plural(a, "appears", "appear"))),
        (0, b, Lang::Fr) => Some(format!("{} {} {}.", n(b), plural(b, "lignée", "lignées"), plural(b, "s'éteint", "s'éteignent"))),
        (0, b, Lang::En) => Some(format!("{} {} {} out.", n(b), plural(b, "lineage", "lineages"), plural(b, "dies", "die"))),
        (a, b, Lang::Fr) => Some(format!(
            "{} {} {}, {} {}.",
            n(a),
            plural(a, "nouvelle lignée", "nouvelles lignées"),
            plural(a, "apparaît", "apparaissent"),
            n(b),
            plural(b, "s'éteint", "s'éteignent")
        )),
        (a, b, Lang::En) => Some(format!(
            "{} new {} {}, {} {} out.",
            n(a),
            plural(a, "lineage", "lineages"),
            plural(a, "appears", "appear"),
            n(b),
            plural(b, "dies", "die")
        )),
    }
}

/// Les événements qui comptent dans une période : au moins notables à
/// vitesse moyenne, hors naissances et extinctions ordinaires et
/// accélérateurs (déjà comptés), les plus intéressants d'abord. Des seuils d'oxygène franchis
/// d'affilée, seul le dernier est dit.
fn highlights<'a>(events: &[&'a Event], interventions: &[u64], max: usize) -> Vec<&'a Event> {
    let superseded = |e: &Event| match e.kind {
        EventKind::OxygenThreshold { rising, .. } => events
            .iter()
            .any(|o| matches!(o.kind, EventKind::OxygenThreshold { rising: r, .. } if r == rising) && (o.years, o.id) > (e.years, e.id)),
        _ => false,
    };
    let mut v: Vec<&Event> = events
        .iter()
        .copied()
        .filter(|e| {
            !matches!(
                e.kind,
                EventKind::NewLineage { .. }
                    | EventKind::LineageExtinct { .. }
                    | EventKind::AcceleratorOn { .. }
                    | EventKind::AcceleratorOff { .. }
            )
        })
        .filter(|e| match &e.kind {
            EventKind::OrderApplied { order, .. } => interventions.contains(order),
            _ => true,
        })
        .filter(|e| !superseded(e))
        .filter(|e| level(e, 1.0e5) >= Level::Notable)
        .collect();
    v.sort_by(|a, b| b.interest.total_cmp(&a.interest).then(a.id.cmp(&b.id)));
    v.truncate(max);
    // Dans l'ordre du temps pour le récit.
    v.sort_by(|a, b| a.years.total_cmp(&b.years).then(a.id.cmp(&b.id)));
    v
}

fn period_text(
    events: &[&Event],
    opening_event: Option<&Event>,
    interventions: &[u64],
    lang: Lang,
    name: &dyn Fn(u32) -> String,
) -> (String, Vec<u64>) {
    let mut parts: Vec<String> = Vec::new();
    if let Some(e) = opening_event {
        parts.push(body(e, lang, name));
    }
    let t = tally(events, interventions);
    if let Some(s) = turnover(&t, lang) {
        parts.push(s);
    }
    let hl = highlights(events, interventions, 4);
    for e in &hl {
        if opening_event.is_some_and(|o| o.id == e.id) {
            continue;
        }
        let when = duration(e.years, lang);
        parts.push(match lang {
            Lang::Fr => format!("À {when}, {}", lower_first(&body(e, lang, name))),
            Lang::En => format!("At {when}, {}", lower_first(&body(e, lang, name))),
        });
    }
    if t.player > 0 {
        parts.push(match lang {
            Lang::Fr => format!("Vous êtes intervenu {} fois.", t.player),
            Lang::En => format!("You intervened {} time{}.", t.player, if t.player > 1 { "s" } else { "" }),
        });
    }
    if t.accelerated {
        parts.push(match lang {
            Lang::Fr => "Un accélérateur discret a aidé l'évolution pendant cette période.".into(),
            Lang::En => "A discreet accelerator helped evolution during this period.".into(),
        });
    }
    let mut ids: Vec<u64> = hl.iter().map(|e| e.id).collect();
    if let Some(o) = opening_event {
        if !ids.contains(&o.id) {
            ids.insert(0, o.id);
        }
    }
    (parts.join(" "), ids)
}

fn lower_first(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        // Les noms de lignée gardent leur majuscule.
        Some(f) if f.is_uppercase() && c.clone().next().is_some_and(|n| n.is_lowercase() || n == '\'') => {
            f.to_lowercase().collect::<String>() + c.as_str()
        }
        _ => s.to_string(),
    }
}

/// Le récit de la partie jusqu'à `now_years`, chapitre par chapitre.
/// `interventions` : numéros des ordres qui sont des interventions du
/// joueur (ses autres ordres, comme l'ensemencement, ne comptent pas).
pub fn narrative(events: &[Event], now_years: f64, interventions: &[u64], lang: Lang, name: &dyn Fn(u32) -> String) -> Vec<Chapter> {
    // Ouvertures : le premier événement de chaque rang, dans l'ordre.
    let mut opens: Vec<(usize, &Event)> = Vec::new();
    for e in events {
        if let Some(r) = opening(e) {
            if opens.iter().all(|(seen, _)| *seen != r) {
                opens.push((r, e));
            }
        }
    }
    opens.sort_by(|a, b| a.1.years.total_cmp(&b.1.years).then(a.1.id.cmp(&b.1.id)));
    if opens.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    for (k, (rank, open)) in opens.iter().enumerate() {
        let from = open.years;
        let to = opens.get(k + 1).map_or(now_years.max(from), |(_, e)| e.years);
        let last = k + 1 == opens.len();
        let period: Vec<&Event> = events
            .iter()
            .filter(|e| e.years >= from && (e.years < to || (last && e.years <= to)))
            .filter(|e| opens.get(k + 1).is_none_or(|(_, n)| e.id != n.id))
            .collect();
        let (text, highlights) = period_text(&period, Some(open), interventions, lang, name);
        let span = match lang {
            Lang::Fr => format!("{} — de {} à {}", title(*rank, lang), duration(from, lang), duration(to, lang)),
            Lang::En => format!("{} — from {} to {}", title(*rank, lang), duration(from, lang), duration(to, lang)),
        };
        out.push(Chapter { title: span, from_years: from, to_years: to, text, highlights });
    }
    out
}

/// « Pendant votre absence » : ce qui a changé entre `since_years` et
/// `now_years`, ou rien si rien ne vaut d'être dit.
pub fn absence(
    events: &[Event],
    since_years: f64,
    now_years: f64,
    interventions: &[u64],
    lang: Lang,
    name: &dyn Fn(u32) -> String,
) -> Option<(String, Vec<u64>)> {
    let period: Vec<&Event> = events.iter().filter(|e| e.years > since_years && e.years <= now_years).collect();
    if period.is_empty() {
        return None;
    }
    let (text, ids) = period_text(&period, None, interventions, lang, name);
    if text.is_empty() {
        return None;
    }
    let head = match lang {
        Lang::Fr => format!("En {} :", duration(now_years - since_years, lang)),
        Lang::En => format!("In {}:", duration(now_years - since_years, lang)),
    };
    Some((format!("{head} {text}"), ids))
}

#[cfg(test)]
mod tests {
    use super::*;
    use evo_core::events::EventLog;

    fn game() -> EventLog {
        let mut log = EventLog::default();
        log.push(1e5, Some(3), EventKind::LifeSeeded { lineage: 0 });
        for i in 1..6 {
            log.push(1e6 * i as f64, None, EventKind::NewLineage { lineage: i, parent: 0, signature: i });
        }
        log.push(
            3e7,
            Some(5),
            EventKind::Innovation { lineage: 2, pathway: "photosynthèse".into(), stage: 1, label: "pigment protecteur".into() },
        );
        log.push(3.5e7, None, EventKind::LineageExtinct { lineage: 1 });
        log.push(
            9e7,
            Some(5),
            EventKind::Innovation { lineage: 2, pathway: "photosynthèse".into(), stage: 4, label: "photosynthèse oxygénique".into() },
        );
        log.push(1.2e8, None, EventKind::OxygenThreshold { mixing_ratio: 1e-4, rising: true });
        log.push(1.3e8, None, EventKind::Snowball { ice_fraction: 0.9, starts: true });
        log
    }

    #[test]
    fn the_story_opens_a_chapter_at_each_turning_point() {
        let log = game();
        let name = |id: u32| format!("Lignée {id}");
        let ch = narrative(&log.events, 2e8, &[], Lang::Fr, &name);
        assert_eq!(ch.len(), 3, "{ch:#?}");
        assert!(ch[0].title.starts_with("Les premières cellules"));
        assert!(ch[0].text.contains("5 nouvelles lignées apparaissent"), "{}", ch[0].text);
        assert!(ch[1].title.starts_with("La lumière captée"));
        assert!(ch[1].text.contains("1 lignée s'éteint"), "{}", ch[1].text);
        // La photosynthèse oxygénique est racontée dans son chapitre.
        assert!(ch[1].text.contains("photosynthèse oxygénique"), "{}", ch[1].text);
        assert!(ch[2].title.starts_with("L'air change"));
        assert!(ch[2].text.contains("boule de neige"), "{}", ch[2].text);
        assert_eq!(ch[2].to_years, 2e8);
        // Chaque chapitre mène à ses moments.
        assert!(ch.iter().all(|c| !c.highlights.is_empty()));
        let en = narrative(&log.events, 2e8, &[], Lang::En, &name);
        assert!(en[0].text.contains("5 new lineages appear"), "{}", en[0].text);
    }

    #[test]
    fn the_absence_summary_tells_what_changed() {
        let log = game();
        let name = |id: u32| format!("Lignée {id}");
        let (s, ids) = absence(&log.events, 2e7, 2e8, &[], Lang::Fr, &name).unwrap();
        assert!(s.starts_with("En 180 Ma :"), "{s}");
        assert!(s.contains("1 lignée s'éteint"), "{s}");
        assert!(s.contains("boule de neige"), "{s}");
        assert!(!ids.is_empty());
        assert!(absence(&log.events, 2e8, 3e8, &[], Lang::Fr, &name).is_none());
    }

    #[test]
    fn only_the_last_threshold_and_real_interventions_are_told() {
        let mut log = game();
        for k in [1e-6, 1e-5] {
            log.push(1.4e8, None, EventKind::OxygenThreshold { mixing_ratio: k, rising: true });
        }
        // L'ensemencement est un ordre du joueur, pas une intervention.
        log.push_with(1.5e8, None, EventKind::OrderApplied { order: 1, label: "ensemencement".into() }, Origin::Player, None);
        log.push_with(1.6e8, Some(4), EventKind::OrderApplied { order: 2, label: "impact".into() }, Origin::Player, None);
        let name = |id: u32| format!("Lignée {id}");
        let (s, _) = absence(&log.events, 1.1e8, 2e8, &[2], Lang::Fr, &name).unwrap();
        assert_eq!(s.matches("seuil").count(), 1, "{s}");
        assert!(s.contains("intervenu 1 fois"), "{s}");
        assert!(s.contains("l'oxygène"), "{s}");
    }

    #[test]
    fn chapters_follow_the_planet_not_the_earth() {
        let mut log = EventLog::default();
        log.push(1e5, Some(3), EventKind::LifeSeeded { lineage: 0 });
        log.push(
            4e8,
            None,
            EventKind::Innovation { lineage: 2, pathway: "cellule complexe".into(), stage: 5, label: "colonie clonale".into() },
        );
        log.push(8e8, None, EventKind::OxygenThreshold { mixing_ratio: 1e-6, rising: true });
        log.push(
            9e8,
            None,
            EventKind::Innovation { lineage: 4, pathway: "cellule complexe".into(), stage: 2, label: "eucaryote (endosymbiose)".into() },
        );
        let name = |id: u32| format!("Lignée {id}");
        let ch = narrative(&log.events, 1e9, &[], Lang::Fr, &name);
        let titles: Vec<&str> = ch.iter().map(|c| c.title.split(" — ").next().unwrap()).collect();
        assert_eq!(titles, ["Les premières cellules", "Les premiers corps", "L'air change", "La cellule complexe"]);
    }
}
