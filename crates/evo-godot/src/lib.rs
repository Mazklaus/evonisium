//! Pont entre le moteur Rust et le client Godot 4 (GDExtension, crate
//! godot-rust). Le client GDScript ne voit qu'une classe, `EvoSession`.
//!
//! Le moteur (`evo-engine`) tourne sur son propre fil ; [`runner`] le tient ; la logique d'affichage
//! testable sans fenêtre vit dans `evo-view` et l'apparence du vivant dans
//! `evo-morph`. Ce crate ne fait que traduire.

// Affichage seulement : rien de ce crate n'entre dans l'état simulé, les
// mathématiques de la plateforme y suffisent (voir clippy.toml).
#![allow(clippy::disallowed_methods)]

use godot::prelude::*;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

pub mod jobs;
pub mod runner;
pub mod session;

struct Evonisium;

#[gdextension]
unsafe impl ExtensionLibrary for Evonisium {}
