extends CanvasLayer
## Mesures de performance (F3). Un petit cadre en haut à droite donne les
## images par seconde, la pire image de la seconde, la vitesse tenue et la
## part de chaque poste ; tant qu'il est ouvert, une ligne par seconde
## s'écrit dans user://mesures/mesures-DATE.csv, à renvoyer tel quel. Le
## bouton « Ouvrir le dossier » montre le fichier dans l'explorateur.
##
## Lancé avec `-- --mesures`, l'enregistrement démarre tout seul.

const DIR := "user://mesures"

var panel: PanelContainer
var label: Label
var file: FileAccess
var path := ""
var t0 := 0
var acc := 0.0
var frames := 0
var worst := 0.0
var cpu_ms := 0.0
var gpu_ms := 0.0
var proc_ms := 0.0

func _ready() -> void:
	layer = 90
	panel = PanelContainer.new()
	panel.add_theme_stylebox_override("panel", Atlas.cartouche(Color(Atlas.PAPER, 0.92), Atlas.INK, 1, false))
	panel.set_anchors_preset(Control.PRESET_TOP_RIGHT)
	panel.grow_horizontal = Control.GROW_DIRECTION_BEGIN
	panel.offset_top = 8
	panel.offset_right = -8
	panel.visible = false
	add_child(panel)
	var box := VBoxContainer.new()
	panel.add_child(box)
	label = Label.new()
	label.add_theme_font_size_override("font_size", 14)
	label.add_theme_color_override("font_color", Atlas.INK)
	box.add_child(label)
	var open := Atlas.button(App.t("perf_open"), func(): OS.shell_open(ProjectSettings.globalize_path(DIR)))
	open.add_theme_font_size_override("font_size", 14)
	box.add_child(open)
	RenderingServer.viewport_set_measure_render_time(get_viewport().get_viewport_rid(), true)
	if "--mesures" in OS.get_cmdline_user_args():
		toggle()

func _input(event: InputEvent) -> void:
	if event is InputEventKey and event.pressed and not event.echo and event.keycode == KEY_F3:
		toggle()
		get_viewport().set_input_as_handled()

func toggle() -> void:
	panel.visible = not panel.visible
	if panel.visible:
		_open_file()
	elif file:
		file.close()
		file = null

func _open_file() -> void:
	DirAccess.make_dir_recursive_absolute(DIR)
	var stamp := Time.get_datetime_string_from_system().replace(":", "-").replace("T", "_")
	path = DIR.path_join("mesures-%s.csv" % stamp)
	file = FileAccess.open(path, FileAccess.WRITE)
	t0 = Time.get_ticks_msec()
	acc = 0.0
	frames = 0
	worst = 0.0
	App.perf.clear()
	var cols := ["temps_s", "ecran", "date_ans", "o2", "vitesse_demandee", "vitesse_tenue", "vitesse_calcul",
		"pas_ans", "pas_moteur_ms", "ips", "image_max_ms", "script_ms", "rendu_cpu_ms", "rendu_gpu_ms",
		"appels_dessin", "primitives", "objets", "memoire_mo", "memoire_video_mo", "lignees", "especes"]
	for k in App.PERF_KEYS:
		cols.append(k + "_ms")
	if file:
		file.store_line(",".join(cols))
	App.perf_on = true

func _process(delta: float) -> void:
	if not panel.visible:
		App.perf_on = false
		return
	App.perf_on = true
	acc += delta
	frames += 1
	worst = max(worst, delta)
	var vp := get_viewport().get_viewport_rid()
	cpu_ms += RenderingServer.viewport_get_measured_render_time_cpu(vp) + RenderingServer.get_frame_setup_time_cpu()
	gpu_ms += RenderingServer.viewport_get_measured_render_time_gpu(vp)
	proc_ms += Performance.get_monitor(Performance.TIME_PROCESS) * 1000.0
	if acc < 1.0:
		return
	var n := float(frames)
	var info: Dictionary = App.session.frame_info() if App.session and App.session.is_running() else {}
	var fps := n / acc
	var parts := {}
	for k in App.PERF_KEYS:
		parts[k] = float(App.perf.get(k, 0)) / 1000.0 / n
	var row := [
		"%.1f" % ((Time.get_ticks_msec() - t0) / 1000.0),
		App.current_screen,
		"%.0f" % float(info.get("years", 0.0)),
		_sci(float(info.get("o2", 0.0))),
		"%.0f" % (App.session.speed() if App.session else 0.0),
		"%.0f" % float(info.get("real_speed", 0.0)),
		"%.0f" % float(info.get("compute_speed", 0.0)),
		"%.0f" % float(info.get("step_years", 0.0)),
		"%.1f" % (float(info.get("step_seconds", 0.0)) * 1000.0),
		"%.1f" % fps,
		"%.1f" % (worst * 1000.0),
		"%.2f" % (proc_ms / n),
		"%.2f" % (cpu_ms / n),
		"%.2f" % (gpu_ms / n),
		"%d" % Performance.get_monitor(Performance.RENDER_TOTAL_DRAW_CALLS_IN_FRAME),
		"%d" % Performance.get_monitor(Performance.RENDER_TOTAL_PRIMITIVES_IN_FRAME),
		"%d" % Performance.get_monitor(Performance.OBJECT_COUNT),
		"%.0f" % (Performance.get_monitor(Performance.MEMORY_STATIC) / 1048576.0),
		"%.0f" % (Performance.get_monitor(Performance.RENDER_VIDEO_MEM_USED) / 1048576.0),
		"%d" % int(info.get("lineages", 0)),
		"%d" % int(info.get("species", 0)),
	]
	for k in App.PERF_KEYS:
		row.append("%.2f" % parts[k])
	if file:
		file.store_line(",".join(row))
		file.flush()
	# Les trois postes les plus lourds de la seconde.
	var keys: Array = App.PERF_KEYS.duplicate()
	keys.sort_custom(func(a, b): return parts[a] > parts[b])
	var top := PackedStringArray()
	for k in keys.slice(0, 3):
		top.append("%s %.1f" % [k, parts[k]])
	var speed := ""
	if not info.is_empty():
		speed = "\n%s %s / %s" % [App.t("real_speed"), info["real_speed_text"], App.session.format_speed(App.session.speed())]
	label.text = "%.0f %s · %s %.0f ms%s\n%s %.1f · CPU %.1f · GPU %.1f ms\n%s\n● %s" % [
		fps, App.t("perf_fps"), App.t("perf_worst"), worst * 1000.0, speed,
		App.t("perf_script"), proc_ms / n, cpu_ms / n, gpu_ms / n,
		" · ".join(top), path.get_file()]
	acc = 0.0
	frames = 0
	worst = 0.0
	cpu_ms = 0.0
	gpu_ms = 0.0
	proc_ms = 0.0
	App.perf.clear()

## Nombre en notation scientifique courte (« 3.20e-05 ») : GDScript n'a pas %g.
static func _sci(x: float) -> String:
	if x == 0.0:
		return "0"
	var e := floori(log(absf(x)) / log(10.0))
	return "%.2fe%d" % [x / pow(10.0, e), e]
