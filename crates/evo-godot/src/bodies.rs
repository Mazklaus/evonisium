//! Corps du palier 2 calculés en tâche de fond : le maillage d'un plan coûte
//! quelques dizaines de millisecondes, jamais dans le budget d'une image. Le
//! résultat reste en cache (un corps par génotype de développement, document
//! Rendu du vivant) tant que la partie dure.

use evo_morph::body::{Body, BodyPlan};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub struct BodyEntry {
    pub plan: BodyPlan,
    pub body: Body,
}

#[derive(Default)]
pub struct Bodies {
    done: Arc<Mutex<HashMap<String, Arc<BodyEntry>>>>,
    asked: HashMap<String, u64>,
}

impl Bodies {
    /// Lance le calcul d'un corps, sauf si la même demande est déjà servie
    /// ou en cours.
    pub fn spawn(&mut self, key: String, seed: u64, work: impl FnOnce() -> Option<BodyPlan> + Send + 'static) {
        if self.asked.get(&key) == Some(&seed) {
            return;
        }
        self.asked.insert(key.clone(), seed);
        let done = self.done.clone();
        std::thread::spawn(move || {
            if let Some(plan) = work() {
                let body = evo_morph::body::build(&plan, 3);
                done.lock().unwrap().insert(key, Arc::new(BodyEntry { plan, body }));
            }
        });
    }

    pub fn get(&self, key: &str) -> Option<Arc<BodyEntry>> {
        self.done.lock().unwrap().get(key).cloned()
    }

    pub fn clear(&mut self) {
        self.done.lock().unwrap().clear();
        self.asked.clear();
    }
}
