extends "res://scripts/ui/fiche.gd"
## « Et sans mon intervention ? » (document Fonctionnalités, « Avec et
## sans ») : le jeu rejoue la planète depuis le point de sauvegarde écrit
## juste avant l'intervention, sur la même graine, sans elle, en même temps
## que la partie ou seulement pendant les pauses, puis compare les deux
## histoires : grandeurs, jalons, courbes, et l'écart sur le globe.

var list_box: VBoxContainer
var pauses_only: CheckBox
var status_label: Label
var progress: ProgressBar
var table: GridContainer
var milestones: GridContainer
var curves: Control
var map_button: CheckButton
var stop_button: Button
var refresh := 0.0
var last: Dictionary = {}

func _ready() -> void:
	set_title(App.t("with_without"))
	custom_minimum_size = Vector2(760, 0)
	var intro := Atlas.text(App.t("with_without_text"), 16, true)
	intro.custom_minimum_size.x = 720
	content.add_child(intro)
	pauses_only = CheckBox.new()
	pauses_only.text = App.t("branch_pauses_only")
	content.add_child(pauses_only)
	list_box = VBoxContainer.new()
	content.add_child(list_box)
	_fill_list()
	content.add_child(Atlas.hsep())
	var h := HBoxContainer.new()
	content.add_child(h)
	status_label = Atlas.text("", 16)
	status_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	h.add_child(status_label)
	stop_button = Atlas.button(App.t("branch_stop"), _stop)
	h.add_child(stop_button)
	progress = ProgressBar.new()
	progress.min_value = 0.0
	progress.max_value = 1.0
	progress.custom_minimum_size = Vector2(720, 14)
	progress.show_percentage = false
	content.add_child(progress)
	var cols := HBoxContainer.new()
	cols.add_theme_constant_override("separation", 26)
	content.add_child(cols)
	table = GridContainer.new()
	table.columns = 4
	table.add_theme_constant_override("h_separation", 16)
	cols.add_child(table)
	milestones = GridContainer.new()
	milestones.columns = 3
	milestones.add_theme_constant_override("h_separation", 12)
	cols.add_child(milestones)
	curves = Control.new()
	curves.custom_minimum_size = Vector2(720, 150)
	curves.draw.connect(_draw_curves)
	content.add_child(curves)
	map_button = CheckButton.new()
	map_button.text = App.t("branch_map")
	map_button.toggled.connect(_toggle_map)
	content.add_child(map_button)
	_update()

func _fill_list() -> void:
	for c in list_box.get_children():
		c.queue_free()
	var items: Array = App.session.interventions()
	if items.is_empty():
		list_box.add_child(Atlas.text(App.t("no_intervention"), 16, true))
		return
	for it in items:
		var h := HBoxContainer.new()
		list_box.add_child(h)
		var l := Atlas.text("%s · %s" % [it["date"], it["label"]], 16)
		l.custom_minimum_size.x = 560
		h.add_child(l)
		var b := Atlas.button(App.t("and_without"), func(): _start(int(it["order"])))
		b.disabled = bool(it["refused"]) or not bool(it["ready"])
		if bool(it["refused"]):
			b.tooltip_text = App.t("refused")
		h.add_child(b)

func _start(order: int) -> void:
	var err: String = App.session.start_branch(order, pauses_only.button_pressed)
	if err != "":
		status_label.text = err
	_update()

func _stop() -> void:
	App.session.stop_branch()
	map_button.button_pressed = false
	_update()

func _toggle_map(on: bool) -> void:
	App.globe.set_comparison(on)

func _process(delta: float) -> void:
	refresh += delta
	if refresh >= 0.5:
		refresh = 0.0
		_update()

func _clear(grid: GridContainer) -> void:
	for c in grid.get_children():
		grid.remove_child(c)
		c.queue_free()

func _cell(grid: GridContainer, t: String, bold := false, colour := Color(0, 0, 0, 0)) -> void:
	var l := Atlas.text(t, 15, bold)
	l.autowrap_mode = TextServer.AUTOWRAP_OFF
	if colour.a > 0.0:
		l.add_theme_color_override("font_color", colour)
	grid.add_child(l)

func _update() -> void:
	var st: Dictionary = App.session.branch_status()
	var active := bool(st.get("active", false))
	stop_button.visible = active
	progress.visible = active
	map_button.disabled = not active
	if not active:
		status_label.text = App.t("branch_none")
		_clear(table)
		_clear(milestones)
		last = {}
		curves.queue_redraw()
		return
	progress.value = float(st["progress"])
	var err: String = st.get("error", "")
	if err != "":
		status_label.text = err
	elif bool(st["caught_up"]):
		status_label.text = App.t("branch_done") % [st["label"], st["date"]]
	else:
		status_label.text = App.t("branch_running") % [st["label"], st["date"], st["target"]]
	last = App.session.branch_comparison()
	_clear(table)
	_clear(milestones)
	var rows: Array = last.get("rows", [])
	if not rows.is_empty():
		for t in [App.t("quantity"), App.t("with"), App.t("without"), App.t("difference")]:
			_cell(table, t, true)
		for r in rows:
			_cell(table, r["label"])
			_cell(table, r["with"])
			_cell(table, r["without"])
			var ch := float(r["change"])
			_cell(table, r["change_text"], false, Atlas.VERMILION if absf(ch) > 0.05 else Atlas.INK)
		_cell(table, App.t("map_changed"))
		_cell(table, str(last.get("changed_share", "")))
		_cell(table, "")
		_cell(table, "")
	for t in [App.t("milestone"), App.t("with"), App.t("without")]:
		_cell(milestones, t, true)
	for m in last.get("milestones", []):
		_cell(milestones, m["label"])
		_cell(milestones, m["with"])
		_cell(milestones, m["without"])
	curves.queue_redraw()
	if map_button.button_pressed:
		App.globe.set_comparison(true)

## Courbes de l'O₂ et de la biomasse depuis l'intervention, avec (encre)
## et sans (vermillon), en log.
func _draw_curves() -> void:
	var w := curves.size.x
	var h := curves.size.y
	curves.draw_rect(Rect2(0, 0, w, h), Atlas.INK, false, 1.0)
	if last.is_empty():
		return
	var y_a: PackedFloat64Array = last.get("years", PackedFloat64Array())
	var y_b: PackedFloat64Array = last.get("years_without", PackedFloat64Array())
	if y_a.size() < 2 and y_b.size() < 2:
		return
	var t0 := INF
	var t1 := -INF
	for arr in [y_a, y_b]:
		if arr.size() > 0:
			t0 = min(t0, arr[0])
			t1 = max(t1, arr[arr.size() - 1])
	if t1 <= t0:
		return
	# Seules les grandeurs non nulles ont une courbe (l'O₂ reste à zéro
	# avant la photosynthèse oxygénique) ; elles se partagent la largeur.
	var shown := []
	for key in ["o2", "biomass"]:
		var lo := INF
		var hi := -INF
		for arr in [last.get(key, PackedFloat64Array()), last.get(key + "_without", PackedFloat64Array())]:
			for v in arr:
				if v > 0.0:
					lo = min(lo, log(v) / log(10.0))
					hi = max(hi, log(v) / log(10.0))
		if is_finite(lo):
			shown.append([key, lo, hi])
	if shown.is_empty():
		return
	var half := w / float(shown.size()) - 8.0
	for k in shown.size():
		var key: String = shown[k][0]
		var lo: float = shown[k][1]
		var hi: float = shown[k][2]
		var x0 := k * (half + 16.0)
		if hi - lo < 0.5:
			hi += 0.25
			lo -= 0.25
		curves.draw_string(Atlas.body_font(), Vector2(x0 + 6, 16), App.t("curve_" + key), HORIZONTAL_ALIGNMENT_LEFT, -1, 14, Atlas.INK)
		for side in 2:
			var ys: PackedFloat64Array = y_a if side == 0 else y_b
			var vs: PackedFloat64Array = last.get(key if side == 0 else key + "_without", PackedFloat64Array())
			var pts := PackedVector2Array()
			for i in min(ys.size(), vs.size()):
				if vs[i] <= 0.0:
					continue
				var x: float = x0 + (ys[i] - t0) / (t1 - t0) * half
				var y: float = h - 6.0 - (log(vs[i]) / log(10.0) - lo) / (hi - lo) * (h - 28.0)
				pts.append(Vector2(x, y))
			if pts.size() >= 2:
				curves.draw_polyline(pts, Atlas.INK if side == 0 else Atlas.VERMILION, 2.0 if side == 0 else 1.5, true)

func close() -> void:
	App.globe.set_comparison(false)
	super.close()
