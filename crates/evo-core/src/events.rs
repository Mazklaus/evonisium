//! Journal des événements : la matière première de la chronique, de l'arbre du
//! vivant, des alertes et du détecteur d'événements. Les identifiants sont
//! stables et jamais réutilisés.
//!
//! Depuis l'étape 2, chaque événement porte son origine (moteur, joueur ou
//! accélérateur), un lien facultatif vers l'événement qui l'a causé et un
//! score d'intérêt (document « Fonctionnalités d'interface et confort »,
//! section « Alertes et chronique »). Le stockage est un fichier texte
//! tabulé ; SQLite viendra avec le client de l'étape 3.

use std::borrow::Cow;
use std::fmt::Write as _;

/// Qui est à l'origine d'un événement.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Origin {
    /// Les lois du moteur, sans aide.
    Engine,
    /// Un ordre du joueur (file d'ordres).
    Player,
    /// Un accélérateur de l'émergence assistée, signalé après coup.
    Accelerator,
}

impl Origin {
    pub fn label(self) -> &'static str {
        match self {
            Origin::Engine => "moteur",
            Origin::Player => "joueur",
            Origin::Accelerator => "accélérateur",
        }
    }
}

/// Ce qui s'est passé. Les substitutions, colonisations et extinctions
/// locales sont trop nombreuses pour être journalisées une à une : elles sont
/// comptées dans les statistiques du monde (et les substitutions fixées vont
/// au journal des modifications de génome).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum EventKind {
    /// Des cellules minimales ont été déposées.
    LifeSeeded {
        lineage: u32,
    },
    /// Un mutant au métabolisme nouveau pour la cellule fonde une lignée.
    NewLineage {
        lineage: u32,
        parent: u32,
        signature: u32,
    },
    /// Plus aucune population ne porte cette lignée.
    LineageExtinct {
        lineage: u32,
    },
    /// Première apparition sur la planète d'une étape d'une innovation à
    /// plusieurs pièces (par exemple le chemin vers la photosynthèse).
    Innovation {
        lineage: u32,
        pathway: Cow<'static, str>,
        stage: u8,
        label: Cow<'static, str>,
    },
    /// Le détecteur de stagnation active ou coupe un accélérateur.
    AcceleratorOn {
        pathway: Cow<'static, str>,
        stage: u8,
    },
    AcceleratorOff {
        pathway: Cow<'static, str>,
    },
    /// L'oxygène de l'atmosphère franchit un seuil (fraction molaire).
    OxygenThreshold {
        mixing_ratio: f64,
        rising: bool,
    },
    /// Glaciation globale (boule de neige) : début ou fin.
    Snowball {
        ice_fraction: f64,
        starts: bool,
    },
    /// Réorganisation des plaques (nouveaux pôles de rotation).
    PlateReorganisation {
        plates: u32,
    },
    /// Un ordre de la file a été appliqué.
    OrderApplied {
        order: u64,
        label: String,
    },
    /// Un ordre d'intervention a été refusé (réserve d'influence).
    OrderRefused {
        order: u64,
        reason: Cow<'static, str>,
    },
    /// Extinction de masse (étape 5) : le nombre de lignées vivantes perd
    /// au moins 40 % de son maximum récent en peu de temps.
    MassExtinction {
        lost_share: f64,
        lineages_before: u32,
    },
    /// Radiation (étape 5) : le nombre de lignées vivantes double au moins
    /// en peu de temps.
    Radiation {
        gain: f64,
        lineages: u32,
    },
    /// Mode historique (étape 6) : une espèce accumule ses savoirs d'une
    /// génération à l'autre, et le pas devient d'un an ; ou il prend fin,
    /// faute de groupes culturels.
    HistoricalMode {
        species: u32,
        starts: bool,
    },
    /// Un peuple fait un mythe d'un phénomène qu'il a vu (étape 6).
    Myth {
        myth: u32,
        people: u32,
        intervention: bool,
    },
}

impl EventKind {
    /// Nom court du type, pour les filtres de la chronique.
    pub fn type_name(&self) -> &'static str {
        match self {
            EventKind::LifeSeeded { .. } => "vie déposée",
            EventKind::NewLineage { .. } => "nouvelle lignée",
            EventKind::LineageExtinct { .. } => "extinction de lignée",
            EventKind::Innovation { .. } => "innovation",
            EventKind::AcceleratorOn { .. } => "accélérateur activé",
            EventKind::AcceleratorOff { .. } => "accélérateur coupé",
            EventKind::OxygenThreshold { .. } => "seuil d'oxygène",
            EventKind::Snowball { .. } => "glaciation globale",
            EventKind::PlateReorganisation { .. } => "réorganisation des plaques",
            EventKind::OrderApplied { .. } => "ordre appliqué",
            EventKind::OrderRefused { .. } => "ordre refusé",
            EventKind::MassExtinction { .. } => "extinction de masse",
            EventKind::Radiation { .. } => "radiation",
            EventKind::HistoricalMode { .. } => "mode historique",
            EventKind::Myth { .. } => "mythe",
        }
    }

    /// Libellé en français, pour la chronique et les alertes.
    pub fn describe(&self) -> String {
        match self {
            EventKind::LifeSeeded { .. } => "Des cellules minimales sont déposées dans les eaux.".into(),
            EventKind::NewLineage { lineage, parent, .. } => {
                format!("La lignée {lineage} naît de la lignée {parent} avec un métabolisme nouveau.")
            }
            EventKind::LineageExtinct { lineage } => format!("La lignée {lineage} s'éteint."),
            EventKind::Innovation { lineage, label, .. } => format!("Première apparition : {label} (lignée {lineage})."),
            EventKind::AcceleratorOn { stage, .. } => {
                format!("L'évolution stagne à l'étape {stage} du chemin vers la photosynthèse : l'émergence assistée intervient.")
            }
            EventKind::AcceleratorOff { .. } => "L'émergence assistée s'arrête : l'étape suivante est franchie.".into(),
            EventKind::OxygenThreshold { mixing_ratio, rising: true } => format!("L'oxygène de l'air dépasse {mixing_ratio:.0e}."),
            EventKind::OxygenThreshold { mixing_ratio, rising: false } => format!("L'oxygène de l'air retombe sous {mixing_ratio:.0e}."),
            EventKind::Snowball { starts: true, .. } => "La planète entre en glaciation globale.".into(),
            EventKind::Snowball { starts: false, .. } => "La glaciation globale prend fin.".into(),
            EventKind::PlateReorganisation { plates } => format!("Les plaques se réorganisent ({plates} plaques)."),
            EventKind::OrderApplied { label, .. } => format!("Ordre appliqué : {label}."),
            EventKind::OrderRefused { reason, .. } => format!("Ordre refusé : {reason}."),
            EventKind::MassExtinction { lost_share, .. } => {
                format!("Extinction de masse : {:.0} % des lignées disparaissent.", 100.0 * lost_share)
            }
            EventKind::Radiation { gain, .. } => format!("Radiation : le nombre de lignées est multiplié par {gain:.1}."),
            EventKind::HistoricalMode { starts: true, .. } => {
                "Une espèce accumule ses savoirs de génération en génération : le temps passe désormais année par année.".into()
            }
            EventKind::HistoricalMode { starts: false, .. } => {
                "Plus aucune espèce ne transmet ses savoirs : le temps reprend son cours.".into()
            }
            EventKind::Myth { intervention: true, .. } => "Un peuple fait un mythe de votre intervention.".into(),
            EventKind::Myth { intervention: false, .. } => "Un peuple fait un mythe d'un phénomène qu'il ne s'explique pas.".into(),
        }
    }

    /// Ampleur et rareté propres au type, de 0 à 1 (première composante du
    /// score d'intérêt).
    pub fn base_interest(&self) -> f64 {
        match self {
            EventKind::LifeSeeded { .. } => 0.9,
            EventKind::NewLineage { .. } => 0.15,
            EventKind::LineageExtinct { .. } => 0.1,
            EventKind::Innovation { stage, .. } => 0.5 + 0.1 * (*stage as f64).min(5.0),
            EventKind::AcceleratorOn { .. } | EventKind::AcceleratorOff { .. } => 0.4,
            EventKind::OxygenThreshold { .. } => 0.9,
            EventKind::Snowball { .. } => 0.85,
            EventKind::PlateReorganisation { .. } => 0.3,
            EventKind::OrderApplied { .. } => 0.5,
            EventKind::OrderRefused { .. } => 0.3,
            EventKind::MassExtinction { .. } => 0.9,
            EventKind::Radiation { .. } => 0.6,
            EventKind::HistoricalMode { .. } => 0.95,
            EventKind::Myth { .. } => 0.5,
        }
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Event {
    pub id: u64,
    pub years: f64,
    pub cell: Option<u32>,
    pub kind: EventKind,
    pub origin: Origin,
    /// Événement qui a causé celui-ci, s'il est connu.
    pub cause: Option<u64>,
    /// Score d'intérêt, de 0 à 1 : ampleur et rareté du type, majoré pour
    /// une première fois dans la partie.
    pub interest: f64,
}

/// Journal en mémoire, dans l'ordre déterministe d'application.
///
/// [Simplification] Les naissances et extinctions de lignées se comptent par
/// dizaines de millions dans une partie : passé `MINOR_EVENTS_KEPT_YEARS`,
/// elles sortent du journal en mémoire (la base de la partie les garde). Les
/// identifiants restent uniques et croissants, le journal devient creux.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct EventLog {
    /// Événements gardés, par identifiant croissant.
    pub events: Vec<Event>,
    /// Types déjà vus, pour le bonus de nouveauté.
    seen: Vec<Cow<'static, str>>,
    /// Identifiant du prochain événement : nombre d'événements inscrits.
    next_id: u64,
    /// Date du dernier élagage.
    pruned_years: f64,
}

/// Durée pendant laquelle une naissance ou une extinction de lignée reste au
/// journal en mémoire, en années.
pub const MINOR_EVENTS_KEPT_YEARS: f64 = 50e6;

/// Intervalle entre deux élagages du journal, en années.
pub const PRUNE_EVENTS_EVERY_YEARS: f64 = 10e6;

impl EventLog {
    /// Inscrit un événement du moteur, sans cause liée.
    pub fn push(&mut self, years: f64, cell: Option<u32>, kind: EventKind) -> u64 {
        self.push_with(years, cell, kind, Origin::Engine, None)
    }

    /// Inscrit un événement avec son origine et sa cause ; renvoie son
    /// identifiant.
    pub fn push_with(&mut self, years: f64, cell: Option<u32>, kind: EventKind, origin: Origin, cause: Option<u64>) -> u64 {
        let id = self.total();
        self.next_id = id + 1;
        let name = kind.type_name();
        let novelty = if self.seen.iter().any(|s| s == name) {
            0.0
        } else {
            self.seen.push(Cow::Borrowed(name));
            0.25
        };
        let interest = (kind.base_interest() + novelty).min(1.0);
        self.events.push(Event { id, years, cell, kind, origin, cause, interest });
        id
    }

    /// Nombre d'événements inscrits depuis le début de la partie : le
    /// curseur des lecteurs du journal.
    pub fn total(&self) -> u64 {
        self.next_id.max(self.events.last().map_or(0, |e| e.id + 1))
    }

    /// Événements gardés à partir de l'identifiant `id`.
    pub fn since(&self, id: u64) -> &[Event] {
        &self.events[self.events.partition_point(|e| e.id < id)..]
    }

    pub fn get(&self, id: u64) -> Option<&Event> {
        self.events.binary_search_by_key(&id, |e| e.id).ok().map(|i| &self.events[i])
    }

    /// Retire les naissances et extinctions de lignées plus vieilles que
    /// `MINOR_EVENTS_KEPT_YEARS`, au plus une fois par
    /// `PRUNE_EVENTS_EVERY_YEARS` ; renvoie le nombre d'événements retirés.
    pub fn prune(&mut self, years: f64) -> usize {
        if years < self.pruned_years + PRUNE_EVENTS_EVERY_YEARS {
            return 0;
        }
        self.pruned_years = years;
        self.next_id = self.total();
        let limit = years - MINOR_EVENTS_KEPT_YEARS;
        let before = self.events.len();
        self.events.retain(|e| e.years >= limit || !matches!(e.kind, EventKind::NewLineage { .. } | EventKind::LineageExtinct { .. }));
        let removed = before - self.events.len();
        if removed > 0 {
            self.events.shrink_to(self.events.len() + self.events.len() / 4);
        }
        removed
    }

    pub fn count(&self, pred: impl Fn(&EventKind) -> bool) -> usize {
        self.events.iter().filter(|e| pred(&e.kind)).count()
    }

    /// Export tabulé (une ligne par événement) : identifiant, date en années,
    /// cellule, type, origine, cause, intérêt, détail.
    pub fn to_tsv(&self) -> String {
        let mut out = String::from("id\tannees\tcellule\ttype\torigine\tcause\tinteret\tdetail\n");
        for e in &self.events {
            let cell = e.cell.map(|c| c.to_string()).unwrap_or_default();
            let cause = e.cause.map(|c| c.to_string()).unwrap_or_default();
            let _ = writeln!(
                out,
                "{}\t{:.0}\t{}\t{}\t{}\t{}\t{:.2}\t{:?}",
                e.id,
                e.years,
                cell,
                e.kind.type_name(),
                e.origin.label(),
                cause,
                e.interest,
                e.kind
            );
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_of_a_kind_is_more_interesting_and_ids_are_stable() {
        let mut log = EventLog::default();
        let a = log.push(0.0, None, EventKind::LineageExtinct { lineage: 1 });
        let b = log.push_with(1.0, Some(3), EventKind::LineageExtinct { lineage: 2 }, Origin::Accelerator, Some(a));
        assert_eq!((a, b), (0, 1));
        assert!(log.events[0].interest > log.events[1].interest);
        assert_eq!(log.events[1].cause, Some(0));
        assert!(log.to_tsv().contains("accélérateur"));
    }

    #[test]
    fn pruning_keeps_ids_and_notable_events() {
        let mut log = EventLog::default();
        log.push(1e6, None, EventKind::NewLineage { lineage: 1, parent: 0, signature: 1 });
        log.push(2e6, None, EventKind::Snowball { ice_fraction: 1.0, starts: true });
        log.push(90e6, None, EventKind::LineageExtinct { lineage: 1 });
        assert_eq!(log.prune(100e6), 1);
        assert_eq!(log.events.iter().map(|e| e.id).collect::<Vec<_>>(), vec![1, 2]);
        assert_eq!(log.since(2).len(), 1);
        assert_eq!(log.push(101e6, None, EventKind::Snowball { ice_fraction: 1.0, starts: true }), 3);
        assert_eq!(log.total(), 4);
    }
}
