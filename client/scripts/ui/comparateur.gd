extends "res://scripts/ui/fiche.gd"
## Comparateur d'espèces (document Fonctionnalités, « Comparateur ») : deux
## espèces côte à côte à la même échelle, leurs traits en tableau avec les
## différences en vermillon, et leur ancêtre commun.

signal species_requested(species: int)

const Portrait := preload("res://scripts/ui/portrait.gd")

var a := -1
var b := -1
var portraits: Array = []
var overlays: Array = []
var names: Array = []
var picker: OptionButton
var table: GridContainer
var ancestor_box: HBoxContainer
var data := {}
var refresh := 0.0
var choices: Array = []

func open(first: int, second := -1) -> void:
	a = first
	choices = App.session.living_species(40)
	picker.clear()
	for c in choices:
		picker.add_item(str(c["name"]))
	if second < 0:
		for c in choices:
			if int(c["species"]) != a:
				second = int(c["species"])
				break
	_set_b(second)

func _ready() -> void:
	set_title(App.t("comparator"))
	custom_minimum_size = Vector2(840, 0)
	var row := HBoxContainer.new()
	row.add_theme_constant_override("separation", 20)
	content.add_child(row)
	for k in 2:
		var col := VBoxContainer.new()
		row.add_child(col)
		var n := Atlas.text("", 17, true)
		n.custom_minimum_size.x = 390
		col.add_child(n)
		names.append(n)
		var p = Portrait.new()
		p.custom_minimum_size = Vector2(390, 220)
		col.add_child(p)
		portraits.append(p)
		var o := Control.new()
		o.custom_minimum_size = Vector2(390, 26)
		o.draw.connect(func(): p.draw_scale_bar(o, Vector2(12, 12)))
		col.add_child(o)
		overlays.append(o)
		p.body_ready.connect(func(_d): o.queue_redraw())
	var h := HBoxContainer.new()
	content.add_child(h)
	var cw := Atlas.text(App.t("compare_with"), 16, true)
	cw.autowrap_mode = TextServer.AUTOWRAP_OFF
	h.add_child(cw)
	picker = OptionButton.new()
	picker.custom_minimum_size.x = 380
	picker.item_selected.connect(func(i): _set_b(int(choices[i]["species"])))
	h.add_child(picker)
	content.add_child(Atlas.hsep())
	table = GridContainer.new()
	table.columns = 3
	table.add_theme_constant_override("h_separation", 18)
	content.add_child(table)
	content.add_child(Atlas.hsep())
	ancestor_box = HBoxContainer.new()
	content.add_child(ancestor_box)

func _set_b(s: int) -> void:
	b = s
	for i in choices.size():
		if int(choices[i]["species"]) == b:
			picker.select(i)
	data = App.session.compare_species(a, b)
	portraits[0].show_body(str(data.get("key_a", "")))
	portraits[1].show_body(str(data.get("key_b", "")))
	_fill()

func _process(delta: float) -> void:
	refresh -= delta
	if refresh <= 0.0 and a >= 0 and b >= 0 and not bool(data.get("ready", false)):
		refresh = 0.3
		data = App.session.compare_species(a, b)
		_fill()

func _fill() -> void:
	for c in table.get_children():
		table.remove_child(c)
		c.queue_free()
	var rows: Array = data.get("rows", [])
	if rows.is_empty():
		return
	names[0].text = str(rows[0]["a"])
	names[1].text = str(rows[0]["b"])
	if data.has("relative_a"):
		portraits[0].set_relative(float(data["relative_a"]))
		portraits[1].set_relative(float(data["relative_b"]))
		overlays[0].queue_redraw()
		overlays[1].queue_redraw()
	for t in ["", "A", "B"]:
		table.add_child(Atlas.text(t, 15, true))
	for r in rows.slice(1):
		var lab := Atlas.text(r["label"], 15, true)
		lab.autowrap_mode = TextServer.AUTOWRAP_OFF
		lab.custom_minimum_size.x = 150
		table.add_child(lab)
		for side in ["a", "b"]:
			var l := Atlas.text(str(r[side]), 15)
			l.custom_minimum_size.x = 310
			if bool(r["differs"]):
				l.add_theme_color_override("font_color", Atlas.VERMILION)
			table.add_child(l)
	for c in ancestor_box.get_children():
		c.queue_free()
	if int(data.get("ancestor", -1)) >= 0:
		for t in ["%s :" % App.t("common_ancestor")]:
			var al := Atlas.text(t, 16, true)
			al.autowrap_mode = TextServer.AUTOWRAP_OFF
			ancestor_box.add_child(al)
		var btn := Atlas.button(str(data["ancestor_name"]), species_requested.emit.bind(int(data["ancestor"])))
		ancestor_box.add_child(btn)
		var dl := Atlas.text(App.t("diverged") % str(data["divergence"]), 16)
		dl.autowrap_mode = TextServer.AUTOWRAP_OFF
		ancestor_box.add_child(dl)
	else:
		ancestor_box.add_child(Atlas.text(App.t("no_common_ancestor"), 16, true))
