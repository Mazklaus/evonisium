//! Mondes de banc d'essai : des grilles de cellules reliées, peuplées d'une
//! espèce sociale, pour les tests et les mesures hors partie.

use crate::catalog::{Context, Family};
use crate::cognition::Cognition;
use crate::world::CultureWorld;

/// Grille de `w × h` cellules à partir de `first`, chacune reliée à six
/// voisines (droite, bas, diagonale, et leurs symétriques) avec le poids
/// `link`. Le premier groupe fonde le peuple ; les autres en descendent, de
/// proche en proche. Renvoie les cellules.
#[allow(clippy::too_many_arguments)]
pub fn grid(world: &mut CultureWorld, species: u32, cognition: Cognition, first: u32, w: u32, h: u32, population: f64, link: f64, ctx: Context) -> Vec<u32> {
    if world.species(species).is_none() {
        world.add_species(species, cognition);
    }
    let id = |x: u32, y: u32| first + y * w + x;
    let mut cells = Vec::new();
    for y in 0..h {
        for x in 0..w {
            let from = if x > 0 {
                Some(id(x - 1, y))
            } else if y > 0 {
                Some(id(x, y - 1))
            } else {
                None
            };
            world.add_group(id(x, y), species, population, ctx, from);
            cells.push(id(x, y));
        }
    }
    for y in 0..h {
        for x in 0..w {
            if x + 1 < w {
                world.link(id(x, y), id(x + 1, y), link);
            }
            if y + 1 < h {
                world.link(id(x, y), id(x, y + 1), link);
                if x + 1 < w {
                    world.link(id(x, y), id(x + 1, y + 1), link);
                }
            }
        }
    }
    cells
}

/// Pose une technique dans tous les groupes de l'espèce sur les cellules
/// données.
pub fn seed_technique(world: &mut CultureWorld, species: u32, cells: &[u32], f: Family, freq: f64, mastery: f64) {
    for &c in cells {
        world.seed_trait(c, species, f as u32, freq, mastery);
    }
}
