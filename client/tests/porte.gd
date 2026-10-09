extends Node
## Scénario de la porte de l'étape 3, volet client. Lancé par
##   godot --path client -- --porte [--rejeu] [--monde=terre] [--niveau=4]
##         [--graine=2026] [--sortie=DOSSIER] [--duree-max=900]
##
## Sans --rejeu : le joueur crée une planète, l'ensemence, intervient, et
## voit l'oxygène monter ; le client parcourt ses écrans et en fait des
## captures ; images par seconde et temps du pont sont mesurés.
## Avec --rejeu (sans affichage) : deux parties de même graine suivent deux
## chemins de caméra différents et doivent finir dans le même état, puis un
## point de sauvegarde rechargé doit retrouver ce même état.
##
## Le rapport est écrit dans DOSSIER/rapport.json ; le code de sortie vaut 0
## si la porte est franchie.

const O2_THRESHOLD := 1.0e-4

var opts := {"monde": "terre", "niveau": "4", "graine": "2026", "sortie": "user://porte", "duree-max": "900", "pas": "60"}
var report := {}
var out_dir := ""
var shots: Array = []
var fps_samples: Array = []
var frame_ms: Array = []
var bridge: Array = []

func _ready() -> void:
	for a in OS.get_cmdline_user_args():
		if a.begins_with("--") and "=" in a:
			var kv := a.substr(2).split("=", true, 1)
			opts[kv[0]] = kv[1]
	out_dir = ProjectSettings.globalize_path(opts["sortie"])
	DirAccess.make_dir_recursive_absolute(out_dir)
	# Réglages neutres : pas de narrateur, ni de sauvegarde automatique.
	App.settings["narrator"] = false
	App.settings["autosave_minutes"] = 0
	App.settings["level"] = int(opts["niveau"])
	App.apply_settings()
	if "--rejeu" in OS.get_cmdline_user_args():
		_replay_scenario.call_deferred()
	else:
		_play_scenario.call_deferred()

func _log(s: String) -> void:
	print("[porte] ", s)

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
	await RenderingServer.frame_post_draw
	var img := get_viewport().get_texture().get_image()
	var path := out_dir.path_join(name + ".png")
	img.save_png(path)
	shots.append(name)
	_log("capture " + name)

func _finish(passed: bool) -> void:
	report["passed"] = passed
	var f := FileAccess.open(out_dir.path_join("rapport.json"), FileAccess.WRITE)
	f.store_string(JSON.stringify(report, "  "))
	f.close()
	_log("rapport écrit : " + out_dir.path_join("rapport.json"))
	_log("PORTE " + ("FRANCHIE" if passed else "NON FRANCHIE"))
	App.session.stop()
	get_tree().quit(0 if passed else 1)

func _screen() -> Control:
	return get_parent().screen

# ----------------------------------------------------------------------
# Partie jouée

func _play_scenario() -> void:
	var main := get_parent()
	report["mode"] = "partie"
	report["monde"] = opts["monde"]
	report["niveau"] = int(opts["niveau"])
	report["graine"] = int(opts["graine"])
	report["rendu"] = RenderingServer.get_video_adapter_name() + " — " + RenderingServer.get_video_adapter_vendor()
	report["fenetre"] = [get_viewport().get_visible_rect().size.x, get_viewport().get_visible_rect().size.y]

	App.goto("accueil")
	await _until(func(): return App.session.has_frame(), 60.0)
	await _wait(1.0)
	await _shot("01-accueil")

	# Création : le préréglage et la graine du scénario.
	App.goto("creation")
	var creation = _screen()
	creation.spec["preset"] = opts["monde"]
	creation.spec["seed"] = int(opts["graine"])
	creation.spec["level"] = int(opts["niveau"])
	creation.preset_option.selected = creation.PRESETS.find(opts["monde"])
	creation.seed_edit.text = opts["graine"]
	creation._regenerate()
	var created := await _until(func(): return App.session.has_frame() and not creation.waiting, 120.0)
	report["planete_creee"] = created
	report["code_planete"] = App.session.planet_code()
	await _wait(0.5)
	await _shot("02-creation")

	App.goto("ensemencement")
	await _wait(0.8)
	await _shot("03-ensemencement")
	var seeding = _screen()
	seeding.begin()
	await get_tree().process_frame
	var jeu = _screen()
	# Rien n'arrête le temps pendant le scénario ; vitesse maximale.
	App.session.set_rules_profile("aucun")
	jeu.bar.set_speed(1.0e7)
	var seeded := await _until(func(): return int(App.session.frame_info().get("lineages", 0)) > 0, 60.0)
	report["vie_deposee"] = seeded
	await _wait(2.0)
	await _shot("04-partie")

	# Mesures pendant que le temps file.
	var measuring := true
	var sampler := func():
		while measuring:
			await get_tree().process_frame
			frame_ms.append(get_process_delta_time() * 1000.0)
			bridge.append(App.session.bridge_ms())
	sampler.call()

	# Intervention du joueur : phosphate autour de la cellule où vit
	# l'espèce la plus répandue, dès que la vie est là.
	var target := _populated_cell()
	report["cellule_intervention"] = target
	App.globe.select_cell(target)
	jeu.open_interventions()
	await _wait(0.5)
	await _shot("05-interventions")
	jeu.fiche._confirm()
	var applied := await _until(func(): return _player_event_seen(), 30.0)
	report["intervention_appliquee"] = applied

	# L'inspecteur sur une cellule peuplée, la loupe.
	var cell := _populated_cell()
	report["cellule_inspectee"] = cell
	if cell >= 0:
		App.globe.go_to_cell(cell, 2.0)
		jeu.inspecteur.show_cell(cell)
		await _wait(2.5)
		await _shot("06-inspecteur")

	# Le temps file jusqu'à ce que l'oxygène monte.
	var max_s := float(opts["duree-max"])
	var t0 := Time.get_ticks_msec()
	var reached := -1.0
	var shot_layers := false
	while (Time.get_ticks_msec() - t0) / 1000.0 < max_s:
		await _wait(1.0)
		var info: Dictionary = App.session.frame_info()
		fps_samples.append(Engine.get_frames_per_second())
		if info.is_empty():
			continue
		if not shot_layers and float(info["years"]) > 2.0e7:
			shot_layers = true
			jeu.calques.select_key("biomasse")
			await _wait(1.0)
			await _shot("07-calque-biomasse")
			jeu.calques.select_key("guildes")
			await _wait(1.0)
			await _shot("08-calque-guildes")
			jeu.calques.select_index(-1)
			jeu.calques.buttons[0].button_pressed = true
			jeu.calques._select(-1)
		if float(info["o2"]) >= O2_THRESHOLD and int(info["photosynthesis_stage"]) >= 4:
			reached = float(info["years"])
			break
	measuring = false
	var info: Dictionary = App.session.frame_info()
	report["annees_jouees"] = float(info.get("years", 0.0))
	report["date"] = info.get("date", "")
	report["o2_final"] = float(info.get("o2", 0.0))
	report["etape_photosynthese"] = int(info.get("photosynthesis_stage", 0))
	report["oxygene_monte_a_annees"] = reached
	report["temps_reel_s"] = (Time.get_ticks_msec() - t0) / 1000.0
	report["lignees_vivantes"] = int(info.get("lineages", 0))
	App.session.pause()
	await _wait(0.5)

	# Le joueur voit l'oxygène : calque, frise, fiche, arbre, chronique.
	jeu.calques.select_key("oxygene")
	await _wait(1.0)
	await _shot("09-calque-oxygene")
	jeu.calques.buttons[0].button_pressed = true
	jeu.calques._select(-1)
	App.globe.target_distance = 4.0
	await _wait(1.0)
	await _shot("10-oxygene-frise")
	var top := _top_species()
	report["espece_montree"] = top
	if top >= 0:
		jeu.open_species(top)
		await _wait(3.0)
		await _shot("11-fiche-espece")
	jeu.open_tree()
	await _wait(1.0)
	await _shot("12-arbre")
	jeu.open_chronicle()
	await _wait(1.0)
	await _shot("13-chronique")
	jeu.fiche.close()

	# Point de sauvegarde.
	# Le moteur écrit l'état complet en tâche de fond : on attend son avis
	# (« ok » ou « erreur »), pas seulement l'apparition du fichier.
	var save := App.save_path("porte")
	var t_save := Time.get_ticks_msec()
	App.session.save(save, "porte")
	# L'écran de jeu relève lui-même les réponses du moteur à chaque image.
	await get_tree().process_frame
	await _until(func(): return App.session.saves_pending() == 0, 180.0)
	var saved: bool = App.session.saves_pending() == 0 and FileAccess.file_exists(save) and FileAccess.get_file_as_bytes(save).size() > 0
	report["sauvegarde"] = saved
	report["sauvegarde_s"] = (Time.get_ticks_msec() - t_save) / 1000.0
	_log("sauvegarde : %s en %.1f s" % [saved, report["sauvegarde_s"]])

	report["images_par_seconde"] = _stats(fps_samples)
	report["duree_image_ms"] = _stats(frame_ms)
	report["pont_ms"] = _stats(bridge)
	report["captures"] = shots
	var passed: bool = created and seeded and applied and reached > 0.0 and saved
	_log("créée %s, ensemencée %s, intervention %s, oxygène à %s ans, sauvegarde %s" % [created, seeded, applied, reached, saved])
	_finish(passed)

func _player_event_seen() -> bool:
	for e in App.session.events_page(0, 20, "intervention", ""):
		if bool(e["player"]) and int(e["level"]) >= 0 and str(e["text"]).to_lower().contains("phosph"):
			return true
	return false

func _populated_cell() -> int:
	# Cellule d'apogée de l'espèce la plus répandue (cellule du vivant, qui
	# est aussi une cellule physique : les grilles sont emboîtées).
	var ids: PackedInt64Array = App.session.guild_ids()
	if ids.is_empty():
		return -1
	return int(App.session.species_info(ids[0]).get("peak_cell", -1))

func _top_species() -> int:
	# L'espèce qui domine le plus de cellules.
	var ids: PackedInt64Array = App.session.guild_ids()
	return -1 if ids.is_empty() else int(ids[0])

func _stats(a: Array) -> Dictionary:
	if a.is_empty():
		return {}
	var s := a.duplicate()
	s.sort()
	var sum := 0.0
	for x in s:
		sum += x
	return {
		"n": s.size(),
		"moyenne": sum / s.size(),
		"min": s[0],
		"p05": s[int(s.size() * 0.05)],
		"mediane": s[s.size() / 2],
		"p95": s[min(s.size() - 1, int(s.size() * 0.95))],
		"max": s[s.size() - 1],
	}

# ----------------------------------------------------------------------
# Rejeu : deux chemins de caméra, un point de sauvegarde

func _replay_scenario() -> void:
	report["mode"] = "rejeu"
	report["systeme"] = OS.get_name()
	report["monde"] = opts["monde"]
	report["niveau"] = int(opts["niveau"])
	report["graine"] = int(opts["graine"])
	var steps := int(opts["pas"])
	report["pas"] = steps
	var hashes := []
	for path in [0, 1]:
		var h: Array = await _run_path(path, steps)
		hashes.append(h)
		_log("chemin %d : pas %s, empreinte %s" % [path, str(h[0]) if h.size() > 0 else "—", str(h[1]) if h.size() > 1 else "—"])
		if path == 0:
			App.session.save(App.save_path("porte-rejeu"), "porte-rejeu")
			var notice := [""]
			var saved := func():
				App.session.poll_events()
				var n: String = App.session.take_notice()
				if n != "":
					notice[0] = n
				return notice[0] != ""
			await _until(saved, 60.0)
			_log("sauvegarde : %s" % notice[0])
	# Le point de sauvegarde, rechargé, retrouve l'état.
	var err: String = App.session.load_save(App.save_path("porte-rejeu"))
	var loaded: Array = []
	if err == "":
		await _until(func(): return App.session.loading_progress().y < 0.0 and App.session.state_hash().size() == 2 and _years() >= steps * 1.0e5 - 1.0, 600.0)
		loaded = App.session.state_hash()
	_log("rechargée : %s" % str(loaded))
	report["empreintes"] = {"chemin_1": hashes[0], "chemin_2": hashes[1], "sauvegarde_rechargee": loaded}
	var same: bool = hashes[0].size() == 2 and hashes[0] == hashes[1] and loaded == hashes[0]
	report["identiques"] = same
	_finish(same)

func _years() -> float:
	return float(App.session.frame_info().get("years", 0.0))

func _run_path(path: int, steps: int) -> Array:
	App.session.start(opts["monde"], int(opts["graine"]), int(opts["niveau"]), 1.0, 1.0, 0.0)
	await _until(func(): return App.session.has_frame(), 120.0)
	App.session.set_rules_profile("aucun")
	App.session.set_seeding("sources")
	App.session.seed_life()
	# Pas demandé de 100 ka (le moteur l'allonge aux périodes calmes) : la
	# pause tombe à la fin du pas qui atteint la date visée.
	App.session.set_speed(1.0e6)
	App.session.intervene("phosphate", 1.0e14, 0, 1500.0)
	App.session.pause_at(steps * 1.0e5)
	App.session.resume()
	# Deux caméras : l'une tourne autour de l'équateur de loin, l'autre
	# plonge sur un pôle de près. Le canal d'observation ne doit rien changer.
	var t := 0.0
	var done := func():
		var h: Array = App.session.state_hash()
		return h.size() == 2 and _years() >= steps * 1.0e5 - 1.0
	while not done.call():
		t += get_process_delta_time()
		var dir := Vector3(cos(t), 0.2, sin(t)) if path == 0 else Vector3(0.1 * cos(3.0 * t), 1.0, 0.1 * sin(3.0 * t))
		App.session.observe(dir.normalized(), 1.2 if path == 0 else 0.05, 1 if path == 0 else 6)
		if App.session.has_new_frame():
			App.session.frame_textures(false)
		await get_tree().process_frame
	return App.session.state_hash()
