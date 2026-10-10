//! Individus et comportements (étape 5) : les niveaux 4 et 5 de l'échelle
//! commune du document Vision.
//!
//! - [`sample`] : individus échantillonnés d'une population (génomes
//!   complets, reproduction réelle, développement), calibrés pour reproduire
//!   ses taux de naissance, de mort et de déplacement.
//! - [`agents`] : agents détaillés d'une scène locale, avec sens, système
//!   nerveux, locomotion et besoins, et une action courante du lexique
//!   commun.
//! - [`traits`] : ce que le phénotype et le plan de construction donnent au
//!   comportement.
//! - [`zones`] : où le moteur échantillonne (zones actives).
//!
//! Cette crate lit le monde et ne l'écrit jamais : `evo-sim` n'en dépend
//! pas. C'est la garantie, par construction, que regarder ne change pas
//! l'histoire ; les tests de rejeu le vérifient bit pour bit.

pub mod agents;
pub mod bench;
pub mod sample;
pub mod traits;
pub mod zones;

pub use agents::{Act, Agent, Scene};
pub use sample::{LifeEvent, LifeEventKind, Member, PopRates, Sample, SAMPLE_SIZE};
pub use traits::{Diet, Locomotion, Traits};
