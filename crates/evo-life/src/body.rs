//! Plan de construction : la sortie du développement, au format du document
//! Génétique (« Format du plan de construction : champs par module »), lue
//! par le générateur d'apparence evo-morph et par le client.
//!
//! Un plan est un graphe de modules ; chaque module porte les onze champs du
//! document : identité et types cellulaires, parent et point d'attache, axe et
//! position relative, dimensions et forme, symétrie et répétitions,
//! articulations, matériau interne, tissu de revêtement, pigments, paramètres
//! de motif, fonction et coûts. Rien n'y est décoratif : chaque champ vient du
//! génome ou du phénotype.
//!
//! À l'étape 4, le développement produit des cellules seules (un module
//! « cellule ») et des colonies (un module « corps » dont les couches
//! portent chacune un type cellulaire). Les segments, appendices et organes
//! viendront avec les régulateurs dupliqués des étapes suivantes ; le format
//! les prévoit déjà.
//!
//! [Simplification] Ni articulation ni couleur structurale à cette étape :
//! les listes sont vides. Les dimensions d'une cellule supposent 1 µm pour
//! une bactérie.

use crate::metabolism::REACTIONS;
use crate::phenotype::{Phenotype, Shape};
use crate::Physiology;
use evo_core::math::Det;
use evo_genetics::{DomainFamily, Genome};

/// Version du format, à monter à chaque changement incompatible.
pub const BODY_PLAN_VERSION: u32 = 2;

/// Diamètre d'une bactérie de référence, m.
pub const REFERENCE_CELL_M: f64 = 1e-6;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct BodyPlan {
    pub version: u32,
    pub modules: Vec<Module>,
    /// Types cellulaires de l'organisme.
    pub cell_types: Vec<CellType>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ModuleKind {
    /// Cellule seule (unicellulaire).
    Cell,
    /// Corps d'une colonie ou tronc d'un multicellulaire.
    Body,
    Segment,
    Appendage,
    Organ,
    Leaf,
}

/// Type cellulaire : un jeu de gènes exprimés (un état stable du réseau de
/// régulation).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CellType {
    /// Voies métaboliques utilisables (bits de signature).
    pub pathways: u32,
    /// Libellé fonctionnel court (« photosynthétique », « réserve »…).
    pub role: String,
    pub expressed_genes: u32,
    /// Pic d'absorption moyen de ses pigments, nm.
    pub pigment_nm: Option<f64>,
}

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Profile {
    Round,
    Elongated,
    Flattened,
}

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Dimensions {
    pub length_m: f64,
    pub width_m: f64,
    pub thickness_m: f64,
    pub profile: Profile,
    /// Masse de carbone, en moles.
    pub carbon_mol: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum SymmetryKind {
    None,
    /// Radiale d'ordre n.
    Radial(u8),
    Bilateral,
    /// Sphérique (colonie en boule).
    Spherical,
}

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Symmetry {
    pub kind: SymmetryKind,
    /// Nombre de copies du module et espacement relatif.
    pub copies: u32,
    pub spacing: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum JointKind {
    Rigid,
    Hinge,
    Ball,
    Flexible,
}

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Joint {
    pub kind: JointKind,
    pub axis: [f64; 3],
    pub amplitude_rad: f64,
    /// Section du muscle qui la commande, m², et part de fibres rapides.
    pub muscle_section_m2: f64,
    pub fast_fibres: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Material {
    /// Cytoplasme tenu par le cytosquelette.
    Cytoplasm,
    /// Squelette hydrostatique (cellules collées, matrice d'eau).
    Hydrostatic,
    Cartilage,
    Bone,
    Chitin,
    Wood,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Covering {
    /// Membrane nue.
    Membrane,
    /// Matrice ou mucus d'adhésion.
    Mucus,
    Cuticle,
    Scales,
    Hair,
    Feathers,
    Bark,
    Wax,
}

/// Pigment d'une zone d'un module.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PigmentPatch {
    /// Pic du spectre d'absorption, nm (couleur et rendement).
    pub absorption_nm: f64,
    /// Concentration relative (0 à 1).
    pub concentration: f64,
    /// Zone : profondeur relative de 0 (surface) à 1 (centre).
    pub depth: f64,
}

/// Couleur structurale (irisation) : période et régularité des
/// microstructures.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct StructuralColour {
    pub period_nm: f64,
    pub regularity: f64,
}

/// Paramètres d'un motif de réaction-diffusion à deux espèces (Turing).
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TuringPattern {
    pub activator_diffusion: f64,
    pub inhibitor_diffusion: f64,
    pub activation: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Function {
    /// Voies métaboliques portées (bits de signature).
    pub pathways: u32,
    /// Coût d'entretien, kJ·molC⁻¹·an⁻¹, et coût de fabrication, kJ·molC⁻¹.
    pub maintenance_kj: f64,
    pub build_kj: f64,
    /// Organe sonore porté par le module (étape 5).
    pub sound: Option<SoundOrgan>,
}

/// Couche d'un module de corps : profondeur relative et type cellulaire.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Layer {
    /// Profondeur relative, de 0 (surface) à 1 (centre).
    pub depth: f64,
    /// Part des cellules du module.
    pub weight: f64,
    pub cell_type: u8,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Module {
    pub id: u16,
    // 1. Identité : type de module, types cellulaires, régulateur maître.
    pub kind: ModuleKind,
    pub cell_types: Vec<u8>,
    pub layers: Vec<Layer>,
    pub master_regulator: Option<u16>,
    // 2. Parent et point d'attache (repère du parent).
    pub parent: Option<u16>,
    pub attach: [f64; 3],
    // 3. Axe et position relative.
    pub axis: [f64; 3],
    pub offset: [f64; 3],
    // 4. Dimensions et forme.
    pub dimensions: Dimensions,
    // 5. Symétrie et répétitions.
    pub symmetry: Symmetry,
    // 6. Articulations avec le parent.
    pub joints: Vec<Joint>,
    // 7. Matériau interne.
    pub material: Material,
    // 8. Tissu de revêtement.
    pub covering: Covering,
    // 9. Pigments, et couleurs structurales à part.
    pub pigments: Vec<PigmentPatch>,
    pub structural_colour: Option<StructuralColour>,
    // 10. Paramètres de motif.
    pub pattern: Option<TuringPattern>,
    // 11. Fonction et coûts.
    pub function: Function,
}

/// Façon d'émettre un son (document Design sonore, banc d'écoute) :
/// frottement de pièces dures, tymbale, battement d'ailes, syrinx, larynx,
/// sac vocal, percussion, vessie natatoire, clics.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SoundKind {
    Stridulation,
    Tymbal,
    Wings,
    Syrinx,
    Larynx,
    Sac,
    Percussion,
    SwimBladder,
    Clicks,
}

/// Moment de la journée où l'espèce appelle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CallTime {
    Day,
    Dusk,
    Night,
}

/// Organe sonore d'une espèce (étape 5) : fonction d'un module, lue par le
/// son (crate evo-son) pour la voix de l'espèce. Il naît des muscles
/// (moteurs), d'une pièce rigide qui vibre (matrice) et d'un tissu nerveux
/// qui commande l'appel.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SoundOrgan {
    pub kind: SoundKind,
    /// Taille de l'organe, m (elle fixe la hauteur du son).
    pub size_m: f64,
    /// Puissance et complexité de l'appel, de 0 à 1.
    pub power: f64,
    pub complexity: f64,
    /// Appel en chœur (plutôt que seul).
    pub chorus: bool,
    pub time: CallTime,
    /// Graine héritée : elle ne change qu'avec les gènes qui portent l'appel.
    pub seed: u64,
}

/// Organe sonore d'un corps de longueur `length_m` : aucun sans muscle,
/// pièce rigide et tissu nerveux. Le type suit le corps et le milieu ; la
/// variation cachée des récepteurs (`sensor_nm`, sans effet sur la vue)
/// départage les variantes, le moment et le chœur.
///
/// [Simplification] Ni vol (ailes, syrinx), ni organe propre : l'appel se
/// lit sur les muscles, la matrice et les nerfs de tout le corps.
#[allow(clippy::too_many_arguments)]
pub fn sound_organ(
    motor: f64,
    matrix: f64,
    neural: f64,
    cuticle: f64,
    sensor_nm: f64,
    length_m: f64,
    terrestrial: bool,
    sexual: bool,
) -> Option<SoundOrgan> {
    if motor <= 0.0 || matrix <= 0.0 || neural <= 0.0 {
        return None;
    }
    let variant = sensor_nm.max(0.0);
    let kind = if terrestrial {
        // Carapace (cuticule durcie, peu de matrice) : frottement ou
        // tymbale ; squelette interne : larynx ou sac vocal.
        if matrix < 1.0 && cuticle > 0.0 {
            if motor > 1.0 {
                SoundKind::Tymbal
            } else {
                SoundKind::Stridulation
            }
        } else if variant > 600.0 {
            SoundKind::Sac
        } else {
            SoundKind::Larynx
        }
    } else if length_m < 0.02 {
        SoundKind::Percussion
    } else if neural > 1.0 {
        SoundKind::Clicks
    } else {
        SoundKind::SwimBladder
    };
    let time = if variant < 480.0 {
        CallTime::Day
    } else if variant < 580.0 {
        CallTime::Dusk
    } else {
        CallTime::Night
    };
    let q = |x: f64, step: f64| (x / step).round() as i64 as u64;
    let mut seed = q(variant, 5.0) ^ q(motor, 0.25).rotate_left(16) ^ q(matrix, 0.25).rotate_left(32) ^ q(neural, 0.25).rotate_left(48);
    // Mélange (splitmix64).
    seed = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
    seed = (seed ^ (seed >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    seed = (seed ^ (seed >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    seed ^= seed >> 31;
    Some(SoundOrgan {
        kind,
        size_m: 0.1 * length_m,
        power: -(-motor.min(3.0) * matrix.min(3.0)).dexp_m1(),
        complexity: neural / (1.0 + neural),
        chorus: sexual && q(variant, 10.0) % 2 == 0,
        time,
        seed,
    })
}

/// Rôle d'un type cellulaire, tiré de ce qu'il exprime.
fn role(p: &Phenotype) -> &'static str {
    let pathways = p.signature & crate::phenotype::PATHWAY_MASK;
    if pathways & REACTIONS.iter().filter(|r| r.is_light()).fold(0, |a, r| a | (1 << r.id)) != 0 {
        "photosynthétique"
    } else if p.is_phagotroph() {
        "nourricière"
    } else if pathways != 0 {
        "métabolique"
    } else {
        "réserve"
    }
}

/// Pigments d'un type cellulaire : chaque domaine pigment exprimé, avec sa
/// part de la quantité.
fn pigments_of(genome: &Genome, expression: Option<&[bool]>, depth: f64) -> Vec<PigmentPatch> {
    let host = genome
        .genes
        .iter()
        .enumerate()
        .filter(|(i, g)| g.functional && expression.is_none_or(|m| m[*i]))
        .map(|(_, g)| g)
        .chain(genome.organelle_genes())
        .filter(|g| g.domain.family == DomainFamily::Pigment);
    host.map(|g| PigmentPatch { absorption_nm: g.domain.absorption_nm, concentration: g.domain.efficiency.min(1.0), depth }).collect()
}

/// Construit le plan de construction d'un organisme.
pub fn body_plan(genome: &Genome, p: &Phenotype, physio: &Physiology) -> BodyPlan {
    let cell_m = REFERENCE_CELL_M * p.cell_size;
    let covering = if p.adhesion > 0.0 { Covering::Mucus } else { Covering::Membrane };
    let build_kj =
        if p.signature & crate::phenotype::PATHWAY_MASK != 0 { physio.autotroph_biomass_kj } else { physio.heterotroph_biomass_kj };
    let carbon = physio.carbon_per_cell * p.cell_volume() * p.cells();
    let pattern = (p.signalling > 0.0).then(|| TuringPattern {
        activator_diffusion: physio.morphogen_reach_cells,
        inhibitor_diffusion: physio.diffusion_cells,
        activation: p.signalling.min(1.0),
    });
    let Some(body) = p.body.as_ref() else {
        let ty = CellType {
            pathways: p.signature & crate::phenotype::PATHWAY_MASK,
            role: role(p).into(),
            expressed_genes: p.expressed_genes,
            pigment_nm: p.pigment_nm,
        };
        let module = Module {
            id: 0,
            kind: ModuleKind::Cell,
            cell_types: vec![0],
            layers: vec![Layer { depth: 0.0, weight: 1.0, cell_type: 0 }],
            master_regulator: None,
            parent: None,
            attach: [0.0; 3],
            axis: [1.0, 0.0, 0.0],
            offset: [0.0; 3],
            dimensions: Dimensions { length_m: cell_m, width_m: cell_m, thickness_m: cell_m, profile: Profile::Round, carbon_mol: carbon },
            symmetry: Symmetry { kind: SymmetryKind::Spherical, copies: 1, spacing: 0.0 },
            joints: Vec::new(),
            material: Material::Cytoplasm,
            covering,
            pigments: pigments_of(genome, None, 0.0),
            structural_colour: None,
            pattern: None,
            function: Function { pathways: ty.pathways, maintenance_kj: p.maintenance_kj, build_kj, sound: None },
        };
        return BodyPlan { version: BODY_PLAN_VERSION, modules: vec![module], cell_types: vec![ty] };
    };
    let cell_types: Vec<CellType> = body
        .types
        .iter()
        .map(|t| CellType {
            pathways: t.signature & crate::phenotype::PATHWAY_MASK,
            role: role(t).into(),
            expressed_genes: t.expressed_genes,
            pigment_nm: t.pigment_nm,
        })
        .collect();
    let max_depth = body.zones.iter().map(|z| z.depth_cells).fold(0.0, f64::max).max(1e-9);
    let layers: Vec<Layer> =
        body.zones.iter().map(|z| Layer { depth: z.depth_cells / max_depth, weight: z.weight, cell_type: z.cell_type }).collect();
    let (length, width, thickness, profile, symmetry) = match body.shape {
        Shape::Sphere => {
            // Diamètre d'une boule de cellules jointives.
            let d = cell_m * 1.24 * body.cells.dcbrt();
            (d, d, d, Profile::Round, SymmetryKind::Spherical)
        }
        Shape::Filament => (cell_m * body.cells, cell_m, cell_m, Profile::Elongated, SymmetryKind::Radial(2)),
        Shape::Sheet => {
            let side = cell_m * (body.cells / 2.0).sqrt();
            (side, side, 2.0 * cell_m, Profile::Flattened, SymmetryKind::Bilateral)
        }
    };
    let mut pigments = Vec::new();
    for l in &layers {
        pigments.extend(pigments_of(genome, Some(&body.expression[l.cell_type as usize]), l.depth));
    }
    let a = &p.animal;
    let animal = a.motility > 0.0 && a.segments >= 1 && p.engulfment > 0.0 && p.organelles > 0;
    let plant = p.phototroph && a.support > 0.0 && p.organelles > 0;
    // Matériau et revêtement (étape 5) : la matrice de soutien fait une
    // carapace de chitine avec une enveloppe cireuse, un cartilage puis un os
    // sans elle, du bois chez une plante.
    let cuticle = p.desiccation_tolerance > 0.0;
    let (material, covering) = if a.support <= 0.0 {
        (Material::Hydrostatic, Covering::Mucus)
    } else if plant {
        (Material::Wood, if p.is_terrestrial() { Covering::Bark } else { Covering::Wax })
    } else if cuticle && a.support < 1.0 {
        (Material::Chitin, Covering::Cuticle)
    } else if a.support < 1.5 {
        (Material::Cartilage, if p.is_terrestrial() { Covering::Scales } else { Covering::Mucus })
    } else {
        (Material::Bone, if p.is_terrestrial() { Covering::Scales } else { Covering::Mucus })
    };
    // Un animal est allongé (même volume que la boule) et bilatéral.
    let (length, width, thickness, profile, symmetry) = if animal && body.shape == Shape::Sphere {
        (1.6 * length, width / 1.26, thickness / 1.26, Profile::Elongated, SymmetryKind::Bilateral)
    } else {
        (length, width, thickness, profile, symmetry)
    };
    let pathways = p.signature & crate::phenotype::PATHWAY_MASK;
    let trunk = Module {
        id: 0,
        kind: ModuleKind::Body,
        cell_types: (0..body.types.len() as u8).collect(),
        layers,
        master_regulator: genome.regulated_by().into_iter().flatten().next(),
        parent: None,
        attach: [0.0; 3],
        axis: [1.0, 0.0, 0.0],
        offset: [0.0; 3],
        dimensions: Dimensions { length_m: length, width_m: width, thickness_m: thickness, profile, carbon_mol: carbon },
        symmetry: Symmetry { kind: symmetry, copies: 1, spacing: 0.0 },
        joints: Vec::new(),
        material,
        covering,
        pigments,
        structural_colour: None,
        pattern,
        function: Function { pathways, maintenance_kj: p.maintenance_kj, build_kj, sound: a.sound },
    };
    let mut modules = vec![trunk];
    let child = |modules: &[Module], kind, parent: u16, attach, axis, offset, dims: Dimensions, symmetry, joints| Module {
        id: modules.len() as u16,
        kind,
        cell_types: vec![0],
        layers: vec![Layer { depth: 0.0, weight: 1.0, cell_type: 0 }],
        master_regulator: None,
        parent: Some(parent),
        attach,
        axis,
        offset,
        dimensions: dims,
        symmetry,
        joints,
        material,
        covering,
        pigments: Vec::new(),
        structural_colour: None,
        pattern: None,
        function: Function { pathways: 0, maintenance_kj: p.maintenance_kj, build_kj, sound: None },
    };
    if animal {
        // Segments le long de l'axe (un par régulateur), puis les paires
        // d'appendices, attachées aux premiers segments.
        let n = a.segments.max(1) as usize;
        let seg_len = length / n as f64;
        let per = carbon / n as f64;
        let mut segments = Vec::new();
        if n > 1 {
            for k in 0..n {
                let x = 0.5 - (k as f64 + 0.5) / n as f64;
                let dims =
                    Dimensions { length_m: seg_len, width_m: width, thickness_m: thickness, profile: Profile::Round, carbon_mol: per };
                let m = child(
                    &modules,
                    ModuleKind::Segment,
                    0,
                    [x, 0.0, 0.0],
                    [1.0, 0.0, 0.0],
                    [0.0; 3],
                    dims,
                    Symmetry { kind: SymmetryKind::Bilateral, copies: 1, spacing: 0.0 },
                    vec![Joint {
                        kind: JointKind::Flexible,
                        axis: [0.0, 0.0, 1.0],
                        amplitude_rad: 0.3,
                        muscle_section_m2: 0.0,
                        fast_fibres: 0.5,
                    }],
                );
                segments.push(m.id);
                modules.push(m);
            }
        }
        let limb_len = 0.6 * length.min(4.0 * width);
        let muscle = a.motor.min(3.0) * 0.05 * width * width;
        for k in 0..a.limb_pairs as usize {
            let parent = segments.get(k).copied().unwrap_or(0);
            let x = if parent == 0 { 0.4 - 0.8 * (k as f64 + 0.5) / a.limb_pairs as f64 } else { 0.0 };
            let dims = Dimensions {
                length_m: limb_len,
                width_m: 0.15 * width,
                thickness_m: 0.15 * width,
                profile: Profile::Elongated,
                carbon_mol: 0.02 * carbon,
            };
            let m = child(
                &modules,
                ModuleKind::Appendage,
                parent,
                [x, 0.5, -0.3],
                [0.0, 1.0, -1.0],
                [0.0; 3],
                dims,
                Symmetry { kind: SymmetryKind::Bilateral, copies: 2, spacing: 1.0 },
                vec![Joint {
                    kind: JointKind::Hinge,
                    axis: [1.0, 0.0, 0.0],
                    amplitude_rad: 1.0,
                    muscle_section_m2: muscle,
                    fast_fibres: 0.5,
                }],
            );
            modules.push(m);
        }
    } else if plant && p.is_terrestrial() {
        // Feuilles : un étage par régulateur, au plus huit.
        let n = genome.regulated_by().iter().flatten().count().clamp(1, 8);
        for k in 0..n {
            let dims = Dimensions {
                length_m: 0.3 * length,
                width_m: 0.15 * length,
                thickness_m: 2.0 * cell_m,
                profile: Profile::Flattened,
                carbon_mol: 0.05 * carbon,
            };
            let m = child(
                &modules,
                ModuleKind::Leaf,
                0,
                [0.5 - k as f64 / n as f64, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0; 3],
                dims,
                Symmetry { kind: SymmetryKind::Bilateral, copies: 2, spacing: 1.0 },
                Vec::new(),
            );
            modules.push(m);
        }
    }
    BodyPlan { version: BODY_PLAN_VERSION, modules, cell_types }
}

#[cfg(test)]
mod tests {
    use super::*;
    use evo_genetics::{Domain, Gene};

    #[test]
    fn a_colony_plan_lists_its_layers_and_cell_types() {
        use crate::metabolism::*;
        use DomainFamily::*;
        let physio = Physiology::default();
        let gene = |family, efficiency, affinity, nm| Gene {
            domain: Domain { family, efficiency, affinity, t_opt_k: 300.0, t_width_k: 10.0, absorption_nm: nm },
            functional: true,
        };
        let g = Genome::new(
            vec![
                gene(Catalytic(FERMENTATION), 1.0, 1.0, 500.0),
                gene(Signalling, 1.0, 1.0, 500.0),
                gene(Adhesion, 1.2, 1.0, 600.0),
                gene(Regulator, 1.0, 0.5, 500.0),
                gene(Pigment, 1.0, 1.0, 650.0),
                gene(Catalytic(ANOXYGENIC_PHOTOSYNTHESIS), 1.0, 1.0, 500.0),
            ],
            [0; 32],
        );
        let p = Phenotype::from_genome(&g, &physio);
        let plan = body_plan(&g, &p, &physio);
        assert_eq!(plan.modules.len(), 1);
        let m = &plan.modules[0];
        assert_eq!(m.kind, ModuleKind::Body);
        assert_eq!(plan.cell_types.len(), 2);
        assert_eq!(plan.cell_types[0].role, "photosynthétique");
        assert!(m.layers.len() > 1 && m.layers[0].depth == 0.0);
        // Les pigments ne sont que dans les couches du type photosynthétique,
        // pas au centre.
        assert!(!m.pigments.is_empty());
        assert!(m.pigments.iter().all(|x| x.depth < 1.0 && x.absorption_nm == 650.0));
        assert_eq!(m.layers.last().unwrap().cell_type, 1);
        assert!(m.dimensions.length_m > 5e-6);
        // Le plan se sérialise (lu par le client).
        let bytes = bincode::serialize(&plan).expect("sérialisable");
        let back: BodyPlan = bincode::deserialize(&bytes).expect("relisible");
        assert_eq!(back, plan);
    }

    #[test]
    fn an_animal_plan_has_segments_limbs_and_a_voice() {
        use crate::metabolism::*;
        use evo_genetics::Organelle;
        use DomainFamily::*;
        let physio = Physiology::default();
        let gene = |family, efficiency, nm| Gene {
            domain: Domain { family, efficiency, affinity: 1.0, t_opt_k: 300.0, t_width_k: 10.0, absorption_nm: nm },
            functional: true,
        };
        let mut g = Genome::new(
            vec![
                gene(Catalytic(FERMENTATION), 1.0, 500.0),
                gene(Cytoskeleton, 1.0, 500.0),
                gene(Signalling, 1.0, 500.0),
                gene(Adhesion, 1.2, 600.0),
                gene(Regulator, 1.0, 400.0),
                gene(Regulator, 1.0, 400.0),
                gene(Regulator, 1.0, 400.0),
                gene(Motor, 1.0, 500.0),
                gene(Neural, 1.0, 500.0),
                gene(Matrix, 0.8, 500.0),
                gene(Sensor, 0.5, 450.0),
            ],
            [0; 32],
        );
        g.organelles.push(Organelle {
            genes: vec![gene(Catalytic(AEROBIC_RESPIRATION), 1.0, 500.0)],
            origin_lineage: 1,
            acquired_years: 0.0,
        });
        let p = Phenotype::from_genome(&g, &physio);
        assert!(p.animal.non_viable.is_none(), "{:?}", p.animal);
        let plan = body_plan(&g, &p, &physio);
        let count = |k| plan.modules.iter().filter(|m| m.kind == k).count();
        assert_eq!(count(ModuleKind::Segment), 3);
        assert!(count(ModuleKind::Appendage) >= 1);
        assert!(plan.modules.iter().all(|m| m.parent.is_none_or(|q| (q as usize) < plan.modules.len())));
        assert!(plan.modules[0].function.sound.is_some());
        let bytes = bincode::serialize(&plan).expect("sérialisable");
        assert_eq!(bincode::deserialize::<BodyPlan>(&bytes).expect("relisible"), plan);
    }
}
