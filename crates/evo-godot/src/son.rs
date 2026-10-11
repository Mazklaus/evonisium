//! Pont du son (`evo-son`) vers Godot : la classe `EvoSon`, un nœud que le
//! client ajoute une fois. La synthèse tourne sur son propre fil et remplit
//! un petit tampon ; à chaque image, le fil principal ne fait que recopier
//! ce tampon dans un `AudioStreamGenerator`. Le son lit la session (monde
//! publié, chronique) et n'écrit jamais dans le moteur.
//!
//! Le narrateur parle par une voix de synthèse installée à part (moteur
//! sherpa-onnx et voix Piper, téléchargés depuis les réglages) : un fil
//! dédié la lance phrase par phrase, en avance sur la lecture, et passe le
//! son au mixage. Sans voix installée, rien n'est dit.

use crate::session::EvoSession;
use evo_son::{recit, Annonce, Bruit, Monde, Ordre, Son, Vue};
use godot::classes::{AudioServer, AudioStreamGenerator, AudioStreamGeneratorPlayback, AudioStreamPlayer, INode, Node};
use godot::prelude::*;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

/// Durée du tampon de Godot et avance du fil de synthèse, en secondes.
const GODOT_BUFFER: f32 = 0.15;
const AHEAD: f32 = 0.08;

#[derive(Default)]
struct Shared {
    ring: Mutex<VecDeque<[f32; 2]>>,
    stop: AtomicBool,
    annonces: Mutex<Vec<Annonce>>,
    /// Génération du narrateur : `taire` l'augmente, et les phrases d'une
    /// génération passée ne sont plus dites.
    generation: AtomicU64,
}

/// Voix de synthèse installée : programme sherpa-onnx et modèle Piper.
#[derive(Clone, Debug)]
struct Voix {
    exe: PathBuf,
    model: PathBuf,
    tokens: PathBuf,
    data: PathBuf,
    speaker: i64,
}

/// Une phrase à synthétiser.
struct Tache {
    id: u64,
    texte: String,
    generation: u64,
}

/// Phrase en attente ou en cours : son texte, son groupe et si elle le clôt.
struct Phrase {
    texte: String,
    groupe: u64,
    derniere: bool,
}

/// Délai minimal entre deux moments clés racontés, en secondes de jeu réel.
const ENTRE_MOMENTS: f64 = 150.0;
/// Score d'intérêt minimal d'un moment raconté.
const INTERET_RACONTE: f64 = 0.75;

#[derive(GodotClass)]
#[class(base = Node)]
pub struct EvoSon {
    base: Base<Node>,
    tx: Option<Sender<Ordre>>,
    shared: Arc<Shared>,
    worker: Option<JoinHandle<()>>,
    playback: Option<Gd<AudioStreamGeneratorPlayback>>,
    cursor: Option<usize>,
    last: Option<Monde>,
    vue: Vue,
    rate: f32,
    voix: Option<Voix>,
    narr: Option<Sender<Tache>>,
    phrases: HashMap<u64, Phrase>,
    next_id: u64,
    /// Groupe en cours et sa priorité.
    groupe: Option<(u64, i64)>,
    /// Le narrateur raconte les moments clés de la chronique.
    moments: bool,
    dernier_moment: Option<std::time::Instant>,
    tirage: u64,
    nouvelles: Vec<VarDictionary>,
}

#[godot_api]
impl INode for EvoSon {
    fn init(base: Base<Node>) -> Self {
        Self {
            base,
            tx: None,
            shared: Arc::default(),
            worker: None,
            playback: None,
            cursor: None,
            last: None,
            vue: Vue::Menu,
            rate: 48_000.0,
            voix: None,
            narr: None,
            phrases: HashMap::new(),
            next_id: 1,
            groupe: None,
            moments: true,
            dernier_moment: None,
            tirage: 0,
            nouvelles: Vec::new(),
        }
    }

    fn ready(&mut self) {
        let rate = AudioServer::singleton().get_mix_rate();
        self.rate = rate;
        let mut stream = AudioStreamGenerator::new_gd();
        stream.set_mix_rate(rate);
        stream.set_buffer_length(GODOT_BUFFER);
        let mut player = AudioStreamPlayer::new_alloc();
        player.set_name("Son");
        player.set_stream(&stream);
        self.base_mut().add_child(&player);
        player.play();
        self.playback = player.get_stream_playback().and_then(|p| p.try_cast::<AudioStreamGeneratorPlayback>().ok());

        let (tx, rx) = channel::<Ordre>();
        let shared = self.shared.clone();
        let ahead = (rate * AHEAD) as usize;
        let seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(1, |d| d.as_nanos() as u64);
        self.worker = std::thread::Builder::new()
            .name("evo-son".into())
            .spawn(move || {
                let mut son = Son::new(rate, seed);
                let mut chunk = vec![[0.0f32; 2]; 256];
                while !shared.stop.load(Ordering::Relaxed) {
                    while let Ok(o) = rx.try_recv() {
                        son.ordre(o);
                    }
                    let len = shared.ring.lock().map_or(usize::MAX, |r| r.len());
                    if len < ahead {
                        son.rendre(&mut chunk);
                        if let Ok(mut r) = shared.ring.lock() {
                            r.extend(chunk.iter().copied());
                        }
                        let a = son.annonces();
                        if !a.is_empty() {
                            if let Ok(mut q) = shared.annonces.lock() {
                                q.extend(a);
                            }
                        }
                    } else {
                        std::thread::sleep(Duration::from_millis(4));
                    }
                }
            })
            .ok();
        self.tx = Some(tx);
    }

    fn process(&mut self, _delta: f64) {
        let Some(pb) = self.playback.as_mut() else { return };
        let room = pb.get_frames_available().max(0) as usize;
        if room == 0 {
            return;
        }
        let frames: Vec<Vector2> = match self.shared.ring.lock() {
            Ok(mut r) => {
                let n = room.min(r.len());
                r.drain(..n).map(|f| Vector2::new(f[0], f[1])).collect()
            }
            Err(_) => return,
        };
        if !frames.is_empty() {
            pb.push_buffer(&PackedVector2Array::from(frames.as_slice()));
        }
        self.lire_annonces();
    }

    fn exit_tree(&mut self) {
        self.shared.stop.store(true, Ordering::Relaxed);
        if let Some(w) = self.worker.take() {
            let _ = w.join();
        }
    }
}

#[godot_api]
impl EvoSon {
    fn send(&self, o: Ordre) {
        if let Some(tx) = &self.tx {
            let _ = tx.send(o);
        }
    }

    /// Lit le monde publié et les nouveaux événements de la chronique ; à
    /// appeler une fois par image pendant une partie.
    #[func]
    fn ecouter(&mut self, session: Gd<EvoSession>) {
        let state = session.bind().sound_state(&mut self.cursor);
        let Some((monde, events, recits)) = state else {
            self.cursor = None;
            return;
        };
        // Un moment clé au plus, rarement, et jamais par-dessus une autre
        // narration.
        if self.moments && self.voix.is_some() {
            let libre = self.dernier_moment.is_none_or(|t| t.elapsed().as_secs_f64() > ENTRE_MOMENTS);
            if let Some(r) = recits.iter().filter(|r| r.interest >= INTERET_RACONTE).max_by(|a, b| a.interest.total_cmp(&b.interest)) {
                if libre && self.groupe.is_none() {
                    let phrases = recit::moment(&r.fait, r.years, r.id);
                    if self.dire_phrases(phrases, 1) >= 0 {
                        self.dernier_moment = Some(std::time::Instant::now());
                    }
                }
            }
        }
        if self.last != Some(monde) {
            self.last = Some(monde);
            self.send(Ordre::Monde(monde));
        }
        for (family, interest) in events {
            self.send(Ordre::Evenement { family: family.to_string(), interest });
        }
    }

    /// Vue courante : « menu », « globe », « microscope » ou « sol ».
    #[func]
    fn vue(&mut self, name: GString) {
        let v = Vue::from_name(&name.to_string());
        if v != self.vue {
            self.vue = v;
            self.send(Ordre::Vue(v));
        }
    }

    /// Bruit d'interface : « plume », « page », « tampon », « cloche », « etape ».
    #[func]
    fn bruit(&mut self, name: GString) {
        if let Some(b) = Bruit::from_name(&name.to_string()) {
            self.send(Ordre::Bruit(b));
        }
    }

    /// Volumes de 0 à 1.
    #[func]
    fn volumes(&mut self, general: f64, musique: f64, ambiances: f64, interface: f64, narrateur: f64) {
        self.send(Ordre::Volumes([general as f32, musique as f32, ambiances as f32, interface as f32, narrateur as f32]));
    }

    /// Installe la voix du narrateur : programme `sherpa-onnx-offline-tts`,
    /// dossier du modèle Piper (avec `tokens.txt` et `espeak-ng-data`) et
    /// locuteur. Renvoie faux si un fichier manque (la voix est alors retirée).
    #[func]
    fn voix(&mut self, exe: GString, modele: GString, speaker: i64) -> bool {
        self.taire();
        let exe = PathBuf::from(exe.to_string());
        let dir = PathBuf::from(modele.to_string());
        let model = std::fs::read_dir(&dir)
            .ok()
            .and_then(|it| it.filter_map(|e| e.ok()).map(|e| e.path()).find(|p| p.extension().is_some_and(|x| x == "onnx")));
        let (Some(model), true) = (model, exe.is_file()) else {
            self.voix = None;
            self.narr = None;
            return false;
        };
        let v = Voix { exe, model, tokens: dir.join("tokens.txt"), data: dir.join("espeak-ng-data"), speaker };
        if !v.tokens.is_file() || !v.data.is_dir() {
            self.voix = None;
            self.narr = None;
            return false;
        }
        let (tx, rx) = channel::<Tache>();
        let (Some(out), rate, shared, voix) = (self.tx.clone(), self.rate, self.shared.clone(), v.clone()) else { return false };
        let ok = std::thread::Builder::new()
            .name("evo-narrateur".into())
            .spawn(move || {
                while let Ok(t) = rx.recv() {
                    if t.generation != shared.generation.load(Ordering::Relaxed) {
                        continue;
                    }
                    let son = synthese(&voix, &t.texte, rate).unwrap_or_default();
                    if t.generation != shared.generation.load(Ordering::Relaxed) {
                        continue;
                    }
                    // Une synthèse ratée devient une phrase muette : le
                    // sous-titre passe quand même, et la file avance.
                    let son = if son.is_empty() { vec![0.0; (rate * 2.5) as usize] } else { son };
                    let _ = out.send(Ordre::Parole { id: t.id, son });
                }
            })
            .is_ok();
        if ok {
            self.voix = Some(v);
            self.narr = Some(tx);
        }
        ok
    }

    #[func]
    fn a_une_voix(&self) -> bool {
        self.voix.is_some()
    }

    /// Le narrateur raconte (ou non) les moments clés de la chronique.
    #[func]
    fn raconter_moments(&mut self, on: bool) {
        self.moments = on;
    }

    /// Dit un texte : il est découpé en phrases, dites dans l'ordre.
    /// Priorités : 0 conseil, 1 moment clé, 2 demande du joueur ; une
    /// priorité plus haute interrompt, une plus basse est ignorée. Renvoie
    /// le numéro du groupe, ou -1 si rien ne sera dit.
    #[func]
    fn dire(&mut self, texte: GString, priorite: i64) -> i64 {
        let phrases = decouper(&texte.to_string());
        self.dire_phrases(phrases, priorite)
    }

    /// Raconte une espèce (option « fiche »).
    #[func]
    fn raconter_espece(&mut self, session: Gd<EvoSession>, espece: i64) -> i64 {
        self.tirage += 1;
        let t = self.tirage;
        let phrases = session.clone().bind_mut().recit_espece(espece.max(0) as u32, t);
        phrases.map_or(-1, |p| self.dire_phrases(p, 2))
    }

    /// Raconte ce qui vit dans une cellule (option « scène »).
    #[func]
    fn raconter_lieu(&mut self, session: Gd<EvoSession>, cellule: i64) -> i64 {
        self.tirage += 1;
        let t = self.tirage;
        let phrases = session.bind().recit_lieu(cellule.max(0) as usize, t);
        phrases.map_or(-1, |p| self.dire_phrases(p, 2))
    }

    /// Coupe le narrateur et oublie les phrases en attente.
    #[func]
    fn taire(&mut self) {
        self.shared.generation.fetch_add(1, Ordering::Relaxed);
        self.send(Ordre::Taire);
        self.phrases.clear();
        if let Some((g, _)) = self.groupe.take() {
            let mut d = VarDictionary::new();
            d.set("debut", false);
            d.set("texte", "");
            d.set("groupe", g as i64);
            d.set("fin_groupe", true);
            self.nouvelles.push(d);
        }
    }

    #[func]
    fn parle(&self) -> bool {
        self.groupe.is_some()
    }

    /// Débuts et fins de phrases depuis le dernier appel : des dictionnaires
    /// `{ "debut": bool, "texte": String, "groupe": int, "fin_groupe": bool }`.
    #[func]
    fn nouvelles(&mut self) -> VarArray {
        let mut a = VarArray::new();
        for d in self.nouvelles.drain(..) {
            a.push(&d.to_variant());
        }
        a
    }

    #[func]
    fn muet(&mut self, on: bool) {
        self.send(Ordre::Muet(on));
    }

    /// Quand la partie change (nouvelle partie, chargement) : l'histoire
    /// déjà écrite ne doit pas sonner.
    #[func]
    fn oublier(&mut self) {
        self.cursor = None;
        self.last = None;
        self.taire();
    }
}

impl EvoSon {
    fn dire_phrases(&mut self, phrases: Vec<String>, priorite: i64) -> i64 {
        let Some(narr) = self.narr.clone() else { return -1 };
        if phrases.is_empty() {
            return -1;
        }
        match self.groupe {
            Some((_, p)) if p > priorite => return -1,
            Some((_, p)) if p < priorite => self.taire(),
            _ => {}
        }
        let groupe = self.next_id;
        let generation = self.shared.generation.load(Ordering::Relaxed);
        let n = phrases.len();
        for (i, texte) in phrases.into_iter().enumerate() {
            let id = self.next_id;
            self.next_id += 1;
            self.phrases.insert(id, Phrase { texte: texte.clone(), groupe, derniere: i + 1 == n });
            let _ = narr.send(Tache { id, texte, generation });
        }
        self.groupe = Some((groupe, priorite));
        groupe as i64
    }

    fn lire_annonces(&mut self) {
        let annonces = match self.shared.annonces.lock() {
            Ok(mut q) => std::mem::take(&mut *q),
            Err(_) => return,
        };
        for a in annonces {
            let (debut, id) = match a {
                Annonce::Debut(id) => (true, id),
                Annonce::Fin(id) => (false, id),
            };
            // Les phrases d'un groupe interrompu ne sont plus connues.
            let Some(p) = self.phrases.get(&id) else { continue };
            let mut d = VarDictionary::new();
            d.set("debut", debut);
            d.set("texte", p.texte.as_str());
            d.set("groupe", p.groupe as i64);
            let fin_groupe = !debut && p.derniere;
            d.set("fin_groupe", fin_groupe);
            let groupe = p.groupe;
            if !debut {
                self.phrases.remove(&id);
            }
            if fin_groupe && self.groupe.is_some_and(|(g, _)| g == groupe) {
                self.groupe = None;
            }
            self.nouvelles.push(d);
        }
    }
}

/// Coupe un texte en phrases (après . ! ? suivis d'une espace).
fn decouper(texte: &str) -> Vec<String> {
    let mut v = Vec::new();
    let mut cur = String::new();
    let mut chars = texte.chars().peekable();
    while let Some(c) = chars.next() {
        cur.push(c);
        if matches!(c, '.' | '!' | '?' | '…') && chars.peek().is_none_or(|n| n.is_whitespace()) {
            let s = cur.trim().to_string();
            if !s.is_empty() {
                v.push(s);
            }
            cur.clear();
        }
    }
    let s = cur.trim().to_string();
    if !s.is_empty() {
        v.push(s);
    }
    v
}

/// Synthétise une phrase avec sherpa-onnx et la rend au taux du mixage.
fn synthese(v: &Voix, texte: &str, rate: f32) -> Option<Vec<f32>> {
    static N: AtomicU64 = AtomicU64::new(0);
    let wav = std::env::temp_dir().join(format!("evo-voix-{}-{}.wav", std::process::id(), N.fetch_add(1, Ordering::Relaxed)));
    let mut cmd = std::process::Command::new(&v.exe);
    cmd.arg(format!("--vits-model={}", v.model.display()))
        .arg(format!("--vits-tokens={}", v.tokens.display()))
        .arg(format!("--vits-data-dir={}", v.data.display()))
        .arg(format!("--sid={}", v.speaker))
        .arg("--vits-length-scale=1.12")
        .arg("--num-threads=1")
        .arg(format!("--output-filename={}", wav.display()))
        .arg(texte)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // Pas de fenêtre de console.
        cmd.creation_flags(0x0800_0000);
    }
    let ok = cmd.status().ok()?.success();
    let bytes = std::fs::read(&wav).ok();
    let _ = std::fs::remove_file(&wav);
    if !ok {
        return None;
    }
    let (src_rate, pcm) = lire_wav(&bytes?)?;
    Some(reechantillonner(&pcm, src_rate as f32, rate))
}

/// Lit un WAV mono : PCM 16 bits ou flottant 32 bits.
fn lire_wav(b: &[u8]) -> Option<(u32, Vec<f32>)> {
    if b.len() < 12 || &b[0..4] != b"RIFF" || &b[8..12] != b"WAVE" {
        return None;
    }
    let (mut i, mut rate, mut bits, mut format, mut channels) = (12usize, 0u32, 0u16, 0u16, 1u16);
    while i + 8 <= b.len() {
        let id = &b[i..i + 4];
        let len = u32::from_le_bytes(b[i + 4..i + 8].try_into().ok()?) as usize;
        let body = b.get(i + 8..(i + 8 + len).min(b.len()))?;
        if id == b"fmt " && body.len() >= 16 {
            format = u16::from_le_bytes([body[0], body[1]]);
            channels = u16::from_le_bytes([body[2], body[3]]).max(1);
            rate = u32::from_le_bytes(body[4..8].try_into().ok()?);
            bits = u16::from_le_bytes([body[14], body[15]]);
        } else if id == b"data" {
            let step = channels as usize;
            let pcm: Vec<f32> = match (format, bits) {
                (1, 16) => body.chunks_exact(2 * step).map(|c| i16::from_le_bytes([c[0], c[1]]) as f32 / 32768.0).collect(),
                (3, 32) => body.chunks_exact(4 * step).map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect(),
                _ => return None,
            };
            return (rate > 0).then_some((rate, pcm));
        }
        i += 8 + len + (len & 1);
    }
    None
}

/// Rééchantillonnage linéaire, suffisant pour une voix.
fn reechantillonner(x: &[f32], from: f32, to: f32) -> Vec<f32> {
    if x.is_empty() || from <= 0.0 || (from - to).abs() < 1.0 {
        return x.to_vec();
    }
    let n = ((x.len() as f64) * (to as f64) / (from as f64)) as usize;
    let k = from as f64 / to as f64;
    (0..n)
        .map(|i| {
            let p = i as f64 * k;
            let j = p as usize;
            let f = (p - j as f64) as f32;
            let a = x[j.min(x.len() - 1)];
            let b = x[(j + 1).min(x.len() - 1)];
            a + (b - a) * f
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sentences_are_split_for_subtitles() {
        assert_eq!(decouper("Une phrase. Deux ! Et trois… Fin"), vec!["Une phrase.", "Deux !", "Et trois…", "Fin"]);
        assert_eq!(decouper("Le pH vaut 7.5 ici."), vec!["Le pH vaut 7.5 ici."]);
    }

    #[test]
    fn wav_is_read_and_resampled() {
        let mut b = Vec::new();
        b.extend(b"RIFF");
        b.extend(36u32.to_le_bytes());
        b.extend(b"WAVEfmt ");
        b.extend(16u32.to_le_bytes());
        b.extend(1u16.to_le_bytes());
        b.extend(1u16.to_le_bytes());
        b.extend(22_050u32.to_le_bytes());
        b.extend(44_100u32.to_le_bytes());
        b.extend(2u16.to_le_bytes());
        b.extend(16u16.to_le_bytes());
        b.extend(b"data");
        b.extend(8u32.to_le_bytes());
        for s in [0i16, 16384, -16384, 0] {
            b.extend(s.to_le_bytes());
        }
        let (rate, pcm) = lire_wav(&b).unwrap();
        assert_eq!((rate, pcm.len()), (22_050, 4));
        assert_eq!(pcm[1], 0.5);
        assert_eq!(reechantillonner(&pcm, 22_050.0, 44_100.0).len(), 8);
    }
}
