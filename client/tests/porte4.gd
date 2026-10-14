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

## Essai de l'affichage de l'étape 6 sur peuples d'essai : territoires et
## noms sur le globe, inspecteur, fenêtre des peuples (liste, arbres des
## peuples et des langues, personnages), fiches, mythes de la chronique et
## mode historique dans la barre du temps.
func _peoples_only() -> void:
	await _run_to(2.0e5)
	var jeu = _screen()
	jeu._toggle_peoples(true)
	var ok := await _until(func(): return App.globe.peoples_ready, 120.0)
	report["peuples_prets"] = ok
	if not ok:
		_finish(false)
		return
	var labels: Array = App.session.culture_labels()
	report["peuples"] = labels.size()
	report["noms"] = labels.map(func(l): return l["name"])
	var mode: Dictionary = App.session.culture_mode()
	report["mode_historique"] = bool(mode.get("historical", false))
	var core := int(labels[0]["cell"])
	App.globe.go_to_cell(core, 2.6)
	await _wait(1.5)
	await _shot("peuples-globe")
	App.globe.go_to_cell(core, 1.5)
	jeu.inspecteur.show_cell(core)
	App.globe.select_cell(core)
	await _wait(1.5)
	report["etat_temps"] = jeu.bar.state_label.text
	await _shot("peuples-inspecteur")
	jeu.inspecteur.show_cell(-1)
	jeu.open_peoples()
	await _wait(0.5)
	await _shot("peuples-liste")
	for k in 3:
		jeu.fiche.tabs.current_tab = k + 1
		await _wait(0.4)
		await _shot(["peuples-arbre", "peuples-langues", "peuples-personnages"][k])
	var info: Dictionary = App.session.people_info(int(labels[0]["id"]))
	report["techniques"] = info.get("techniques", []).size()
	report["coutumes"] = info.get("customs", []).size()
	jeu.open_people(int(labels[0]["id"]))
	await _wait(0.5)
	await _shot("peuples-fiche")
	var notables: Array = info.get("notables", [])
	report["personnages"] = notables.size()
	if not notables.is_empty():
		jeu.open_notable(int(notables[0]["id"]))
		await _wait(0.5)
		await _shot("peuples-personnage")
	var myths: Array = App.session.myths()
	report["mythes"] = myths.map(func(m): return m["title"])
	jeu.open_chronicle()
	await _wait(0.3)
	var tabs: TabContainer = jeu.fiche.tabs
	for i in tabs.get_tab_count():
		if tabs.get_tab_title(i) == App.t("myths"):
			tabs.current_tab = i
	await _wait(0.4)
	await _shot("peuples-mythes")
	jeu.fiche.close()
	ok = labels.size() >= 2 and not myths.is_empty() and report["mode_historique"] and int(report["techniques"]) > 0
	# Palier 3 : le campement du peuple au sol, ses objets et ses gestes.
	jeu.descend(core)
	var down := await _until(func(): return jeu.sol != null and jeu.sol.ready_done, 180.0)
	report["sol_pret"] = down
	if down:
		var sol = jeu.sol
		var cinfo: Dictionary = sol.ground.camp_info()
		report["campement"] = cinfo
		await _wait(20.0)
		await _shot("campement")
		var props := 0
		for buf in sol.ground.camp_instances(false):
			props += (buf as PackedFloat32Array).size() / 12
		report["objets_tenus"] = props
		sol.follow_species(int(cinfo.get("folk_species", 0)))
		for key in ["fabriquer_un_outil", "celebrer_un_rite"]:
			sol.force(key)
			sol.target_distance = 5.0
			sol.pitch = 0.75
			await _wait(3.0)
			await _shot("campement-" + key)
		sol.force("")
		ok = ok and not cinfo.is_empty() and int(cinfo.get("fixed", 0)) >= 3 and props > 0
		jeu.ascend()
		await _wait(1.0)
	else:
		ok = false
	_finish(ok)

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

## Essai de la descente au sol (étape 5) depuis une partie : le globe
## plonge sur un massif, la scène locale se construit depuis la cellule,
## puis « Remonter » rend le globe.
func _ground_only() -> void:
	await _run_to(2.0e5)
	var cell: int = App.session.highest_cell(true)
	var jeu = _screen()
	jeu.descend(cell)
	var ok := await _until(func(): return jeu.sol != null and jeu.sol.ready_done, 120.0)
	await _wait(2.0)
	report["sol_individus"] = jeu.sol.ground.individual_count() if ok else 0
	await _shot("sol-descente")
	jeu.ascend()
	await _wait(1.0)
	ok = ok and jeu.sol == null
	# Les fiches s'ouvrent au centre de l'écran (elles sont déplaçables).
	jeu.open_tree()
	await _wait(1.5)
	var f: Control = jeu.fiche
	var gap := (f.get_viewport_rect().size - f.size) / 2.0 - f.global_position
	report["fiche_centree"] = gap.length() < 2.0
	await _shot("fiche-centree")
	ok = ok and report["fiche_centree"]
	_finish(ok)

## Essai de la lame de microscope en 3D : la goutte d'eau d'une cellule
## peuplée se construit, ses microbes nagent, un clic ouvre une fiche
## d'espèce au-dessus, puis « Remonter » rend le globe.
func _slide_only() -> void:
	await _run_to(float(opts.get("fin", "300")) * 1.0e6)
	var jeu = _screen()
	# La cellule la plus peuplée d'un échantillon du globe.
	var best := -1
	var most := 0
	var rng := RandomNumberGenerator.new()
	rng.seed = 7
	for i in 60:
		var dir := Vector3(rng.randfn(), rng.randfn(), rng.randfn()).normalized()
		var c: int = App.session.cell_at(dir)
		App.session.cell_info(c)
		await _wait(0.1)
		var n: int = (App.session.cell_info(c).get("populations", []) as Array).size()
		if n > most:
			most = n
			best = c
	report["lame_cellule"] = best
	jeu.open_tool("lame", best)
	var ok := await _until(func(): return jeu.lame != null and jeu.lame.ready_done, 60.0)
	await _wait(2.0)
	if ok:
		report["lame_especes"] = jeu.lame.slide.species_count()
		ok = jeu.lame.slide.species_count() > 0
	await _shot("lame-ensemble")
	if ok:
		jeu.lame.target_distance = 25.0
		await _wait(2.0)
		await _shot("lame-proche")
		jeu.open_species(int(jeu.lame.slide.species_info(0)["species"]))
		await _wait(1.5)
		await _shot("lame-fiche")
		ok = jeu.fiche.get_parent() == jeu.overlay and jeu.overlay.get_index() > jeu.lame.get_index()
	jeu.close_slide()
	await _wait(1.0)
	ok = ok and jeu.lame == null
	_finish(ok)

## Essai de l'arbre du vivant fractal : la partie court jusqu'à `fin` Ma,
## puis l'arbre s'ouvre en vue d'ensemble et zoome sur un petit clade.
func _tree_only() -> void:
	App.session.set_rules_profile("aucun")
	App.session.set_speed(1.0e7)
	await _run_to(float(opts.get("fin", "300")) * 1.0e6)
	var jeu = _screen()
	jeu.open_tree()
	await _wait(2.0)
	var f = jeu.fiche
	report["arbre_noeuds"] = f.count
	report["arbre_vivantes"] = int(f.data.get("living", 0))
	await _shot("arbre-ensemble")
	# Un clade d'une dizaine de lignées, cadré de près.
	var k := -1
	for i in f.count:
		if f.leaves[i] >= 6 and f.leaves[i] <= 14:
			k = i
	if k >= 0:
		f._frame(k, false)
		await _wait(0.5)
		await _shot("arbre-clade")
	_finish(f.count > 0 and k >= 0)

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

## Essai de la première partie guidée : depuis l'accueil, la Terre
## ensemencée près des sources, le narrateur présente ses outils un à un
## (on accélère ses silences), la chronique raconte la partie en chapitres
## et le retour après une absence en fait le résumé.
func _guide_only() -> void:
	App.goto("accueil")
	await get_tree().process_frame
	await _wait(0.5)
	await _shot("guide-00-accueil")
	_screen()._guided()
	await get_tree().process_frame
	var ok := await _until(func(): return App.session.has_frame(), 120.0)
	await _wait(0.5)
	# Sans règles d'arrêt : une pause automatique posée sur la naissance de
	# la vie, juste après la reprise, figerait la partie.
	App.settings["stop_profile"] = "aucun"
	_screen()._begin()
	await get_tree().process_frame
	# La partie part aussitôt : on l'arrête, pour que chaque arrêt du guide
	# soit posé avant que le monde ne l'ait dépassé (sinon la pause et la
	# reprise tombent au même pas et s'annulent).
	App.session.pause()
	await _until(func(): return bool(App.session.frame_info().get("paused", false)), 60.0)
	var jeu = _screen()
	var n = jeu.narrateur
	App.session.set_rules_profile("aucun")
	App.session.set_speed(1.0e7)
	var hints := []
	var spoken := {}
	# Le guide suit la partie : à chaque étape, on lit la phrase montrée,
	# on fait ce qu'elle propose quand c'est simple, puis on la congédie.
	# --fin (Ma) : 600 par défaut ; la CI, sur des machines bien plus lentes
	# que ce conteneur, s'arrête plus tôt (--fin=30).
	var end := float(opts.get("fin", "600")) * 1.0e6
	var stops := []
	for t in [2.0e5, 3.0e6, 3.0e7, 2.0e8, 6.0e8]:
		if t < end:
			stops.append(t)
	stops.append(end)
	if opts.get("arrets", "oui") == "non":
		stops = [end]
	for target in stops:
		var reached := await _run_to(target)
		_log("%s : %s" % [App.session.frame_info().get("date", ""), "atteint" if reached else "non atteint"])
		for i in 6:
			n.quiet = 0.0
			n.poll = 0.0
			var shown := await _until(func(): return n.visible, 3.0)
			if not shown:
				break
			hints.append("%s : %s" % [n.current, n.label.text])
			if not spoken.has(n.current_tool):
				spoken[n.current_tool] = true
				await _wait(0.3)
				await _shot("guide-%02d-%s" % [hints.size(), n.current])
			match n.current_tool:
				"globe":
					var cell := _populated_cell()
					if cell < 0:
						cell = App.session.highest_cell(false)
					App.globe.select_cell(cell)
					jeu._on_cell_selected(cell)
				"chronique":
					jeu.open_chronicle()
					await _wait(0.3)
					jeu.fiche.close()
				_:
					n.dismiss()
			n.dismiss()
	report["guide_phrases"] = hints
	# Les pauses du guide ne changent pas l'histoire : même état du monde à
	# la date de fin avec ou sans arrêts (--arrets=non). L'empreinte du moteur compte
	# aussi les événements, donc les ordres de pause : on compare l'état.
	var fi: Dictionary = App.session.frame_info()
	report["etat_fin"] = "%s %d %s %s" % [fi["years"], int(fi["lineages"]), str(fi["o2"]), str(fi["biomass"])]
	report["guide_vu"] = App.settings.get("guide_vu", [])
	_log("guide : %s" % str(hints))
	ok = ok and hints.size() >= 4

	# La chronique : l'onglet du récit.
	jeu.open_chronicle()
	await _wait(0.5)
	var ch = jeu.fiche
	ch.tabs.current_tab = 1
	var chapters: Array = App.session.story()
	var titles := []
	for c in chapters:
		titles.append(c["title"])
	report["recit_chapitres"] = titles
	report["recit_premier"] = chapters[0]["text"] if not chapters.is_empty() else ""
	await _wait(0.5)
	await _shot("guide-recit")
	ch.close()
	ok = ok and chapters.size() >= 2

	# L'absence : le joueur revient après une longue avance.
	var since := _years()
	jeu.looked_years = since
	await _run_to(since + end / 2.0)
	jeu.idle = jeu.ABSENT_AFTER + 1.0
	var key := InputEventKey.new()
	key.keycode = KEY_SHIFT
	key.pressed = true
	Input.parse_input_event(key)
	var back := await _until(func(): return jeu.absence_card.visible, 5.0)
	report["absence"] = jeu.absence_label.text
	await _wait(0.5)
	await _shot("guide-absence")
	ok = ok and back
	_finish(ok)

## Mesures de performance jusqu'à l'arrivée de l'oxygène : la partie
## tourne à --vitesse (ans/s, 10 Ma/s par défaut) jusqu'à --fin Ma (120) ;
## le cadre des mesures (F3) écrit sa ligne par seconde, le rapport garde
## la vitesse tenue mesurée et les pires secondes.
func _perf_only() -> void:
	var m = get_parent().get_node("Mesures")
	if not m.panel.visible:
		m.toggle()
	var speed := float(opts.get("vitesse", "1e7"))
	App.session.set_speed(speed)
	var end := float(opts.get("fin", "120")) * 1.0e6
	App.session.pause_at(end)
	App.session.resume()
	var t0 := Time.get_ticks_msec()
	var y0 := _years()
	var samples := []
	while true:
		await _wait(5.0)
		var fi: Dictionary = App.session.frame_info()
		samples.append([fi["date"], fi["o2"], fi["real_speed"], Engine.get_frames_per_second()])
		_log("%s O2 %s tenue %s calcul %.0f ips %d" % [fi["date"], m._sci(float(fi["o2"])), fi["real_speed_text"], float(fi["compute_speed"]), Engine.get_frames_per_second()])
		if bool(fi["paused"]) or Time.get_ticks_msec() - t0 > float(opts.get("max_s", "1800")) * 1000.0:
			break
	var wall := (Time.get_ticks_msec() - t0) / 1000.0
	report["mesures_fichier"] = ProjectSettings.globalize_path(m.path)
	report["vitesse_demandee"] = speed
	report["vitesse_moyenne"] = (_years() - y0) / max(wall, 0.001)
	report["echantillons"] = samples
	_finish(true)

## Essai de l'affichage de l'étape 7 sur sociétés d'essai : territoires,
## champs, villes, routes et lumières la nuit sur le globe, inspecteur,
## fenêtre des sociétés (liste, chronique, récit, planète), fiche,
## comparaison de civilisations et encyclopédie.
func _societies_only() -> void:
	await _run_to(2.0e5)
	var jeu = _screen()
	jeu._toggle_societies(true)
	var ok := await _until(func(): return App.globe.societies_ready, 180.0)
	report["societes_pretes"] = ok
	if not ok:
		report["statut"] = App.session.society_status()
		_finish(false)
		return
	var target := int(opts.get("an", "1500"))
	await _until(func(): return App.session.society_year_value() >= target or App.session.society_status() == 3, 900.0)
	await _wait(1.0)
	report["annee"] = App.session.society_year_value()
	var rows: Array = App.session.society_list()
	report["societes"] = rows.size()
	if rows.is_empty():
		_finish(false)
		return
	report["noms"] = rows.slice(0, 12).map(func(r): return "%s (%s)" % [r["name"], r["label"]])
	var cities: Array = App.session.society_cities()
	report["villes"] = cities.size()
	var first: Dictionary = App.session.society_sheet(int(rows[0]["id"]))
	var core := int(first["cell"])
	App.globe.go_to_cell(core, 2.4)
	await _wait(2.0)
	await _shot("societes-globe")
	App.globe.go_to_cell(core, 1.35)
	await _wait(2.0)
	await _shot("societes-proche")
	# Lumières : l'étoile est fixe quand le terminateur est affiché ; on
	# vise la ville la plus peuplée du côté de la nuit, près du terminateur.
	App.settings["terminator"] = true
	App.globe._apply_settings()
	var light := Vector3(-0.6, 0.35, 0.72).normalized()
	var night := -1
	for c in cities:
		var d: float = (c["position"] as Vector3).normalized().dot(light)
		if d < -0.15 and d > -0.7:
			night = int(c["cell"])
			break
	report["ville_de_nuit"] = night
	if night >= 0:
		App.globe.go_to_cell(night, 2.2)
		await _wait(2.5)
		await _shot("societes-nuit")
	App.settings["terminator"] = false
	App.globe._apply_settings()
	App.globe.go_to_cell(core, 1.5)
	jeu.inspecteur.show_cell(core)
	App.globe.select_cell(core)
	await _wait(1.5)
	await _shot("societes-inspecteur")
	jeu.inspecteur.show_cell(-1)
	jeu.open_societies()
	await _wait(0.6)
	await _shot("societes-liste")
	for k in 3:
		jeu.fiche.tabs.current_tab = k + 1
		await _wait(0.6)
		await _shot(["societes-chronique", "societes-recit", "societes-planete"][k])
	var span: Vector2i = App.session.society_span()
	report["recit"] = Array(App.session.society_story(span.x, span.y)).slice(0, 3)
	jeu.open_society(int(rows[0]["id"]))
	await _wait(0.6)
	await _shot("societes-fiche")
	jeu.fiche.body.get_parent().scroll_vertical = 900
	await _wait(0.4)
	await _shot("societes-fiche-suite")
	if rows.size() >= 2:
		jeu.open_society_compare(int(rows[0]["id"]), int(rows[1]["id"]))
		await _wait(0.6)
		await _shot("societes-comparaison")
	jeu.open_encyclopedia()
	await _wait(0.6)
	await _shot("encyclopedie-simplifications")
	jeu.fiche.tabs.current_tab = 1
	await _wait(0.4)
	await _shot("encyclopedie-techniques")
	_finish(true)

## Villes au sol (étape 7, palier 3b) : la plus grande ville des sociétés
## d'essai à plusieurs années, vue de haut puis dans ses rues.
func _town_only() -> void:
	await _run_to(2.0e5)
	var jeu = _screen()
	jeu._toggle_societies(true)
	var ok := await _until(func(): return App.globe.societies_ready, 180.0)
	if not ok:
		_finish(false)
		return
	var years: Array = []
	for y in String(opts.get("ans", "-4000,800,1950")).split(","):
		years.append(int(y))
	var towns := []
	for target in years:
		await _until(func(): return App.session.society_year_value() >= target or App.session.society_status() == 3, 900.0)
		var cities: Array = App.session.society_cities()
		if cities.is_empty():
			ok = false
			continue
		var cell := int(cities[0]["cell"])
		jeu.descend(cell)
		var down := await _until(func(): return jeu.sol != null and jeu.sol.ready_done, 180.0)
		if not down:
			ok = false
			continue
		var sol = jeu.sol
		var info: Dictionary = sol.ground.town_info()
		info.erase("centre")
		info["annee"] = App.session.society_year_value()
		towns.append(info)
		await _wait(4.0)
		var tag := ("m" + str(-target)) if target < 0 else str(target)
		await _shot("ville-" + tag)
		# Au bord de la place, à hauteur d'homme ou presque.
		sol.target_distance = 18.0
		sol.pitch = 0.12
		sol.yaw += 0.8
		await _wait(3.0)
		await _shot("ville-" + tag + "-rue")
		ok = ok and not info.is_empty() and int(info.get("buildings", 0)) >= 20
		jeu.ascend()
		await _wait(1.5)
	report["villes_au_sol"] = towns
	_finish(ok)

## Galerie de rejeux : une partie sauvegardée et exportée en rejeu, le
## rejeu rejoué depuis la graine jusqu'à la même date, puis les jalons des
## deux histoires comparés dans la galerie (ils doivent être identiques).
func _gallery_only() -> void:
	var target := float(opts.get("ma", "6")) * 1.0e6
	await _run_to(target)
	var a_name := "Galerie, essai A"
	App.session.save(App.save_path(a_name), a_name)
	var replay := App.replay_path(a_name)
	if FileAccess.file_exists(replay):
		DirAccess.remove_absolute(replay)
	App.session.export_replay(replay, a_name)
	var written := await _until(func(): return FileAccess.file_exists(replay) and App.session.saves_pending() == 0, 120.0)
	var described: Dictionary = App.session.describe_replay(replay)
	report["rejeu_ecrit"] = written
	report["rejeu"] = {"ordres": described.get("orders", 0), "jalons": (described.get("milestones", []) as Array).size(), "date": described.get("date", "")}
	var jeu = _screen()
	jeu.open_gallery()
	await _wait(0.8)
	await _shot("galerie")
	var started: bool = App.session.start_replay(replay)
	report["rejeu_lance"] = started
	if not started:
		_finish(false)
		return
	App.globe.reset()
	App.goto("jeu", {"new": false})
	await get_tree().process_frame
	App.session.set_rules_profile("aucun")
	App.session.set_speed(1.0e6)
	App.session.resume()
	var back := await _until(func(): return bool(App.session.frame_info().get("paused", false)) and _years() >= target - 1.0, 900.0)
	report["rejeu_arrive"] = back
	await _wait(1.0)
	report["rejeu_bandeau"] = App.session.replay_info()
	var b_name := "Galerie, essai B"
	App.session.save(App.save_path(b_name), b_name)
	await _until(func(): return App.session.saves_pending() == 0, 120.0)
	var ma: Array = App.session.describe_save(App.save_path(a_name)).get("milestones", [])
	var mb: Array = App.session.describe_save(App.save_path(b_name)).get("milestones", [])
	report["jalons_a"] = ma.map(func(m): return "%s : %s" % [m["date"], m["title"]])
	report["jalons_b"] = mb.map(func(m): return "%s : %s" % [m["date"], m["title"]])
	var same := ma.size() == mb.size() and not ma.is_empty()
	for k in mini(ma.size(), mb.size()):
		same = same and str(ma[k]["title"]) == str(mb[k]["title"]) and absf(float(ma[k]["years"]) - float(mb[k]["years"])) < 1.0
	report["jalons_identiques"] = same
	jeu = _screen()
	jeu.open_gallery()
	await _wait(0.8)
	var g = jeu.fiche
	var ticked := 0
	for en in g.entries:
		if str(en["info"].get("name", "")) in [a_name, b_name] and ticked < 2 and en["kind"] == "save":
			(en["check"] as CheckBox).button_pressed = true
			ticked += 1
	g._compare()
	await _wait(0.6)
	await _shot("galerie-comparaison")
	_finish(written and started and back and same)

func _scenario() -> void:
	report["monde"] = opts["monde"]
	report["niveau"] = int(opts["niveau"])
	report["graine"] = int(opts["graine"])
	if opts.get("seul", "") == "guide":
		await _guide_only()
		return
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
	if opts.get("seul", "") == "sol":
		await _ground_only()
		return
	if opts.get("seul", "") == "lame":
		await _slide_only()
		return
	if opts.get("seul", "") == "arbre":
		await _tree_only()
		return
	if opts.get("seul", "") == "colonie":
		await _colony_only()
		return
	if opts.get("seul", "") == "peuples":
		await _peoples_only()
		return
	if opts.get("seul", "") == "societes":
		await _societies_only()
		return
	if opts.get("seul", "") == "galerie":
		await _gallery_only()
		return
	if opts.get("seul", "") == "ville":
		await _town_only()
		return
	if opts.get("seul", "") == "mesures":
		await _perf_only()
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
