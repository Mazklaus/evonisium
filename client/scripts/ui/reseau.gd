extends "res://scripts/ui/fiche.gd"
## Réseau trophique d'une cellule (document Fonctionnalités : qui mange
## qui ?). Les producteurs en bas, ceux qui vivent de leurs produits
## au-dessus ; un trait plein pour la matière organique qui nourrit, un trait
## fin pour les échanges (O₂, CH₄, sulfure…), un tireté vermillon pour la
## compétition. L'épaisseur suit le flux estimé.

signal species_requested(species: int)

var cell := -1
var graph: Control
var legend: Label
var data: Dictionary = {}
var positions: Array = []
var refresh := 0.0

func open(c: int) -> void:
	cell = c
	_reload()

func _ready() -> void:
	set_title(App.t("food_web"))
	custom_minimum_size = Vector2(760, 0)
	var intro := Atlas.text(App.t("food_web_text"), 15, true)
	intro.custom_minimum_size.x = 720
	content.add_child(intro)
	graph = Control.new()
	graph.custom_minimum_size = Vector2(720, 420)
	graph.draw.connect(_draw_graph)
	graph.gui_input.connect(_on_input)
	content.add_child(graph)
	legend = Atlas.text("", 15)
	legend.custom_minimum_size.x = 720
	content.add_child(legend)

func _process(delta: float) -> void:
	refresh -= delta
	if refresh <= 0.0:
		_reload()

func _reload() -> void:
	if cell < 0:
		return
	data = App.session.food_web(cell)
	refresh = 0.2 if bool(data.get("pending", true)) else 2.0
	var info: Dictionary = App.session.cell_info(cell)
	set_title("%s — %s" % [App.t("food_web"), str(info.get("region", ""))])
	_layout()
	graph.queue_redraw()

## Niveaux en lignes, espèces réparties sur chaque ligne.
func _layout() -> void:
	positions.clear()
	var nodes: Array = data.get("nodes", [])
	if nodes.is_empty():
		legend.text = "…" if bool(data.get("pending", true)) else App.t("food_web_empty")
		return
	var levels := {}
	var top := 0
	for i in nodes.size():
		var l := int(nodes[i]["level"])
		top = max(top, l)
		if not levels.has(l):
			levels[l] = []
		levels[l].append(i)
	positions.resize(nodes.size())
	var w := graph.custom_minimum_size.x
	var h := graph.custom_minimum_size.y
	for l in levels.keys():
		var row: Array = levels[l]
		for k in row.size():
			var x := w * (float(k) + 0.5) / float(row.size())
			var y := h - 40.0 - (h - 80.0) * (float(l) / float(max(top, 1)))
			positions[row[k]] = Vector2(x, y)
	legend.text = App.t("food_web_legend")

func _draw_graph() -> void:
	graph.draw_rect(Rect2(Vector2.ZERO, graph.size), Atlas.INK, false, 1.0)
	var nodes: Array = data.get("nodes", [])
	if nodes.is_empty() or positions.size() != nodes.size():
		return
	var font := Atlas.body_font()
	for e in data.get("edges", []):
		var a: Vector2 = positions[int(e["from"])]
		var b: Vector2 = positions[int(e["to"])]
		var wgt := float(e["weight"])
		match String(e["link"]):
			"nourrit":
				_arrow(a, b, Atlas.INK, 1.0 + 6.0 * wgt)
			"echange":
				_arrow(a, b, Atlas.WATER.darkened(0.45), 1.0 + 3.0 * wgt)
			_:
				graph.draw_dashed_line(a, b, Atlas.VERMILION, 1.0 + 2.0 * wgt, 6.0)
	for i in nodes.size():
		var n: Dictionary = nodes[i]
		var p: Vector2 = positions[i]
		graph.draw_circle(p, 16.0, n["colour"])
		graph.draw_arc(p, 16.0, 0.0, TAU, 32, Atlas.INK, 2.0, true)
		var name: String = n["name"]
		graph.draw_string(font, p + Vector2(-90, 32), name, HORIZONTAL_ALIGNMENT_CENTER, 180, 13, Atlas.INK)
		graph.draw_string(font, p + Vector2(-90, 46), "%s mol C" % n["biomass"], HORIZONTAL_ALIGNMENT_CENTER, 180, 11, Atlas.INK.lightened(0.25))

func _arrow(a: Vector2, b: Vector2, c: Color, width: float) -> void:
	var dir := (b - a).normalized()
	var from := a + dir * 18.0
	var to := b - dir * 18.0
	graph.draw_line(from, to, c, width, true)
	var side := Vector2(-dir.y, dir.x) * (4.0 + width)
	graph.draw_colored_polygon(PackedVector2Array([to, to - dir * (8.0 + width) + side, to - dir * (8.0 + width) - side]), c)

func _on_input(ev: InputEvent) -> void:
	if ev is InputEventMouseButton and ev.pressed and ev.button_index == MOUSE_BUTTON_LEFT:
		var nodes: Array = data.get("nodes", [])
		for i in positions.size():
			if positions[i].distance_to(ev.position) < 18.0:
				species_requested.emit(int(nodes[i]["species"]))
				return
