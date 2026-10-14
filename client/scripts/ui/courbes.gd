extends Control
## Courbes dans le temps (comparaison de civilisations, fiche de société) :
## une à trois séries (année, valeur) sur un même axe des années, chacune à
## sa couleur, échelle linéaire ou logarithmique. Le survol lit l'année et
## les valeurs.

var series: Array = []  # [{points: PackedVector2Array, colour: Color, name: String}]
var log_scale := false
var title := ""
var hover_x := -1.0

func _init() -> void:
	custom_minimum_size = Vector2(380, 170)
	mouse_filter = Control.MOUSE_FILTER_STOP

func set_series(s: Array, t: String, logarithmic: bool) -> void:
	series = s
	if logarithmic:
		# Le logarithme ne montre pas les zéros (une société pas encore née).
		for e in series:
			var kept := PackedVector2Array()
			for p in (e["points"] as PackedVector2Array):
				if p.y > 0.0:
					kept.append(p)
			e["points"] = kept
	title = t
	log_scale = logarithmic
	queue_redraw()

func _gui_input(event: InputEvent) -> void:
	if event is InputEventMouseMotion:
		hover_x = (event as InputEventMouseMotion).position.x
		queue_redraw()

func _notification(what: int) -> void:
	if what == NOTIFICATION_MOUSE_EXIT:
		hover_x = -1.0
		queue_redraw()

func _v(y: float) -> float:
	return log(maxf(y, 1e-6)) / log(10.0) if log_scale else y

func _draw() -> void:
	var font := Atlas.body_font()
	var fs := int(13 * App.text_scale())
	var r := Rect2(Vector2(52, 20), size - Vector2(62, 42))
	draw_string(font, Vector2(4, 14), title, HORIZONTAL_ALIGNMENT_LEFT, -1, fs + 1, Atlas.INK)
	draw_rect(r, Color(Atlas.INK, 0.5), false, 1.0)
	var x0 := INF
	var x1 := -INF
	var y0 := INF
	var y1 := -INF
	for s in series:
		for p in (s["points"] as PackedVector2Array):
			x0 = minf(x0, p.x)
			x1 = maxf(x1, p.x)
			y0 = minf(y0, _v(p.y))
			y1 = maxf(y1, _v(p.y))
	if x0 >= x1:
		draw_string(font, r.position + Vector2(8, 24), App.t("none"), HORIZONTAL_ALIGNMENT_LEFT, -1, fs, Color(Atlas.INK, 0.7))
		return
	if y1 - y0 < 1e-9:
		y1 = y0 + 1.0
	if not log_scale:
		y0 = minf(y0, 0.0)
	var to_px := func(p: Vector2) -> Vector2:
		return Vector2(r.position.x + (p.x - x0) / (x1 - x0) * r.size.x, r.end.y - (_v(p.y) - y0) / (y1 - y0) * r.size.y)
	# Axes : années en bas, valeurs à gauche.
	for k in 3:
		var yv := y0 + (y1 - y0) * k / 2.0
		var py := r.end.y - r.size.y * k / 2.0
		var label := _short(pow(10.0, yv)) if log_scale else _short(yv)
		draw_string(font, Vector2(2, py + 4), label, HORIZONTAL_ALIGNMENT_LEFT, 48, fs, Color(Atlas.INK, 0.8))
	draw_string(font, Vector2(r.position.x, r.end.y + 16), str(int(x0)), HORIZONTAL_ALIGNMENT_LEFT, -1, fs, Color(Atlas.INK, 0.8))
	draw_string(font, Vector2(r.end.x - 50, r.end.y + 16), str(int(x1)), HORIZONTAL_ALIGNMENT_RIGHT, 50, fs, Color(Atlas.INK, 0.8))
	var legend_x := r.position.x + 8
	for s in series:
		var pts := PackedVector2Array()
		for p in (s["points"] as PackedVector2Array):
			pts.append(to_px.call(p))
		if pts.size() >= 2:
			draw_polyline(pts, s["colour"], 2.0, true)
		draw_string(font, Vector2(legend_x, r.position.y + 14), s["name"], HORIZONTAL_ALIGNMENT_LEFT, 170, fs, s["colour"])
		legend_x += 175
	if hover_x >= r.position.x and hover_x <= r.end.x:
		draw_line(Vector2(hover_x, r.position.y), Vector2(hover_x, r.end.y), Color(Atlas.INK, 0.4), 1.0)
		var year := x0 + (hover_x - r.position.x) / r.size.x * (x1 - x0)
		var text := str(int(year))
		for s in series:
			var best := Vector2.ZERO
			var bd := INF
			for p in (s["points"] as PackedVector2Array):
				if absf(p.x - year) < bd:
					bd = absf(p.x - year)
					best = p
			text += "   %s %s" % [s["name"], _short(best.y)]
		draw_string(font, Vector2(r.position.x + 4, r.end.y - 6), text, HORIZONTAL_ALIGNMENT_LEFT, r.size.x - 8, fs, Atlas.INK)

func _short(v: float) -> String:
	var a := absf(v)
	if a >= 1e9:
		return "%.1f G" % (v / 1e9)
	if a >= 1e6:
		return "%.1f M" % (v / 1e6)
	if a >= 1e4:
		return "%.0f k" % (v / 1e3)
	if a >= 10.0:
		return "%.0f" % v
	return "%.2f" % v
