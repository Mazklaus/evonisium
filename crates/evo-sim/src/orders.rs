//! File d'ordres (document « Interface du moteur », canal joueur → moteur).
//!
//! Toute action extérieure à la simulation passe par ici : interventions du
//! joueur sur l'environnement et commandes du temps (la vitesse fixe la durée
//! du pas, donc l'histoire). Chaque ordre porte une date de jeu ; il est
//! appliqué au début du premier pas qui commence à cette date ou après, entre
//! deux pas, jamais pendant. Les ordres appliqués sont notés avec le pas et la
//! date réels : la graine et ce registre suffisent à rejouer une partie à
//! l'identique.
//!
//! [Simplification] Le canal d'observation (zone d'intérêt de la caméra)
//! arrive avec le client à l'étape 3 ; il ne change jamais l'histoire.

use evo_planet::Gas;

/// Ce que demande un ordre.
#[derive(Clone, Debug, PartialEq)]
pub enum OrderKind {
    /// Change la durée du pas planétaire, années (commande de vitesse).
    SetStepYears(f64),
    /// Met la simulation en pause ou la relance. Une pause n'arrête que
    /// l'avance du temps : les ordres dus continuent d'être appliqués.
    Pause,
    Resume,
    /// Dépose des cellules minimales près des sources hydrothermales.
    SeedLife,
    /// Apporte du phosphate à l'océan profond (intervention du joueur :
    /// altération accrue d'un massif, par exemple), mol.
    AddPhosphate {
        moles: f64,
    },
    /// Injecte un gaz dans l'atmosphère (éruption provoquée), mol.
    InjectGas {
        gas: Gas,
        moles: f64,
    },
    /// Marque une lignée pour la suivre (sans effet sur l'histoire).
    MarkLineage {
        lineage: u32,
    },
}

impl OrderKind {
    pub fn label(&self) -> String {
        match self {
            OrderKind::SetStepYears(y) => format!("pas de {y} ans"),
            OrderKind::Pause => "pause".into(),
            OrderKind::Resume => "reprise".into(),
            OrderKind::SeedLife => "dépôt de cellules minimales".into(),
            OrderKind::AddPhosphate { moles } => format!("apport de {moles:.2e} mol de phosphate"),
            OrderKind::InjectGas { gas, moles } => format!("injection de {moles:.2e} mol de {}", gas.label()),
            OrderKind::MarkLineage { lineage } => format!("suivi de la lignée {lineage}"),
        }
    }

    /// Les commandes du temps et le suivi ne sont pas des interventions sur
    /// le monde.
    pub fn is_intervention(&self) -> bool {
        matches!(self, OrderKind::SeedLife | OrderKind::AddPhosphate { .. } | OrderKind::InjectGas { .. })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Order {
    pub id: u64,
    /// Date de jeu demandée, années.
    pub due_years: f64,
    pub kind: OrderKind,
}

/// Ordre appliqué : date et pas réels.
#[derive(Clone, Debug, PartialEq)]
pub struct AppliedOrder {
    pub order: Order,
    pub step: u64,
    pub years: f64,
    /// Événement inscrit au journal pour cet ordre.
    pub event: u64,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct OrderQueue {
    /// Ordres en attente, triés par date puis par identifiant.
    pending: Vec<Order>,
    pub applied: Vec<AppliedOrder>,
    next_id: u64,
}

impl OrderQueue {
    /// Ajoute un ordre et renvoie son identifiant.
    pub fn submit(&mut self, due_years: f64, kind: OrderKind) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.insert(Order { id, due_years, kind });
        id
    }

    fn insert(&mut self, order: Order) {
        let at = self.pending.partition_point(|o| (o.due_years, o.id) <= (order.due_years, order.id));
        self.pending.insert(at, order);
    }

    /// Retire et renvoie les ordres dus à la date `years`, dans l'ordre.
    pub fn take_due(&mut self, years: f64) -> Vec<Order> {
        let n = self.pending.partition_point(|o| o.due_years <= years);
        self.pending.drain(..n).collect()
    }

    pub fn pending(&self) -> &[Order] {
        &self.pending
    }

    /// Tous les ordres reçus (appliqués et en attente), pour rejouer la
    /// partie depuis sa graine.
    pub fn log(&self) -> Vec<Order> {
        let mut all: Vec<Order> = self.applied.iter().map(|a| a.order.clone()).collect();
        all.extend(self.pending.iter().cloned());
        all.sort_by_key(|a| a.id);
        all
    }

    /// File reconstruite à partir d'un registre (rejeu).
    pub fn from_log(orders: &[Order]) -> Self {
        let mut q = Self::default();
        for o in orders {
            q.next_id = q.next_id.max(o.id + 1);
            q.insert(o.clone());
        }
        q
    }

    /// Registre lisible des ordres appliqués.
    pub fn to_tsv(&self) -> String {
        let mut out = String::from("id\tdate_demandee\tpas\tdate_appliquee\tordre\n");
        for a in &self.applied {
            out.push_str(&format!("{}\t{}\t{}\t{}\t{}\n", a.order.id, a.order.due_years, a.step, a.years, a.order.kind.label()));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orders_come_out_by_date_then_id() {
        let mut q = OrderQueue::default();
        let a = q.submit(500.0, OrderKind::Pause);
        let b = q.submit(100.0, OrderKind::SeedLife);
        let c = q.submit(100.0, OrderKind::Resume);
        assert!(q.take_due(50.0).is_empty());
        let due: Vec<u64> = q.take_due(100.0).iter().map(|o| o.id).collect();
        assert_eq!(due, vec![b, c]);
        assert_eq!(q.take_due(1e9)[0].id, a);
    }

    #[test]
    fn log_round_trips() {
        let mut q = OrderQueue::default();
        q.submit(10.0, OrderKind::SetStepYears(5000.0));
        q.submit(0.0, OrderKind::AddPhosphate { moles: 1e12 });
        let r = OrderQueue::from_log(&q.log());
        assert_eq!(r.pending(), q.pending());
        let mut r2 = r.clone();
        assert_eq!(r2.submit(1.0, OrderKind::Pause), 2);
    }
}
