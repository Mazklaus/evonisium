//! Pont du son (`evo-son`) vers Godot : la classe `EvoSon`, un nœud que le
//! client ajoute une fois. La synthèse tourne sur son propre fil et remplit
//! un petit tampon ; à chaque image, le fil principal ne fait que recopier
//! ce tampon dans un `AudioStreamGenerator`. Le son lit la session (monde
//! publié, chronique) et n'écrit jamais dans le moteur.

use crate::session::EvoSession;
use evo_son::{Bruit, Monde, Ordre, Son, Vue};
use godot::classes::{AudioServer, AudioStreamGenerator, AudioStreamGeneratorPlayback, AudioStreamPlayer, INode, Node};
use godot::prelude::*;
use std::collections::VecDeque;
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
}

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
}

#[godot_api]
impl INode for EvoSon {
    fn init(base: Base<Node>) -> Self {
        Self { base, tx: None, shared: Arc::default(), worker: None, playback: None, cursor: None, last: None, vue: Vue::Menu }
    }

    fn ready(&mut self) {
        let rate = AudioServer::singleton().get_mix_rate();
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
        let Some((monde, events)) = state else {
            self.cursor = None;
            return;
        };
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
    fn volumes(&mut self, general: f64, musique: f64, ambiances: f64, interface: f64) {
        self.send(Ordre::Volumes([general as f32, musique as f32, ambiances as f32, interface as f32]));
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
    }
}
