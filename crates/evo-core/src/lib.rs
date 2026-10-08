//! Socle commun d'Evonisium.
//!
//! Ce crate ne contient que ce que tous les chantiers partagent : le hasard à
//! graine (un flux par système, pour le déterminisme), les horloges, le journal
//! des événements et le registre de flux qui vérifie la conservation de la
//! matière. Aucune constante terrestre n'y figure.

pub mod clock;
pub mod events;
pub mod flux;
pub mod rng;
pub mod units;

pub use clock::{MasterClock, Scheduler, SystemClock};
pub use events::{Event, EventKind, EventLog, Origin};
pub use flux::{Element, FluxRegistry, ELEMENTS};
pub use rng::{rng_for, SimRng, Stream};
