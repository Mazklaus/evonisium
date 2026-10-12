//! Cognition et cultures (étape 6) : ce que les esprits apprennent les uns
//! des autres, et ce qui en reste dans l'histoire.
//!
//! - [`cognition`] : les quatre lectures cognitives du corps et les
//!   grandeurs de l'apprentissage qu'on en tire (mode, fidélité, contacts,
//!   gain culturel de Henrich) ;
//! - [`catalog`] : les techniques de l'étape 6 et leurs voies de
//!   remplacement ;
//! - [`traits`] : registre des traits, contenus générés et effets bornés ;
//! - [`language`] : lexiques, phonologies, noms ;
//! - [`myths`] : mythes nés des phénomènes inexpliqués ;
//! - [`world`] : le modèle agrégé des groupes culturels, à l'équilibre ou pas
//!   à pas, avec le critère de bascule vers le mode historique ;
//! - [`labels`] : peuples et langues, étiquettes émergentes ;
//! - [`state`] : l'état culturel publié pour le client et le son ;
//! - [`bench`] : mondes de banc d'essai.
//!
//! La crate ne dépend d'aucun système du monde : le moteur lui donnera les
//! populations, les liens entre cellules et les lectures cognitives, et
//! lira ses effets (intégration avec le fil « Animaux et écosystèmes »).

pub mod bench;
pub mod catalog;
pub mod cognition;
pub mod labels;
pub mod language;
pub mod myths;
pub mod state;
pub mod traits;
pub mod world;

pub use catalog::{Context, Family, Milieu, Route};
pub use cognition::{Capacity, Channels, Cognition, Learning, Manipulation, Mode, Organ};
pub use myths::Phenomenon;
pub use state::CulturalState;
pub use world::CultureWorld;
