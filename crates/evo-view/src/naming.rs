//! Noms générés : lieux et espèces (document Fonctionnalités, « Nommage »).
//!
//! Une espèce reçoit un nom binomial tiré de ses traits (le genre dit son
//! métabolisme principal) et de son lieu d'origine (l'épithète), plus un nom
//! courant. Tout est déterministe : même graine, même lignée, même nom.
//! Le joueur pourra renommer ; l'identifiant interne ne change pas.

use crate::format::Lang;
use evo_core::rng::splitmix64;
use evo_genetics::ReactionId;
use evo_life::metabolism::*;
use evo_planet::grid::GeodesicGrid;

/// Nombre de régions nommées : la grille du niveau 2 (162 cellules d'environ
/// 1 800 km sur Terre).
pub const REGION_LEVEL: u32 = 2;

/// Région (cellule du niveau 2 la plus proche) d'une cellule.
pub fn region_of(grid: &GeodesicGrid, cell: usize) -> usize {
    let n = GeodesicGrid::cell_count_for_level(REGION_LEVEL).min(grid.len());
    let p = grid.centers[cell];
    (0..n)
        .max_by(|&a, &b| {
            let s = |c: usize| p[0] * grid.centers[c][0] + p[1] * grid.centers[c][1] + p[2] * grid.centers[c][2];
            s(a).total_cmp(&s(b))
        })
        .unwrap_or(0)
}

const ONSETS: [&str; 18] = ["t", "v", "k", "m", "s", "l", "r", "d", "b", "n", "th", "br", "gr", "st", "al", "or", "es", "ir"];
const VOWELS: [&str; 8] = ["a", "e", "i", "o", "u", "ae", "ei", "ou"];
const CODAS: [&str; 10] = ["", "", "r", "n", "l", "s", "m", "th", "rn", "x"];

/// Nom d'une région, à partir de la graine de la partie.
pub fn place_name(seed: u64, region: usize) -> String {
    let mut h = splitmix64(seed ^ splitmix64(0x504C_4143_4500_0000 ^ region as u64));
    let mut next = |n: usize| {
        h = splitmix64(h);
        (h % n as u64) as usize
    };
    let syllables = 2 + next(2);
    let mut s = String::new();
    for i in 0..syllables {
        s.push_str(ONSETS[next(ONSETS.len())]);
        s.push_str(VOWELS[next(VOWELS.len())]);
        if i == syllables - 1 {
            s.push_str(CODAS[next(CODAS.len())]);
        }
    }
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => s,
    }
}

/// Métabolisme qui nomme une guilde : les voies lumineuses d'abord.
pub fn main_reaction(signature: u32) -> Option<ReactionId> {
    const PRIORITY: [ReactionId; REACTION_COUNT] = [
        OXYGENIC_PHOTOSYNTHESIS,
        ANOXYGENIC_PHOTOSYNTHESIS,
        PHOTOFERROTROPHY,
        MANGANESE_PHOTOTROPHY,
        METHANOGENESIS,
        METHANOTROPHY,
        SULFATE_REDUCTION,
        AEROBIC_RESPIRATION,
        FERMENTATION,
    ];
    PRIORITY.into_iter().find(|&r| signature & (1 << r) != 0)
}

fn genus(reaction: Option<ReactionId>) -> &'static str {
    match reaction {
        Some(OXYGENIC_PHOTOSYNTHESIS) => "Oxyphycus",
        Some(ANOXYGENIC_PHOTOSYNTHESIS) => "Thiophotus",
        Some(PHOTOFERROTROPHY) => "Ferrophotus",
        Some(MANGANESE_PHOTOTROPHY) => "Manganophotus",
        Some(METHANOGENESIS) => "Methanobius",
        Some(METHANOTROPHY) => "Methylophagus",
        Some(SULFATE_REDUCTION) => "Desulfobius",
        Some(AEROBIC_RESPIRATION) => "Aerobius",
        Some(FERMENTATION) => "Zymobius",
        _ => "Protobius",
    }
}

/// Nom courant du métabolisme principal (au féminin : « une bactérie »).
pub fn guild_common(reaction: Option<ReactionId>, lang: Lang) -> &'static str {
    match (lang, reaction) {
        (Lang::Fr, Some(OXYGENIC_PHOTOSYNTHESIS)) => "photosynthétique à oxygène",
        (Lang::Fr, Some(ANOXYGENIC_PHOTOSYNTHESIS)) => "phototrophe soufrée",
        (Lang::Fr, Some(PHOTOFERROTROPHY)) => "phototrophe du fer",
        (Lang::Fr, Some(MANGANESE_PHOTOTROPHY)) => "phototrophe du manganèse",
        (Lang::Fr, Some(METHANOGENESIS)) => "méthanogène",
        (Lang::Fr, Some(METHANOTROPHY)) => "méthanotrophe",
        (Lang::Fr, Some(SULFATE_REDUCTION)) => "sulfato-réductrice",
        (Lang::Fr, Some(AEROBIC_RESPIRATION)) => "respiratrice",
        (Lang::Fr, Some(FERMENTATION)) => "fermentatrice",
        (Lang::Fr, _) => "cellule",
        (Lang::En, Some(OXYGENIC_PHOTOSYNTHESIS)) => "oxygenic phototroph",
        (Lang::En, Some(ANOXYGENIC_PHOTOSYNTHESIS)) => "sulfur phototroph",
        (Lang::En, Some(PHOTOFERROTROPHY)) => "iron phototroph",
        (Lang::En, Some(MANGANESE_PHOTOTROPHY)) => "manganese phototroph",
        (Lang::En, Some(METHANOGENESIS)) => "methanogen",
        (Lang::En, Some(METHANOTROPHY)) => "methanotroph",
        (Lang::En, Some(SULFATE_REDUCTION)) => "sulfate reducer",
        (Lang::En, Some(AEROBIC_RESPIRATION)) => "aerobe",
        (Lang::En, Some(FERMENTATION)) => "fermenter",
        (Lang::En, _) => "cell",
    }
}

/// Noms d'une lignée.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpeciesNames {
    /// Nom binomial, en italique à l'affichage.
    pub scientific: String,
    pub common: String,
}

pub fn species_names(seed: u64, grid: &GeodesicGrid, lineage: u32, signature: u32, origin_cell: u32, lang: Lang) -> SpeciesNames {
    let place = place_name(seed, region_of(grid, (origin_cell as usize).min(grid.len().saturating_sub(1))));
    let reaction = main_reaction(signature);
    let common = match lang {
        Lang::Fr => format!("{} de {} n° {}", capitalise(guild_common(reaction, lang)), place, lineage),
        Lang::En => format!("{} {} no. {}", place, guild_common(reaction, lang), lineage),
    };
    SpeciesNames { scientific: binomial(seed, &place, reaction, lineage as u64), common }
}

/// Nom savant d'une espèce (guilde) : genre de son métabolisme, épithète de
/// son lieu d'origine.
pub fn scientific_name(seed: u64, grid: &GeodesicGrid, signature: u32, origin_cell: u32) -> String {
    let place = place_name(seed, region_of(grid, (origin_cell as usize).min(grid.len().saturating_sub(1))));
    binomial(seed, &place, main_reaction(signature), 0x5350_0000_0000 ^ signature as u64)
}

fn binomial(seed: u64, place: &str, reaction: Option<ReactionId>, key: u64) -> String {
    // Plusieurs espèces d'un même lieu : une variété (« tarvelensis
    // obscura ») les distingue le plus souvent.
    let stem = place.to_lowercase();
    let stem = stem.trim_end_matches(['a', 'e', 'i', 'o', 'u']);
    let mut scientific = format!("{} {}ensis", genus(reaction), stem);
    let h = splitmix64(splitmix64(seed ^ key.wrapping_mul(0x9E37_79B9)));
    const VARIETAS: [&str; 8] = ["", "", "", "minor", "major", "pallida", "obscura", "gracilis"];
    let v = VARIETAS[(h % VARIETAS.len() as u64) as usize];
    if !v.is_empty() {
        scientific.push(' ');
        scientific.push_str(v);
    }
    scientific
}

fn capitalise(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

/// Voies métaboliques d'une signature, en clair.
pub fn metabolism_list(signature: u32, lang: Lang) -> Vec<&'static str> {
    REACTIONS
        .iter()
        .filter(|r| signature & (1 << r.id) != 0)
        .map(|r| match lang {
            Lang::Fr => r.name,
            Lang::En => match r.id {
                METHANOGENESIS => "methanogenesis",
                FERMENTATION => "fermentation",
                AEROBIC_RESPIRATION => "aerobic respiration",
                SULFATE_REDUCTION => "sulfate reduction",
                METHANOTROPHY => "methanotrophy",
                ANOXYGENIC_PHOTOSYNTHESIS => "anoxygenic photosynthesis (sulfur)",
                OXYGENIC_PHOTOSYNTHESIS => "oxygenic photosynthesis",
                PHOTOFERROTROPHY => "anoxygenic photosynthesis (iron)",
                MANGANESE_PHOTOTROPHY => "anoxygenic photosynthesis (manganese)",
                _ => "?",
            },
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_stable_and_distinct() {
        let grid = GeodesicGrid::new(4);
        let a = species_names(42, &grid, 3, 1 << METHANOGENESIS, 100, Lang::Fr);
        let b = species_names(42, &grid, 3, 1 << METHANOGENESIS, 100, Lang::Fr);
        assert_eq!(a, b);
        assert!(a.scientific.starts_with("Methanobius "));
        assert!(a.common.starts_with("Méthanogène de "));
        let c = species_names(42, &grid, 4, (1 << OXYGENIC_PHOTOSYNTHESIS) | (1 << AEROBIC_RESPIRATION), 100, Lang::En);
        assert!(c.scientific.starts_with("Oxyphycus "));
        assert!(c.common.contains("oxygenic phototroph"));
        let places: std::collections::BTreeSet<String> = (0..162).map(|r| place_name(42, r)).collect();
        assert!(places.len() > 150, "{}", places.len());
    }

    #[test]
    fn regions_are_coarse_cells() {
        let grid = GeodesicGrid::new(4);
        for c in 0..162 {
            assert_eq!(region_of(&grid, c), c);
        }
    }
}
