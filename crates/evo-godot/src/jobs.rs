//! Tâches de fond du client : décors, vues microscope, figures. Elles
//! tournent hors du fil de rendu (budget de l'architecture : moins de
//! 200 ms par décor, jamais dans les 16 ms d'une image) et leur résultat est
//! gardé en cache jusqu'à ce que le client le prenne.

use crate::session::VarDictionaryLite;
use evo_morph::Canvas;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

type Done = HashMap<String, (Canvas, VarDictionaryLite)>;

#[derive(Default)]
pub struct Jobs {
    done: Arc<Mutex<Done>>,
    /// Graine de la dernière demande par clé : une demande identique déjà
    /// servie n'est pas recalculée.
    asked: HashMap<String, u64>,
    pub last_bar_um: f32,
}

impl Jobs {
    pub fn spawn(&mut self, key: String, seed: u64, work: impl FnOnce() -> (Canvas, VarDictionaryLite) + Send + 'static) {
        if self.asked.get(&key) == Some(&seed) {
            return;
        }
        self.asked.insert(key.clone(), seed);
        let done = self.done.clone();
        std::thread::spawn(move || {
            let result = work();
            done.lock().unwrap().insert(key, result);
        });
    }

    pub fn take(&mut self, key: &str) -> Option<(Canvas, VarDictionaryLite)> {
        let r = self.done.lock().unwrap().remove(key);
        if let Some((_, meta)) = &r {
            self.last_bar_um = meta.bar_um;
            // Rendue : une nouvelle demande la recalculera.
            self.asked.remove(key);
        }
        r
    }
}
