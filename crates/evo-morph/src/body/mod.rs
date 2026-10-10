//! Palier 2 : corps complets (document Rendu du vivant, « De l'anatomie à
//! l'apparence »). Le plan de construction devient un corps 3D, une fois par
//! génotype de développement :
//!
//! 1. squelette de forme : une primitive par module, symétrie appliquée
//!    ([`shape`]) ;
//! 2. surface : union douce des primitives en champ de distance signée,
//!    maillée par surface nets à trois niveaux de détail ([`mesh`]) ;
//! 3. organes visibles (yeux, bouche, branchies) dans la peau, organes
//!    internes à part pour la vue anatomie ;
//! 4. matériaux : couleur des pigments, motif de Turing ([`pattern`]),
//!    brillance du revêtement ;
//! 5. squelette d'animation : un os par module placé, chaque sommet rattaché
//!    aux deux primitives les plus proches.
//!
//! [Simplification] Les pièces rigides (carapace) fusionnent sans douceur
//! dans la même peau au lieu d'être des maillages séparés, et les yeux
//! restent des ellipsoïdes sombres quel que soit leur stade.

pub mod generate;
pub mod mesh;
pub mod pattern;
pub mod plan;
pub mod rig;
pub mod shape;

pub use generate::{from_microbe, random_plan};
pub use mesh::{build, extent_m, impostor_views, plate, silhouette, Body, Mesh, LOD_RESOLUTION};
pub use plan::{BodyPlan, Covering, EyeStage, Joint, Module, ModuleKind, Pattern, Symmetry, System, PLAN_VERSION};
pub use rig::{pose, rig, Action, Affine, Locomotion, Motion, Rig};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::microbe::{form, MicrobeTraits};

    fn checksum(m: &Mesh) -> u64 {
        let mut h = 0u64;
        for p in &m.positions {
            for v in p {
                h = crate::canvas::mix64(h ^ v.to_bits() as u64);
            }
        }
        h ^ m.indices.len() as u64
    }

    #[test]
    fn random_plans_give_closed_outward_bodies() {
        for seed in 0..6u64 {
            let plan = random_plan(seed);
            assert!(plan.is_valid(), "plan {seed}");
            let body = build(&plan, 3);
            assert_eq!(body.lods.len(), 3);
            let fine = &body.lods[0];
            assert!(fine.triangles() > 500, "graine {seed} : {} triangles", fine.triangles());
            assert!(body.lods[2].triangles() < fine.triangles());
            assert!(!body.organs.positions.is_empty());
            // Normales sortantes : vers l'extérieur du centre de chaque
            // primitive la plus proche, pour presque tous les sommets.
            let shape = shape::place(&plan);
            let skin: Vec<_> = shape.primitives.iter().filter(|p| !p.internal).copied().collect();
            let mut out = 0;
            for (p, n) in fine.positions.iter().zip(&fine.normals) {
                let q = shape::V3(p[0], p[1], p[2]);
                let e = body.extent_m * 1e-3;
                let d = shape::field(&skin, q.add(shape::V3(n[0], n[1], n[2]).mul(e))).0 - shape::field(&skin, q).0;
                if d > 0.0 {
                    out += 1;
                }
            }
            assert!(out as f32 > 0.97 * fine.positions.len() as f32);
            // Triangles tournés vers l'extérieur.
            let mut agree = 0;
            for t in fine.indices.chunks(3) {
                let p = |i: u32| {
                    let v = fine.positions[i as usize];
                    shape::V3(v[0], v[1], v[2])
                };
                let face = p(t[1]).sub(p(t[0])).cross(p(t[2]).sub(p(t[0])));
                let n = fine.normals[t[0] as usize];
                if face.dot(shape::V3(n[0], n[1], n[2])) >= 0.0 {
                    agree += 1;
                }
            }
            assert!(agree as f32 > 0.97 * fine.triangles() as f32);
            for w in &fine.weights {
                assert!((w[0] + w[1] - 1.0).abs() < 1e-5 && w[0] >= w[1]);
            }
            for b in &fine.bones {
                assert!((b[0] as usize) < body.bones.len() && (b[1] as usize) < body.bones.len());
            }
        }
    }

    #[test]
    fn same_plan_same_body() {
        let a = build(&random_plan(42), 1);
        let b = build(&random_plan(42), 1);
        assert_eq!(checksum(&a.lods[0]), checksum(&b.lods[0]));
        assert_ne!(checksum(&a.lods[0]), checksum(&build(&random_plan(43), 1).lods[0]));
    }

    #[test]
    fn symmetry_makes_copies() {
        let mut plan = random_plan(7);
        plan.modules.truncate(1);
        let mut arm = Module::new(ModuleKind::Antenna, 0.5, 0.05);
        arm.parent = Some(0);
        arm.attach = 0.5;
        arm.around_deg = 90.0;
        arm.elevation_deg = 90.0;
        arm.symmetry = Symmetry::Radial(5);
        plan.modules.push(arm.clone());
        assert_eq!(shape::place(&plan).bones.len(), 6);
        plan.modules[1].symmetry = Symmetry::Bilateral;
        let s = shape::place(&plan);
        assert_eq!(s.bones.len(), 3);
        // Miroir par le plan sagittal.
        assert!((s.bones[1].tail.2 + s.bones[2].tail.2).abs() < 1e-5 && s.bones[1].tail.2.abs() > 0.1);
    }

    #[test]
    fn microbes_become_bodies() {
        for lineage in 0..8 {
            let t = MicrobeTraits {
                lineage,
                signature: lineage,
                pigment_rgb: Some([90, 140, 60]),
                gene_count: 6,
                phototroph: true,
                oxygenic: lineage % 2 == 0,
            };
            let f = form(&t, 9);
            let plan = from_microbe(&f);
            assert!(plan.is_valid());
            let body = build(&plan, 1);
            assert!(
                body.lods[0].triangles() > 100,
                "{:?} {} {:?}",
                f.shape,
                body.lods[0].triangles(),
                plan.modules.iter().map(|m| (m.kind, m.length_m, m.radius_m)).collect::<Vec<_>>()
            );
            // Échelle vraie : un microbe mesure quelques micromètres.
            assert!(body.extent_m > 2e-7 && body.extent_m < 2e-4, "{}", body.extent_m);
            if f.thylakoids {
                assert!(!body.organs.positions.is_empty());
            }
        }
    }

    #[test]
    fn turing_patterns_grow() {
        let p = Pattern { feed: 0.035, kill: 0.062, contrast: 1.0, scale: 1.0 };
        let tex = pattern::PatternTexture::grow(&p, 3);
        let c = tex.coverage();
        assert!(c > 0.03 && c < 0.9, "couverture {c}");
        assert_eq!(pattern::PatternTexture::grow(&Pattern::NONE, 3).coverage(), 0.0);
    }

    #[test]
    fn silhouette_is_filled() {
        let (mask, mpp) = silhouette(&random_plan(5), 64, 48);
        let filled = mask.iter().filter(|v| **v > 0.5).count();
        assert!(filled > 64 * 48 / 20 && filled < 64 * 48, "{filled}");
        assert!(mpp > 0.0);
    }
}
