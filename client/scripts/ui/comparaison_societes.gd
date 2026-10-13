extends "res://scripts/ui/fiche.gd"
## Comparaison de civilisations (étape 7 ; document Fonctionnalités,
## « Comparateur » : deux sociétés trait par trait, les différences
## surlignées), et leurs courbes dans le temps sur un même axe.

signal society_requested(id: int)

const Courbes := preload("res://scripts/ui/courbes.gd")

var body: VBoxContainer
var a := -1
var b := -1

func _ready() -> void:
	custom_minimum_size = Vector2(820, 0)
	var scroll := ScrollContainer.new()
	scroll.custom_minimum_size = Vector2(800, 620)
	scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	content.add_child(scroll)
	body = VBoxContainer.new()
	body.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	body.add_theme_constant_override("separation", 4)
	scroll.add_child(body)

func open_pair(x: int, y: int) -> void:
	a = x
	b = y
	for c in body.get_children():
		c.queue_free()
	var da: Dictionary = App.session.society_sheet(x)
	var db: Dictionary = App.session.society_sheet(y)
	set_title(App.t("compare_societies"))
	if da.is_empty() or db.is_empty():
		body.add_child(Atlas.text(App.t("society_gone"), 17, true))
		return
	var ca: Color = (da["colour"] as Color).darkened(0.25)
	var cb: Color = (db["colour"] as Color).darkened(0.25)
	var head := HBoxContainer.new()
	var blank := Control.new()
	blank.custom_minimum_size.x = 300
	head.add_child(blank)
	for pair in [[da, x, ca], [db, y, cb]]:
		var btn := Atlas.button(pair[0]["name"], society_requested.emit.bind(pair[1]))
		btn.custom_minimum_size.x = 230
		btn.add_theme_color_override("font_color", pair[2])
		head.add_child(btn)
	body.add_child(head)
	var warn := Atlas.text(App.t("societies_bench_short"), 14, true)
	warn.add_theme_color_override("font_color", Atlas.VERMILION)
	warn.custom_minimum_size.x = 760
	body.add_child(warn)
	var hint := Atlas.text(App.t("compare_hint"), 14, true)
	hint.custom_minimum_size.x = 760
	body.add_child(hint)
	for r in App.session.society_compare(x, y):
		var h := HBoxContainer.new()
		var n := Atlas.text(r["name"], 15, true)
		n.custom_minimum_size.x = 300
		h.add_child(n)
		for k in ["a", "b"]:
			var v := Atlas.text(r[k], 15)
			v.custom_minimum_size.x = 230
			if r["differs"]:
				v.add_theme_color_override("font_color", Atlas.VERMILION)
			h.add_child(v)
		body.add_child(h)
	body.add_child(Atlas.hsep())
	body.add_child(Atlas.title(App.t("society_curves"), 20))
	var flow := HFlowContainer.new()
	body.add_child(flow)
	for k in [["population", "population", true], ["cells", "society_cells", false], ["techniques", "techniques_known", false], ["gdp_pc", "society_gdp", true], ["urban", "society_urban", false], ["literacy", "literacy", false]]:
		var c := Courbes.new()
		c.custom_minimum_size = Vector2(380, 160)
		c.set_series([
			{"points": App.session.society_series(x, k[0]), "colour": ca, "name": da["name"]},
			{"points": App.session.society_series(y, k[0]), "colour": cb, "name": db["name"]},
		], App.t(k[1]), k[2])
		flow.add_child(c)
