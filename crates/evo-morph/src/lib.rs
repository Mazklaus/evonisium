//! Générateur d'apparence du vivant (document « Rendu du vivant »), pur et
//! déterministe : il transforme ce que le moteur sait d'un organisme en
//! dessin, Godot ne fait qu'afficher.
//!
//! Paliers (architecture, « Apparence du vivant ») : 0 banc d'essai, 1 vivant
//! microbien et vue microscope (étape 3, ce crate), 2 corps complets
//! (étapes 4-5), 3 objets et villes (étapes 6-7). Le style est celui de
//! l'Atlas naturaliste retenu : plume, lavis, hachures, barre d'échelle.
//! La DA est une peau posée sur la forme : la forme et le sens des couleurs
//! viennent des traits, jamais d'un choix artistique.

// Affichage seulement : rien de ce crate n'entre dans l'état simulé, les
// mathématiques de la plateforme y suffisent (voir clippy.toml).
#![allow(clippy::disallowed_methods)]

pub mod canvas;
pub mod microbe;

pub use canvas::Canvas;
pub use microbe::{figure, form, microscope_field, Member, MicrobeForm, MicrobeTraits, Shape};
