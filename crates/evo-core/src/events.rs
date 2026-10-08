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
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct EventLog {
    pub events: Vec<Event>,
    /// Types déjà vus, pour le bonus de nouveauté.
    seen: Vec<Cow<'static, str>>,
}

impl EventLog {
    /// Inscrit un événement du moteur, sans cause liée.
    pub fn push(&mut self, years: f64, cell: Option<u32>, kind: EventKind) -> u64 {
        self.push_with(years, cell, kind, Origin::Engine, None)
    }

    /// Inscrit un événement avec son origine et sa cause ; renvoie son
    /// identifiant.
    pub fn push_with(&mut self, years: f64, cell: Option<u32>, kind: EventKind, origin: Origin, cause: Option<u64>) -> u64 {
        let id = self.events.len() as u64;
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
}
