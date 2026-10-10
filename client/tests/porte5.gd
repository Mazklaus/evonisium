extends Node
## Scénario de la porte de l'étape 5, volet affichage des animaux. Lancé par
##   godot --path client -- --porte5 [--graine=2026] [--total=101000]
##         [--animes=1000] [--sortie=DOSSIER]
##
## Une scène au sol de banc d'essai (terre, puis mer) : `animes` individus
## animés près de la caméra, les autres en imposteurs. La caméra suit un
## individu ; chacune des douze actions du lexique lui est imposée tour à
## tour et capturée. Le rapport donne les images par seconde et le coût de
## la foule côté processeur (la carte graphique du conteneur est logicielle :
## les 60 images/s se mesurent sur une vraie machine).
##
## Rapport dans DOSSIER/rapport.json ; code de sortie 0 si la porte est
## franchie (effectifs atteints, poses finies, toutes les actions jouées,
## foule sous 4 ms par image).

const Sol := preload("res://scripts/screens/sol.gd")

var opts := {"graine": "2026", "total": "101000", "animes": "1000", "sortie": "user://porte5", "mesure": "5"}
var report := {}
var out_dir := ""
var shots: Array = []
var sol: Control

func _ready() -> void:
	for a in OS.get_cmdline_user_args():
		if a.begins_with("--") and "=" in a:
			var kv := a.substr(2).split("=", true, 1)
			opts[kv[0]] = kv[1]
	out_dir = ProjectSettings.globalize_path(opts["sortie"])
	DirAccess.make_dir_recursive_absolute(out_dir)
	App.globe.visible = false
	_scenario.call_deferred()

func _log(s: String) -> void:
	print("[porte5] ", s)

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
	get_tree().quit(0 if passed else 1)

## Ouvre une scène et attend qu'elle soit prête.
func _open(ocean: bool) -> Dictionary:
	if sol:
		sol.queue_free()
	var g := EvoGround.bench(ocean, int(opts["graine"]), int(opts["total"]))
	sol = Sol.new()
	sol.animated = int(opts["animes"])
	sol.theme = Atlas.theme
	get_parent().ui.add_child(sol)
	sol.open(g)
	var ok := await _until(func(): return sol.ready_done, 300.0)
	var st: Dictionary = g.stats()
	_log("scène %s prête en %.0f ms : %d individus, %d espèces" % ["mer" if ocean else "terre", float(st.get("build_ms", 0)), int(st.get("individuals", 0)), int(st.get("species", 0))])
	return {"ok": ok, "ground": g, "stats": st}

## Mesure pendant `seconds` : images par seconde, temps de la foule et des
## tampons.
func _measure(g: EvoGround, seconds: float) -> Dictionary:
	var frames := 0
	var step_ms := 0.0
	var worst := 0.0
	var t0 := Time.get_ticks_usec()
	var last := t0
	while Time.get_ticks_usec() - t0 < seconds * 1.0e6:
		await get_tree().process_frame
		var now := Time.get_ticks_usec()
		worst = max(worst, (now - last) / 1000.0)
		last = now
		frames += 1
		step_ms += float(g.stats()["step_ms"])
	var dt := (Time.get_ticks_usec() - t0) / 1.0e6
	return {"fps": frames / dt, "frames": frames, "step_ms": step_ms / max(frames, 1), "worst_frame_ms": worst}

func _check_followed(g: EvoGround) -> bool:
	if sol.followed < 0:
		return false
	var d: Dictionary = g.individual(sol.followed)
	var p: Vector3 = d["position"]
	return bool(d["animated"]) and is_finite(p.x) and is_finite(p.y) and is_finite(p.z)

func _scenario() -> void:
	var passed := true
	var land := await _open(false)
	if not land["ok"]:
		report["error"] = "scène non prête"
		_finish(false)
		return
	var g: EvoGround = land["ground"]
	report["build_ms_land"] = land["stats"]["build_ms"]
	var species := []
	for k in g.species_count():
		species.append(g.species_info(k))
	report["species_land"] = species
	await _wait(1.5)
	var st: Dictionary = g.stats()
	report["individuals"] = st["individuals"]
	report["animated"] = st["animated"]
	report["impostors"] = int(st["individuals"]) - int(st["animated"])
	var counts_ok := int(st["animated"]) >= int(opts["animes"]) and int(report["impostors"]) >= 100000
	report["counts_ok"] = counts_ok
	passed = passed and counts_ok
	# Suivre un marcheur et lui faire jouer les douze actions.
	sol.follow_species(0)
	await _wait(1.0)
	await _shot("01-suivi")
	var played := []
	var n := 2
	for a in g.actions():
		sol.force(a["key"])
		await _wait(1.2)
		var ok: bool = _check_followed(g) and String(g.individual(sol.followed)["action"]) == a["key"]
		played.append({"action": a["key"], "ok": ok})
		passed = passed and ok
		await _shot("%02d-%s" % [n, a["key"]])
		n += 1
	report["actions"] = played
	sol.force("")
	# Vue d'ensemble : imposteurs au loin.
	sol.target_distance = 220.0
	sol.pitch = 0.5
	await _wait(2.0)
	await _shot("%02d-foule" % n)
	n += 1
	sol.target_distance = 6.0
	sol.follow_species(0)
	await _wait(1.0)
	var m := await _measure(g, float(opts["mesure"]))
	report["land"] = m
	_log("terre : %.1f img/s, foule %.2f ms par image, pire image %.0f ms" % [m["fps"], m["step_ms"], m["worst_frame_ms"]])
	passed = passed and float(m["step_ms"]) < 4.0
	# La mer : nageurs et méduses.
	var sea := await _open(true)
	if not sea["ok"]:
		report["error"] = "scène marine non prête"
		_finish(false)
		return
	var gs: EvoGround = sea["ground"]
	report["build_ms_sea"] = sea["stats"]["build_ms"]
	await _wait(1.5)
	sol.follow_species(0)
	await _wait(1.5)
	await _shot("%02d-mer" % n)
	n += 1
	sol.follow_species(1)
	await _wait(1.5)
	await _shot("%02d-mer-meduse" % n)
	var ms := await _measure(gs, float(opts["mesure"]))
	report["sea"] = ms
	passed = passed and float(ms["step_ms"]) < 4.0 and _check_followed(gs)
	_finish(passed)
