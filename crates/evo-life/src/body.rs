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
pub const BODY_PLAN_VERSION: u32 = 1;

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
            function: Function { pathways: ty.pathways, maintenance_kj: p.maintenance_kj, build_kj },
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
    let module = Module {
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
        material: Material::Hydrostatic,
        covering: Covering::Mucus,
        pigments,
        structural_colour: None,
        pattern,
        function: Function { pathways: p.signature & crate::phenotype::PATHWAY_MASK, maintenance_kj: p.maintenance_kj, build_kj },
    };
    BodyPlan { version: BODY_PLAN_VERSION, modules: vec![module], cell_types }
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
}
