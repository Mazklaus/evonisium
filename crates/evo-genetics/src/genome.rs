//! Génome à domaines.
//!
//! Un génome est une liste ordonnée de gènes ; chaque gène code une protéine à
//! un domaine, décrit par une famille et des paramètres continus. Le génome ne
//! stocke aucun trait : le chantier Organismes calcule le phénotype à partir
//! des domaines exprimés.
//!
//! [Simplification] Étape 1 : un domaine par protéine, expression constante
//! (pas de sites régulateurs), un chromosome circulaire unique, haploïde.

/// Identifiant d'une réaction du catalogue métabolique (tenu par Organismes).
pub type ReactionId = u8;

/// Famille d'un domaine protéique.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DomainFamily {
    /// Catalyse une réaction du catalogue métabolique.
    Catalytic(ReactionId),
    /// Réparation de l'ADN : abaisse le taux de mutation, coûte de l'énergie.
    Repair,
    /// Défense contre l'oxygène (équivalent catalase, superoxyde dismutase).
    OxidativeDefense,
    /// Pigment captant la lumière (équivalent chlorophylles) : sans lui, une
    /// enzyme de photosynthèse ne sert à rien.
    Pigment,
}

/// Paramètres d'un domaine. Leur sens précis est fixé par le chantier qui les
/// lit ; la mutation les fait varier sans connaître leur effet.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Domain {
    pub family: DomainFamily,
    /// Activité maximale, relative (1 = enzyme typique).
    pub efficiency: f64,
    /// Affinité pour le substrat, relative (1 = typique ; plus haut = capte
    /// mieux les faibles concentrations).
    pub affinity: f64,
    /// Température optimale, K.
    pub t_opt_k: f64,
    /// Largeur de la plage de tolérance thermique, K.
    pub t_width_k: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Gene {
    pub domain: Domain,
    /// Faux une fois devenu pseudogène.
    pub functional: bool,
}

/// Longueur du marqueur neutre, en bases.
pub const MARKER_LEN: usize = 32;

#[derive(Clone, Debug, PartialEq)]
pub struct Genome {
    pub genes: Vec<Gene>,
    /// Séquence neutre (bases 0 à 3) : horloge moléculaire et parenté.
    pub marker: [u8; MARKER_LEN],
}

impl Genome {
    pub fn functional_genes(&self) -> impl Iterator<Item = &Gene> {
        self.genes.iter().filter(|g| g.functional)
    }

    /// Efficacité cumulée des gènes fonctionnels d'une famille.
    pub fn family_efficiency(&self, family: DomainFamily) -> f64 {
        self.functional_genes().filter(|g| g.domain.family == family).map(|g| g.domain.efficiency).sum()
    }

    /// Signature des réactions catalysées par au moins un gène fonctionnel
    /// (un bit par réaction) : c'est elle qui définit la guilde métabolique.
    pub fn reaction_signature(&self) -> u32 {
        self.functional_genes().fold(0, |acc, g| match g.domain.family {
            DomainFamily::Catalytic(r) => acc | (1 << r),
            _ => acc,
        })
    }

    /// Nombre de sites différents entre deux marqueurs neutres.
    pub fn marker_distance(&self, other: &Genome) -> usize {
        self.marker.iter().zip(&other.marker).filter(|(a, b)| a != b).count()
    }

    /// Mémoire occupée, en octets.
    pub fn memory_bytes(&self) -> usize {
        std::mem::size_of::<Genome>() + self.genes.capacity() * std::mem::size_of::<Gene>()
    }
}
