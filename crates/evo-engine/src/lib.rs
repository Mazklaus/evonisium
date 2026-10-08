//! Le moteur d'Evonisium vu du client (étape 3).
//!
//! Le client (crate `evo-godot`) ne touche jamais au monde simulé. Il parle au
//! moteur par deux canaux et lit ce que le moteur publie :
//!
//! - la **file d'ordres** ([`Engine::submit`]) porte tout ce qui change
//!   l'histoire : commandes du temps et interventions. Chaque ordre est daté
//!   et inscrit au registre ; la graine et ce registre redonnent la partie ;
//! - le **canal d'observation** ([`Engine::set_interest`]) porte la zone
//!   d'intérêt de la caméra. Il ne change jamais l'histoire : il décide
//!   seulement de ce que le moteur détaille dans l'état publié ;
//! - l'**état publié** ([`Engine::frame`]) est la photographie immuable et
//!   datée du monde à la fin de chaque pas, avec la précédente pour
//!   interpoler. La lire coûte le clonage de deux `Arc` ;
//! - les **requêtes** ([`Engine::query`]) répondent de façon asynchrone :
//!   événements, historiques, milieu type d'une espèce ;
//! - les **points de sauvegarde** ([`Engine::save`], [`Engine::load`])
//!   écrivent l'état complet sur disque ; une partie reprise continue
//!   exactement comme si elle n'avait pas été interrompue.
//!
//! La simulation tourne dans son propre fil, avec son propre groupe de fils
//! de calcul (6 par défaut : le document Vision en réserve 2 sur 8 à
//! l'affichage).

mod engine;
mod geometry;
mod query;

pub use engine::{Engine, EngineStatus, Frame, NewGame, When};
pub use geometry::GridGeometry;
pub use query::{Answer, Query};

pub use evo_planet::Gas;
pub use evo_sim::history::{CellView, ClimateMode, PublishedState, Sample};
pub use evo_sim::observation::InterestZone;
pub use evo_sim::orders::{Intervention, OrderKind};
