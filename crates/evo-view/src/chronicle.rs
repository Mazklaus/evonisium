//! Chronique, alertes et règles d'arrêt (document Fonctionnalités, sections
//! « Contrôle du temps » et « Alertes et chronique »).
//!
//! La chronique est écrite par des modèles de phrases à partir des
//! événements du moteur. Le niveau d'alerte d'un événement dépend de son
//! score d'intérêt et de la vitesse du temps : plus on va vite, plus il faut
//! être important pour apparaître. Le joueur règle, pour chaque famille
//! d'événements, ce que le jeu fait : ignorer, noter, alerter, ralentir ou
//! mettre en pause.

use crate::format::{duration, power_of_ten_in, Lang};
use evo_core::events::{Event, EventKind, Origin};
use evo_sim::history::EventView;

/// Familles d'événements des règles d'arrêt.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Family {
    Speciation,
    Extinction,
    Innovation,
    Catastrophe,
    Milestone,
    Intervention,
    Geology,
    Accelerator,
}

impl Family {
    pub const ALL: [Family; 8] = [
        Family::Milestone,
        Family::Innovation,
        Family::Catastrophe,
        Family::Speciation,
        Family::Extinction,
        Family::Intervention,
        Family::Geology,
        Family::Accelerator,
    ];

    pub fn of(e: &Event) -> Family {
        match &e.kind {
            EventKind::LifeSeeded { .. } | EventKind::OxygenThreshold { .. } => Family::Milestone,
            EventKind::NewLineage { .. } => Family::Speciation,
            EventKind::LineageExtinct { .. } => Family::Extinction,
            EventKind::Innovation { .. } => Family::Innovation,
            EventKind::Snowball { .. } => Family::Catastrophe,
            EventKind::PlateReorganisation { .. } => Family::Geology,
            EventKind::AcceleratorOn { .. } | EventKind::AcceleratorOff { .. } => Family::Accelerator,
            EventKind::OrderApplied { .. } | EventKind::OrderRefused { .. } => Family::Intervention,
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Family::Speciation => "speciation",
            Family::Extinction => "extinction",
            Family::Innovation => "innovation",
            Family::Catastrophe => "catastrophe",
            Family::Milestone => "jalon",
            Family::Intervention => "intervention",
            Family::Geology => "geologie",
            Family::Accelerator => "accelerateur",
        }
    }

    pub fn from_key(k: &str) -> Option<Family> {
        Family::ALL.into_iter().find(|f| f.key() == k)
    }

    pub fn label(self, lang: Lang) -> &'static str {
        match (lang, self) {
            (Lang::Fr, Family::Speciation) => "Nouvelles lignées",
            (Lang::Fr, Family::Extinction) => "Extinctions",
            (Lang::Fr, Family::Innovation) => "Innovations",
            (Lang::Fr, Family::Catastrophe) => "Catastrophes",
            (Lang::Fr, Family::Milestone) => "Jalons",
            (Lang::Fr, Family::Intervention) => "Vos ordres",
            (Lang::Fr, Family::Geology) => "Géologie",
            (Lang::Fr, Family::Accelerator) => "Accélérateurs",
            (Lang::En, Family::Speciation) => "New lineages",
            (Lang::En, Family::Extinction) => "Extinctions",
            (Lang::En, Family::Innovation) => "Innovations",
            (Lang::En, Family::Catastrophe) => "Catastrophes",
            (Lang::En, Family::Milestone) => "Milestones",
            (Lang::En, Family::Intervention) => "Your orders",
            (Lang::En, Family::Geology) => "Geology",
            (Lang::En, Family::Accelerator) => "Accelerators",
        }
    }
}

/// Niveau de signalement.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    /// Chronique seulement.
    Routine,
    /// Carte d'alerte et repère.
    Notable,
    /// Bannière au centre.
    Major,
}

/// Seuil d'intérêt d'une alerte notable, selon la vitesse (années par
/// seconde) : 0,45 à 1 ka/s, 0,69 à 1 Ma/s.
pub fn notable_threshold(years_per_second: f64) -> f64 {
    (0.45 + 0.08 * (years_per_second.max(1.0) / 1e3).log10().max(0.0)).min(0.8)
}

pub const MAJOR_THRESHOLD: f64 = 0.85;

/// Événement de la chronique à partir de celui que publie le moteur (la
/// cellule y est une cellule du vivant, que le client emploie telle quelle
/// comme cellule physique : les grilles sont emboîtées).
pub fn from_view(v: &EventView) -> Event {
    Event { id: v.id, years: v.years, cell: v.cell, kind: v.kind.clone(), origin: v.origin, cause: v.cause, interest: v.interest as f64 }
}

pub fn level(e: &Event, years_per_second: f64) -> Level {
    // Les ordres du joueur sont notés, jamais signalés : il sait ce qu'il a
    // demandé. Les pas de vitesse et les pauses ne sont même pas notés.
    // Un refus, en revanche, doit se voir.
    match e.kind {
        EventKind::OrderApplied { .. } => return Level::Routine,
        EventKind::OrderRefused { .. } => return Level::Notable,
        _ => {}
    }
    if e.interest >= MAJOR_THRESHOLD {
        Level::Major
    } else if e.interest >= notable_threshold(years_per_second) {
        Level::Notable
    } else {
        Level::Routine
    }
}

/// Ce que le jeu fait d'un événement.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Action {
    Ignore,
    Note,
    Alert,
    SlowDown,
    Pause,
}

impl Action {
    pub fn from_index(i: i64) -> Action {
        match i {
            0 => Action::Ignore,
            1 => Action::Note,
            2 => Action::Alert,
            3 => Action::SlowDown,
            _ => Action::Pause,
        }
    }

    pub fn index(self) -> i64 {
        self as i64
    }
}

/// Règles d'arrêt : une action par famille, et un plancher par niveau (une
/// alerte majeure n'est jamais seulement notée).
#[derive(Clone, Debug, PartialEq)]
pub struct StopRules {
    pub actions: Vec<(Family, Action)>,
}

impl StopRules {
    /// Profils prêts à l'emploi : « contemplatif », « naturaliste », « tout voir ».
    pub fn profile(name: &str) -> StopRules {
        use Action::*;
        use Family::*;
        let actions = match name {
            // Rien n'arrête le temps (scénarios automatiques, mesures).
            "aucun" => Family::ALL.into_iter().map(|f| (f, Note)).collect(),
            "contemplatif" => vec![
                (Milestone, Pause),
                (Innovation, Alert),
                (Catastrophe, Alert),
                (Speciation, Note),
                (Extinction, Note),
                (Intervention, Note),
                (Geology, Note),
                (Accelerator, Note),
            ],
            "tout" => vec![
                (Milestone, Pause),
                (Innovation, Pause),
                (Catastrophe, Pause),
                (Speciation, Alert),
                (Extinction, Alert),
                (Intervention, Note),
                (Geology, Alert),
                (Accelerator, Alert),
            ],
            _ => vec![
                (Milestone, Pause),
                (Innovation, Pause),
                (Catastrophe, SlowDown),
                (Speciation, Note),
                (Extinction, Note),
                (Intervention, Note),
                (Geology, Note),
                (Accelerator, Note),
            ],
        };
        StopRules { actions }
    }

    pub fn action_for(&self, family: Family) -> Action {
        self.actions.iter().find(|a| a.0 == family).map_or(Action::Note, |a| a.1)
    }

    pub fn set(&mut self, family: Family, action: Action) {
        match self.actions.iter_mut().find(|a| a.0 == family) {
            Some(a) => a.1 = action,
            None => self.actions.push((family, action)),
        }
    }

    /// Décision pour un événement : l'action de sa famille, relevée pour les
    /// événements majeurs (au moins une alerte) et abaissée pour les
    /// événements courants (au plus une note).
    pub fn decide(&self, e: &Event, years_per_second: f64) -> Action {
        let a = self.action_for(Family::of(e));
        if a == Action::Ignore {
            return a;
        }
        match level(e, years_per_second) {
            Level::Major => a.max(Action::Alert),
            Level::Notable => a,
            Level::Routine => a.min(Action::Note),
        }
    }
}

/// Phrase de chronique d'un événement. `name` donne le nom courant d'une
/// lignée.
pub fn sentence(e: &Event, lang: Lang, name: &dyn Fn(u32) -> String) -> String {
    let when = duration(e.years, lang);
    let sep = if lang == Lang::Fr { " : " } else { ": " };
    format!("{when}{sep}{}", body(e, lang, name))
}

/// La phrase sans sa date, quand la date est affichée à part.
pub fn body(e: &Event, lang: Lang, name: &dyn Fn(u32) -> String) -> String {
    let s = match (&e.kind, lang) {
        (EventKind::LifeSeeded { lineage }, Lang::Fr) => {
            format!("Des cellules minimales sont déposées près des sources chaudes : la lignée {} commence.", name(*lineage))
        }
        (EventKind::LifeSeeded { lineage }, Lang::En) => {
            format!("Minimal cells are seeded near hot springs: the {} lineage begins.", name(*lineage))
        }
        (EventKind::NewLineage { lineage, parent, .. }, Lang::Fr) => {
            format!("Une nouvelle lignée, {}, se détache de {}.", name(*lineage), name(*parent))
        }
        (EventKind::NewLineage { lineage, parent, .. }, Lang::En) => {
            format!("A new lineage, {}, branches off from {}.", name(*lineage), name(*parent))
        }
        (EventKind::LineageExtinct { lineage }, Lang::Fr) => format!("La lignée {} s'éteint.", name(*lineage)),
        (EventKind::LineageExtinct { lineage }, Lang::En) => format!("The {} lineage dies out.", name(*lineage)),
        (EventKind::Innovation { lineage, label, .. }, Lang::Fr) => {
            format!("Première sur la planète : {} acquiert {}.", name(*lineage), label)
        }
        (EventKind::Innovation { lineage, label, .. }, Lang::En) => {
            format!("A first on the planet: {} acquires {}.", name(*lineage), label)
        }
        (EventKind::AcceleratorOn { pathway, .. }, Lang::Fr) => {
            format!("L'évolution stagnait sur le chemin de la {pathway} : un accélérateur discret est intervenu.")
        }
        (EventKind::AcceleratorOn { pathway, .. }, Lang::En) => {
            format!("Evolution had stalled on the way to {pathway}: a discreet accelerator stepped in.")
        }
        (EventKind::AcceleratorOff { pathway }, Lang::Fr) => format!("L'accélérateur de la {pathway} se retire."),
        (EventKind::AcceleratorOff { pathway }, Lang::En) => format!("The {pathway} accelerator withdraws."),
        (EventKind::OxygenThreshold { mixing_ratio, rising }, Lang::Fr) => format!(
            "L'oxygène de l'air {} le seuil de {} (fraction molaire).",
            if *rising { "dépasse" } else { "repasse sous" },
            power_of_ten_in(lang, *mixing_ratio)
        ),
        (EventKind::OxygenThreshold { mixing_ratio, rising }, Lang::En) => format!(
            "Atmospheric oxygen {} {} (mixing ratio).",
            if *rising { "rises above" } else { "falls back below" },
            power_of_ten_in(lang, *mixing_ratio)
        ),
        (EventKind::Snowball { starts: true, .. }, Lang::Fr) => {
            "La glace gagne les tropiques : la planète devient une boule de neige.".into()
        }
        (EventKind::Snowball { starts: true, .. }, Lang::En) => "Ice reaches the tropics: the planet turns into a snowball.".into(),
        (EventKind::Snowball { starts: false, .. }, Lang::Fr) => "La boule de neige fond : les mers se rouvrent.".into(),
        (EventKind::Snowball { starts: false, .. }, Lang::En) => "The snowball thaws: the seas open again.".into(),
        (EventKind::PlateReorganisation { plates }, Lang::Fr) => {
            format!("Les plaques se réorganisent : {plates} plaques, de nouveaux pôles de rotation.")
        }
        (EventKind::PlateReorganisation { plates }, Lang::En) => format!("The plates reorganise: {plates} plates, new rotation poles."),
        (EventKind::OrderApplied { label, .. }, Lang::Fr) => {
            if e.origin == Origin::Player {
                format!("Votre intervention s'applique : {label}.")
            } else {
                format!("Commande du temps : {label}.")
            }
        }
        (EventKind::OrderApplied { label, .. }, Lang::En) => {
            if e.origin == Origin::Player {
                format!("Your intervention takes effect: {label}.")
            } else {
                format!("Time command: {label}.")
            }
        }
        (EventKind::OrderRefused { reason, .. }, Lang::Fr) => format!("Intervention refusée : {reason}."),
        (EventKind::OrderRefused { reason, .. }, Lang::En) => format!("Intervention refused: {reason}."),
    };
    let accel = match (e.origin, lang) {
        (Origin::Accelerator, Lang::Fr) => " (un accélérateur a agi)",
        (Origin::Accelerator, Lang::En) => " (an accelerator acted)",
        _ => "",
    };
    format!("{s}{accel}")
}

/// Groupe d'événements semblables, pour ne pas noyer le joueur
/// (« 37 nouvelles lignées en 2 Ma »).
#[derive(Clone, Debug, PartialEq)]
pub struct Group {
    pub family: Family,
    pub first: u64,
    pub last: u64,
    pub count: usize,
    pub from_years: f64,
    pub to_years: f64,
}

/// Regroupe les événements courants consécutifs d'une même famille.
pub fn group_routine(events: &[Event], years_per_second: f64) -> Vec<Group> {
    let mut out: Vec<Group> = Vec::new();
    for e in events.iter().filter(|e| level(e, years_per_second) == Level::Routine) {
        let f = Family::of(e);
        match out.last_mut() {
            Some(g) if g.family == f => {
                g.last = e.id;
                g.count += 1;
                g.to_years = e.years;
            }
            _ => out.push(Group { family: f, first: e.id, last: e.id, count: 1, from_years: e.years, to_years: e.years }),
        }
    }
    out
}

pub fn group_sentence(g: &Group, lang: Lang) -> String {
    let span = duration(g.to_years - g.from_years, lang);
    let what = g.family.label(lang).to_lowercase();
    match lang {
        Lang::Fr => format!("{} : {} {} en {}", duration(g.to_years, lang), g.count, what, span),
        Lang::En => format!("{}: {} {} in {}", duration(g.to_years, lang), g.count, what, span),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use evo_core::events::EventLog;

    fn log() -> EventLog {
        let mut log = EventLog::default();
        log.push(1e5, Some(3), EventKind::LifeSeeded { lineage: 0 });
        for i in 1..5 {
            log.push(2e6 + i as f64, None, EventKind::NewLineage { lineage: i, parent: 0, signature: 1 });
        }
        log.push(2e7, None, EventKind::OxygenThreshold { mixing_ratio: 1e-4, rising: true });
        log
    }

    #[test]
    fn faster_time_raises_the_bar() {
        assert!(notable_threshold(1e6) > notable_threshold(1e4));
        let l = log();
        let new_lineage = &l.events[2];
        assert_eq!(level(new_lineage, 1e6), Level::Routine);
        assert_eq!(level(&l.events[5], 1e6), Level::Major);
    }

    #[test]
    fn rules_pause_on_milestones_and_group_the_rest() {
        let l = log();
        let rules = StopRules::profile("naturaliste");
        assert_eq!(rules.decide(&l.events[5], 1e6), Action::Pause);
        assert_eq!(rules.decide(&l.events[2], 1e6), Action::Note);
        let mut quiet = rules.clone();
        quiet.set(Family::Milestone, Action::Ignore);
        assert_eq!(quiet.decide(&l.events[5], 1e6), Action::Ignore);
        let groups = group_routine(&l.events, 1e6);
        assert_eq!(groups.iter().map(|g| g.count).sum::<usize>(), 4);
        assert!(group_sentence(&groups[0], Lang::Fr).contains("4 nouvelles lignées"));
    }

    #[test]
    fn sentences_read_well() {
        let l = log();
        let name = |id: u32| format!("L{id}");
        let s = sentence(&l.events[5], Lang::Fr, &name);
        assert!(s.starts_with("20,0 Ma : L'oxygène de l'air dépasse le seuil de 10⁻⁴"), "{s}");
        let s = sentence(&l.events[1], Lang::En, &name);
        assert!(s.contains("A new lineage, L1, branches off from L0."), "{s}");
    }
}
