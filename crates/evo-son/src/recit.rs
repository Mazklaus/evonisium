//! Textes du narrateur documentaire, écrits par gabarits (choix de
//! l'utilisateur du 10 octobre 2026 : « gabarits d'abord », un modèle local
//! peut-être plus tard). Trois usages, tous en français parlé :
//!
//! - les moments clés de la chronique, racontés comme dans un documentaire,
//!   rarement ;
//! - l'option « scène » : ce qui vit à l'endroit où le joueur est descendu ;
//! - l'option « fiche » : l'histoire d'une espèce, ses traits, ses habitudes,
//!   et une anecdote locale inventée à partir de son milieu réel.
//!
//! Les textes ne lisent que des valeurs publiées (lieu, espèces, dates) ;
//! les variantes sont tirées d'un hasard propre au récit, jamais de celui du
//! moteur. Les phrases sont rendues une à une : la voix les dit dans l'ordre
//! et le sous-titre les affiche.

use crate::synth::Rng;

/// Métabolismes, dans les mots du narrateur.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Metab {
    PhotoOxygene,
    PhotoSoufre,
    PhotoFer,
    PhotoManganese,
    Methanogene,
    Methanotrophe,
    Sulfato,
    Respiration,
    Fermentation,
}

impl Metab {
    /// Nom de groupe au pluriel : « les méthanogènes ».
    fn groupe(self) -> &'static str {
        match self {
            Metab::PhotoOxygene => "photosynthétiques",
            Metab::PhotoSoufre => "mangeuses de lumière et de soufre",
            Metab::PhotoFer => "cellules qui rouillent le fer à la lumière",
            Metab::PhotoManganese => "cellules du manganèse",
            Metab::Methanogene => "méthanogènes",
            Metab::Methanotrophe => "mangeuses de méthane",
            Metab::Sulfato => "réductrices de soufre",
            Metab::Respiration => "respiratrices",
            Metab::Fermentation => "fermentatrices",
        }
    }
    /// Ce que fait la cellule, en une proposition : « elle boit la lumière ».
    fn geste(self) -> &'static str {
        match self {
            Metab::PhotoOxygene => "elle casse l'eau à la lumière et rejette de l'oxygène",
            Metab::PhotoSoufre => "elle capte la lumière en brûlant le soufre des sources",
            Metab::PhotoFer => "elle capte la lumière en oxydant le fer dissous",
            Metab::PhotoManganese => "elle capte la lumière en oxydant le manganèse",
            Metab::Methanogene => "elle respire l'hydrogène et rejette du méthane",
            Metab::Methanotrophe => "elle se nourrit du méthane que d'autres rejettent",
            Metab::Sulfato => "elle respire le sulfate comme nous respirons l'oxygène",
            Metab::Respiration => "elle brûle sa nourriture à l'oxygène",
            Metab::Fermentation => "elle vit des restes des autres, sans air",
        }
    }
    fn lumiere(self) -> bool {
        matches!(self, Metab::PhotoOxygene | Metab::PhotoSoufre | Metab::PhotoFer | Metab::PhotoManganese)
    }
}

/// Un endroit de la planète, tel que le joueur peut le voir.
#[derive(Clone, Debug, PartialEq)]
pub struct Lieu {
    pub region: String,
    pub mer: bool,
    pub lac: bool,
    pub glace: bool,
    pub temperature_c: f64,
    /// Profondeur en mer, altitude à terre, en mètres.
    pub hauteur_m: f64,
    /// Source hydrothermale dans la cellule.
    pub source: bool,
    /// Lumière au fond ou au sol, en W·m⁻².
    pub lumiere: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tendance {
    Expansion,
    Stable,
    Declin,
    Inconnue,
}

/// Une espèce, telle que la fiche la montre.
#[derive(Clone, Debug, PartialEq)]
pub struct Espece {
    /// Nom savant (binôme).
    pub nom: String,
    pub metabolismes: Vec<Metab>,
    pub pigmentee: bool,
    /// Âge de l'espèce, en années.
    pub age_ans: Option<f64>,
    pub region_origine: String,
    /// Nom savant de l'espèce mère, si elle est connue.
    pub parent: Option<String>,
    /// Part de la planète occupée, de 0 à 1.
    pub aire: f64,
    pub tendance: Tendance,
    pub ecotypes: u32,
}

impl Espece {
    fn principal(&self) -> Option<Metab> {
        self.metabolismes.first().copied()
    }
}

/// Une population vue sur place : métabolisme principal et part de la
/// biomasse locale.
#[derive(Clone, Debug, PartialEq)]
pub struct Presence {
    pub nom: String,
    pub metab: Option<Metab>,
    pub part: f64,
}

/// Chemins d'innovation suivis par la chronique.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Chemin {
    Photosynthese,
    Rhodopsine,
    Complexite,
}

/// Un moment de l'histoire que le narrateur peut raconter.
#[derive(Clone, Debug, PartialEq)]
pub enum Fait {
    Vie { region: String },
    Innovation { chemin: Chemin, etape: u8, region: String },
    Oxygene { montee: bool, ratio: f64 },
    Glaciation { debut: bool },
    Plaques,
}

/// Durée dite à voix haute : « deux milliards et demi d'années ».
pub fn duree(ans: f64) -> String {
    let a = ans.abs();
    if a >= 1e9 {
        let g = a / 1e9;
        let entier = g.floor();
        let reste = g - entier;
        let n = nombre(entier as u64);
        let mot = if entier as u64 == 1 { "milliard" } else { "milliards" };
        if reste >= 0.85 {
            let n = nombre(entier as u64 + 1);
            return format!("près de {n} milliards d'années");
        }
        return match reste {
            r if r < 0.15 => format!("{n} {mot} d'années"),
            r if r < 0.35 => format!("un peu plus de {n} {mot} d'années"),
            r if r < 0.65 => format!("{n} {mot} et demi d'années"),
            _ => format!("{n} {mot} trois quarts d'années"),
        };
    }
    if a >= 1e6 {
        let m = (a / 1e6).round() as u64;
        let m = if m >= 100 { (m / 10) * 10 } else { m };
        return format!("{} {} d'années", nombre(m), if m == 1 { "million" } else { "millions" });
    }
    if a >= 1e3 {
        let k = (a / 1e3).round() as u64;
        return format!("{} mille ans", nombre(k));
    }
    format!("{} ans", nombre(a.round() as u64))
}

/// Nombre en lettres, jusqu'à 999 (au-delà, en chiffres).
pub fn nombre(n: u64) -> String {
    const U: [&str; 20] = [
        "zéro", "un", "deux", "trois", "quatre", "cinq", "six", "sept", "huit", "neuf", "dix", "onze", "douze", "treize", "quatorze",
        "quinze", "seize", "dix-sept", "dix-huit", "dix-neuf",
    ];
    const D: [&str; 10] = ["", "", "vingt", "trente", "quarante", "cinquante", "soixante", "soixante", "quatre-vingt", "quatre-vingt"];
    fn moins_de_cent(n: u64) -> String {
        if n < 20 {
            return U[n as usize].to_string();
        }
        let (d, u) = (n / 10, n % 10);
        let base = if d == 7 || d == 9 { 10 + u } else { u };
        let dix = D[d as usize];
        match (d, base) {
            (8, 0) => "quatre-vingts".into(),
            (_, 0) => dix.into(),
            (2..=6, 1) => format!("{dix} et un"),
            (7, 11) => "soixante et onze".into(),
            _ => format!("{dix}-{}", U[base as usize]),
        }
    }
    match n {
        0..=99 => moins_de_cent(n),
        100..=999 => {
            let (c, r) = (n / 100, n % 100);
            let tete = if c == 1 { "cent".to_string() } else { format!("{} cent", U[c as usize]) };
            match r {
                0 if c > 1 => format!("{tete}s"),
                0 => tete,
                _ => format!("{tete} {}", moins_de_cent(r)),
            }
        }
        _ => n.to_string(),
    }
}

fn degres(t: f64) -> String {
    let n = t.round() as i64;
    if n < 0 {
        format!("moins {} degrés", nombre(n.unsigned_abs()))
    } else if n <= 1 {
        format!("{} degré", nombre(n as u64))
    } else {
        format!("{} degrés", nombre(n as u64))
    }
}

fn metres(m: f64) -> String {
    let m = m.abs();
    if m >= 1000.0 {
        let k = (m / 100.0).round() / 10.0;
        if (k - k.round()).abs() < 0.05 {
            format!("{} {} mètres", nombre(k.round() as u64), "mille")
        } else {
            format!("{} mètres", (m / 100.0).round() as u64 * 100)
        }
    } else {
        let r = if m >= 100.0 { (m / 10.0).round() * 10.0 } else { m.round() };
        format!("{} mètres", nombre(r as u64))
    }
}

/// « de deux milliards », « d'un milliard » : élision devant une voyelle.
fn de(s: &str) -> String {
    match s.chars().next() {
        Some('a' | 'e' | 'é' | 'i' | 'o' | 'u' | 'h') => format!("d'{s}"),
        _ => format!("de {s}"),
    }
}

fn premier(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

/// Graine du récit : mêle une clé stable (événement, espèce, cellule) au
/// tirage, pour que la même fiche ne se répète pas mot pour mot.
fn rng(cle: u64, tirage: u64) -> Rng {
    Rng::new(cle.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ tirage.rotate_left(17) ^ 0x5245_4349_5400)
}

/// Où l'on est, en quelques mots : « dans les eaux chaudes de Torvax ».
fn decor(l: &Lieu) -> String {
    let chaleur = match l.temperature_c {
        t if t < 2.0 => "glacées",
        t if t < 12.0 => "froides",
        t if t < 24.0 => "tièdes",
        t if t < 45.0 => "chaudes",
        _ => "brûlantes",
    };
    if l.mer && l.glace {
        format!("sous la banquise de {}", l.region)
    } else if l.mer && l.source {
        format!("près des sources chaudes du fond, au large de {}", l.region)
    } else if l.mer && l.hauteur_m.abs() < 200.0 {
        format!("dans les eaux {chaleur} et peu profondes de {}", l.region)
    } else if l.mer {
        format!("dans les eaux {chaleur} du large de {}", l.region)
    } else if l.lac {
        format!("dans les marais {} de {}", if l.temperature_c < 12.0 { "froids" } else { "tièdes" }, l.region)
    } else if l.glace {
        format!("sur les glaces de {}", l.region)
    } else {
        format!("sur les terres de {}", l.region)
    }
}

/// Moment clé de la chronique. `ans` : temps écoulé depuis la naissance de
/// la planète simulée ; `cle` : identifiant de l'événement.
pub fn moment(f: &Fait, ans: f64, cle: u64) -> Vec<String> {
    let mut r = rng(cle, 1);
    let quand = duree(ans);
    let mut v: Vec<String> = Vec::new();
    match f {
        Fait::Vie { region } => {
            v.push(
                r.pick(&[
                    "Dans une mer encore sans nom, quelques cellules se mettent à copier leurs gènes.",
                    "Tout commence par presque rien : une poignée de cellules dans l'eau tiède.",
                ])
                .into(),
            );
            v.push(format!("Près de {region}, la vie vient de commencer. Elle ne s'arrêtera plus."));
        }
        Fait::Innovation { chemin: Chemin::Photosynthese, etape, region } => match etape {
            1 => {
                v.push(format!("Au large de {region}, une cellule fabrique un pigment."));
                v.push("Pour l'instant, il ne sert qu'à se protéger du soleil. Mais la lumière vient d'entrer dans l'histoire.".into());
            }
            2 => {
                v.push(format!(
                    "{} ont passé. Pour la première fois, près de {region}, une cellule se nourrit de lumière.",
                    premier(&quand)
                ));
                v.push("Elle n'en tire encore qu'un peu d'énergie. Ses descendantes en tireront un monde.".into());
            }
            3 => {
                v.push(format!("Près de {region}, des cellules apprennent à fabriquer leur matière avec la lumière."));
                v.push(
                    r.pick(&[
                        "Elles brûlent encore le soufre et le fer des sources. L'eau, elles ne savent pas la casser.",
                        "Il leur manque une seule pièce : casser l'eau elle-même.",
                    ])
                    .into(),
                );
            }
            _ => {
                v.push(format!("Au bout {}, près de {region}, une cellule casse l'eau à la lumière.", de(&quand)));
                v.push("Elle rejette un gaz que la planète n'a jamais connu en quantité : l'oxygène.".into());
                v.push(
                    r.pick(&[
                        "Rien, ensuite, ne sera plus comme avant.",
                        "C'est un poison pour presque tous ses voisins. C'est aussi le début de tout ce qui respire.",
                    ])
                    .into(),
                );
            }
        },
        Fait::Innovation { chemin: Chemin::Rhodopsine, region, .. } => {
            v.push(format!(
                "Près de {region}, une cellule invente une autre façon de boire la lumière : une petite pompe teintée de pourpre."
            ));
            v.push("Elle colore la mer, mais ne fabrique rien. C'est un raccourci, pas une révolution.".into());
        }
        Fait::Innovation { chemin: Chemin::Complexite, etape, region } => match etape {
            1 => {
                v.push(format!(
                    "Près de {region}, une cellule ne se contente plus de ce qui flotte autour d'elle : elle avale ses voisines."
                ));
                v.push("La prédation vient de naître, à l'échelle d'un millième de millimètre.".into());
            }
            2 => {
                v.push(format!("Il y a eu, près de {region}, un repas qui n'a jamais été digéré."));
                v.push(
                    "La proie est restée vivante à l'intérieur de son hôte. Elle y est encore, et deviendra sa centrale d'énergie.".into(),
                );
                v.push("La cellule complexe est née.".into());
            }
            3 => {
                v.push(format!("Près de {region}, une cellule complexe avale une algue, et la garde."));
                v.push("Elle porte désormais la lumière en elle.".into());
            }
            4 => {
                v.push("Pour la première fois, deux cellules mêlent leurs gènes pour en faire une troisième.".into());
                v.push(format!(
                    "Près de {region}, le sexe vient d'apparaître, et avec lui une diversité que la simple copie ne donnait pas."
                ));
            }
            5 => {
                v.push(format!("Près de {region}, des cellules restent collées après s'être divisées."));
                v.push("Ce n'est qu'une colonie. Mais c'est la première fois que la vie fait groupe.".into());
            }
            6 => {
                v.push(format!("Dans une colonie près de {region}, toutes les cellules ne font plus le même travail."));
                v.push("Certaines se nourrissent, d'autres se reproduisent. Le corps commence ainsi.".into());
            }
            7 => {
                v.push(format!("Près de {region}, apparaît un être fait de cellules différentes, qui ne peuvent plus vivre seules."));
                v.push(format!("Il aura fallu {quand}."));
            }
            _ => {
                v.push(format!("Sur les rives de {region}, un être complexe quitte l'eau."));
                v.push("La terre ferme était vide. Elle ne le restera pas.".into());
            }
        },
        Fait::Oxygene { montee: true, ratio } => {
            if *ratio >= 0.01 {
                v.push("L'air contient maintenant assez d'oxygène pour que des êtres plus grands puissent un jour le respirer.".into());
            } else {
                v.push(
                    r.pick(&[
                        "Bulle après bulle, l'oxygène s'accumule dans l'air.",
                        "L'oxygène rejeté par les cellules ne disparaît plus : il commence à s'accumuler dans l'air.",
                    ])
                    .into(),
                );
                v.push("Le fer des océans rouille, le méthane recule, et le ciel change de couleur.".into());
            }
        }
        Fait::Oxygene { montee: false, .. } => {
            v.push("L'oxygène de l'air recule. Les cellules qui l'avaient appris à le respirer vont devoir s'en passer.".into());
        }
        Fait::Glaciation { debut: true } => {
            v.push("La glace descend des pôles, et ne s'arrête plus.".into());
            v.push(
                r.pick(&[
                    "Bientôt, la planète entière est blanche. Sous la glace, la vie attend.",
                    "La planète devient une boule de neige. La vie, elle, se réfugie près des sources chaudes.",
                ])
                .into(),
            );
        }
        Fait::Glaciation { debut: false } => {
            v.push("Les volcans ont fini par réchauffer l'air. La glace se retire, et la mer revient à la lumière.".into());
        }
        Fait::Plaques => {
            v.push("Lentement, les plaques de la planète changent de direction. Des mers se ferment, d'autres s'ouvrent.".into());
        }
    }
    v
}

/// Lecture d'une fiche d'espèce : histoire, traits, habitudes, puis une
/// anecdote inventée là où l'espèce est la plus abondante.
pub fn fiche(e: &Espece, chez_elle: &Lieu, voisins: &[Presence], tirage: u64) -> Vec<String> {
    let cle = e.nom.bytes().fold(0xcbf2_9ce4_8422_2325u64, |h, b| (h ^ b as u64).wrapping_mul(0x0100_0000_01b3));
    let mut r = rng(cle, tirage);
    let mut v = Vec::new();
    // Histoire.
    let age = e.age_ans.map(duree);
    match (&e.parent, age) {
        (Some(p), Some(a)) => v.push(format!("{} est née il y a {a}, près de {}, d'une branche de {p}.", e.nom, e.region_origine)),
        (None, Some(a)) => v.push(format!("{} vit sur cette planète depuis {a}. Elle est apparue près de {}.", e.nom, e.region_origine)),
        _ => v.push(format!("{} est apparue près de {}.", e.nom, e.region_origine)),
    }
    // Traits.
    if let Some(m) = e.principal() {
        let mut s = format!("C'est une cellule simple : {}", m.geste());
        if e.metabolismes.len() > 1 {
            s.push_str(", et elle sait aussi faire autrement quand le milieu change");
        }
        s.push('.');
        v.push(s);
    }
    if e.pigmentee && !e.principal().is_some_and(Metab::lumiere) {
        v.push("Elle porte un pigment, une teinte qui la protège du soleil plus qu'elle ne la nourrit.".into());
    }
    // Habitudes : aire et tendance.
    let aire = match e.aire {
        a if a > 0.3 => "On la trouve presque partout sur la planète.",
        a if a > 0.05 => "Elle occupe une bonne partie des mers.",
        a if a > 0.005 => "Elle ne vit que dans quelques régions.",
        _ => "Elle ne survit qu'en quelques points de la planète.",
    };
    v.push(aire.into());
    match e.tendance {
        Tendance::Expansion => v.push(r.pick(&["Et elle gagne du terrain.", "Ses populations grandissent."]).into()),
        Tendance::Declin => v.push(r.pick(&["Mais elle recule.", "Ses populations, pourtant, diminuent."]).into()),
        _ => {}
    }
    if e.ecotypes > 1 {
        v.push(format!("{} formes locales se sont déjà séparées en son sein, chacune à son milieu.", premier(&nombre(e.ecotypes as u64))));
    }
    // Anecdote locale.
    v.extend(anecdote(e, chez_elle, voisins, &mut r));
    v
}

fn anecdote(e: &Espece, l: &Lieu, voisins: &[Presence], r: &mut Rng) -> Vec<String> {
    let mut v = Vec::new();
    let ici = decor(l);
    let m = e.principal();
    let lumiere = m.is_some_and(Metab::lumiere);
    if l.mer && l.source {
        v.push(format!("Elle est la plus nombreuse {ici}."));
        v.push(r.pick(&[
            "Là, l'eau sort du sol à plus de cent degrés et retombe en pluie de minéraux. Une génération y vit à quelques millimètres de la mort.",
            "Quand une source s'éteint, la colonie s'éteint avec elle. Il faut alors qu'un courant porte quelques cellules jusqu'à la suivante.",
        ]).into());
    } else if l.glace {
        v.push(format!("Elle est la plus nombreuse {ici}, là où presque rien ne bouge."));
        v.push(format!(
            "Par {}, elle se divise à peine une fois par saison. Ses cellules attendent, parfois des siècles, que la glace s'ouvre.",
            degres(l.temperature_c)
        ));
    } else if lumiere && l.mer && l.hauteur_m.abs() < 200.0 {
        v.push(format!("Elle est la plus nombreuse {ici}."));
        v.push(r.pick(&[
            "Là, ses cellules s'empilent en tapis, couche après couche. Le jour, la couche du dessus prend la lumière. La nuit, celles du dessous en profitent.",
            "Chaque marée la recouvre de sable fin. Chaque matin, elle remonte vers la lumière, et laisse derrière elle une fine strate de pierre.",
        ]).into());
    } else if lumiere {
        v.push(format!("Elle est la plus nombreuse {ici}, au plus près de la surface."));
        v.push("Les tempêtes la dispersent, les calmes la rassemblent. Certains jours, elle teinte la mer sur des kilomètres.".into());
    } else if l.mer && l.hauteur_m.abs() > 1000.0 {
        v.push(format!("Elle est la plus nombreuse {ici}, par {} de fond.", metres(l.hauteur_m)));
        v.push("Aucune lumière n'y arrive. Elle vit de ce qui tombe d'en haut, lentement, comme une neige.".into());
    } else {
        v.push(format!("Elle est la plus nombreuse {ici}, par {}.", degres(l.temperature_c)));
        v.push(
            r.pick(&[
                "Elle y vit de peu, mais elle y vit depuis longtemps.",
                "Là, une seule de ses cellules peut avoir des milliards de descendantes en un été.",
            ])
            .into(),
        );
    }
    // Les voisins font l'histoire.
    let autre = voisins.iter().filter(|p| p.nom != e.nom && p.part > 0.05).max_by(|a, b| a.part.total_cmp(&b.part));
    if let Some(p) = autre {
        let leurs = p.metab.map_or("d'autres cellules", |x| x.groupe());
        let phrase = match (m, p.metab) {
            (Some(Metab::Methanogene), Some(Metab::Methanotrophe)) => {
                format!(
                    "Elle y partage l'eau avec des {leurs} : ce qu'elle rejette, elles le mangent. Chacune nourrit l'autre sans le savoir."
                )
            }
            (Some(a), Some(b)) if a.lumiere() && !b.lumiere() => {
                format!("Juste en dessous vivent des {leurs}, qui se nourrissent de ses cellules mortes.")
            }
            (_, Some(b)) if b == Metab::PhotoOxygene && m != Some(Metab::PhotoOxygene) && m != Some(Metab::Respiration) => {
                format!(
                    "Ses voisines, des {leurs}, rejettent de l'oxygène. Pour elle, c'est un poison qui monte, un peu plus chaque année."
                )
            }
            _ if p.part > 0.5 => format!(
                "Elle n'y est pourtant pas seule : des {leurs} y sont bien plus nombreuses qu'elle, et lui disputent chaque molécule."
            ),
            _ => format!("Elle y côtoie des {leurs}, ses voisines les plus proches."),
        };
        v.push(phrase);
    }
    v
}

/// Ce que l'on voit au sol : le lieu, puis les espèces présentes, de la plus
/// abondante à la plus discrète (trois au plus).
pub fn scene(l: &Lieu, presences: &[Presence], tirage: u64) -> Vec<String> {
    let cle = l.region.bytes().fold(0x1234_5678u64, |h, b| h.rotate_left(5) ^ b as u64);
    let mut r = rng(cle, tirage);
    let mut v = Vec::new();
    let ici = premier(&decor(l));
    let fond = if l.mer && l.hauteur_m.abs() > 0.0 { format!(", par {} de fond", metres(l.hauteur_m)) } else { String::new() };
    let air = if l.mer || l.lac { "L'eau est à" } else { "Il fait" };
    v.push(format!("{ici}{fond}. {air} {}.", degres(l.temperature_c)));
    if presences.is_empty() {
        v.push(r.pick(&["Ici, rien ne vit encore.", "Le milieu est prêt, mais personne ne l'a encore trouvé."]).into());
        return v;
    }
    let mut p: Vec<&Presence> = presences.iter().collect();
    p.sort_by(|a, b| b.part.total_cmp(&a.part));
    let n = p.len();
    v.push(match n {
        1 => "Une seule espèce occupe la place.".to_string(),
        _ => format!("{} espèces se partagent la place.", premier(&nombre(n as u64))),
    });
    for (i, x) in p.iter().take(3).enumerate() {
        let geste = x.metab.map_or("elle vit ici", |m| m.geste());
        let s = match i {
            0 if x.part > 0.6 => format!("{} domine largement : {geste}.", x.nom),
            0 => format!("La plus abondante est {} : {geste}.", x.nom),
            1 => format!("Plus discrète, {} : {geste}.", x.nom),
            _ => format!("Et, presque invisible, {}.", x.nom),
        };
        v.push(s);
    }
    if l.lumiere < 1.0 && l.mer {
        v.push("Ici, il fait nuit en permanence.".into());
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lieu() -> Lieu {
        Lieu {
            region: "Torvax".into(),
            mer: true,
            lac: false,
            glace: false,
            temperature_c: 18.0,
            hauteur_m: -60.0,
            source: false,
            lumiere: 80.0,
        }
    }

    #[test]
    fn numbers_and_durations_read_aloud() {
        assert_eq!(nombre(71), "soixante et onze");
        assert_eq!(nombre(80), "quatre-vingts");
        assert_eq!(nombre(92), "quatre-vingt-douze");
        assert_eq!(nombre(200), "deux cents");
        assert_eq!(nombre(321), "trois cent vingt et un");
        assert_eq!(duree(2.5e9), "deux milliards et demi d'années");
        assert_eq!(duree(1.0e9), "un milliard d'années");
        assert_eq!(duree(3.9e9), "près de quatre milliards d'années");
        assert_eq!(duree(4.0e6), "quatre millions d'années");
    }

    #[test]
    fn every_moment_has_words() {
        let mut faits = vec![Fait::Vie { region: "Torvax".into() }, Fait::Plaques];
        for etape in 1..=4 {
            faits.push(Fait::Innovation { chemin: Chemin::Photosynthese, etape, region: "Ka".into() });
        }
        for etape in 1..=8 {
            faits.push(Fait::Innovation { chemin: Chemin::Complexite, etape, region: "Ka".into() });
        }
        faits.push(Fait::Innovation { chemin: Chemin::Rhodopsine, etape: 1, region: "Ka".into() });
        for b in [true, false] {
            faits.push(Fait::Glaciation { debut: b });
            faits.push(Fait::Oxygene { montee: b, ratio: 1e-3 });
        }
        for (i, f) in faits.iter().enumerate() {
            let v = moment(f, 1.8e9, i as u64);
            assert!(!v.is_empty() && v.iter().all(|s| s.len() > 10 && s.ends_with(['.', '!', '?'])), "{f:?} : {v:?}");
        }
    }

    #[test]
    fn a_species_sheet_tells_a_local_story() {
        let e = Espece {
            nom: "Methanobius torvaxensis".into(),
            metabolismes: vec![Metab::Methanogene],
            pigmentee: false,
            age_ans: Some(3.2e8),
            region_origine: "Torvax".into(),
            parent: Some("Protobius kaensis".into()),
            aire: 0.08,
            tendance: Tendance::Expansion,
            ecotypes: 3,
        };
        let voisins = [
            Presence { nom: e.nom.clone(), metab: Some(Metab::Methanogene), part: 0.6 },
            Presence { nom: "X".into(), metab: Some(Metab::Methanotrophe), part: 0.3 },
        ];
        let v = fiche(&e, &Lieu { source: true, hauteur_m: -2400.0, ..lieu() }, &voisins, 0);
        let texte = v.join(" ");
        assert!(texte.contains("trois cent vingt millions d'années"), "{texte}");
        assert!(texte.contains("sources chaudes"), "{texte}");
        assert!(texte.contains("elles le mangent"), "{texte}");
        // Un autre tirage peut changer les mots, jamais les faits.
        let w = fiche(&e, &Lieu { source: true, ..lieu() }, &voisins, 1);
        assert_eq!(v[0], w[0]);
    }

    #[test]
    fn a_scene_names_who_lives_there() {
        let p = [
            Presence { nom: "Oxyphycus torvaxensis".into(), metab: Some(Metab::PhotoOxygene), part: 0.7 },
            Presence { nom: "Zymobius kaensis".into(), metab: Some(Metab::Fermentation), part: 0.3 },
        ];
        let v = scene(&lieu(), &p, 0);
        let t = v.join(" ");
        assert!(t.contains("Deux espèces") && t.contains("domine largement : elle casse l'eau"), "{t}");
        assert!(scene(&lieu(), &[], 0).len() == 2);
    }
}
