//! Sons des villes (livraison 3, volet villes, étape 7) : la rumeur d'une
//! ville que l'on regarde de près sur le globe, faite de ce que ses gens
//! savent faire.
//!
//! Le son lit l'état des sociétés publié (`evo_society::state::SocietyState`,
//! version 1) par l'intermédiaire du client, qui lui passe une [`Ville`] :
//! effectif, part urbaine, techniques connues (clés du catalogue), bétail,
//! force du temple et du marché, guerre, et le peuple (sa langue, sa
//! tradition musicale). Rien n'en remonte au moteur.
//!
//! Chaque technique ajoute sa couche : bêtes et sabots, charrettes, forges,
//! chantiers, cloches du temple, crieurs et monnaie du marché, moulins,
//! machines à vapeur, trains, trafic, avions, et un musicien de rue qui joue
//! avec les instruments que permettent les techniques du peuple. Les couches
//! d'un âge s'effacent quand le suivant les remplace (plus de charrettes ni
//! de sabots dans une ville d'automobiles).
//!
//! Dosage, comme pour la faune : un fond de foule et de trafic, puis un
//! événement à la fois ou presque, tiré parmi les couches présentes, jamais
//! deux qui démarrent ensemble ; chaque couche se repose après avoir sonné.
//!
//! [Simplification] Pas de jour ni de nuit, pas de météo ; la ville sonne de
//! la même façon à toute heure.

use crate::faune::{self, Genre, Heure, Organe};
use crate::peuples::{self, Instrument, Locuteur, Peuple, Rumeur, Tradition};
use crate::synth::{Bed, Bus, FilterKind, Make, Rng, Timbre, Voice};

/// Ce que le son sait d'une ville (ou d'une cellule habitée).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Ville {
    pub nom: String,
    /// Identifiant stable (ville, ou cellule sans ville).
    pub cle: u64,
    /// Âmes dans la cellule.
    pub population: f32,
    /// Part urbaine, de 0 à 1.
    pub urbain: f32,
    /// Clés des techniques que connaît la cellule.
    pub techniques: Vec<String>,
    /// Bêtes par habitant (cheptel de la cellule rapporté à son effectif).
    pub betail: f32,
    /// Force du temple et du marché de la société, de 0 à 1.
    pub temple: f32,
    pub marche: f32,
    pub guerre: bool,
    /// Le peuple de la ville : sa langue pour les voix, sa tradition pour la
    /// musique. Ses techniques sont celles de la ville.
    pub peuple: Peuple,
}

impl Ville {
    pub fn a(&self, cle: &str) -> bool {
        self.techniques.iter().any(|t| t == cle)
    }
    fn un_de(&self, cles: &[&str]) -> bool {
        cles.iter().any(|c| self.a(c))
    }
    /// Gens en ville.
    pub fn citadins(&self) -> f32 {
        (self.population * self.urbain.clamp(0.0, 1.0)).max(0.0)
    }
    /// Taille ressentie de 0 (hameau) à 1 (métropole) : logarithme des gens
    /// en ville, de cent à dix millions.
    pub fn taille(&self) -> f32 {
        ((self.citadins().max(1.0).log10() - 2.0) / 5.0).clamp(0.0, 1.0)
    }
    pub fn moderne(&self) -> bool {
        self.un_de(&["automobile", "moteur_combustion"])
    }
    pub fn industrielle(&self) -> bool {
        self.un_de(&["machine_vapeur", "locomotive", "filature_mecanique", "pompe_vapeur"])
    }
}

/// Une couche de la ville : un genre d'événement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Couche {
    Betes,
    Sabots,
    Charrette,
    Forge,
    Chantier,
    Cloche,
    Crieur,
    Moulin,
    Vapeur,
    Train,
    Voiture,
    Klaxon,
    Avion,
    Musicien,
    Guerre,
}

impl Couche {
    pub const TOUTES: [Couche; 15] = [
        Couche::Betes,
        Couche::Sabots,
        Couche::Charrette,
        Couche::Forge,
        Couche::Chantier,
        Couche::Cloche,
        Couche::Crieur,
        Couche::Moulin,
        Couche::Vapeur,
        Couche::Train,
        Couche::Voiture,
        Couche::Klaxon,
        Couche::Avion,
        Couche::Musicien,
        Couche::Guerre,
    ];
    pub fn nom(self) -> &'static str {
        match self {
            Couche::Betes => "bêtes",
            Couche::Sabots => "sabots",
            Couche::Charrette => "charrettes",
            Couche::Forge => "forge",
            Couche::Chantier => "chantier",
            Couche::Cloche => "cloche du temple",
            Couche::Crieur => "crieurs du marché",
            Couche::Moulin => "moulin",
            Couche::Vapeur => "machines à vapeur",
            Couche::Train => "train",
            Couche::Voiture => "voitures",
            Couche::Klaxon => "klaxons",
            Couche::Avion => "avion",
            Couche::Musicien => "musicien de rue",
            Couche::Guerre => "guerre",
        }
    }

    /// Poids de la couche dans cette ville (0 : absente) et repos après
    /// qu'elle a sonné, s.
    pub fn poids(self, v: &Ville) -> (f32, f32) {
        let moderne = v.moderne();
        let indus = v.industrielle();
        let t = v.taille();
        match self {
            Couche::Betes if v.betail > 0.02 && !moderne => ((v.betail * 4.0).min(1.0) * (1.0 - 0.6 * t), 20.0),
            Couche::Sabots if v.un_de(&["cheval", "equitation"]) && !moderne => (0.6, 8.0),
            Couche::Charrette if v.un_de(&["roue", "chariot"]) && !moderne => (0.5, 10.0),
            Couche::Forge if v.un_de(&["cuivre_martele", "fonte_cuivre", "bronze", "fonte_fer"]) && !indus => (0.6, 15.0),
            Couche::Chantier if v.un_de(&["brique_crue", "urbanisme", "beton_romain", "ciment_portland"]) => (0.35 + 0.3 * t, 20.0),
            Couche::Cloche if v.temple > 0.3 && v.un_de(&["bronze", "fonte_fer"]) => (0.25 * v.temple, 120.0),
            Couche::Crieur if (v.marche > 0.3 || v.un_de(&["biens_monnaie", "pieces"])) && !moderne => (0.5 + 0.4 * v.marche, 12.0),
            Couche::Moulin if v.un_de(&["moulin_eau", "moulin_vent"]) && !indus => (0.4, 20.0),
            Couche::Vapeur if indus && !moderne => (0.7, 10.0),
            Couche::Train if v.a("locomotive") => (if moderne { 0.25 } else { 0.4 }, 60.0),
            Couche::Voiture if moderne => (0.9, 4.0),
            Couche::Klaxon if v.a("automobile") => (0.3, 15.0),
            Couche::Avion if v.a("aviation") => (0.2, 90.0),
            Couche::Musicien => (0.35, 45.0),
            Couche::Guerre if v.guerre => (0.5, 25.0),
            _ => (0.0, 0.0),
        }
    }
}

/// Écart minimal entre les débuts de deux événements, s.
pub const ECART: f32 = 2.0;
/// Événements qui sonnent ensemble au plus.
pub const ENSEMBLE: usize = 2;

/// La ville que l'on écoute : fonds continus et événements.
pub struct Rue {
    pub ville: Ville,
    /// De 0 (loin, ou rien) à 1 (au plus près).
    pub proche: f32,
    /// Faux quand on a quitté le globe : les fonds se taisent.
    actif: bool,
    foule: Bed,
    trafic: Bed,
    rumeur: Rumeur,
    tradition: Tradition,
    horloge: f32,
    libre: f32,
    /// Fin des événements en cours.
    en_cours: Vec<f32>,
    repos: [f32; Couche::TOUTES.len()],
}

impl Rue {
    pub fn new(ville: Ville, proche: f32, rate: f32, rng: &mut Rng) -> Self {
        let mut peuple = ville.peuple.clone();
        peuple.techniques = ville.techniques.clone();
        let tradition = Tradition::de(peuple.cle).avec_techniques(&peuple.techniques, peuple.cle);
        let mut r = Self {
            foule: Bed::new(true, FilterKind::BandPass, 520.0, 0.8, rate).breathe_freq(0.09, 90.0).breathe_gain(0.13, 0.35),
            trafic: Bed::new(true, FilterKind::LowPass, 170.0, 0.7, rate).breathe_gain(0.05, 0.4),
            rumeur: Rumeur::new(peuple, rng),
            tradition,
            ville,
            proche,
            actif: true,
            horloge: 0.0,
            libre: rng.range(1.0, 3.0),
            en_cours: Vec::new(),
            repos: [0.0; Couche::TOUTES.len()],
        };
        r.regler();
        r
    }

    /// La même ville, lue à nouveau (elle a grandi, appris une technique).
    pub fn maj(&mut self, ville: Ville, proche: f32) {
        if ville.techniques != self.ville.techniques {
            self.tradition = Tradition::de(ville.peuple.cle).avec_techniques(&ville.techniques, ville.peuple.cle);
        }
        self.rumeur.peuple.population = ville.peuple.population;
        self.rumeur.peuple.techniques = ville.techniques.clone();
        self.ville = ville;
        self.proche = proche;
        self.regler();
    }

    /// On écoute la ville (sur le globe) ou non.
    pub fn ecoute(&mut self, on: bool) {
        if on != self.actif {
            self.actif = on;
            self.regler();
        }
    }

    fn regler(&mut self) {
        let t = self.ville.taille();
        let p = if self.actif { self.proche } else { 0.0 };
        // La foule d'avant les moteurs ; le trafic la couvre ensuite.
        let moderne = self.ville.moderne();
        self.foule.gain.target = p * (0.004 + 0.022 * t) * if moderne { 0.5 } else { 1.0 };
        self.trafic.gain.target = if moderne { p * (0.02 + 0.06 * t) } else { 0.0 };
    }

    #[inline]
    pub fn run(&mut self, rng: &mut Rng, dt: f32, rate: f32) -> f32 {
        self.foule.run(rng, dt, rate) + self.trafic.run(rng, dt, rate)
    }

    /// Avance de `dt` ; `parleurs` : voix parlées qui sonnent déjà.
    pub fn tick(&mut self, dt: f32, parleurs: usize, mk: &Make, rng: &mut Rng, out: &mut Vec<Voice>) {
        self.horloge += dt;
        for r in self.repos.iter_mut() {
            *r -= dt;
        }
        let h = self.horloge;
        self.en_cours.retain(|&f| f > h);
        if self.proche <= 0.01 {
            return;
        }
        // Des gens qui se parlent, plus souvent dans une grande ville.
        let voix = (0.3 + 0.7 * self.ville.taille()) * self.proche * if self.ville.moderne() { 0.5 } else { 1.0 };
        self.rumeur.tick(dt, voix, mk, rng, out, parleurs);
        if h < self.libre || self.en_cours.len() >= ENSEMBLE {
            return;
        }
        // Une ville plus grande a plus à dire.
        self.libre = h + ECART + rng.range(1.0, 7.0) * (1.3 - self.ville.taille());
        let poids: Vec<f32> =
            Couche::TOUTES.iter().enumerate().map(|(k, c)| if self.repos[k] > 0.0 { 0.0 } else { c.poids(&self.ville).0 }).collect();
        let total: f32 = poids.iter().sum();
        if total <= 0.0 {
            return;
        }
        let mut x = rng.range(0.0, total);
        let k = poids.iter().position(|&w| {
            x -= w;
            x <= 0.0
        });
        let k = k.unwrap_or(poids.len() - 1);
        let c = Couche::TOUTES[k];
        self.repos[k] = c.poids(&self.ville).1 * rng.range(0.7, 1.3);
        let duree = self.evenement(c, mk, rng, out);
        self.en_cours.push(h + duree);
    }

    /// Lance un événement de la couche `c` ; renvoie sa durée.
    pub fn evenement(&mut self, c: Couche, mk: &Make, rng: &mut Rng, out: &mut Vec<Voice>) -> f32 {
        let r = mk.rate;
        let g = self.proche;
        let pan = rng.bipolar() * 0.8;
        let mut push = |v: Voice, at: f32| out.push(v.at(at, r));
        match c {
            Couche::Betes => {
                // Une bête du troupeau : mugissement, bêlement.
                let o = Organe {
                    genre: Genre::Larynx,
                    taille_m: rng.range(0.05, 0.35),
                    puissance: 0.6,
                    complexite: 0.2,
                    choeur: false,
                    heure: Heure::Jour,
                    graine: self.ville.cle ^ (rng.u() * 3.0) as u64,
                };
                let mut v = Vec::new();
                faune::appel(&o, rng.range(8.0, 25.0), pan, 0.0, false, mk, rng, &mut v);
                for x in v {
                    let x = Voice { gain: x.gain * g * 2.0, ..x };
                    push(x, 0.0);
                }
                faune::duree(&o)
            }
            Couche::Sabots => {
                // Un cheval qui passe au pas : deux sons alternés.
                let n = 8 + (rng.u() * 10.0) as usize;
                let pas = rng.range(0.22, 0.32);
                for i in 0..n {
                    let f = if i % 2 == 0 { 1100.0 } else { 850.0 } * rng.range(0.95, 1.05);
                    let enveloppe = (i as f32 / n as f32 * std::f32::consts::PI).sin();
                    let p = pan + (i as f32 / n as f32 - 0.5) * 0.6;
                    push(
                        Voice::new(
                            mk.burst(FilterKind::BandPass, f, f * 0.8, 3.0, 0.002, 0.06, false),
                            Bus::Ambiance,
                            0.22 * g * enveloppe,
                        )
                        .pan(p)
                        .send(0.25),
                        i as f32 * pas + if i % 2 == 1 { 0.04 } else { 0.0 },
                    );
                }
                n as f32 * pas
            }
            Couche::Charrette => {
                // Roues sur la pierre et essieu qui grince.
                let len = rng.range(3.0, 6.0);
                push(
                    Voice::new(mk.burst(FilterKind::LowPass, 260.0, 180.0, 0.9, len * 0.4, len, true), Bus::Ambiance, 0.1 * g)
                        .pan(pan)
                        .send(0.2),
                    0.0,
                );
                let mut t = rng.range(0.3, 0.8);
                while t < len - 0.5 {
                    let f = rng.range(380.0, 520.0);
                    push(Voice::new(mk.trille(f, f * 0.85, 9.0, 0.3, 0.4, 0.6, 0.35, 0.05), Bus::Ambiance, 0.03 * g).pan(pan).send(0.2), t);
                    t += rng.range(0.6, 1.4);
                }
                len
            }
            Couche::Forge => {
                // Le marteau sur l'enclume, par groupes de trois à sept coups.
                let n = 3 + (rng.u() * 5.0) as usize;
                let pas = rng.range(0.45, 0.65);
                let m = rng.range(86.0, 92.0);
                for i in 0..n {
                    let at = i as f32 * pas;
                    push(Voice::new(mk.bell(m, 3.17, 1.6, 0.5), Bus::Ambiance, 0.03 * g).pan(pan).send(0.3), at);
                    push(Voice::new(mk.thump(220.0, 0.06), Bus::Ambiance, 0.07 * g).pan(pan), at);
                }
                n as f32 * pas + 0.5
            }
            Couche::Chantier => {
                // Maillets sur le bois, puis une scie qui va et vient.
                let n = 3 + (rng.u() * 4.0) as usize;
                for i in 0..n {
                    push(Voice::new(mk.thump(rng.range(150.0, 190.0), 0.12), Bus::Ambiance, 0.1 * g).pan(pan).send(0.3), i as f32 * 0.55);
                    push(
                        Voice::new(mk.burst(FilterKind::BandPass, 700.0, 500.0, 1.5, 0.001, 0.05, false), Bus::Ambiance, 0.05 * g).pan(pan),
                        i as f32 * 0.55,
                    );
                }
                let mut t = n as f32 * 0.55 + rng.range(0.5, 1.5);
                for k in 0..(4 + (rng.u() * 5.0) as usize) {
                    let (a, b) = if k % 2 == 0 { (1800.0, 2600.0) } else { (2600.0, 1800.0) };
                    push(
                        Voice::new(mk.burst(FilterKind::BandPass, a, b, 2.5, 0.12, 0.34, false), Bus::Ambiance, 0.025 * g)
                            .pan(-pan)
                            .send(0.2),
                        t,
                    );
                    t += 0.36;
                }
                t
            }
            Couche::Cloche => {
                // Une cloche grave du temple, lointaine, deux ou trois coups.
                let n = 2 + (rng.u() * 2.0) as usize;
                let m = rng.range(48.0, 55.0);
                for i in 0..n {
                    push(Voice::new(mk.bell(m, 2.0, 1.1, 7.0), Bus::Ambiance, 0.03 * g).pan(pan * 0.5).send(0.7), i as f32 * 3.2);
                }
                n as f32 * 3.2 + 4.0
            }
            Couche::Crieur => {
                // Un marchand appelle, des pièces tintent.
                let p = &self.rumeur.peuple;
                let reserve = p.langue.syllabes();
                let loc = Locuteur::tire(rng, if p.taille > 0.0 { p.taille } else { 1.0 });
                let mots: Vec<_> = (0..(1 + (rng.u() * 2.0) as usize)).map(|_| p.langue.babil(rng, &reserve)).collect();
                let mut segs = peuples::phrase(&mots, loc, p.langue.accent(), rng);
                for s in segs.iter_mut() {
                    s.f0 *= 1.35;
                    s.dur *= 1.3;
                }
                let gosier = peuples::Gosier::new(segs, loc, 0.08, r);
                let d = gosier.duree();
                push(Voice::new(crate::synth::Kind::Gosier(Box::new(gosier)), Bus::Ambiance, 0.2 * g).pan(pan).send(0.35), 0.0);
                if self.ville.un_de(&["pieces", "biens_monnaie"]) {
                    let t0 = d + rng.range(0.3, 1.0);
                    for i in 0..(2 + (rng.u() * 3.0) as usize) {
                        push(
                            Voice::new(mk.bell(rng.range(98.0, 102.0), 3.7, 0.8, 0.15), Bus::Ambiance, 0.025 * g).pan(-pan * 0.5),
                            t0 + i as f32 * 0.09,
                        );
                    }
                }
                d + 1.5
            }
            Couche::Moulin => {
                // La roue du moulin qui grince, l'eau qui retombe.
                let tours = 3 + (rng.u() * 3.0) as usize;
                for i in 0..tours {
                    let t = i as f32 * 2.2;
                    push(Voice::new(mk.trille(170.0, 140.0, 6.0, 0.2, 0.3, 0.8, 0.9, 0.15), Bus::Ambiance, 0.04 * g).pan(pan).send(0.3), t);
                    push(
                        Voice::new(mk.burst(FilterKind::BandPass, 1400.0, 900.0, 0.7, 0.2, 1.4, false), Bus::Ambiance, 0.05 * g)
                            .pan(pan)
                            .send(0.3),
                        t + 0.8,
                    );
                }
                tours as f32 * 2.2
            }
            Couche::Vapeur => {
                // Une machine à vapeur : souffles réguliers et cliquetis, parfois
                // le sifflet de l'usine.
                let n = 8 + (rng.u() * 10.0) as usize;
                let pas = rng.range(0.35, 0.55);
                for i in 0..n {
                    let at = i as f32 * pas;
                    push(
                        Voice::new(mk.burst(FilterKind::BandPass, 900.0, 600.0, 0.8, 0.01, 0.22, true), Bus::Ambiance, 0.35 * g)
                            .pan(pan)
                            .send(0.2),
                        at,
                    );
                    if i % 2 == 0 {
                        push(Voice::new(mk.bell(rng.range(80.0, 83.0), 2.4, 0.6, 0.1), Bus::Ambiance, 0.025 * g).pan(pan), at + pas * 0.5);
                    }
                }
                if rng.chance(0.3) {
                    sifflet(mk, rng.range(68.0, 72.0), 2.2, 0.05 * g, -pan, n as f32 * pas, out);
                }
                n as f32 * pas
            }
            Couche::Train => {
                // Un train qui passe au loin : le halètement accélère, le sifflet.
                let mut t = 0.0;
                let mut pas = 0.45;
                let len = rng.range(10.0, 14.0);
                let mut k = 0;
                while t < len {
                    let env = (t / len * std::f32::consts::PI).sin();
                    let p = -0.9 + 1.8 * t / len;
                    out.push(
                        Voice::new(mk.burst(FilterKind::BandPass, 700.0, 500.0, 0.8, 0.01, 0.16, true), Bus::Ambiance, 0.25 * g * env)
                            .pan(p)
                            .send(0.45)
                            .at(t, r),
                    );
                    t += pas;
                    pas = (pas * 0.97).max(0.18);
                    k += 1;
                }
                let _ = k;
                sifflet(mk, rng.range(66.0, 70.0), 1.6, 0.045 * g, 0.0, len * 0.4, out);
                len
            }
            Couche::Voiture => {
                // Une voiture passe : le moteur monte puis descend (Doppler).
                let len = rng.range(2.5, 4.5);
                let (a, b) = if rng.chance(0.5) { (-0.8, 0.8) } else { (0.8, -0.8) };
                push(
                    Voice::new(mk.burst(FilterKind::BandPass, 420.0, 230.0, 1.2, len * 0.45, len, true), Bus::Ambiance, 0.28 * g)
                        .pan(a)
                        .send(0.2),
                    0.0,
                );
                push(
                    Voice::new(mk.burst(FilterKind::BandPass, 1200.0, 700.0, 0.8, len * 0.45, len, false), Bus::Ambiance, 0.02 * g)
                        .pan(b * 0.2),
                    0.0,
                );
                len
            }
            Couche::Klaxon => {
                let carre = Timbre {
                    harm: [1.0, 0.0, 0.33, 0.0, 0.2, 0.0, 0.14, 0.0],
                    eclat: 0.0,
                    att: 0.01,
                    rel: 0.04,
                    chute: 0.0,
                    vib: (0.0, 0.0),
                    sature: 0.3,
                    raideur: 0.0,
                };
                let m = rng.range(62.0, 68.0);
                let d = rng.range(0.25, 0.6);
                for dm in [0.0, 4.0] {
                    push(Voice::new(mk.additif(m + dm, d, carre), Bus::Ambiance, 0.01 * g).pan(pan).send(0.3), 0.0);
                }
                d + 0.3
            }
            Couche::Avion => {
                let len = rng.range(12.0, 18.0);
                push(
                    Voice::new(mk.burst(FilterKind::BandPass, 260.0, 170.0, 1.4, len * 0.45, len, true), Bus::Ambiance, 0.22 * g)
                        .pan(pan)
                        .send(0.6),
                    0.0,
                );
                len
            }
            Couche::Musicien => self.musicien(mk, rng, out),
            Couche::Guerre => {
                // Au loin : tambours de guerre, ou canons quand on a la poudre.
                if self.ville.un_de(&["canon", "poudre"]) {
                    let n = 2 + (rng.u() * 4.0) as usize;
                    let mut t = 0.0;
                    for _ in 0..n {
                        push(Voice::new(mk.thump(rng.range(45.0, 60.0), 1.2), Bus::Ambiance, 0.06 * g).pan(pan).send(0.7), t);
                        push(
                            Voice::new(mk.burst(FilterKind::LowPass, 400.0, 150.0, 0.7, 0.01, 1.5, true), Bus::Ambiance, 0.06 * g)
                                .pan(pan)
                                .send(0.7),
                            t,
                        );
                        t += rng.range(0.8, 3.0);
                    }
                    t + 1.5
                } else {
                    let n = 12 + (rng.u() * 12.0) as usize;
                    for i in 0..n {
                        let fort = i % 4 == 0;
                        push(
                            Voice::new(mk.thump(rng.range(70.0, 80.0), 0.4), Bus::Ambiance, if fort { 0.05 } else { 0.03 } * g)
                                .pan(pan)
                                .send(0.6),
                            i as f32 * 0.42,
                        );
                    }
                    n as f32 * 0.42
                }
            }
        }
    }

    /// Un musicien joue un air de sa tradition, sur l'instrument le plus
    /// récent que permettent les techniques de son peuple.
    fn musicien(&mut self, mk: &Make, rng: &mut Rng, out: &mut Vec<Voice>) -> f32 {
        let tr = &self.tradition;
        let inst = tr
            .instruments
            .iter()
            .rev()
            .copied()
            .find(|i| i.melodique() && !matches!(i, Instrument::Conque | Instrument::Guimbarde | Instrument::Arc))
            .unwrap_or(Instrument::Flute);
        let racine = tr.tonique + 12;
        let n = 8 + (rng.u() * 9.0) as usize;
        let beat = 60.0 / rng.range(80.0, 120.0);
        let pan = rng.bipolar() * 0.6;
        let g = self.proche * 0.25;
        let mut deg = 0i32;
        let mut t = 0.0;
        for i in 0..n {
            let dur = beat
                * if rng.chance(0.25) {
                    2.0
                } else if rng.chance(0.3) {
                    0.5
                } else {
                    1.0
                };
            let m = crate::synth::degree(racine, tr.gamme, deg);
            let mut v = match inst {
                Instrument::Flute => Voice::new(mk.flute(m + 12.0, dur * 0.95, 0.1), Bus::Ambiance, 0.1),
                Instrument::Lithophone => Voice::new(mk.bell(m + 12.0, 2.76, 1.0, 1.4), Bus::Ambiance, 0.08),
                Instrument::Guitare => peuples::note_de_technique(inst, m - 12.0, dur, mk, rng),
                Instrument::Piano | Instrument::Lyre => peuples::note_de_technique(inst, m + 12.0, dur, mk, rng),
                _ => peuples::note_de_technique(inst, m, dur, mk, rng),
            };
            v.gain *= g;
            out.push(v.pan(pan).send(0.45).at(t, mk.rate));
            t += dur;
            // Pas conjoints surtout, retour vers la tonique en fin d'air.
            deg += if i + 2 >= n { -deg.signum() } else { rng.pick(&[-1, -1, 1, 1, 2, -2, 0]) };
            deg = deg.clamp(-3, 6);
        }
        t + 1.0
    }
}

/// Sifflet de vapeur : deux tuyaux rauques qui sonnent ensemble.
fn sifflet(mk: &Make, m: f32, dur: f32, g: f32, pan: f32, at: f32, out: &mut Vec<Voice>) {
    for dm in [0.0, 3.0] {
        out.push(Voice::new(mk.flute(m + dm, dur, 0.6), Bus::Ambiance, g).pan(pan).send(0.6).at(at, mk.rate));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::peuples::Langue;

    pub fn ville(techniques: &[&str], population: f32) -> Ville {
        Ville {
            nom: "Kalo".into(),
            cle: 7,
            population,
            urbain: 0.4,
            techniques: techniques.iter().map(|t| t.to_string()).collect(),
            betail: 0.3,
            temple: 0.6,
            marche: 0.5,
            guerre: false,
            peuple: Peuple {
                nom: "Kalo".into(),
                langue: Langue { mots: vec!["kalo".into(), "mira".into()], cle: 3 },
                rites: Vec::new(),
                population,
                taille: 1.0,
                cle: 3,
                techniques: Vec::new(),
            },
        }
    }

    #[test]
    fn layers_follow_techniques() {
        let presentes = |v: &Ville| Couche::TOUTES.iter().filter(|c| c.poids(v).0 > 0.0).map(|c| c.nom()).collect::<Vec<_>>();
        let village = presentes(&ville(&["poterie"], 500.0));
        assert!(village.contains(&"bêtes") && !village.contains(&"forge"), "{village:?}");
        let bronze = presentes(&ville(&["roue", "bronze", "cheval", "pieces"], 20_000.0));
        for c in ["forge", "charrettes", "sabots", "cloche du temple", "crieurs du marché"] {
            assert!(bronze.contains(&c), "{c} : {bronze:?}");
        }
        let moderne = presentes(&ville(&["roue", "bronze", "cheval", "machine_vapeur", "locomotive", "automobile", "aviation"], 2e6));
        for c in ["voitures", "klaxons", "avion", "train"] {
            assert!(moderne.contains(&c), "{c} : {moderne:?}");
        }
        for c in ["charrettes", "sabots", "forge", "machines à vapeur"] {
            assert!(!moderne.contains(&c), "{c} : {moderne:?}");
        }
    }

    #[test]
    fn every_layer_sounds_and_ends() {
        let mk = Make { rate: 22_050.0 };
        let mut rng = Rng::new(2);
        let v = ville(&["roue", "bronze", "cheval", "pieces", "automobile", "canon"], 1e5);
        let mut rue = Rue::new(v, 1.0, mk.rate, &mut rng);
        for c in Couche::TOUTES {
            let mut out = Vec::new();
            let d = rue.evenement(c, &mk, &mut rng, &mut out);
            assert!(!out.is_empty() && (0.2..40.0).contains(&d), "{} : {d} s, {} voix", c.nom(), out.len());
            let mut fin = 0;
            let dt = 1.0 / mk.rate;
            let (mut crete, mut energie) = (0.0f32, 0.0f32);
            for _ in 0..((d + 15.0) * mk.rate) as usize {
                let mut x = 0.0;
                out.retain_mut(|v| match v.next(&mut rng, dt, mk.rate) {
                    Some(s) => {
                        x += s;
                        true
                    }
                    None => false,
                });
                crete = crete.max(x.abs());
                energie += x * x;
                fin += 1;
            }
            assert!(out.is_empty(), "{} : l'événement finit", c.nom());
            assert!(energie > 1e-3 && crete < 1.0, "{} : crête {crete}, énergie {energie} sur {fin}", c.nom());
        }
    }

    #[test]
    fn events_take_turns() {
        let mk = Make { rate: 22_050.0 };
        let mut rng = Rng::new(5);
        let v = ville(&["roue", "bronze", "cheval", "pieces", "moulin_eau", "brique_crue"], 3e5);
        let mut rue = Rue::new(v, 1.0, mk.rate, &mut rng);
        let mut debuts = Vec::new();
        for pas in 0..(600 * 10) {
            let mut out = Vec::new();
            let avant = rue.en_cours.len();
            rue.tick(0.1, 4, &mk, &mut rng, &mut out);
            if rue.en_cours.len() > avant {
                debuts.push(pas as f32 * 0.1);
            }
            assert!(rue.en_cours.len() <= ENSEMBLE);
        }
        assert!(debuts.len() > 30, "{} événements en dix minutes", debuts.len());
        assert!(debuts.windows(2).all(|w| w[1] - w[0] >= ECART - 0.11));
    }
}
