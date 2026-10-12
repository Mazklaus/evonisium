extends "res://scripts/ui/fiche.gd"
## Peuples (étape 6, document Fonctionnalités, « Arbre des cultures ») : la
## liste des peuples à leur couleur, l'arbre des peuples et celui des
## langues (temps en abscisse, vivants au trait plein, disparus en
## pointillé), et les personnages notables. Un clic ouvre la fiche.

signal people_requested(id: int)
signal notable_requested(id: int)

const MAX_LEAVES := 40

var tabs: TabContainer
var list: VBoxContainer
var notables: VBoxContainer
var canvases: Array = []
var trees: Array = [{}, {}]
var hover := [-1, -1]
var row_h := 22.0
var margin_l := 12.0
var margin_r := 220.0

func _ready() -> void:
	set_title(App.t("peoples"))
	custom_minimum_size = Vector2(860, 0)
	var warn := Atlas.text(App.t("peoples_bench"), 15, true)
	warn.custom_minimum_size.x = 820
	warn.add_theme_color_override("font_color", Atlas.VERMILION)
	content.add_child(warn)
	var mode: Dictionary = App.session.culture_mode()
	if not mode.is_empty():
		content.add_child(Atlas.text(mode["text"], 16, true))
	tabs = TabContainer.new()
	tabs.custom_minimum_size = Vector2(830, 540)
	content.add_child(tabs)

	var scroll := ScrollContainer.new()
	scroll.name = App.t("peoples")
	scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	tabs.add_child(scroll)
	list = VBoxContainer.new()
	list.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	scroll.add_child(list)

	for k in 2:
		var s := ScrollContainer.new()
		s.name = App.t("people_tree") if k == 0 else App.t("language_tree")
		tabs.add_child(s)
		var canvas := Control.new()
		canvas.custom_minimum_size = Vector2(800, 500)
		canvas.mouse_filter = Control.MOUSE_FILTER_STOP
		canvas.draw.connect(_draw_tree.bind(k))
		canvas.gui_input.connect(_input_tree.bind(k))
		s.add_child(canvas)
		canvases.append(canvas)

	var ns := ScrollContainer.new()
	ns.name = App.t("notables")
	ns.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	tabs.add_child(ns)
	notables = VBoxContainer.new()
	notables.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	ns.add_child(notables)
	_fill()

func _fill() -> void:
	var labels: Array = App.session.culture_labels()
	if labels.is_empty():
		list.add_child(Atlas.text(App.t("peoples_none"), 16, true))
	for p in labels:
		var info: Dictionary = App.session.people_info(int(p["id"]))
		var h := HBoxContainer.new()
		var sw := ColorRect.new()
		sw.color = p["colour"]
		sw.custom_minimum_size = Vector2(18, 18)
		sw.size_flags_vertical = Control.SIZE_SHRINK_CENTER
		h.add_child(sw)
		var b := Atlas.button(p["name"], people_requested.emit.bind(int(p["id"])))
		b.custom_minimum_size.x = 160
		h.add_child(b)
		var t := Atlas.text(info.get("summary", ""), 15)
		t.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		t.custom_minimum_size.x = 560
		h.add_child(t)
		list.add_child(h)
	for k in 2:
		trees[k] = App.session.culture_tree(k == 1, MAX_LEAVES)
		if not trees[k].is_empty():
			canvases[k].custom_minimum_size.y = max(500.0, (int(trees[k]["leaves"]) + 1) * row_h + 40.0)
		canvases[k].queue_redraw()
	# Personnages : un par peuple vivant, dans l'ordre de la liste.
	for p in labels:
		var info: Dictionary = App.session.people_info(int(p["id"]))
		var ns: Array = info.get("notables", [])
		if ns.is_empty():
			continue
		notables.add_child(Atlas.text(p["name"], 17, true))
		var flow := HFlowContainer.new()
		for n in ns:
			var b := Atlas.button(str(n["name"]) + ("" if bool(n["alive"]) else " †"), notable_requested.emit.bind(int(n["id"])))
			b.add_theme_font_size_override("font_size", int(15 * App.text_scale()))
			flow.add_child(b)
		notables.add_child(flow)

func _x(k: int, years: float) -> float:
	var t0 := float(trees[k]["from_years"])
	var t1 := float(trees[k]["to_years"])
	return margin_l + (years - t0) / max(t1 - t0, 1.0) * (canvases[k].size.x - margin_l - margin_r)

func _y(y: float) -> float:
	return 30.0 + y * row_h

func _draw_tree(k: int) -> void:
	var canvas: Control = canvases[k]
	var tree: Dictionary = trees[k]
	if tree.is_empty():
		return
	var ids: PackedInt32Array = tree["ids"]
	var parents: PackedInt32Array = tree["parents"]
	var born: PackedFloat64Array = tree["born"]
	var end: PackedFloat64Array = tree["end"]
	var ys: PackedFloat32Array = tree["y"]
	var living: PackedByteArray = tree["living"]
	var names: PackedStringArray = tree["names"]
	var colours: PackedColorArray = tree["colours"]
	var font := Atlas.body_font()
	var fs := int(15 * App.text_scale())
	# Échelle du temps : années avant le présent du banc.
	var t0 := float(tree["from_years"])
	var t1 := float(tree["to_years"])
	for s in 5:
		var yrs := t0 + (t1 - t0) * s / 4.0
		var x := _x(k, yrs)
		canvas.draw_line(Vector2(x, 2), Vector2(x, 8), Atlas.INK, 1.0)
		var d: String = App.session.format_duration(t1 - yrs)
		var ago := App.t("today") if t1 - yrs < 0.5 else ("il y a " + d if App.settings["lang"] == "fr" else d + " ago")
		canvas.draw_string(font, Vector2(x + 3, 14), ago, HORIZONTAL_ALIGNMENT_LEFT, -1, int(12 * App.text_scale()), Color(Atlas.INK, 0.8))
	for i in ids.size():
		var y := _y(ys[i])
		var xa := _x(k, born[i])
		var xb := _x(k, end[i])
		var p := parents[i]
		var c: Color = Atlas.VERMILION if i == hover[k] else colours[i].darkened(0.3)
		if p != i and p >= 0:
			canvas.draw_line(Vector2(xa, _y(ys[p])), Vector2(xa, y), Color(Atlas.INK, 0.8), 1.0)
		if living[i] != 0:
			canvas.draw_line(Vector2(xa, y), Vector2(xb, y), c, 3.0)
		else:
			canvas.draw_dashed_line(Vector2(xa, y), Vector2(xb, y), c, 1.5, 4.0)
			canvas.draw_line(Vector2(xb, y - 4), Vector2(xb, y + 4), c, 1.2)
		canvas.draw_string(font, Vector2(xb + 6, y + 5), names[i], HORIZONTAL_ALIGNMENT_LEFT, margin_r - 10, fs, Atlas.INK if living[i] != 0 else Color(Atlas.INK, 0.6))

func _node_at(k: int, pos: Vector2) -> int:
	var tree: Dictionary = trees[k]
	if tree.is_empty():
		return -1
	var ids: PackedInt32Array = tree["ids"]
	var ys: PackedFloat32Array = tree["y"]
	var born: PackedFloat64Array = tree["born"]
	var end: PackedFloat64Array = tree["end"]
	for i in ids.size():
		if abs(pos.y - _y(ys[i])) < row_h * 0.5 and pos.x >= _x(k, born[i]) - 4 and pos.x <= _x(k, end[i]) + margin_r:
			return i
	return -1

func _input_tree(event: InputEvent, k: int) -> void:
	if event is InputEventMouseMotion:
		var n := _node_at(k, event.position)
		if n != hover[k]:
			hover[k] = n
			canvases[k].queue_redraw()
	elif event is InputEventMouseButton and event.pressed and event.button_index == MOUSE_BUTTON_LEFT and k == 0:
		var n := _node_at(k, event.position)
		if n >= 0:
			people_requested.emit(int(trees[k]["ids"][n]))
