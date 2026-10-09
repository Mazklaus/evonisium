//! Outils de l'étape 4 vus depuis Godot : interventions du monde
//! multicellulaire, branches « avec et sans », réseau trophique, colonne
//! stratigraphique, perturbations dessinées sur le globe.

use super::*;
use crate::runner::BranchPace;
use evo_engine::strata::Rock;
use evo_view::compare;
use evo_view::foodweb;

fn num(d: &VarDictionary, key: &str, default: f64) -> f64 {
    d.get(key).and_then(|v| v.try_to::<f64>().ok().or_else(|| v.try_to::<i64>().ok().map(|x| x as f64))).unwrap_or(default)
}

fn flag(d: &VarDictionary, key: &str) -> bool {
    d.get(key).and_then(|v| v.try_to::<bool>().ok()).unwrap_or(false)
}

impl EvoSession {
    /// Intervention décrite par un type et ses réglages.
    fn intervention_from(kind: &str, p: &VarDictionary, cell: u32) -> Option<Intervention> {
        Some(match kind {
            "impact" => Intervention::Impact { cell, diameter_km: num(p, "diameter_km", 5.0).clamp(0.1, 100.0) },
            "isolement" => Intervention::Isolate {
                cell,
                azimuth_deg: num(p, "azimuth_deg", 0.0),
                length_km: num(p, "length_km", 1500.0).clamp(100.0, 10_000.0),
                duration_years: num(p, "duration_years", 1e6).clamp(1e3, 1e8),
                sea: flag(p, "sea"),
            },
            "climat" => Intervention::ClimatePulse {
                cell,
                radius_km: num(p, "radius_km", 1500.0).clamp(100.0, 10_000.0),
                delta_k: num(p, "delta_k", -5.0).clamp(-20.0, 20.0),
                rain_factor: num(p, "rain_factor", 1.0).clamp(0.1, 5.0),
                duration_years: num(p, "duration_years", 1e5).clamp(10.0, 1e8),
            },
            other => return Self::intervention(other, num(p, "moles", 1e13), cell, num(p, "radius_km", 1500.0).max(1.0)),
        })
    }

    fn stratum_name(rock: Rock, fr: bool) -> &'static str {
        match (rock, fr) {
            (Rock::Limestone, true) => "calcaire",
            (Rock::Marl, true) => "marnes",
            (Rock::BlackShale, true) => "schistes noirs",
            (Rock::BandedIron, true) => "fer rubané",
            (Rock::Sandstone, true) => "grès",
            (Rock::RedBeds, true) => "grès rouges",
            (Rock::Tillite, true) => "tillite",
            (Rock::Limestone, false) => "limestone",
            (Rock::Marl, false) => "marl",
            (Rock::BlackShale, false) => "black shale",
            (Rock::BandedIron, false) => "banded iron",
            (Rock::Sandstone, false) => "sandstone",
            (Rock::RedBeds, false) => "red beds",
            (Rock::Tillite, false) => "tillite",
        }
    }
}

#[godot_api(secondary)]
impl EvoSession {
    /// Coût en influence d'une intervention décrite par ses réglages
    /// (`impact` : diameter_km ; `isolement` : length_km, azimuth_deg,
    /// duration_years, sea ; `climat` : radius_km, delta_k, rain_factor,
    /// duration_years ; les autres : moles, radius_km).
    #[func]
    fn intervention_cost_ex(&self, kind: GString, params: VarDictionary) -> f64 {
        Self::intervention_from(&kind.to_string(), &params, 0).map_or(0.0, |i| i.cost())
    }

    /// Aperçu des effets attendus d'une intervention de l'étape 4 (chaîne
    /// vide pour les autres, dont l'aperçu est un texte fixe).
    #[func]
    fn intervention_preview(&self, kind: GString, params: VarDictionary) -> GString {
        let fr = self.lang == Lang::Fr;
        let dur = |y: f64| format::duration(y, self.lang);
        let n = |x: f64, d: usize| format::number_in(self.lang, x, d);
        let text = match Self::intervention_from(&kind.to_string(), &params, 0) {
            Some(Intervention::Impact { diameter_km, .. }) => {
                use evo_sim::disturbance as dist;
                let r = dist::kill_radius_m(diameter_km) / 1e3;
                let cool = dist::impact_cooling_k(diameter_km);
                let winter = dist::impact_winter_years(diameter_km);
                let co2 = dist::impact_co2(diameter_km, 0.8);
                let e = dist::impact_energy(diameter_km) / dist::CHICXULUB_J;
                if fr {
                    format!(
                        "Énergie : {} fois Chicxulub. La vie meurt dans un rayon de {} km (toute au centre, de moins en moins vers le bord). Hiver d'impact : −{} K sur toute la planète pendant {} environ. Jusqu'à {} mol de CO₂ si la cible est une plate-forme carbonatée. Une couche à iridium marquera toutes les colonnes de roches.",
                        format::power_of_ten_in(self.lang, e), n(r, 0), n(cool, 1), dur(winter), format::power_of_ten_in(self.lang, co2)
                    )
                } else {
                    format!(
                        "Energy: {} times Chicxulub. Life dies within {} km (all of it at the centre, less towards the edge). Impact winter: −{} K planet-wide for about {}. Up to {} mol of CO₂ if the target is a carbonate platform. An iridium layer will mark every rock column.",
                        format::power_of_ten_in(self.lang, e), n(r, 0), n(cool, 1), dur(winter), format::power_of_ten_in(self.lang, co2)
                    )
                }
            }
            Some(Intervention::Isolate { length_km, duration_years, sea, .. }) => {
                let what = match (sea, fr) {
                    (true, true) => "Un bras de mer",
                    (false, true) => "Une chaîne de montagnes",
                    (true, false) => "A sea strait",
                    (false, false) => "A mountain range",
                };
                if fr {
                    format!("{what} de {} km coupe les migrations pendant {}. Les populations des deux côtés évoluent séparément : c'est le premier pas d'une spéciation par isolement.", n(length_km, 0), dur(duration_years))
                } else {
                    format!("{what} {} km long blocks migration for {}. Populations on either side evolve apart: the first step of speciation by isolation.", n(length_km, 0), dur(duration_years))
                }
            }
            Some(Intervention::ClimatePulse { radius_km, delta_k, rain_factor, duration_years, .. }) => {
                if fr {
                    format!("Écart de {}{} K et pluie ×{} sur {} km de rayon pendant {}. Le climat du moteur est à l'équilibre à chaque pas : une poussée plus courte que le pas n'agit qu'au prorata de sa durée.", if delta_k >= 0.0 { "+" } else { "−" }, n(delta_k.abs(), 1), n(rain_factor, 2), n(radius_km, 0), dur(duration_years))
                } else {
                    format!("A {}{} K shift and rain ×{} over a {} km radius for {}. The engine's climate is at equilibrium each step: a pulse shorter than the step only acts pro rata.", if delta_k >= 0.0 { "+" } else { "−" }, n(delta_k.abs(), 1), n(rain_factor, 2), n(radius_km, 0), dur(duration_years))
                }
            }
            _ => String::new(),
        };
        GString::from(&text)
    }

    /// Intervention avec son point de sauvegarde juste avant ; renvoie
    /// l'identifiant de l'ordre, ou −1 si elle n'est pas possible.
    #[func]
    fn intervene_ex(&mut self, kind: GString, params: VarDictionary, cell: i64, save_path: GString, save_name: GString) -> i64 {
        let Some(f) = self.frame() else { return -1 };
        if cell < 0 || cell as usize >= f.cells().len() {
            return -1;
        }
        let Some(i) = Self::intervention_from(&kind.to_string(), &params, cell as u32) else { return -1 };
        let inf = &f.state.influence;
        if !inf.sandbox && (inf.points as f64) < i.cost() {
            return -1;
        }
        let Some(g) = &mut self.game else { return -1 };
        g.intervene(i, &save_path.to_string(), &save_name.to_string()) as i64
    }

    /// Interventions passées : ordre, date, libellé, refusée ou non.
    #[func]
    fn interventions(&self) -> VarArray {
        let mut out = VarArray::new();
        let Some(g) = &self.game else { return out };
        for i in g.interventions.iter().rev() {
            let refused = g.events.iter().any(|e| matches!(e.kind, EventKind::OrderRefused { order, .. } if order == i.order));
            let mut d = VarDictionary::new();
            d.set("order", i.order as i64);
            d.set("years", i.years);
            d.set("date", format::game_date(i.years, self.lang));
            d.set("label", i.label.as_str());
            d.set("refused", refused);
            d.set("ready", g.saves.iter().all(|(p, _)| *p != i.save) && std::path::Path::new(&i.save).exists());
            out.push(&d.to_variant());
        }
        out
    }

    /// Lance « et sans mon intervention ? » ; renvoie un message d'erreur
    /// ou une chaîne vide.
    #[func]
    fn start_branch(&mut self, order: i64, pauses_only: bool) -> GString {
        let Some(g) = &mut self.game else { return GString::from("pas de partie") };
        let pace = if pauses_only { BranchPace::Pauses } else { BranchPace::Together };
        // La branche prend deux cœurs ; ils sont pris à la partie quand les
        // deux tournent ensemble.
        match g.start_branch(order.max(0) as u64, pace, 2) {
            Ok(()) => GString::new(),
            Err(e) => GString::from(&e),
        }
    }

    #[func]
    fn stop_branch(&mut self) {
        if let Some(g) = &mut self.game {
            g.branch = None;
        }
    }

    /// Où en est la branche : active, avancement, date, intervention retirée.
    #[func]
    fn branch_status(&self) -> VarDictionary {
        let mut d = VarDictionary::new();
        let Some(b) = self.game.as_ref().and_then(|g| g.branch.as_ref()) else {
            d.set("active", false);
            return d;
        };
        let st = b.branch.status();
        d.set("active", true);
        d.set("progress", st.progress());
        d.set("caught_up", st.caught_up());
        d.set("running", st.running);
        d.set("date", if st.years.is_finite() { format::game_date(st.years, self.lang) } else { "—".into() });
        d.set("target", format::game_date(st.target_years, self.lang));
        d.set("label", b.without.label.as_str());
        d.set("order", b.without.order as i64);
        d.set("pauses_only", b.pace == BranchPace::Pauses);
        d.set("error", st.error.unwrap_or_default().as_str());
        d
    }

    /// Comparaison de la partie et de sa branche à la date atteinte par la
    /// branche : tableau, jalons, courbes, part de la carte changée.
    #[func]
    fn branch_comparison(&self) -> VarDictionary {
        let mut d = VarDictionary::new();
        let Some(g) = &self.game else { return d };
        let Some(b) = &g.branch else { return d };
        let (Some(without), with) = (b.branch.state(), g.engine.frame().current) else { return d };
        // Même date : la branche est à jour, ou la partie est comparée à la
        // date que la branche a atteinte (courbes seulement).
        let same_date = (without.years - with.years).abs() < 1e-6;
        d.set("same_date", same_date);
        let mut rows = VarArray::new();
        if same_date {
            for r in compare::rows(&with, &without, self.lang, self.celsius) {
                let mut x = VarDictionary::new();
                x.set("label", r.label.as_str());
                x.set("with", r.with.as_str());
                x.set("without", r.without.as_str());
                x.set("change", if r.change.is_finite() { r.change } else { 1e9 });
                x.set("change_text", r.change_text.as_str());
                rows.push(&x.to_variant());
            }
            d.set("changed_share", format::percent(compare::changed_share(&with, &without), self.lang).as_str());
        }
        d.set("rows", &rows);
        let history_b = b.branch.history();
        let mut ms = VarArray::new();
        for m in compare::milestones(&g.history, &history_b, self.lang) {
            let mut x = VarDictionary::new();
            x.set("label", m.label.as_str());
            x.set("with", m.with.map_or("—".into(), |y| format::game_date(y, self.lang)).as_str());
            x.set("without", m.without.map_or("—".into(), |y| format::game_date(y, self.lang)).as_str());
            ms.push(&x.to_variant());
        }
        d.set("milestones", &ms);
        // Courbes depuis l'intervention : O₂ et biomasse, avec et sans.
        let from = b.without.years;
        let mut years = PackedFloat64Array::new();
        let mut o2_a = PackedFloat64Array::new();
        let mut bio_a = PackedFloat64Array::new();
        for s in g.history.iter().filter(|s| s.years >= from && s.years <= without.years) {
            years.push(s.years);
            o2_a.push(s.o2_mixing);
            bio_a.push(s.biomass);
        }
        let mut years_b = PackedFloat64Array::new();
        let mut o2_b = PackedFloat64Array::new();
        let mut bio_b = PackedFloat64Array::new();
        for s in history_b.iter().filter(|s| s.years >= from) {
            years_b.push(s.years);
            o2_b.push(s.o2_mixing);
            bio_b.push(s.biomass);
        }
        d.set("years", &years);
        d.set("o2", &o2_a);
        d.set("biomass", &bio_a);
        d.set("years_without", &years_b);
        d.set("o2_without", &o2_b);
        d.set("biomass_without", &bio_b);
        d.set("label", b.without.label.as_str());
        d
    }

    /// Texture de comparaison pour le globe (calque « avec et sans »), ou
    /// rien si la branche n'est pas à la date de la partie.
    #[func]
    fn comparison_texture(&self) -> Option<Gd<Image>> {
        let g = self.game.as_ref()?;
        let without = g.branch.as_ref()?.branch.state()?;
        let with = g.engine.frame().current;
        if (without.years - with.years).abs() > 1e-6 {
            return None;
        }
        let (w, h) = layers::texture_size(with.cells.len());
        image_rgbaf(w, h, &compare::texture(&with, &without))
    }

    /// Réseau trophique de la cellule du vivant qui contient `cell` :
    /// espèces (nom, biomasse, niveau, couleur) et liens (de, vers, nature,
    /// ce qui passe, flux relatif de 0 à 1).
    #[func]
    fn food_web(&mut self, cell: i64) -> VarDictionary {
        let mut d = VarDictionary::new();
        let Some(f) = self.frame() else { return d };
        if cell < 0 || cell as usize >= f.cells().len() {
            return d;
        }
        let step = f.state.step;
        let detail = {
            let Some(g) = &mut self.game else { return d };
            g.request_cell(cell as u32, step);
            g.cell.as_ref().filter(|(x, _)| x.cell == cell as u32).map(|(x, _)| x.clone())
        };
        d.set("pending", detail.is_none());
        let Some(x) = detail else { return d };
        let web = foodweb::build(&x.populations);
        let max_flux = web.edges.iter().map(|e| e.flux).fold(0.0f64, f64::max);
        let mut nodes = VarArray::new();
        for n in &web.nodes {
            let mut nd = VarDictionary::new();
            nd.set("species", n.species as i64);
            nd.set("name", self.species_name(&f, n.species).as_str());
            nd.set("biomass", format::power_of_ten_in(self.lang, n.biomass).as_str());
            nd.set("production", format::power_of_ten_in(self.lang, n.production).as_str());
            nd.set("level", n.level as i64);
            nd.set("autotroph", n.autotroph);
            nd.set("colour", pigment_colour(n.pigment_rgb));
            nodes.push(&nd.to_variant());
        }
        let mut edges = VarArray::new();
        for e in &web.edges {
            let mut ed = VarDictionary::new();
            ed.set("from", e.from as i64);
            ed.set("to", e.to as i64);
            ed.set("link", e.link.key());
            ed.set("what", e.what);
            // Épaisseur : logarithme du flux, rapporté au plus fort.
            let rel = if max_flux > 0.0 { (1.0 + (e.flux / max_flux * 1e3).max(0.0)).log10() / 3.0f64.max(1.0) } else { 0.0 };
            ed.set("weight", rel.clamp(0.05, 1.0));
            ed.set("flux", format::power_of_ten_in(self.lang, e.flux).as_str());
            edges.push(&ed.to_variant());
        }
        d.set("nodes", &nodes);
        d.set("edges", &edges);
        d
    }

    /// Colonne stratigraphique de la région d'une cellule, de la couche la
    /// plus ancienne à la plus récente (`pending` tant que le moteur n'a
    /// pas répondu).
    #[func]
    fn strata(&mut self, cell: i64, layers: i64) -> VarDictionary {
        let mut d = VarDictionary::new();
        let Some(f) = self.frame() else { return d };
        if cell < 0 || cell as usize >= f.cells().len() {
            return d;
        }
        let fr = self.lang == Lang::Fr;
        let step = f.state.step;
        let list = {
            let Some(g) = &mut self.game else { return d };
            g.request_strata(cell as u32, step, layers.clamp(1, 200) as usize);
            g.strata.as_ref().filter(|(c, _, _)| *c == cell as u32).map(|(_, _, l)| l.clone())
        };
        d.set("pending", list.is_none());
        let mut out = VarArray::new();
        for s in list.unwrap_or_default() {
            let mut x = VarDictionary::new();
            x.set("rock", s.rock.key());
            x.set("rock_name", Self::stratum_name(s.rock, fr));
            x.set("marine", s.rock.marine());
            x.set("from", format::game_date(s.from_years, self.lang).as_str());
            x.set("to", format::game_date(s.to_years, self.lang).as_str());
            x.set("thickness_m", s.thickness_m);
            x.set("delta13c", if s.delta13c.is_finite() { s.delta13c } else { f64::NAN });
            x.set(
                "delta13c_text",
                if s.delta13c.is_finite() {
                    format!("δ¹³C {} ‰", format::number_in(self.lang, s.delta13c, 1))
                } else {
                    "δ¹³C —".into()
                },
            );
            x.set("stromatolites", s.stromatolites);
            x.set("impact", s.impact);
            x.set("fossil", if s.dominant_species == 0 { String::new() } else { self.species_name(&f, s.dominant_species) }.as_str());
            x.set("species_count", s.species_count as i64);
            out.push(&x.to_variant());
        }
        d.set("layers", &out);
        d
    }

    /// Perturbations en cours, pour le globe : arcs des barrières (points
    /// sur la sphère unité, repère Godot) et calottes des anomalies
    /// (centre, rayon angulaire, signe de l'écart de température).
    #[func]
    fn disturbances(&self) -> VarDictionary {
        let mut d = VarDictionary::new();
        let Some(f) = self.frame() else { return d };
        let mut barriers = VarArray::new();
        for b in &f.state.disturbances.barriers {
            let pts: Vec<Vector3> = b.polyline(24).into_iter().map(to_godot).collect();
            let mut x = VarDictionary::new();
            x.set("points", &PackedVector3Array::from(&pts[..]));
            x.set("sea", b.sea);
            x.set("until", format::game_date(b.until_years, self.lang).as_str());
            barriers.push(&x.to_variant());
        }
        let mut anomalies = VarArray::new();
        for a in &f.state.disturbances.anomalies {
            let mut x = VarDictionary::new();
            x.set("centre", to_godot(a.center));
            x.set("radius", if a.global { std::f64::consts::PI } else { a.radius });
            x.set("delta_k", a.delta_k);
            x.set("global", a.global);
            x.set("until", format::game_date(a.until_years, self.lang).as_str());
            anomalies.push(&x.to_variant());
        }
        d.set("barriers", &barriers);
        d.set("anomalies", &anomalies);
        d
    }
}
