//! Arbre du vivant : mise en page d'une phylogénie élaguée (document
//! Fonctionnalités, « Arbre du vivant » ; DA : arbre de traité ancien).
//!
//! Le monde microbien fonde des milliers de lignées, presque toutes
//! éphémères. L'arbre montre les lignées vivantes et leurs ancêtres (les
//! branches mortes qui ont une descendance restent, en gris) ; au-delà d'un
//! nombre de feuilles lisible, les plus petits clades sont repliés en une
//! seule feuille qui dit combien de lignées elle contient.

use crate::frame::LineageFrame;
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq)]
pub struct TreeNode {
    pub lineage: u32,
    /// Indice du parent dans `Tree::nodes` (la racine pointe sur elle-même).
    pub parent: usize,
    pub born_years: f64,
    /// Fin de la branche : extinction, ou date courante pour une lignée vivante.
    pub end_years: f64,
    /// Rang vertical (0 = en haut), en unités de feuilles.
    pub y: f32,
    pub living: bool,
    /// Lignées repliées dans ce noeud (0 s'il n'est pas replié).
    pub collapsed: usize,
    pub signature: u32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Tree {
    pub nodes: Vec<TreeNode>,
    pub leaves: usize,
    pub from_years: f64,
    pub to_years: f64,
}

/// Construit l'arbre élagué ; `now` est la date courante.
pub fn build(lineages: &[LineageFrame], now: f64, max_leaves: usize) -> Tree {
    if lineages.is_empty() {
        return Tree::default();
    }
    let by_id: BTreeMap<u32, &LineageFrame> = lineages.iter().map(|l| (l.id, l)).collect();
    // Lignées gardées : les vivantes et leurs ancêtres.
    let mut keep: BTreeMap<u32, ()> = BTreeMap::new();
    for l in lineages.iter().filter(|l| l.extinct_years.is_none()) {
        let mut id = l.id;
        while keep.insert(id, ()).is_none() {
            match by_id.get(&id) {
                Some(r) if r.parent != r.id => id = r.parent,
                _ => break,
            }
        }
    }
    // Une planète sans survivant montre quand même ses premières lignées.
    if keep.is_empty() {
        for l in lineages.iter().take(max_leaves.max(1)) {
            keep.insert(l.id, ());
        }
    }
    let ids: Vec<u32> = keep.keys().copied().collect();
    let mut children: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    let mut roots = Vec::new();
    for &id in &ids {
        let l = by_id[&id];
        if l.parent != id && keep.contains_key(&l.parent) {
            children.entry(l.parent).or_default().push(id);
        } else {
            roots.push(id);
        }
    }
    // Nombre de feuilles de chaque sous-arbre et fin la plus tardive.
    let mut leaf_count: BTreeMap<u32, usize> = BTreeMap::new();
    let mut last_end: BTreeMap<u32, f64> = BTreeMap::new();
    for &id in ids.iter().rev() {
        // Les identifiants croissent avec la date de fondation : un enfant
        // est toujours traité avant son parent en ordre décroissant.
        let l = by_id[&id];
        let own_end = l.extinct_years.unwrap_or(now);
        let (count, end) = match children.get(&id) {
            Some(cs) => cs.iter().fold((0, own_end), |(n, e), c| (n + leaf_count[c], e.max(last_end[c]))),
            None => (1, own_end),
        };
        leaf_count.insert(id, count);
        last_end.insert(id, end);
    }
    // Seuil de repli : le plus petit qui tient dans `max_leaves`.
    let visible = |t: usize| {
        fn walk(id: u32, t: usize, children: &BTreeMap<u32, Vec<u32>>, leaf_count: &BTreeMap<u32, usize>) -> usize {
            match children.get(&id) {
                Some(cs) if leaf_count[&id] >= t => cs.iter().map(|&c| walk(c, t, children, leaf_count)).sum(),
                _ => 1,
            }
        }
        roots.iter().map(|&r| walk(r, t, &children, &leaf_count)).sum::<usize>()
    };
    let total: usize = roots.iter().map(|r| leaf_count[r]).sum();
    let (mut lo, mut hi) = (1usize, total.max(1) + 1);
    if visible(lo) > max_leaves {
        while lo < hi {
            let mid = (lo + hi) / 2;
            if visible(mid) <= max_leaves {
                hi = mid;
            } else {
                lo = mid + 1;
            }
        }
    }
    let threshold = lo;
    let mut tree = Tree { from_years: f64::INFINITY, to_years: now, ..Default::default() };
    fn place(
        id: u32,
        parent: Option<usize>,
        threshold: usize,
        ctx: (&BTreeMap<u32, &LineageFrame>, &BTreeMap<u32, Vec<u32>>, &BTreeMap<u32, usize>, &BTreeMap<u32, f64>),
        now: f64,
        tree: &mut Tree,
    ) -> f32 {
        let (by_id, children, leaf_count, last_end) = ctx;
        let l = by_id[&id];
        let index = tree.nodes.len();
        let collapse = children.contains_key(&id) && leaf_count[&id] < threshold;
        tree.nodes.push(TreeNode {
            lineage: id,
            parent: parent.unwrap_or(index),
            born_years: l.born_years,
            end_years: if collapse { last_end[&id] } else { l.extinct_years.unwrap_or(now) },
            y: 0.0,
            living: l.extinct_years.is_none() || (collapse && last_end[&id] >= now),
            collapsed: if collapse { leaf_count[&id] } else { 0 },
            signature: l.signature,
        });
        tree.from_years = tree.from_years.min(l.born_years);
        let y = match children.get(&id) {
            Some(cs) if !collapse => {
                let ys: Vec<f32> = cs.iter().map(|&c| place(c, Some(index), threshold, ctx, now, tree)).collect();
                (ys[0] + ys[ys.len() - 1]) / 2.0
            }
            _ => {
                tree.leaves += 1;
                (tree.leaves - 1) as f32
            }
        };
        tree.nodes[index].y = y;
        y
    }
    for &r in &roots {
        place(r, None, threshold, (&by_id, &children, &leaf_count, &last_end), now, &mut tree);
    }
    tree
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lf(id: u32, parent: u32, born: f64, extinct: Option<f64>) -> LineageFrame {
        LineageFrame { id, parent, born_years: born, extinct_years: extinct, origin_cell: 0, signature: 1 }
    }

    #[test]
    fn prunes_dead_ends_and_keeps_dead_ancestors() {
        let ls =
            vec![lf(0, 0, 0.0, Some(5.0)), lf(1, 0, 1.0, None), lf(2, 0, 2.0, Some(3.0)), lf(3, 1, 4.0, None), lf(4, 2, 2.5, Some(2.8))];
        let t = build(&ls, 10.0, 100);
        let ids: Vec<u32> = t.nodes.iter().map(|n| n.lineage).collect();
        assert_eq!(ids, vec![0, 1, 3]);
        assert!(!t.nodes[0].living);
        assert_eq!(t.leaves, 1);
        assert_eq!(t.nodes[2].parent, 1);
    }

    #[test]
    fn collapses_small_clades_to_fit() {
        // Une racine, 10 clades de 1 à 10 feuilles vivantes.
        let mut ls = vec![lf(0, 0, 0.0, None)];
        let mut id = 1;
        for k in 1..=10u32 {
            let head = id;
            ls.push(lf(head, 0, k as f64, None));
            id += 1;
            for _ in 1..k {
                ls.push(lf(id, head, k as f64 + 0.5, None));
                id += 1;
            }
        }
        let full = build(&ls, 20.0, 1000);
        let small = build(&ls, 20.0, 12);
        assert!(full.leaves > 12);
        assert!(small.leaves <= 12, "{}", small.leaves);
        let hidden: usize = small.nodes.iter().map(|n| n.collapsed).sum();
        assert!(hidden > 0);
        // Les feuilles sont rangées de 0 à leaves − 1.
        let max_y = small.nodes.iter().map(|n| n.y).fold(0.0, f32::max);
        assert_eq!(max_y as usize, small.leaves - 1);
    }
}
