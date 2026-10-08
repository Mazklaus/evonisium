extends "res://scripts/ui/fiche.gd"
## Arbre du vivant (document Fonctionnalités, « Arbre du vivant ») : temps
## en abscisse, lignées en branches à l'encre, vivantes au trait plein,
## éteintes en pointillé, lignées sans descendance repliées. Un clic sur une
## branche ouvre la fiche de l'espèce.

signal species_requested(species: int)

const MAX_LEAVES := 60

var canvas: Control
var tree := {}
var hover := -1
var tip: Label
var refresh_timer := 0.0
var row_h := 18.0
var margin_l := 12.0
var margin_r := 330.0

func _ready() -> void:
	set_title(App.t("tree"))
	custom_minimum_size = Vector2(980, 0)
	tip = Atlas.text(App.t("tree_hint"), 15, true)
	tip.custom_minimum_size.x = 900
	content.add_child(tip)
	var scroll := ScrollContainer.new()
	scroll.custom_minimum_size = Vector2(950, 560)
	content.add_child(scroll)
	canvas = Control.new()
	canvas.custom_minimum_size = Vector2(930, 540)
	canvas.mouse_filter = Control.MOUSE_FILTER_STOP
	canvas.draw.connect(_draw_tree)
	canvas.gui_input.connect(_input_tree)
	scroll.add_child(canvas)
	_reload()

func _reload() -> void:
	tree = App.session.tree(MAX_LEAVES)
	if tree.is_empty():
		return
	var leaves := int(tree["leaves"])
	canvas.custom_minimum_size.y = max(540.0, (leaves + 1) * row_h + 30.0)
	canvas.queue_redraw()

func _process(delta: float) -> void:
	refresh_timer -= delta
	if refresh_timer <= 0.0:
		refresh_timer = 2.0
		_reload()

func _x(years: float) -> float:
	var t0 := float(tree["from_years"])
	var t1 := float(tree["to_years"])
	return margin_l + (years - t0) / max(t1 - t0, 1.0) * (canvas.size.x - margin_l - margin_r)

func _y(y: float) -> float:
	return 20.0 + y * row_h

func _draw_tree() -> void:
	if tree.is_empty():
		return
	var ids: PackedInt32Array = tree["ids"]
	if ids.is_empty():
		var font := Atlas.body_font()
		canvas.draw_string(font, Vector2(20, 40), App.t("no_species"), HORIZONTAL_ALIGNMENT_LEFT, -1, int(18 * App.text_scale()), Atlas.INK)
		return
	var parents: PackedInt32Array = tree["parents"]
	var born: PackedFloat64Array = tree["born"]
	var end: PackedFloat64Array = tree["end"]
	var ys: PackedFloat32Array = tree["y"]
	var living: PackedByteArray = tree["living"]
	var collapsed: PackedInt32Array = tree["collapsed"]
	var names: PackedStringArray = tree["names"]
	var font := Atlas.body_font()
	var italic: Font = Atlas.text_italic
	var fs := int(14 * App.text_scale())
	# Échelle du temps en haut.
	var t0 := float(tree["from_years"])
	var t1 := float(tree["to_years"])
	for k in 5:
		var yrs := t0 + (t1 - t0) * k / 4.0
		var x := _x(yrs)
		canvas.draw_line(Vector2(x, 2), Vector2(x, 8), Atlas.INK, 1.0)
		canvas.draw_string(font, Vector2(x + 3, 12), App.session.format_duration(yrs), HORIZONTAL_ALIGNMENT_LEFT, -1, int(12 * App.text_scale()), Color(Atlas.INK, 0.8))
	for i in ids.size():
		var y := _y(ys[i])
		var xa := _x(born[i])
		var xb := _x(end[i])
		var p := parents[i]
		var c := Atlas.VERMILION if i == hover else Atlas.INK
		if p != i and p >= 0:
			# Attache verticale à la branche parente.
			canvas.draw_line(Vector2(xa, _y(ys[p])), Vector2(xa, y), Color(Atlas.INK, 0.8), 1.0)
		if living[i] != 0:
			canvas.draw_line(Vector2(xa, y), Vector2(xb, y), c, 2.0)
		else:
			canvas.draw_dashed_line(Vector2(xa, y), Vector2(xb, y), c, 1.2, 4.0)
			canvas.draw_line(Vector2(xb, y - 4), Vector2(xb, y + 4), c, 1.2)
		var label := names[i]
		if collapsed[i] > 0:
			# Le nombre d'abord : un nom long ne doit pas le cacher.
			label = ("%d lignées · %s" if App.settings["lang"] == "fr" else "%d lineages · %s") % [collapsed[i], label]
		if living[i] != 0 or i == hover:
			canvas.draw_string(italic if living[i] == 0 else font, Vector2(xb + 6, y + 5), label, HORIZONTAL_ALIGNMENT_LEFT, margin_r - 10, fs, c)

func _node_at(pos: Vector2) -> int:
	if tree.is_empty():
		return -1
	var ids: PackedInt32Array = tree["ids"]
	var ys: PackedFloat32Array = tree["y"]
	var born: PackedFloat64Array = tree["born"]
	var end: PackedFloat64Array = tree["end"]
	for i in ids.size():
		if abs(pos.y - _y(ys[i])) < row_h * 0.5 and pos.x >= _x(born[i]) - 4 and pos.x <= _x(end[i]) + margin_r:
			return i
	return -1

func _input_tree(event: InputEvent) -> void:
	if event is InputEventMouseMotion:
		var n := _node_at(event.position)
		if n != hover:
			hover = n
			canvas.queue_redraw()
			if n >= 0:
				var born: PackedFloat64Array = tree["born"]
				tip.text = "%s — %s %s" % [tree["names"][n], App.t("born"), App.session.format_duration(float(tree["to_years"]) - born[n])]
	elif event is InputEventMouseButton and event.pressed and event.button_index == MOUSE_BUTTON_LEFT:
		var n := _node_at(event.position)
		if n >= 0:
			species_requested.emit(int(tree["species"][n]))
