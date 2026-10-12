//! Arbre du vivant fractal, à la manière d'OneZoom (document Fonctionnalités,
//! « Arbre du vivant », « arbre type OneZoom ») : chaque clade est une
//! branche qui se divise en deux, la taille d'une branche suit la racine
//! carrée du nombre de lignées vivantes qu'elle porte, et les petits clades
//! s'enroulent en spirales sur le côté des grands. Tout l'arbre tient dans un
//! seul dessin dans lequel on zoome sans fin : une lignée isolée est une
//! feuille minuscule au bout de sa spirale.
//!
//! La géométrie ne dit que la parenté, pas le temps (comme OneZoom) ; les
//! dates sont dans les infobulles. Une lignée qui a fondé plusieurs lignées
//! filles est une polytomie : ses filles et sa propre continuation sont
//! regroupées en paires équilibrées par nombre de feuilles, dans l'ordre de
//! leur naissance, ce qui garde l'arbre compact.

use crate::frame::LineageFrame;
use std::collections::BTreeMap;

/// Raccourcissement d'une branche à chaque division, en plus de la racine
/// carrée de la part des feuilles : une chenille (un clade qui bourgeonne
/// une lignée après l'autre) devient une spirale qui se resserre.
const SHRINK: f32 = 0.9;
/// Angles de division, radians : écart minimal, plus une part qui croît
/// avec la taille du clade d'en face.
const TURN_MIN: f32 = 0.12;
const TURN_SPAN: f32 = 0.6;
/// Au-delà de cette part du petit clade, une division est équilibrée.
const BALANCED: f32 = 0.25;
/// Rayon d'une feuille, en part de la longueur de sa branche.
pub const LEAF_RADIUS: f32 = 0.38;

#[derive(Clone, Debug, PartialEq)]
pub struct FractalNode {
    /// Lignée portée : la feuille, ou la lignée dont le clade se divise ici.
    pub lineage: u32,
    /// Branche, de sa base à sa pointe (la feuille est au bout).
    pub from: [f32; 2],
    pub to: [f32; 2],
    /// Longueur de la branche (sert d'épaisseur et de taille de feuille).
    pub length: f32,
    pub leaf: bool,
    pub living: bool,
    /// Lignées vivantes du clade.
    pub leaves: u32,
    /// Indice qui suit le sous-arbre dans l'ordre préfixe : pour sauter un
    /// clade hors champ ou trop petit à l'écran.
    pub next: u32,
    /// Boîte englobante du sous-arbre : min x, min y, max x, max y.
    pub bbox: [f32; 4],
    /// Grand clade de la racine (0 à 7) : la couleur de l'aquarelle.
    pub clade: u8,
    /// La branche ouvre le clade d'une lignée fille (`lineage`) : elle porte
    /// son nom quand elle est assez grande à l'écran.
    pub founds: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Fractal {
    pub nodes: Vec<FractalNode>,
    pub leaves: usize,
}

enum Bin {
    Leaf { lineage: u32, living: bool },
    Split { a: usize, b: usize, lineage: u32 },
}

struct Arena {
    bins: Vec<Bin>,
    leaves: Vec<u32>,
    founds: Vec<Option<u32>>,
}

impl Arena {
    fn push(&mut self, b: Bin, leaves: u32) -> usize {
        self.bins.push(b);
        self.leaves.push(leaves);
        self.founds.push(None);
        self.bins.len() - 1
    }

    /// Regroupe des sous-arbres en paires équilibrées, dans l'ordre.
    fn combine(&mut self, items: &[usize], lineage: u32) -> usize {
        if items.len() == 1 {
            return items[0];
        }
        let total: u32 = items.iter().map(|&i| self.leaves[i].max(1)).sum();
        let mut acc = 0;
        let mut cut = 1;
        let mut best = u32::MAX;
        for (k, &i) in items[..items.len() - 1].iter().enumerate() {
            acc += self.leaves[i].max(1);
            let gap = (2 * acc).abs_diff(total);
            if gap < best {
                best = gap;
                cut = k + 1;
            }
        }
        let a = self.combine(&items[..cut], lineage);
        let b = self.combine(&items[cut..], lineage);
        let n = self.leaves[a] + self.leaves[b];
        self.push(Bin::Split { a, b, lineage }, n)
    }
}

/// Construit l'arbre fractal des lignées vivantes et de leurs ancêtres.
pub fn build(lineages: &[LineageFrame]) -> Fractal {
    let by_id: BTreeMap<u32, &LineageFrame> = lineages.iter().map(|l| (l.id, l)).collect();
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
    let mut children: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    let mut roots = Vec::new();
    for &id in keep.keys() {
        let l = by_id[&id];
        if l.parent != id && keep.contains_key(&l.parent) {
            children.entry(l.parent).or_default().push(id);
        } else {
            roots.push(id);
        }
    }
    // Sous-arbre de chaque lignée, les filles avant leurs mères : les
    // identifiants croissent avec la date de fondation.
    let mut arena = Arena { bins: Vec::new(), leaves: Vec::new(), founds: Vec::new() };
    let mut top: BTreeMap<u32, usize> = BTreeMap::new();
    for &id in keep.keys().rev() {
        let l = by_id[&id];
        let mut items: Vec<(f64, usize)> = Vec::new();
        for c in children.get(&id).into_iter().flatten() {
            if let Some(t) = top.remove(c) {
                items.push((by_id[c].born_years, t));
            }
        }
        if l.extinct_years.is_none() {
            let leaf = arena.push(Bin::Leaf { lineage: id, living: true }, 1);
            items.push((l.extinct_years.unwrap_or(f64::INFINITY), leaf));
        }
        if items.is_empty() {
            continue;
        }
        items.sort_by(|a, b| a.0.total_cmp(&b.0));
        let list: Vec<usize> = items.into_iter().map(|x| x.1).collect();
        let t = arena.combine(&list, id);
        arena.founds[t] = Some(id);
        top.insert(id, t);
    }
    let mut root_items: Vec<usize> = roots.iter().filter_map(|r| top.remove(r)).collect();
    if root_items.is_empty() {
        return Fractal::default();
    }
    root_items.sort();
    let root = arena.combine(&root_items, roots[0]);
    let total = arena.leaves[root].max(1);
    layout(&arena, root, total)
}

fn layout(arena: &Arena, root: usize, total: u32) -> Fractal {
    let mut out = Fractal { nodes: Vec::new(), leaves: total as usize };
    // (bin, base, direction, longueur, clade)
    let mut stack: Vec<(usize, [f32; 2], f32, f32, u8, u32)> = vec![(root, [0.0, 0.0], -std::f32::consts::FRAC_PI_2, 0.45, 0, 0)];
    // Les grands clades de la racine reçoivent chacun une couleur.
    let mut next_clade = 0u8;
    let mut open: Vec<(usize, usize)> = Vec::new();
    while let Some((bin, base, dir, len, clade, depth)) = stack.pop() {
        let to = [base[0] + len * dir.cos(), base[1] + len * dir.sin()];
        let index = out.nodes.len();
        let n = arena.leaves[bin];
        let mut clade = clade;
        if n * 12 >= total && n * 2 <= total + 1 && arena.founds[bin].is_some() {
            next_clade = next_clade % 7 + 1;
            clade = next_clade;
        }
        let (lineage, leaf, living) = match arena.bins[bin] {
            Bin::Leaf { lineage, living } => (lineage, true, living),
            Bin::Split { lineage, .. } => (lineage, false, true),
        };
        out.nodes.push(FractalNode {
            lineage: arena.founds[bin].unwrap_or(lineage),
            from: base,
            to,
            length: len,
            leaf,
            living,
            leaves: n,
            next: 0,
            bbox: [0.0; 4],
            clade,
            founds: arena.founds[bin].is_some(),
        });
        // Fin des sous-arbres terminés : l'ordre préfixe d'une pile.
        while let Some(&(i, remaining)) = open.last() {
            if remaining == 0 {
                out.nodes[i].next = index as u32;
                open.pop();
            } else {
                break;
            }
        }
        if let Some(last) = open.last_mut() {
            last.1 -= 1;
        }
        if let Bin::Split { a, b, .. } = arena.bins[bin] {
            let (big, small) = if arena.leaves[a] >= arena.leaves[b] { (a, b) } else { (b, a) };
            let (nb, ns) = (arena.leaves[big] as f32, arena.leaves[small] as f32);
            let share_b = nb / (nb + ns);
            let share_s = ns / (nb + ns);
            let turn_b = TURN_MIN + TURN_SPAN * share_s;
            let turn_s = TURN_MIN + TURN_SPAN * share_b;
            // Côté du grand clade : il s'enroule toujours du même côté sur
            // une chenille (spirale), et alterne quand les deux se valent,
            // pour que deux grands clades voisins ne se recouvrent pas.
            let side = if share_s > BALANCED && depth % 2 == 1 { -1.0 } else { 1.0 };
            // Le petit clade d'abord sur la pile : le grand est dessiné avant.
            stack.push((small, to, dir - side * turn_s, len * SHRINK * share_s.sqrt(), clade, depth + 1));
            stack.push((big, to, dir + side * turn_b, len * SHRINK * share_b.sqrt(), clade, depth + 1));
            open.push((index, 2));
        } else {
            out.nodes[index].next = index as u32 + 1;
            // Une feuille ferme aussitôt son sous-arbre.
        }
    }
    let end = out.nodes.len() as u32;
    for (i, _) in open {
        out.nodes[i].next = end;
    }
    // Boîtes englobantes, des feuilles vers la racine.
    for i in (0..out.nodes.len()).rev() {
        let nd = &out.nodes[i];
        let r = if nd.leaf { nd.length * LEAF_RADIUS } else { 0.0 };
        let mut b =
            [nd.from[0].min(nd.to[0] - r), nd.from[1].min(nd.to[1] - r), nd.from[0].max(nd.to[0] + r), nd.from[1].max(nd.to[1] + r)];
        let mut j = i + 1;
        while j < nd.next as usize {
            let c = &out.nodes[j];
            b = [b[0].min(c.bbox[0]), b[1].min(c.bbox[1]), b[2].max(c.bbox[2]), b[3].max(c.bbox[3])];
            j = c.next as usize;
        }
        out.nodes[i].bbox = b;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lf(id: u32, parent: u32, born: f64, extinct: Option<f64>) -> LineageFrame {
        LineageFrame { id, parent, born_years: born, extinct_years: extinct, origin_cell: 0, signature: 1 }
    }

    /// Une racine qui bourgeonne `n` lignées, chacune fondant une fille.
    fn caterpillar(n: u32) -> Vec<LineageFrame> {
        let mut ls = vec![lf(0, 0, 0.0, None)];
        for k in 1..=n {
            ls.push(lf(2 * k - 1, 0, k as f64, None));
            ls.push(lf(2 * k, 2 * k - 1, k as f64 + 0.5, if k % 3 == 0 { Some(k as f64 + 0.7) } else { None }));
        }
        ls
    }

    #[test]
    fn every_living_lineage_is_a_leaf() {
        let ls = caterpillar(40);
        let t = build(&ls);
        let living = ls.iter().filter(|l| l.extinct_years.is_none()).count();
        assert_eq!(t.leaves, living);
        assert_eq!(t.nodes.iter().filter(|n| n.leaf).count(), living);
        // Arbre binaire : 2 n − 1 noeuds.
        assert_eq!(t.nodes.len(), 2 * living - 1);
    }

    #[test]
    fn next_skips_whole_subtrees() {
        let t = build(&caterpillar(30));
        assert_eq!(t.nodes[0].next as usize, t.nodes.len());
        for (i, n) in t.nodes.iter().enumerate() {
            assert!(n.next as usize > i);
            // Les feuilles d'un sous-arbre sont celles qu'il compte.
            let leaves = t.nodes[i..n.next as usize].iter().filter(|c| c.leaf).count();
            assert_eq!(leaves as u32, n.leaves, "noeud {i}");
            // La boîte englobe les enfants.
            for c in &t.nodes[i + 1..n.next as usize] {
                assert!(c.to[0] >= n.bbox[0] - 1e-6 && c.to[0] <= n.bbox[2] + 1e-6);
                assert!(c.to[1] >= n.bbox[1] - 1e-6 && c.to[1] <= n.bbox[3] + 1e-6);
            }
        }
    }

    #[test]
    fn sizes_follow_the_clades() {
        let t = build(&caterpillar(200));
        let leaf_len: Vec<f32> = t.nodes.iter().filter(|n| n.leaf).map(|n| n.length).collect();
        let min = leaf_len.iter().cloned().fold(f32::MAX, f32::min);
        assert!(min > 0.0);
        // L'arbre reste compact : la spirale se resserre.
        let b = t.nodes[0].bbox;
        assert!(b[2] - b[0] < 20.0 && b[3] - b[1] < 20.0, "{b:?}");
        // Le fondateur d'une lignée fille porte son nom.
        assert!(t.nodes.iter().any(|n| n.founds && !n.leaf));
    }

    /// Dessin d'essai : `EVO_FRACTAL_DUMP=chemin cargo test -p evo-view dump -- --ignored`.
    #[test]
    #[ignore]
    fn dump() {
        // Naissances et morts au hasard jusqu'à 3 000 lignées vivantes.
        let mut s = 12345u64;
        let mut rnd = move || {
            s ^= s << 13;
            s ^= s >> 7;
            s ^= s << 17;
            (s >> 11) as f64 / (1u64 << 53) as f64
        };
        let mut ls = vec![lf(0, 0, 0.0, None)];
        let mut t = 0.0;
        while ls.iter().filter(|l| l.extinct_years.is_none()).count() < 3000 {
            t += 1.0;
            let alive: Vec<usize> = (0..ls.len()).filter(|&i| ls[i].extinct_years.is_none()).collect();
            let i = alive[(rnd() * alive.len() as f64) as usize];
            if rnd() < 0.42 && alive.len() > 1 {
                ls[i].extinct_years = Some(t);
            } else {
                let id = ls.len() as u32;
                ls.push(lf(id, ls[i].id, t, None));
            }
        }
        let tr = build(&ls);
        let mut out = String::new();
        for n in &tr.nodes {
            out += &format!("{} {} {} {} {} {} {}\n", n.from[0], n.from[1], n.to[0], n.to[1], n.length, n.leaf as u8, n.clade);
        }
        std::fs::write(std::env::var("EVO_FRACTAL_DUMP").unwrap(), out).unwrap();
    }

    #[test]
    fn extinct_world_gives_an_empty_tree() {
        let t = build(&[lf(0, 0, 0.0, Some(1.0))]);
        assert!(t.nodes.is_empty());
    }
}
