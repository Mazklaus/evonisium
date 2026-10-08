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
//! Depuis l'étape 3, les interventions coûtent de l'influence (décision du
//! 8 octobre 2026) : le moteur valide chaque ordre contre la réserve au
//! moment de l'appliquer, de façon déterministe. Seule la demande est
//! inscrite au registre ; au rejeu, la même validation redonne le même
//! verdict. Le canal d'observation (zone d'intérêt de la caméra, module
//! `observation`) ne passe pas par ici : il ne change jamais l'histoire.

use evo_planet::Gas;

/// Intervention du joueur sur l'environnement (jamais sur les gènes).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Intervention {
    /// Apport de nutriments : du phosphate venu de l'extérieur du système
    /// suivi (altération accrue d'un massif, cendres volcaniques, poussières)
    /// est versé dans les eaux de surface autour d'une cellule physique.
    /// Sans eau de surface dans le rayon, il rejoint l'océan profond.
    Fertilize { cell: u32, radius_km: f64, moles_p: f64 },
    /// Éruption provoquée : un gaz injecté dans l'atmosphère (CO₂ pour
    /// réchauffer, H₂ ou CH₄ pour réduire, O₂ n'est pas permis). La cellule
    /// situe l'événement dans la chronique.
    Eruption { cell: u32, gas: Gas, moles: f64 },
    /// Impact météoritique d'un bolide de `diameter_km` (étape 4) : même
    /// module que les impacts naturels (voir `disturbance`).
    Impact { cell: u32, diameter_km: f64 },
    /// Isoler un groupe : bras de mer (`sea`) ou chaîne de montagnes
    /// locale, arc de `length_km` centré sur la cellule et orienté selon
    /// `azimuth_deg` (0 : nord-sud), que les migrations ne franchissent pas
    /// pendant `duration_years`.
    Isolate { cell: u32, azimuth_deg: f64, length_km: f64, duration_years: f64, sea: bool },
    /// Poussée climatique régionale : écart de température (K) et facteur
    /// de pluie pendant `duration_years`, dans un rayon autour de la cellule.
    ClimatePulse { cell: u32, radius_km: f64, delta_k: f64, rain_factor: f64, duration_years: f64 },
}

impl Intervention {
    /// Coût en points d'influence. Un apport de 10¹² mol de phosphate (la
    /// moitié de ce que l'altération de la Terre actuelle apporte en un
    /// siècle) coûte 10 points ; une éruption de 10¹⁶ mol de gaz (un grand
    /// épanchement basaltique) en coûte 20. Le coût croît comme la racine de
    /// la quantité, borné.
    pub fn cost(&self) -> f64 {
        match self {
            Intervention::Fertilize { moles_p, .. } => (10.0 * (moles_p.max(0.0) / 1e12).sqrt()).clamp(2.0, 60.0),
            Intervention::Eruption { moles, .. } => (20.0 * (moles.abs() / 1e16).sqrt()).clamp(2.0, 60.0),
            // Un bolide de 10 km (Chicxulub) coûte 50 points, un de 1 km 16.
            Intervention::Impact { diameter_km, .. } => (50.0 * (diameter_km.max(0.0) / 10.0).sqrt()).clamp(5.0, 90.0),
            // Une barrière de 1 000 km pendant 1 Ma coûte 20 points.
            Intervention::Isolate { length_km, duration_years, .. } => {
                (10.0 + 10.0 * length_km.max(0.0) / 1000.0 * (duration_years.max(0.0) / 1e6).sqrt()).clamp(5.0, 60.0)
            }
            // ±5 K sur 1 000 km pendant 100 ka : 20 points.
            Intervention::ClimatePulse { radius_km, delta_k, rain_factor, duration_years, .. } => {
                let strength = delta_k.abs() + 10.0 * (rain_factor.max(0.01) - 1.0).abs();
                (4.0 * strength * (duration_years.max(0.0) / 1e5).sqrt() * (radius_km.max(0.0) / 1000.0).sqrt()).clamp(2.0, 60.0)
            }
        }
    }

    pub fn label(&self) -> String {
        match self {
            Intervention::Fertilize { cell, radius_km, moles_p } => {
                format!("apport de {moles_p:.2e} mol de phosphate dans un rayon de {radius_km:.0} km autour de la cellule {cell}")
            }
            Intervention::Eruption { cell, gas, moles } => format!("éruption de {moles:.2e} mol de {} à la cellule {cell}", gas.label()),
            Intervention::Impact { cell, diameter_km } => format!("impact d'un bolide de {diameter_km:.1} km à la cellule {cell}"),
            Intervention::Isolate { cell, length_km, duration_years, sea, .. } => format!(
                "{} de {length_km:.0} km pendant {:.2} Ma à la cellule {cell}",
                if *sea { "bras de mer" } else { "chaîne de montagnes" },
                duration_years / 1e6
            ),
            Intervention::ClimatePulse { cell, radius_km, delta_k, rain_factor, duration_years } => format!(
                "poussée climatique de {delta_k:+.1} K, pluie ×{rain_factor:.2}, sur {radius_km:.0} km pendant {:.0} ka à la cellule {cell}",
                duration_years / 1e3
            ),
        }
    }

    pub fn cell(&self) -> u32 {
        match self {
            Intervention::Fertilize { cell, .. }
            | Intervention::Eruption { cell, .. }
            | Intervention::Impact { cell, .. }
            | Intervention::Isolate { cell, .. }
            | Intervention::ClimatePulse { cell, .. } => *cell,
        }
    }
}

/// Ce que demande un ordre.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum OrderKind {
    /// Change la durée du pas planétaire, années (commande de vitesse).
    SetStepYears(f64),
    /// Met la simulation en pause ou la relance. Une pause n'arrête que
    /// l'avance du temps : les ordres dus continuent d'être appliqués.
    Pause,
    Resume,
    /// Dépose des cellules minimales près des sources hydrothermales.
    SeedLife,
    /// Intervention sur l'environnement, payée en influence.
    Intervene(Intervention),
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
            OrderKind::Intervene(i) => i.label(),
            OrderKind::MarkLineage { lineage } => format!("suivi de la lignée {lineage}"),
        }
    }

    /// Les commandes du temps et le suivi ne sont pas des interventions sur
    /// le monde.
    pub fn is_intervention(&self) -> bool {
        matches!(self, OrderKind::SeedLife | OrderKind::Intervene(_))
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Order {
    pub id: u64,
    /// Date de jeu demandée, années.
    pub due_years: f64,
    pub kind: OrderKind,
}

/// Ordre appliqué : date et pas réels.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AppliedOrder {
    pub order: Order,
    pub step: u64,
    pub years: f64,
    /// Événement inscrit au journal pour cet ordre.
    pub event: u64,
}

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
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
        self.submit_with_id(id, due_years, kind);
        id
    }

    /// Ajoute un ordre dont l'identifiant a été attribué par l'appelant (le
    /// client numérote ses ordres sans attendre le fil de simulation).
    pub fn submit_with_id(&mut self, id: u64, due_years: f64, kind: OrderKind) {
        self.next_id = self.next_id.max(id + 1);
        self.insert(Order { id, due_years, kind });
    }

    /// Prochain identifiant libre.
    pub fn next_id(&self) -> u64 {
        self.next_id
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
        q.submit(0.0, OrderKind::Intervene(Intervention::Fertilize { cell: 3, radius_km: 500.0, moles_p: 1e12 }));
        let r = OrderQueue::from_log(&q.log());
        assert_eq!(r.pending(), q.pending());
        let mut r2 = r.clone();
        assert_eq!(r2.submit(1.0, OrderKind::Pause), 2);
    }
}
