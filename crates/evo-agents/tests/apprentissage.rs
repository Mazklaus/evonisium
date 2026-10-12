//! Calage du modèle agrégé des groupes culturels (evo-culture) sur des
//! agents qui apprennent : il les suit aux fluctuations près.

use evo_agents::agents::{Act, Scene};
use evo_agents::bench::with_body;
use evo_agents::learning::{Band, CultureHint, Signalling, SkillBand};
use evo_agents::sample::PopRates;
use evo_agents::Diet;
use evo_culture::catalog::{Context, Family};
use evo_culture::cognition::{presets, Learning};
use evo_culture::world::CultureWorld;
use evo_culture::Organ;

fn mean_sd(v: &[f64]) -> (f64, f64) {
    let m = v.iter().sum::<f64>() / v.len() as f64;
    let var = v.iter().map(|x| (x - m) * (x - m)).sum::<f64>() / (v.len() - 1) as f64;
    (m, var.sqrt())
}

/// Koshima : la diffusion d'une invention dans une troupe de 60 macaques,
/// par agents qui observent et par le modèle agrégé, donne les mêmes
/// fréquences moyennes aux fluctuations près.
#[test]
fn aggregated_diffusion_follows_the_agents() {
    let cog = presets::macaque();
    let l = Learning::of(&cog);
    let appeal = 1.0 + Family::Percussion.technique().benefit;
    let runs = 300;
    let checkpoints = [3usize, 6, 10, 20];
    let mut agents = vec![Vec::new(); checkpoints.len()];
    let mut model = vec![Vec::new(); checkpoints.len()];
    for seed in 0..runs {
        let mut band = Band::new(l, appeal, 60, 1, seed);
        let mut w = CultureWorld::new(seed + 10_000);
        w.add_species(2, cog);
        w.add_group(0, 2, 60.0, Context::land(Organ::Hand), None);
        w.seed_trait(0, 2, Family::Percussion as u32, 1.0 / 60.0, 1.0);
        let mut year = 0;
        for (k, &c) in checkpoints.iter().enumerate() {
            while year < c {
                band.advance(1.0, 0.05);
                w.step(1.0);
                year += 1;
            }
            agents[k].push(band.freq());
            model[k].push(w.group(0, 2).unwrap().slot(Family::Percussion as u32).map(|s| s.freq).unwrap_or(0.0));
        }
    }
    for (k, &c) in checkpoints.iter().enumerate() {
        let (ma, sa) = mean_sd(&agents[k]);
        let (mm, sm) = mean_sd(&model[k]);
        let se = ((sa * sa + sm * sm) / runs as f64).sqrt();
        eprintln!("année {c} : agents {ma:.3} ± {sa:.3}, modèle {mm:.3} ± {sm:.3}");
        assert!((ma - mm).abs() <= 4.0 * se + 0.03, "année {c} : agents {ma:.3}, modèle {mm:.3}");
    }
}

/// Henrich (2004) : individu par individu, le savoir-faire moyen d'une
/// technique difficile baisse dans une île de 4 000 personnes et monte sur un
/// continent de 28 000, au rythme β(γ + ln n) − α du modèle agrégé.
#[test]
fn skill_follows_henrich() {
    let l = Learning::of(&presets::human());
    let d = Family::BoneTool.technique().variants[0].difficulty;
    for n in [4000usize, 28_000] {
        let mut band = SkillBand::new(l, d, n, d / 2.0, n as u64);
        for _ in 0..5 {
            band.generation();
        }
        let start = band.mean();
        let gens = 200;
        for _ in 0..gens {
            band.generation();
        }
        let slope = (band.mean() - start) / gens as f64;
        let expected = l.skill_gain(d, n as f64);
        eprintln!("n = {n} : {slope:.3} par génération, modèle {expected:.3}");
        // Écart type du maximum de n tirages de Gumbel : 1,28 β par
        // génération, soit 0,09 sur la pente de 200 générations.
        assert!((slope - expected).abs() < 0.3, "n = {n} : {slope:.3} contre {expected:.3}");
        assert_eq!(slope > 0.0, expected > 0.0);
    }
}

/// Jeu de signalisation : une population converge vers une convention
/// partagée ; une espèce moins expressive ne dit qu'une part des situations.
#[test]
fn signalling_game_reaches_a_convention() {
    let mut g = Signalling::new(12, 6, 6, 1);
    g.play(200_000);
    let late = g.play(20_000);
    eprintln!("réussite {late:.2}, convention {:.2}", g.convention());
    assert!(late > 0.8, "réussite {late}");
    assert!(g.convention() >= 0.8);
    let mut poor = Signalling::new(12, 6, 3, 1);
    poor.play(200_000);
    let p = poor.play(20_000);
    assert!(p < 0.55 && p > 0.35, "expressivité 3 sur 6 : réussite {p}");
}

/// Les agents de la scène font les actions culturelles de leur groupe, sans
/// rien changer à leur démographie.
#[test]
fn scene_agents_show_cultural_actions() {
    let rates = PopRates { birth: 0.5, death: 0.5, predation: 0.0, emigration: 0.0 };
    let band = with_body(evo_agents::bench::sample(rates, 40, 3, true), 1.5, 3.0, 30.0, Diet::Grazer);
    let mut scene = Scene::new(vec![band], 200.0, 9, 0.5);
    let hint = CultureHint { tool: 0.1, teach: 0.05, trade: 0.03, rite: 0.05 };
    scene.set_culture(0, hint);
    for _ in 0..(120.0 / 0.05) as usize {
        scene.step(0.05);
    }
    let mut seen = [0.0f32; Act::ALL.len()];
    for a in &scene.agents {
        for (k, t) in seen.iter_mut().enumerate() {
            *t += a.time_in[k];
        }
    }
    for act in [Act::MakeTool, Act::Teach, Act::Trade, Act::Rite] {
        assert!(seen[act.index()] > 0.0, "action jamais vue : {act:?}");
    }
}
