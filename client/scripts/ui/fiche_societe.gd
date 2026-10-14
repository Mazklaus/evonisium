extends "res://scripts/ui/fiche.gd"
## Fiche d'une société (étape 7, sur sociétés d'essai) : forme et rang le
## plus haut, grandeurs agrégées, six dimensions et sept institutions en
## jauges, ce qu'elle croit de chaque domaine des sciences à côté de la vraie
## valeur, techniques connues (les plus avancées d'abord, avec leur
## équivalent terrestre), relations, villes, personnages, et ses courbes
## dans le temps. Les liens mènent d'une société à l'autre ; « Aller voir »
## pose le globe sur sa capitale.

signal society_requested(id: int)
signal compare_requested(a: int, b: int)
signal people_requested(id: int)
signal go_to_cell(cell: int)

const Courbes := preload("res://scripts/ui/courbes.gd")
const Raconter := preload("res://scripts/ui/raconter.gd")

var body: VBoxContainer
var id := -1

func _ready() -> void:
	custom_minimum_size = Vector2(760, 0)
	var scroll := ScrollContainer.new()
	scroll.custom_minimum_size = Vector2(740, 620)
	scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	content.add_child(scroll)
	body = VBoxContainer.new()
	body.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	body.add_theme_constant_override("separation", 6)
	scroll.add_child(body)

func _para(t: String, font_size: int = 17, italic: bool = false) -> Label:
	var l := Atlas.text(t, font_size, italic)
	l.custom_minimum_size.x = 700
	body.add_child(l)
	return l

func _section(t: String) -> void:
	body.add_child(Atlas.hsep())
	body.add_child(Atlas.title(t, 20))

func _gauge(label_text: String, value: float, colour: Color) -> void:
	var h := HBoxContainer.new()
	var l := Atlas.text(label_text, 16, true)
	l.autowrap_mode = TextServer.AUTOWRAP_OFF
	l.custom_minimum_size.x = 170
	h.add_child(l)
	var bar := ProgressBar.new()
	bar.min_value = 0.0
	bar.max_value = 1.0
	bar.value = clampf(value, 0.0, 1.0)
	bar.show_percentage = false
	bar.custom_minimum_size = Vector2(260, 12)
	bar.size_flags_vertical = Control.SIZE_SHRINK_CENTER
	var fill := StyleBoxFlat.new()
	fill.bg_color = colour
	bar.add_theme_stylebox_override("fill", fill)
	h.add_child(bar)
	var v := Atlas.text("%d %%" % roundi(value * 100.0), 15)
	v.autowrap_mode = TextServer.AUTOWRAP_OFF
	h.add_child(v)
	body.add_child(h)

func open_society(society: int) -> void:
	id = society
	for c in body.get_children():
		c.queue_free()
	var d: Dictionary = App.session.society_sheet(society)
	if d.is_empty():
		set_title(App.t("societies"))
		_para(App.t("society_gone"), 17, true)
		return
	set_title(d["name"])
	var colour: Color = d["colour"]
	var warn := _para(App.t("societies_bench_short"), 14, true)
	warn.add_theme_color_override("font_color", Atlas.VERMILION)
	_para(d["summary"], 18)
	var row := HFlowContainer.new()
	body.add_child(row)
	row.add_child(Atlas.button(App.t("go_see"), go_to_cell.emit.bind(int(d["cell"]))))
	var others: Array = App.session.society_list().filter(func(r): return int(r["id"]) != society).slice(0, 30)
	if not others.is_empty():
		var menu := MenuButton.new()
		menu.text = App.t("compare_with")
		menu.flat = false
		for r in others:
			menu.get_popup().add_item(str(r["name"]), int(r["id"]))
		menu.get_popup().id_pressed.connect(func(other): compare_requested.emit(society, other))
		row.add_child(menu)
	if App.globe.peoples_ready or App.session.culture_status() == 2:
		row.add_child(Atlas.button(App.t("people_sheet"), people_requested.emit.bind(int(d["people"]))))
	row.add_child(Raconter.new(func(): return Son.raconter_lieu(int(d["cell"]))))
	if int(d["parent"]) >= 0:
		var h := HBoxContainer.new()
		var pl := Atlas.text(App.t("society_parent"), 16, true)
		pl.autowrap_mode = TextServer.AUTOWRAP_OFF
		pl.size_flags_vertical = Control.SIZE_SHRINK_CENTER
		h.add_child(pl)
		h.add_child(Atlas.button(d["parent_name"], society_requested.emit.bind(int(d["parent"]))))
		body.add_child(h)
	field(body, App.t("society_form"), "%s (%s %s)" % [d["label"], App.t("society_peak"), d["peak"]])
	if str(d["capital"]) != "":
		field(body, App.t("society_capital"), d["capital"])
	for f in d["facts"]:
		field(body, f[0], f[1])

	_section(App.t("society_curves"))
	var curves := HFlowContainer.new()
	body.add_child(curves)
	for k in [["population", "population", true], ["techniques", "techniques_known", false], ["gdp_pc", "society_gdp", true], ["literacy", "literacy", false]]:
		var c := Courbes.new()
		c.custom_minimum_size = Vector2(350, 150)
		c.set_series([{"points": App.session.society_series(society, k[0]), "colour": colour.darkened(0.2), "name": d["name"]}], App.t(k[1]), k[2])
		curves.add_child(c)

	_section(App.t("society_dims"))
	for g in d["dims"]:
		_gauge(g["name"], g["value"], colour)
	_section(App.t("society_institutions"))
	for g in d["institutions"]:
		_gauge(g["name"], g["value"], Color(Atlas.INK, 0.7))

	_section(App.t("society_sciences"))
	_para(App.t("society_sciences_hint"), 15, true)
	for s in d["sciences"]:
		var h := HBoxContainer.new()
		var l := Atlas.text("%s : %s" % [s["domain"], s["quantity"]], 15, true)
		l.custom_minimum_size.x = 380
		h.add_child(l)
		var v := Atlas.text(App.t("society_estimate") % [s["estimate"], s["truth"]], 15)
		v.custom_minimum_size.x = 220
		h.add_child(v)
		var p := Atlas.text("%d %%" % roundi(float(s["precision"]) * 100.0), 15)
		p.autowrap_mode = TextServer.AUTOWRAP_OFF
		p.tooltip_text = App.t("society_precision")
		h.add_child(p)
		body.add_child(h)

	_section(App.t("techniques_known") + " (%d)" % int(d["technique_count"]))
	var techs: Array = d["techniques"]
	var flow := HFlowContainer.new()
	body.add_child(flow)
	for t in techs.slice(0, 60):
		var b := Atlas.text(str(t[0]) + " ·", 15)
		b.autowrap_mode = TextServer.AUTOWRAP_OFF
		b.tooltip_text = App.t("on_earth") % t[1]
		b.mouse_filter = Control.MOUSE_FILTER_PASS
		flow.add_child(b)
	if techs.size() > 60:
		_para(App.t("and_more") % (techs.size() - 60), 15, true)

	_links(App.t("society_relations"), d["relations"], func(e): society_requested.emit(int(e["id"])))
	_links(App.t("society_cities"), d["cities"], func(e): go_to_cell.emit(int(e["id"])))
	if not (d["notables"] as Array).is_empty():
		_section(App.t("notables"))
		for n in d["notables"]:
			_para("%s, %s" % [n["name"], n["detail"]], 16)

func _links(title: String, items: Array, callback: Callable) -> void:
	if items.is_empty():
		return
	_section(title)
	var flow := HFlowContainer.new()
	body.add_child(flow)
	for e in items:
		var b := Atlas.button("%s (%s)" % [e["name"], e["detail"]], callback.bind(e))
		flow.add_child(b)
