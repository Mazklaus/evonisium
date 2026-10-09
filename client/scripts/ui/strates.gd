extends "res://scripts/ui/fiche.gd"
## Colonne stratigraphique (document Fonctionnalités : que retiendront les
## roches ?). Les couches déposées par la partie dans la région d'une
## cellule, de la plus ancienne (en bas) à la plus récente, dessinées comme
## dans un traité de géologie : motif conventionnel par roche, épaisseur
## estimée, fossiles, δ¹³C des carbonates, couche à iridium des impacts.

var cell := -1
var column: Control
var detail: Label
var data: Dictionary = {}
var refresh := 0.0
var hovered := -1

const LAYERS := 24

## Couleur et motif de chaque roche (lavis de l'Atlas).
const ROCKS := {
	"calcaire": [Color("#D9CFAE"), "briques"],
	"marnes": [Color("#C9C2A4"), "tirets"],
	"schistes-noirs": [Color("#5A5048"), "tirets"],
	"fer-rubane": [Color("#9A3B22"), "bandes"],
	"gres": [Color("#D6B77F"), "points"],
	"gres-rouges": [Color("#B5694A"), "points"],
	"tillite": [Color("#B9C6BF"), "cailloux"],
}

func open(c: int) -> void:
	cell = c
	_reload()

func _ready() -> void:
	set_title(App.t("strata"))
	custom_minimum_size = Vector2(620, 0)
	var intro := Atlas.text(App.t("strata_text"), 15, true)
	intro.custom_minimum_size.x = 580
	content.add_child(intro)
	var h := HBoxContainer.new()
	content.add_child(h)
	column = Control.new()
	column.custom_minimum_size = Vector2(300, 520)
	column.draw.connect(_draw_column)
	column.gui_input.connect(_on_input)
	column.mouse_exited.connect(func():
		hovered = -1
		column.queue_redraw())
	h.add_child(column)
	detail = Atlas.text("", 15)
	detail.custom_minimum_size.x = 270
	detail.size_flags_vertical = Control.SIZE_SHRINK_BEGIN
	h.add_child(detail)

func _process(delta: float) -> void:
	refresh -= delta
	if refresh <= 0.0:
		_reload()

func _reload() -> void:
	if cell < 0:
		return
	data = App.session.strata(cell, LAYERS)
	refresh = 0.3 if bool(data.get("pending", true)) else 5.0
	var info: Dictionary = App.session.cell_info(cell)
	set_title("%s — %s" % [App.t("strata"), str(info.get("region", ""))])
	if data.get("layers", []).is_empty():
		detail.text = "…" if bool(data.get("pending", true)) else App.t("strata_empty")
	elif hovered < 0:
		detail.text = App.t("strata_hint")
	column.queue_redraw()

func _rects() -> Array:
	var layers: Array = data.get("layers", [])
	var total := 0.0
	for l in layers:
		total += float(l["thickness_m"])
	var out := []
	if total <= 0.0:
		return out
	var h := column.custom_minimum_size.y - 20.0
	var y := h + 10.0
	for l in layers:
		var t := float(l["thickness_m"]) / total * h
		y -= t
		out.append(Rect2(60.0, y, 120.0, t))
	return out

func _draw_column() -> void:
	var layers: Array = data.get("layers", [])
	var rects := _rects()
	var font := Atlas.body_font()
	for i in rects.size():
		var r: Rect2 = rects[i]
		var l: Dictionary = layers[i]
		var style: Array = ROCKS.get(String(l["rock"]), [Atlas.OCHRE, "points"])
		column.draw_rect(r, style[0])
		_pattern(r, style[1])
		if bool(l["impact"]):
			column.draw_line(Vector2(r.position.x - 6, r.end.y - 1), Vector2(r.end.x + 6, r.end.y - 1), Atlas.VERMILION, 3.0)
		if bool(l["stromatolites"]) and r.size.y > 8.0:
			_stromatolite(Vector2(r.end.x - 16, r.end.y - 2), min(r.size.y - 2.0, 12.0))
		if i == hovered:
			column.draw_rect(r, Atlas.VERMILION, false, 2.0)
		# δ¹³C en courbe à droite : de −10 à +10 ‰.
		var d := float(l["delta13c"])
		if is_finite(d):
			var x := 200.0 + (clampf(d, -10.0, 10.0) + 10.0) / 20.0 * 90.0
			column.draw_circle(Vector2(x, r.get_center().y), 2.5, Atlas.INK)
	column.draw_rect(Rect2(60.0, 10.0, 120.0, column.custom_minimum_size.y - 20.0), Atlas.INK, false, 1.5)
	column.draw_line(Vector2(245, 10), Vector2(245, column.custom_minimum_size.y - 10), Atlas.INK.lightened(0.5), 1.0)
	column.draw_string(font, Vector2(196, column.custom_minimum_size.y + 4), "δ¹³C  −10   0   +10 ‰", HORIZONTAL_ALIGNMENT_LEFT, -1, 11, Atlas.INK)
	if not layers.is_empty():
		column.draw_string(font, Vector2(0, column.custom_minimum_size.y - 8), str(layers[0]["from"]), HORIZONTAL_ALIGNMENT_LEFT, 58, 11, Atlas.INK)
		column.draw_string(font, Vector2(0, 20), str(layers[layers.size() - 1]["to"]), HORIZONTAL_ALIGNMENT_LEFT, 58, 11, Atlas.INK)

func _pattern(r: Rect2, kind: String) -> void:
	var ink := Atlas.INK
	ink.a = 0.55
	match kind:
		"briques":
			var y := r.position.y + 5.0
			var row := 0
			while y < r.end.y:
				column.draw_line(Vector2(r.position.x, y), Vector2(r.end.x, y), ink, 1.0)
				var x := r.position.x + (10.0 if row % 2 == 0 else 0.0)
				while x < r.end.x:
					column.draw_line(Vector2(x, y - 5.0), Vector2(x, y), ink, 1.0)
					x += 20.0
				y += 5.0
				row += 1
		"tirets":
			var y2 := r.position.y + 3.0
			while y2 < r.end.y:
				column.draw_dashed_line(Vector2(r.position.x + 2, y2), Vector2(r.end.x - 2, y2), ink, 1.0, 5.0)
				y2 += 4.0
		"bandes":
			var y3 := r.position.y
			var dark := false
			while y3 < r.end.y:
				var t: float = min(2.0, r.end.y - y3)
				if dark:
					column.draw_rect(Rect2(r.position.x, y3, r.size.x, t), Color(0.25, 0.2, 0.2, 0.6))
				dark = not dark
				y3 += 2.0
		"cailloux":
			var k := 0
			var y4 := r.position.y + 4.0
			while y4 < r.end.y:
				var x4 := r.position.x + 6.0 + float((k * 37) % 23)
				while x4 < r.end.x - 4.0:
					column.draw_arc(Vector2(x4, y4), 2.0 + float(k % 3), 0, TAU, 8, ink, 1.0)
					x4 += 19.0
				y4 += 8.0
				k += 1
		_:
			var j := 0
			var y5 := r.position.y + 2.0
			while y5 < r.end.y:
				var x5 := r.position.x + 3.0 + float(j % 2) * 4.0
				while x5 < r.end.x:
					column.draw_circle(Vector2(x5, y5), 0.8, ink)
					x5 += 8.0
				y5 += 3.0
				j += 1

func _stromatolite(base: Vector2, height: float) -> void:
	for k in 3:
		var w := 10.0 - k * 3.0
		column.draw_arc(base - Vector2(0, k * height / 3.0), w, PI, TAU, 10, Atlas.INK, 1.0)

func _on_input(ev: InputEvent) -> void:
	if ev is InputEventMouseMotion:
		var rects := _rects()
		var found := -1
		for i in rects.size():
			if rects[i].grow_individual(60, 0, 120, 0).has_point(ev.position):
				found = i
		if found != hovered:
			hovered = found
			column.queue_redraw()
			_describe()

func _describe() -> void:
	var layers: Array = data.get("layers", [])
	if hovered < 0 or hovered >= layers.size():
		detail.text = App.t("strata_hint")
		return
	var l: Dictionary = layers[hovered]
	var lines := ["%s" % String(l["rock_name"]).capitalize(), "%s → %s" % [l["from"], l["to"]], "%s %d m" % [App.t("thickness"), int(round(float(l["thickness_m"])))], str(l["delta13c_text"])]
	if String(l["fossil"]) != "":
		lines.append("%s : %s" % [App.t("dominant"), l["fossil"]])
	if bool(l["stromatolites"]):
		lines.append(App.t("stromatolites"))
	if bool(l["impact"]):
		lines.append(App.t("iridium"))
	detail.text = "\n".join(lines)
