//! Pont entre le moteur Rust et le client Godot 4 (GDExtension, crate
//! godot-rust). Le client GDScript ne voit qu'une classe, `EvoSession`.
//!
//! Le moteur tourne sur son propre fil ([`runner`]) ; la logique d'affichage
//! testable sans fenêtre vit dans `evo-view` et l'apparence du vivant dans
//! `evo-morph`. Ce crate ne fait que traduire.

use godot::prelude::*;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

pub mod jobs;
pub mod runner;
pub mod session;

struct Evonisium;

#[gdextension]
unsafe impl ExtensionLibrary for Evonisium {}
