//! Ce que le son lit de la session : le monde publié et les événements de
//! la chronique, avec son propre curseur (la chronique du client garde le
//! sien), et ce que le narrateur en raconte (moments clés, scène, fiche
//! d'espèce). Lecture seule.

use super::*;
use evo_son::recit::{self, Chemin, Espece, Fait, Lieu, Metab, Presence, Tendance};

/// Monde publié, familles et scores des nouveaux événements, ce que le
/// narrateur peut en raconter, et l'ère la plus avancée qu'ils annoncent.
pub(crate) type Ecoute = (evo_son::Monde, Vec<(&'static str, f64)>, Vec<Recit>, Option<u8>);

/// Ère du fond musical (`evo_son::fond::ERES`) qu'annonce un événement ; les
/// mêmes jalons que les chapitres de la chronique (`evo_view::story`).
/// L'âge des géants et les premiers peuples n'ont pas encore de jalon dans
/// le moteur.
fn ere(e: &Event) -> Option<u8> {
    match &e.kind {
        EventKind::LifeSeeded { .. } => Some(0),
        EventKind::OxygenThreshold { rising: true, .. } => Some(1),
        EventKind::Innovation { pathway, stage, .. } if pathway.as_ref() == evo_sim::world::COMPLEXITY_PATHWAY => match stage {
            0..2 => None,
            2..5 => Some(2),
            5..8 => Some(3),
            _ => Some(4),
        },
        _ => None,
    }
}

impl EvoSession {
    /// Monde publié et familles des événements arrivés depuis `cursor`
    /// (avec leur score d'intérêt). Au premier appel d'une partie, l'histoire
    /// déjà écrite est sautée : seuls les nouveaux événements sonnent.
    pub(crate) fn sound_state(&self, cursor: &mut Option<usize>) -> Option<Ecoute> {
        let g = self.game.as_ref()?;
        let f = self.frame()?;
        let st = &f.state;
        let gl = &st.globals;
        let monde = evo_son::Monde {
            o2: gl.o2_mixing,
            temperature_k: gl.mean_temperature_k,
            ice: gl.ice_fraction,
            ocean: gl.ocean_fraction,
            lineages: gl.living_lineages as u32,
            paused: st.paused,
        };
        let n = g.events.len();
        let from = match *cursor {
            Some(c) if c <= n => c,
            _ => n,
        };
        // Au premier appel, toute l'histoire dit l'ère où l'on en est.
        let ere = g.events[if cursor.is_some() { from } else { 0 }..].iter().filter_map(ere).max();
        // Au plus 50 par image : un rattrapage ne doit pas tout faire sonner.
        let fresh = &g.events[from.max(n.saturating_sub(50))..];
        let events = fresh.iter().map(|e| (Family::of(e).key(), e.interest)).collect();
        let recits = self.recits(fresh);
        *cursor = Some(n);
        Some((monde, events, recits, ere))
    }
}

/// Métabolisme principal d'une signature, dans les mots du narrateur.
fn metab(r: evo_genetics::ReactionId) -> Option<Metab> {
    use evo_life::metabolism::*;
    Some(match r {
        OXYGENIC_PHOTOSYNTHESIS => Metab::PhotoOxygene,
        ANOXYGENIC_PHOTOSYNTHESIS => Metab::PhotoSoufre,
        PHOTOFERROTROPHY => Metab::PhotoFer,
        MANGANESE_PHOTOTROPHY => Metab::PhotoManganese,
        METHANOGENESIS => Metab::Methanogene,
        METHANOTROPHY => Metab::Methanotrophe,
        SULFATE_REDUCTION => Metab::Sulfato,
        AEROBIC_RESPIRATION => Metab::Respiration,
        FERMENTATION => Metab::Fermentation,
        _ => return None,
    })
}

/// Tous les métabolismes d'une signature, le principal en tête.
fn metabs(signature: u32) -> Vec<Metab> {
    let main = main_reaction(signature);
    let mut v: Vec<Metab> = main.and_then(metab).into_iter().collect();
    for r in 0..32 {
        if signature & (1 << r) != 0 && Some(r as evo_genetics::ReactionId) != main {
            if let Some(m) = metab(r as evo_genetics::ReactionId) {
                v.push(m);
            }
        }
    }
    v
}

/// Ce que le narrateur peut raconter d'un événement de la chronique.
pub(crate) struct Recit {
    pub fait: Fait,
    pub years: f64,
    pub id: u64,
    pub interest: f64,
}

impl EvoSession {
    fn region(&self, cell: Option<u32>) -> String {
        let (Some(g), Some(game)) = (&self.grid, &self.game) else { return "nulle part".into() };
        let c = (cell.unwrap_or(0) as usize).min(g.len().saturating_sub(1));
        evo_view::naming::place_name(game.seed, evo_view::naming::region_of(g, c))
    }

    /// Faits racontables parmi les événements donnés (familles du narrateur).
    pub(crate) fn recits(&self, events: &[Event]) -> Vec<Recit> {
        events
            .iter()
            .filter_map(|e| {
                let fait = match &e.kind {
                    EventKind::LifeSeeded { .. } => Fait::Vie { region: self.region(e.cell) },
                    EventKind::Innovation { pathway, stage, .. } => {
                        let chemin = match pathway.as_ref() {
                            evo_life::metabolism::PHOTOSYNTHESIS_PATHWAY => Chemin::Photosynthese,
                            evo_life::metabolism::RHODOPSIN_PATHWAY => Chemin::Rhodopsine,
                            evo_sim::world::COMPLEXITY_PATHWAY => Chemin::Complexite,
                            _ => return None,
                        };
                        Fait::Innovation { chemin, etape: *stage, region: self.region(e.cell) }
                    }
                    EventKind::OxygenThreshold { mixing_ratio, rising } => Fait::Oxygene { montee: *rising, ratio: *mixing_ratio },
                    EventKind::Snowball { starts, .. } => Fait::Glaciation { debut: *starts },
                    EventKind::PlateReorganisation { .. } => Fait::Plaques,
                    _ => return None,
                };
                Some(Recit { fait, years: e.years, id: e.id, interest: e.interest })
            })
            .collect()
    }

    /// Un lieu de la planète publiée, avec le détail du moteur s'il est déjà
    /// là (le son ne demande jamais de détail lui-même : l'inspecteur s'en
    /// charge, et deux demandes se disputeraient la même place).
    fn lieu(&self, f: &Frame, cell: usize) -> Option<(Lieu, Vec<Presence>)> {
        let c = f.cells().get(cell)?;
        let detail = self.game.as_ref().and_then(|g| g.cell.as_ref()).map(|(d, _)| d).filter(|d| d.cell as usize == cell);
        let lieu = Lieu {
            region: self.region(Some(cell as u32)),
            mer: c.is_ocean,
            lac: detail.is_some_and(|d| !d.is_ocean && d.lake_fraction > 0.3),
            glace: c.ice_cover > 0.5,
            temperature_c: c.temperature_k as f64 - 273.15,
            hauteur_m: c.elevation_m as f64,
            source: detail.is_some_and(|d| d.vent_h2 + d.vent_h2s + d.vent_fe > 0.0),
            lumiere: c.light_w_m2 as f64,
        };
        let mut presences: Vec<Presence> = Vec::new();
        if let Some(d) = detail {
            let total: f32 = d.populations.iter().map(|p| p.biomass).sum();
            for p in &d.populations {
                let nom = f.species(p.species).map_or_else(String::new, |_| self.scientific(f, p.species));
                match presences.iter_mut().find(|q| q.nom == nom) {
                    Some(q) => q.part += (p.biomass / total.max(1e-30)) as f64,
                    None => presences.push(Presence {
                        nom,
                        metab: main_reaction(p.species).and_then(metab),
                        part: (p.biomass / total.max(1e-30)) as f64,
                    }),
                }
            }
            presences.retain(|p| !p.nom.is_empty());
        }
        Some((lieu, presences))
    }

    fn scientific(&self, f: &Frame, species: u32) -> String {
        let (Some(g), Some(s)) = (&self.grid, f.species(species)) else { return String::new() };
        evo_view::naming::scientific_name(f.planet.seed, g, s.signature, s.origin_bio_cell)
    }

    /// Ce que le narrateur dit d'un lieu (option « scène »).
    pub(crate) fn recit_lieu(&self, cell: usize, tirage: u64) -> Option<Vec<String>> {
        let f = self.frame()?;
        let (lieu, presences) = self.lieu(&f, cell)?;
        Some(recit::scene(&lieu, &presences, tirage))
    }

    /// Ce que le narrateur dit d'une espèce (option « fiche »).
    pub(crate) fn recit_espece(&mut self, species: u32, tirage: u64) -> Option<Vec<String>> {
        let f = self.frame()?;
        let grid = self.grid.clone()?;
        let sv = f.species(species)?;
        let lineages = self.lineages();
        let s = evo_view::species::summary(&f, &grid, sv, &lineages, self.lang);
        let tendance =
            match evo_view::species::Status::from_series(self.watch.get(&s.species).map_or(&[][..], |v| &v[..]), s.range_cells == 0) {
                evo_view::species::Status::Expanding => Tendance::Expansion,
                evo_view::species::Status::Declining => Tendance::Declin,
                evo_view::species::Status::Stable => Tendance::Stable,
                evo_view::species::Status::Extinct => Tendance::Inconnue,
            };
        let parent = s.parent_lineage.and_then(|p| Self::lineage(&lineages, p)).map(|l| l.signature).filter(|&p| p != s.species);
        // La cellule physique où l'espèce est la plus abondante.
        let peak = f.cells().iter().position(|c| c.bio_cell == s.peak_cell).unwrap_or(0);
        let origin = f.cells().iter().position(|c| c.bio_cell == s.origin_cell).unwrap_or(peak);
        let e = Espece {
            nom: s.scientific.clone(),
            metabolismes: metabs(sv.signature),
            pigmentee: s.pigment_rgb.is_some(),
            age_ans: s.born_years.map(|y| f.state.years - y),
            region_origine: self.region(Some(origin as u32)),
            parent: parent.map(|p| self.scientific(&f, p)).filter(|n| !n.is_empty()),
            aire: s.range_share as f64,
            tendance,
            ecotypes: s.ecotypes,
        };
        let (lieu, voisins) = self.lieu(&f, peak)?;
        Some(recit::fiche(&e, &lieu, &voisins, tirage))
    }
}

/// Clés des techniques connues (catalogue d'`evo-society`).
fn cles(k: &evo_society::catalog::Known) -> Vec<String> {
    evo_society::catalog::catalog().techniques.iter().filter(|t| k.has(t.id)).map(|t| t.key.clone()).collect()
}

impl EvoSession {
    /// Le peuple d'une société pour le son : les mots de sa langue (état
    /// culturel de l'étape 6, s'il est prêt), sa taille, ses techniques.
    fn peuple_du_son(&mut self, people: u32, population: f64, techniques: Vec<String>) -> evo_son::peuples::Peuple {
        let mut langue = evo_son::peuples::Langue { mots: Vec::new(), cle: people as u64 };
        let mut nom = String::new();
        if let Some(st) = self.culture() {
            if let Some(p) = st.peoples.iter().find(|p| p.id == people) {
                nom = p.name.clone();
                if let Some(l) = p.language.and_then(|id| st.languages.iter().find(|l| l.id == id)) {
                    langue = evo_son::peuples::Langue { mots: l.words.iter().map(|w| w.1.clone()).collect(), cle: l.id as u64 };
                }
            }
        }
        evo_son::peuples::Peuple {
            nom,
            langue,
            rites: Vec::new(),
            population: population as f32,
            taille: 1.0,
            cle: people as u64,
            techniques,
        }
    }

    /// La ville (ou le peuplement) d'une cellule, pour le son des villes
    /// (livraison 3) ; `None` si personne n'y vit en ville. Lecture seule de
    /// l'état des sociétés publié.
    pub(crate) fn ville_du_son(&mut self, cell: i64) -> Option<evo_son::villes::Ville> {
        let st = self.society_state()?;
        let c = st.cells.iter().find(|c| c.cell as i64 == cell)?;
        if !c.sedentary || c.urban <= 0.0 {
            return None;
        }
        let s = st.societies.iter().find(|s| s.id == c.society)?;
        let institution = |i: evo_society::society::Institution| s.institutions[i as usize] as f32;
        let city = c.city.and_then(|id| st.cities.iter().find(|x| x.id == id));
        let techniques = cles(&c.known);
        let peuple = self.peuple_du_son(s.people, s.agg.population, techniques.clone());
        Some(evo_son::villes::Ville {
            nom: city.map_or_else(|| s.name.clone(), |x| x.name.clone()),
            cle: city.map_or(1 << 32 | c.cell as u64, |x| x.id as u64),
            population: c.pop as f32,
            urbain: c.urban as f32,
            techniques,
            betail: (c.livestock / c.pop.max(1.0)) as f32,
            temple: institution(evo_society::society::Institution::Temple),
            marche: institution(evo_society::society::Institution::Market),
            guerre: s.at_war,
            peuple,
        })
    }

    /// Techniques de la société la plus peuplée d'un peuple (étape 7), pour
    /// les instruments de ses chants ; vide sans sociétés.
    pub(crate) fn techniques_du_peuple(&self, people: i64) -> Vec<String> {
        let Some(st) = self.society_state() else { return Vec::new() };
        st.societies
            .iter()
            .filter(|s| s.people as i64 == people)
            .max_by(|a, b| a.agg.population.total_cmp(&b.agg.population))
            .map_or_else(Vec::new, |s| cles(&s.known))
    }
}
