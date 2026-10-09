extends Node
## Scénario de la porte de l'étape 4, volet client. Lancé par
##   godot --path client -- --porte4 [--monde=terre] [--niveau=4]
##         [--graine=2026] [--sortie=DOSSIER]
##
## Le joueur laisse la vie s'installer, provoque un impact météoritique,
## puis demande « et sans mon intervention ? » : la branche rejoue la
## planète sans l'impact depuis le point de sauvegarde écrit juste avant,
## rattrape la partie et se compare dans l'interface (tableau, jalons,
## courbes, écart sur le globe). Il pose aussi une barrière et une poussée
## climatique, ouvre le réseau trophique et la colonne stratigraphique
## d'une cellule, l'anatomie et le comparateur d'espèces.
##
## Rapport dans DOSSIER/rapport.json ; code de sortie 0 si la porte est
## franchie.

var opts := {"monde": "terre", "niveau": "4", "graine": "2026", "sortie": "user://porte4", "ma": "6"}
var report := {}
var out_dir := ""
var shots: Array = []

func _ready() -> void:
	for a in OS.get_cmdline_user_args():
		if a.begins_with("--") and "=" in a:
			var kv := a.substr(2).split("=", true, 1)
			opts[kv[0]] = kv[1]
	out_dir = ProjectSettings.globalize_path(opts["sortie"])
	DirAccess.make_dir_recursive_absolute(out_dir)
	App.settings["narrator"] = false
	App.settings["autosave_minutes"] = 0
	App.settings["level"] = int(opts["niveau"])
	App.apply_settings()
	_scenario.call_deferred()

func _log(s: String) -> void:
	print("[porte4] ", s)

func _wait(seconds: float) -> void:
	await get_tree().create_timer(seconds).timeout

func _until(cond: Callable, timeout: float) -> bool:
	var t0 := Time.get_ticks_msec()
	while not cond.call():
		if Time.get_ticks_msec() - t0 > timeout * 1000.0:
			return false
		await get_tree().process_frame
	return true

func _shot(name: String) -> void:
	if DisplayServer.get_name() == "headless":
		return
	await RenderingServer.frame_post_draw
	var img := get_viewport().get_texture().get_image()
	img.save_png(out_dir.path_join(name + ".png"))
	shots.append(name)
	_log("capture " + name)

func _finish(passed: bool) -> void:
	report["passed"] = passed
	report["captures"] = shots
	var f := FileAccess.open(out_dir.path_join("rapport.json"), FileAccess.WRITE)
	f.store_string(JSON.stringify(report, "  "))
	f.close()
	_log("PORTE " + ("FRANCHIE" if passed else "NON FRANCHIE"))
	App.session.stop()
	get_tree().quit(0 if passed else 1)

func _screen() -> Control:
	return get_parent().screen

func _years() -> float:
	return float(App.session.frame_info().get("years", 0.0))

## Avance jusqu'à `years` puis met en pause.
func _run_to(years: float) -> bool:
	App.session.pause_at(years)
	App.session.resume()
	return await _until(func(): return bool(App.session.frame_info().get("paused", false)) and _years() >= years - 1.0, 900.0)

func _populated_cell() -> int:
	var ids: PackedInt64Array = App.session.guild_ids()
	if ids.is_empty():
		return -1
	return int(App.session.species_info(ids[0]).get("peak_cell", -1))

## Essai court du globe G3 : un massif vu en région puis en paysage.
func _g3_only() -> void:
	await _run_to(2.0e5)
	var top: int = App.session.highest_cell(true)
	App.globe.go_to_cell(top, 1.22)
	var ok := await _until(func(): return App.globe.region.visible, 60.0)
	await _wait(1.5)
	await _shot("g3-region")
	App.globe.target_distance = 1.03
	ok = ok and await _until(func(): return App.globe.region.visible and App.globe.region_params.y == 5, 60.0)
	await _wait(2.0)
	await _shot("g3-paysage")
	_finish(ok)

## Essai des corps simulés : la Terre tourne jusqu'à sa première colonie
## pluricellulaire, dont on ouvre la fiche, l'anatomie et le comparateur.
func _colony_only() -> void:
	var sp := -1
	var limit := float(opts.get("ma_max", "400")) * 1.0e6
	App.session.set_speed(1.0e7)
	await _run_to(float(opts.get("ma_min", "0")) * 1.0e6)
	while sp < 0 and _years() < limit:
		await _run_to(_years() + 1.0e7)
		sp = App.session.most_abundant_complex(false)
	report["colonie"] = sp
	report["colonie_date"] = App.session.frame_info().get("date", "")
	_log("colonie : %d, %s" % [sp, report["colonie_date"]])
	if sp < 0:
		_finish(false)
		return
	var jeu = _screen()
	jeu.open_species(sp)
	await _until(func(): return jeu.fiche.figure.texture != null and jeu.fiche.decor.texture != null, 60.0)
	await _wait(0.5)
	await _shot("15-colonie-fiche")
	jeu.open_anatomy(sp)
	var body_ok := await _until(func(): return not jeu.fiche.portrait.info.is_empty(), 60.0)
	if body_ok:
		report["colonie_triangles"] = jeu.fiche.portrait.info["lods"][0]["indices"].size() / 3
		report["colonie_taille"] = jeu.fiche.portrait.info["traits"]["size"]
		report["colonie_types"] = int(jeu.fiche.portrait.info["traits"]["cell_types"])
	await _wait(1.0)
	await _shot("16-colonie-anatomie")
	jeu.fiche.anatomy_button.button_pressed = true
	await _wait(0.6)
	await _shot("17-colonie-organes")
	jeu.open_comparator(sp)
	var cmp_ok := await _until(func(): return bool(jeu.fiche.data.get("ready", false)), 60.0)
	await _wait(1.0)
	var rows := []
	for r in jeu.fiche.data.get("rows", []):
		rows.append("%s : %s / %s" % [r["label"], r["a"], r["b"]])
	report["colonie_comparateur"] = rows
	await _shot("18-colonie-comparateur")
	_finish(body_ok and cmp_ok)

func _scenario() -> void:
	report["monde"] = opts["monde"]
	report["niveau"] = int(opts["niveau"])
	report["graine"] = int(opts["graine"])
	# Bac à sable : les trois interventions passent sans attendre la recharge
	# de l'influence (le coût reste compté).
	App.session.start_game(opts["monde"], int(opts["graine"]), int(opts["niveau"]), 1.0, 1.0, 0.0, true)
	await _until(func(): return App.session.has_frame(), 120.0)
	App.session.set_rules_profile("aucun")
	App.session.set_seeding("sources")
	App.session.seed_life()
	App.goto("jeu", {"new": false})
	await get_tree().process_frame
	var jeu = _screen()
	App.session.set_rules_profile("aucun")
	App.session.set_speed(1.0e6)
	if opts.get("seul", "") == "g3":
		await _g3_only()
		return
	if opts.get("seul", "") == "colonie":
		await _colony_only()
		return
	var ma := float(opts["ma"]) * 1.0e6
	var grown := await _run_to(ma)
	report["vie_installee"] = grown and int(App.session.frame_info().get("lineages", 0)) > 0
	_log("vie installée : %s, %s" % [grown, App.session.frame_info().get("date", "")])

	# Impact météoritique de 10 km sur la cellule la plus peuplée.
	var cell := _populated_cell()
	report["cellule"] = cell
	App.globe.select_cell(cell)
	App.globe.go_to_cell(cell, 2.4)
	jeu.open_interventions()
	await _wait(0.3)
	var panel = jeu.fiche
	panel._choose("impact")
	panel.sliders["diameter_km"].value = 1.0
	await _wait(0.5)
	await _shot("01-impact")
	panel._confirm()
	await get_tree().process_frame
	var interventions: Array = App.session.interventions()
	report["intervention"] = interventions[0]["label"] if not interventions.is_empty() else ""
	var order := int(interventions[0]["order"]) if not interventions.is_empty() else -1

	# Barrière et poussée climatique, en bac à sable ou si l'influence suit.
	var iso := {"length_km": 2500.0, "azimuth_deg": 30.0, "duration_years": 5.0e6, "sea": true}
	var hot := {"delta_k": 6.0, "rain_factor": 0.6, "radius_km": 2000.0, "duration_years": 3.0e6}
	var o_iso: int = App.session.intervene_ex("isolement", iso, cell, App.save_path("porte4-isolement"), "porte4-isolement")
	var o_hot: int = App.session.intervene_ex("climat", hot, cell, App.save_path("porte4-climat"), "porte4-climat")
	report["isolement_envoye"] = o_iso >= 0
	report["climat_envoye"] = o_hot >= 0
	var after := ma + 2.0e6
	await _run_to(after)
	var dist: Dictionary = App.session.disturbances()
	report["barrieres"] = dist.get("barriers", []).size()
	report["anomalies"] = dist.get("anomalies", []).size()
	App.globe.target_distance = 2.6
	await _wait(1.5)
	await _shot("02-barriere-et-climat")

	# Et sans l'impact ? La branche tourne pendant la pause.
	jeu.open_with_without()
	await _wait(0.3)
	var aw = jeu.fiche
	aw.pauses_only.button_pressed = true
	aw._start(order)
	var t_branch := Time.get_ticks_msec()
	var caught := await _until(func(): return bool(App.session.branch_status().get("caught_up", false)), 900.0)
	report["branche_rattrapee"] = caught
	report["branche_s"] = (Time.get_ticks_msec() - t_branch) / 1000.0
	await _wait(1.0)
	var cmp: Dictionary = App.session.branch_comparison()
	report["comparaison_meme_date"] = bool(cmp.get("same_date", false))
	var rows := []
	for r in cmp.get("rows", []):
		rows.append("%s : %s / %s (%s)" % [r["label"], r["with"], r["without"], r["change_text"]])
	report["comparaison"] = rows
	report["jalons"] = cmp.get("milestones", []).size()
	report["carte_changee"] = cmp.get("changed_share", "")
	_log("comparaison : %s" % str(rows))
	await _shot("03-avec-et-sans")
	aw.map_button.button_pressed = true
	aw.visible = false
	App.globe.target_distance = 3.6
	await _wait(1.5)
	await _shot("04-ecart-sur-le-globe")
	aw.visible = true
	aw.close()

	# Réseau trophique et colonne stratigraphique de la cellule.
	cell = _populated_cell()
	jeu.inspecteur.show_cell(cell)
	jeu.open_tool("reseau", cell)
	var web_ok := await _until(func(): return not jeu.fiche.data.get("nodes", []).is_empty(), 30.0)
	report["reseau_especes"] = jeu.fiche.data.get("nodes", []).size()
	report["reseau_liens"] = jeu.fiche.data.get("edges", []).size()
	await _wait(0.5)
	await _shot("05-reseau-trophique")
	jeu.open_tool("strates", cell)
	var strata_ok := await _until(func(): return not jeu.fiche.data.get("layers", []).is_empty(), 30.0)
	var layers: Array = jeu.fiche.data.get("layers", [])
	report["strates"] = layers.size()
	var iridium := false
	for l in layers:
		iridium = iridium or bool(l["impact"])
	report["iridium"] = iridium
	await _wait(0.5)
	await _shot("06-colonne-stratigraphique")
	jeu.fiche.close()

	# Globe G3 : la région subdivisée autour de la cellule, puis le
	# paysage vu de près, caméra redressée vers l'horizon.
	App.globe.go_to_cell(cell, 1.22)
	var region_ok := await _until(func(): return App.globe.region.visible, 60.0)
	await _wait(1.5)
	await _shot("12-region")
	App.globe.target_distance = 1.03
	var landscape_ok := await _until(func(): return App.globe.region.visible and App.globe.region_params.y == 5, 60.0)
	await _wait(2.0)
	if landscape_ok:
		report["g3_paysage_triangles"] = App.globe.region.mesh.surface_get_array_index_len(0) / 3
	report["g3_region"] = region_ok
	report["g3_paysage"] = landscape_ok
	await _shot("13-paysage")
	App.globe.target_distance = 3.0
	await _wait(1.0)

	# Fiche d'espèce : planche devant le décor, avec les silhouettes des
	# espèces voisines.
	var sp := int(App.session.guild_ids()[0])
	jeu.open_species(sp)
	await _until(func(): return jeu.fiche.decor.texture != null, 30.0)
	await _wait(0.5)
	await _shot("07-fiche-et-decor")
	# Anatomie depuis le plan de construction (palier 2).
	jeu.open_anatomy(sp)
	var body_ok := await _until(func(): return not jeu.fiche.portrait.info.is_empty(), 30.0)
	if body_ok:
		var lods: Array = jeu.fiche.portrait.info["lods"]
		report["anatomie_triangles"] = lods[0]["indices"].size() / 3
		report["anatomie_taille"] = jeu.fiche.portrait.info["traits"]["size"]
	jeu.fiche.anatomy_button.button_pressed = true
	await _wait(1.0)
	await _shot("08-anatomie")
	# Comparateur : deux espèces à la même échelle, ancêtre commun.
	jeu.open_comparator(sp)
	var cmp_ok := await _until(func(): return bool(jeu.fiche.data.get("ready", false)), 30.0)
	await _wait(1.0)
	report["comparateur_lignes"] = jeu.fiche.data.get("rows", []).size()
	report["ancetre_commun"] = str(jeu.fiche.data.get("ancestor_name", ""))
	await _shot("09-comparateur")
	# Banc d'essai du générateur : un corps pluricellulaire tiré au hasard,
	# peau puis organes.
	var test = jeu._open(jeu.Anatomie.new())
	test.open_test(int(opts["graine"]))
	var test_ok := await _until(func(): return not test.portrait.info.is_empty(), 30.0)
	if test_ok:
		report["essai_triangles"] = test.portrait.info["lods"][0]["indices"].size() / 3
		report["essai_os"] = int(test.portrait.info["bone_count"])
	await _wait(1.0)
	await _shot("10-corps-essai")
	test.anatomy_button.button_pressed = true
	await _wait(0.6)
	await _shot("11-corps-essai-anatomie")
	test.close()

	var passed: bool = report["vie_installee"] and order >= 0 and caught and report["comparaison_meme_date"] and rows.size() >= 5 \
		and web_ok and strata_ok and iridium and report["barrieres"] >= 1 \
		and body_ok and cmp_ok and int(report["comparateur_lignes"]) >= 10 and test_ok \
		and region_ok and landscape_ok
	_finish(passed)
