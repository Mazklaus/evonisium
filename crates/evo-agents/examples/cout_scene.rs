//! Coût d'une image de la scène des agents : quatre espèces de 300
//! individus (trois proies, un prédateur), en millisecondes par pas.

use evo_agents::bench::{sample, with_body};
use evo_agents::{Diet, PopRates, Scene};
use std::time::Instant;

fn main() {
    let r = PopRates { birth: 1.0, death: 1.0, predation: 0.5, emigration: 0.2 };
    let mut samples = Vec::new();
    for k in 0..3 {
        samples.push(with_body(sample(r, 300, k, true), 0.05 + 0.05 * k as f64, 1.0, 3.0, Diet::Grazer));
    }
    samples.push(with_body(sample(r, 300, 9, true), 1.0, 3.0, 12.0, Diet::Predator));
    let mut scene = Scene::new(samples, 1150.0, 1, 0.5);
    for _ in 0..200 {
        scene.step(1.0 / 60.0);
    }
    let t0 = Instant::now();
    let n = 600;
    for _ in 0..n {
        scene.step(1.0 / 60.0);
    }
    let ms = t0.elapsed().as_secs_f64() * 1000.0 / n as f64;
    println!("{} agents : {ms:.3} ms par image", scene.agents.len());
}
