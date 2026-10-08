//! Logique d'affichage du client Evonisium, indépendante de Godot (le crate
//! `evo-godot` n'en est que le pont). Tout ce qui se teste sans fenêtre vit
//! ici : maillage du globe, textures de données et calques, palettes,
//! sélection, mise en forme, noms, chronique et règles d'arrêt, arbre du
//! vivant, décor de milieu, sauvegardes.
//!
//! Le client lit l'état publié par le moteur et n'écrit que par deux canaux :
//! la file d'ordres et le canal d'observation (architecture, « Deux canaux
//! du client vers le moteur »).

pub mod chronicle;
pub mod decor;
pub mod format;
pub mod frame;
pub mod layers;
pub mod mesh;
pub mod naming;
pub mod palette;
pub mod pick;
pub mod save;
pub mod species;
pub mod tree;

pub use format::Lang;
pub use frame::{Frame, LineageFrame, PlanetInfo};
