extends PanelContainer
## Frise du temps : l'oxygène (échelle logarithmique) et la température
## moyenne depuis le début de la partie, avec les événements notables
## marqués d'un trait. Un clic sur une marque ouvre l'événement.

signal event_clicked(event: Dictionary)

var chart: Control
var history := {}
var marks: Array = []
var hover_mark := -1
var tip: Label
var refresh_timer := 0.0
## Boutons des fiches (chronique, arbre, interventions…), à gauche.
var menu: GridContainer

func _ready() -> void:
	custom_minimum_size = Vector2(0, 124)
	var row := HBoxContainer.new()
	row.add_theme_constant_override("separation", 14)
	add_child(row)
	menu = GridContainer.new()
	menu.columns = 3
	menu.add_theme_constant_override("h_separation", 6)
	menu.add_theme_constant_override("v_separation", 6)
	row.add_child(menu)
	row.add_child(VSeparator.new())
	var v := VBoxContainer.new()
	v.add_theme_constant_override("separation", 0)
	v.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	row.add_child(v)
	var h := HBoxContainer.new()
	v.add_child(h)
	var l1 := Atlas.text("— " + App.t("oxygen") + " (log)", 14, true)
	l1.autowrap_mode = TextServer.AUTOWRAP_OFF
	l1.add_theme_color_override("font_color", Color("#3B6E8F"))
	h.add_child(l1)
	var l2 := Atlas.text("— " + App.t("temperature"), 14, true)
	l2.autowrap_mode = TextServer.AUTOWRAP_OFF
	l2.add_theme_color_override("font_color", Atlas.VERMILION)
	h.add_child(l2)
	var l3 := Atlas.text("| " + App.t("chronicle"), 14, true)
	l3.autowrap_mode = TextServer.AUTOWRAP_OFF
	h.add_child(l3)
	tip = Atlas.text("", 14, true)
	tip.autowrap_mode = TextServer.AUTOWRAP_OFF
	tip.clip_text = true
	tip.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	h.add_child(tip)
	chart = Control.new()
	chart.size_flags_vertical = Control.SIZE_EXPAND_FILL
	chart.custom_minimum_size = Vector2(200, 82)
	chart.mouse_filter = Control.MOUSE_FILTER_STOP
	chart.draw.connect(_draw_chart)
	chart.gui_input.connect(_chart_input)
	v.add_child(chart)

func add_mark(e: Dictionary) -> void:
	marks.append(e)
	if marks.size() > 400:
		marks.pop_front()

func clear() -> void:
	marks.clear()
	history = {}

func _process(delta: float) -> void:
	refresh_timer -= delta
	if refresh_timer <= 0.0:
		refresh_timer = 0.5
		history = App.session.history()
		chart.queue_redraw()

func _x_of(years: float, t0: float, t1: float, w: float) -> float:
	return (years - t0) / max(t1 - t0, 1.0) * w

func _draw_chart() -> void:
	var w := chart.size.x
	var h := chart.size.y
	chart.draw_line(Vector2(0, h - 1), Vector2(w, h - 1), Color(Atlas.INK, 0.6), 1.0)
	if history.is_empty():
		return
	var years: PackedFloat64Array = history["years"]
	if years.size() < 2:
		return
	var t0 := years[0]
	var t1 := years[years.size() - 1]
	var o2: PackedFloat64Array = history["o2"]
	var temp: PackedFloat64Array = history["temperature_k"]
	# Oxygène : de 10⁻¹² à 0,3 en log.
	var pts := PackedVector2Array()
	var tpts := PackedVector2Array()
	var tmin := 1e9
	var tmax := -1e9
	for k in temp:
		tmin = min(tmin, k)
		tmax = max(tmax, k)
	if tmax - tmin < 5.0:
		var mid := (tmax + tmin) * 0.5
		tmin = mid - 2.5
		tmax = mid + 2.5
	var stride: int = max(1, years.size() / int(max(w, 1.0)))
	for i in range(0, years.size(), stride):
		var x := _x_of(years[i], t0, t1, w)
		var lo: float = (log(max(o2[i], 1e-12)) / log(10.0) + 12.0) / 11.5
		pts.append(Vector2(x, h - 4 - clamp(lo, 0.0, 1.0) * (h - 10)))
		tpts.append(Vector2(x, h - 4 - (temp[i] - tmin) / (tmax - tmin) * (h - 10)))
	if pts.size() >= 2:
		chart.draw_polyline(pts, Color("#3B6E8F"), 2.0, true)
		chart.draw_polyline(tpts, Atlas.VERMILION, 1.5, true)
	# Marques des événements notables.
	for i in marks.size():
		var e: Dictionary = marks[i]
		var x := _x_of(float(e["years"]), t0, t1, w)
		var major: bool = int(e["level"]) >= 2
		var c := Atlas.VERMILION if major else Color(Atlas.INK, 0.7)
		var top := 2.0 if major else h * 0.45
		chart.draw_line(Vector2(x, top), Vector2(x, h - 1), c, 2.0 if i == hover_mark else 1.0)

func _mark_at(x: float) -> int:
	if history.is_empty():
		return -1
	var years: PackedFloat64Array = history["years"]
	if years.size() < 2:
		return -1
	var best := -1
	var best_d := 6.0
	for i in marks.size():
		var mx := _x_of(float(marks[i]["years"]), years[0], years[years.size() - 1], chart.size.x)
		if abs(mx - x) < best_d:
			best_d = abs(mx - x)
			best = i
	return best

func _chart_input(event: InputEvent) -> void:
	if event is InputEventMouseMotion:
		var m := _mark_at(event.position.x)
		if m != hover_mark:
			hover_mark = m
			tip.text = "" if m < 0 else "  %s — %s" % [marks[m]["date"], marks[m]["text"]]
			chart.queue_redraw()
	elif event is InputEventMouseButton and event.pressed and event.button_index == MOUSE_BUTTON_LEFT:
		var m := _mark_at(event.position.x)
		if m >= 0:
			event_clicked.emit(marks[m])
