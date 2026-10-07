//! Génome à domaines.
//!
//! Un génome est une liste ordonnée de gènes ; chaque gène code une protéine à
//! un domaine, décrit par une famille et des paramètres continus. Le génome ne
//! stocke aucun trait : le chantier Organismes calcule le phénotype à partir
//! des domaines exprimés.
//!
//! [Simplification] Un domaine par protéine, expression constante (pas de
//! sites régulateurs), un chromosome circulaire unique, haploïde.

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
    /// Pigment de la famille des porphyrines (équivalent chlorophylles) :
    /// seul, il protège des ultraviolets ; couplé à une chaîne de transport
    /// d'électrons ou à un centre réactionnel, il capte la lumière.
    Pigment,
    /// Transporteur d'électrons à hème (équivalent cytochromes), porphyrine
    /// lui aussi : seul, il améliore le rendement des voies chimiques.
    Cytochrome,
    /// Pompe à protons activée par la lumière (équivalent rhodopsine
    /// microbienne) : une protéine, un peu d'énergie, pas de carbone fixé.
    Rhodopsin,
    /// Complexe à manganèse qui oxyde l'eau : seul, il détruit les espèces
    /// réactives de l'oxygène (équivalent catalase à manganèse).
    WaterOxidation,
}

/// Parenté déclarée entre familles de domaines : une copie d'un domaine
/// `from` peut diverger vers `to` avec le poids relatif `weight`. La table est
/// tenue par le chantier qui donne leur sens aux familles (Organismes).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DomainRelation {
    pub from: DomainFamily,
    pub to: DomainFamily,
    pub weight: f64,
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
    /// Pic du spectre d'absorption, nm. Il ne compte que pour les familles
    /// qui captent la lumière (pigments, rhodopsines, cytochromes) : il donne
    /// à la fois la couleur affichée et le rendement sous l'étoile. Ailleurs il
    /// dérive librement, variation cachée disponible pour l'exaptation.
    pub absorption_nm: f64,
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

/// Pourquoi un génome a changé : les huit causes du document Génétique
/// (« Une seule porte d'entrée pour modifier un génome »). L'étape 1 n'utilise
/// que la mutation spontanée ; les autres sont réservées.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GenomeChangeCause {
    SpontaneousMutation(crate::mutation::MutationKind),
    InducedMutation(crate::mutation::MutationKind),
    Recombination,
    HorizontalTransfer,
    Endosymbiosis,
    Accelerator,
    ArtificialSelection,
    SocietyTechnique,
}

pub const GENOME_CHANGE_CAUSE_COUNT: usize = 8;

impl GenomeChangeCause {
    /// Rang de la cause, pour les compteurs par cause.
    pub fn index(self) -> usize {
        match self {
            Self::SpontaneousMutation(_) => 0,
            Self::InducedMutation(_) => 1,
            Self::Recombination => 2,
            Self::HorizontalTransfer => 3,
            Self::Endosymbiosis => 4,
            Self::Accelerator => 5,
            Self::ArtificialSelection => 6,
            Self::SocietyTechnique => 7,
        }
    }
}

impl GenomeChangeCause {
    pub fn label(self) -> &'static str {
        match self {
            Self::SpontaneousMutation(_) => "mutation spontanée",
            Self::InducedMutation(_) => "mutation induite",
            Self::Recombination => "recombinaison",
            Self::HorizontalTransfer => "transfert horizontal",
            Self::Endosymbiosis => "endosymbiose",
            Self::Accelerator => "accélérateur",
            Self::ArtificialSelection => "sélection artificielle",
            Self::SocietyTechnique => "technique d'une société",
        }
    }
}

/// Élément modifié par un changement de génome (pour le journal).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ChangedElement {
    /// Paramètre d'un gène existant modifié, ou gène devenu pseudogène.
    Gene { index: u16, family: DomainFamily },
    /// Gène ajouté (duplication, divergence, de novo, transfert).
    Inserted { index: u16, family: DomainFamily },
    /// Gène retiré.
    Removed { index: u16, family: DomainFamily },
    /// Site du marqueur neutre.
    Marker { site: u16 },
    /// Plusieurs éléments à la fois (double mutant d'un tunnel).
    Several,
}

/// Génome dérivé d'un autre, avec la cause du changement et l'élément touché.
#[derive(Clone, Debug, PartialEq)]
pub struct GenomeChange {
    pub genome: Genome,
    pub cause: GenomeChangeCause,
    pub element: ChangedElement,
}

impl Genome {
    /// Interface unique de modification d'un génome : toute transformation
    /// passe par ici et en note la cause. C'est l'un des points d'accroche
    /// prévus par le document Vision pour étendre le moteur sans refonte.
    ///
    /// La fermeture applique le changement et renvoie l'élément touché.
    ///
    /// [Simplification] Chez les microbes, seules les modifications qui se
    /// fixent entrent au journal détaillé ([`crate::journal::GenomeJournal`]) ;
    /// les autres ne sont que des candidats évalués.
    pub fn derive(&self, cause: GenomeChangeCause, change: impl FnOnce(&mut Genome) -> ChangedElement) -> GenomeChange {
        let mut genome = self.clone();
        let element = change(&mut genome);
        GenomeChange { genome, cause, element }
    }
}
