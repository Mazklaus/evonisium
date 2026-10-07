//! Journal des événements : la matière première de la chronique, de l'arbre du
//! vivant et du détecteur d'événements. Les identifiants sont stables et jamais
//! réutilisés.

/// Ce qui s'est passé. Les substitutions, colonisations et extinctions
/// locales sont trop nombreuses pour être journalisées une à une : elles sont
/// comptées dans les statistiques du monde.
#[derive(Clone, Debug, PartialEq)]
pub enum EventKind {
    /// Des cellules minimales ont été déposées.
    LifeSeeded { lineage: u32 },
    /// Un mutant au métabolisme nouveau pour la cellule fonde une lignée.
    NewLineage { lineage: u32, parent: u32, signature: u32 },
    /// Plus aucune population ne porte cette lignée.
    LineageExtinct { lineage: u32 },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Event {
    pub years: f64,
    pub cell: Option<u32>,
    pub kind: EventKind,
}

/// Journal en mémoire, dans l'ordre déterministe d'application.
#[derive(Clone, Debug, Default)]
pub struct EventLog {
    pub events: Vec<Event>,
}

impl EventLog {
    pub fn push(&mut self, years: f64, cell: Option<u32>, kind: EventKind) {
        self.events.push(Event { years, cell, kind });
    }

    pub fn count(&self, pred: impl Fn(&EventKind) -> bool) -> usize {
        self.events.iter().filter(|e| pred(&e.kind)).count()
    }
}
