extends "res://scripts/ui/fiche.gd"
## Fiche d'un peuple ou d'un personnage notable (étape 6, document
## Fonctionnalités, « Fiches de peuple et de personnage ») : origine, langue
## et quelques mots, techniques avec leur part et leur maîtrise, coutumes,
## mythes, personnages. Les liens mènent d'une fiche à l'autre ; « Aller
## voir » pose le globe sur le cœur du peuple ou la cellule du personnage.

signal go_to_cell(cell: int)

var body: VBoxContainer
var history: Array = []

func _ready() -> void:
	custom_minimum_size = Vector2(700, 0)
	var scroll := ScrollContainer.new()
	scroll.custom_minimum_size = Vector2(680, 600)
	scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	content.add_child(scroll)
	body = VBoxContainer.new()
	body.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	body.add_theme_constant_override("separation", 6)
	scroll.add_child(body)

func _clear() -> void:
	for c in body.get_children():
		c.queue_free()

func _para(t: String, size: int = 17, italic: bool = false) -> Label:
	var l := Atlas.text(t, size, italic)
	l.custom_minimum_size.x = 640
	body.add_child(l)
	return l

func _back_row() -> void:
	if history.size() < 2:
		return
	var b := Atlas.button("‹ " + App.t("back"), func():
		history.pop_back()
		var prev: Array = history.pop_back()
		if prev[0] == "people":
			open_people(prev[1])
		else:
			open_notable(prev[1]))
	b.size_flags_horizontal = Control.SIZE_SHRINK_BEGIN
	body.add_child(b)

func _links(title: String, items: Array, callback: Callable) -> void:
	if items.is_empty():
		return
	body.add_child(Atlas.text(title, 17, true))
	var flow := HFlowContainer.new()
	for it in items:
		var b := Atlas.button(str(it["name"]) + ("" if bool(it.get("alive", true)) else " †"), callback.bind(int(it["id"])))
		b.add_theme_font_size_override("font_size", int(15 * App.text_scale()))
		flow.add_child(b)
	body.add_child(flow)

func open_people(id: int) -> void:
	var d: Dictionary = App.session.people_info(id)
	_clear()
	history.append(["people", id])
	if d.is_empty():
		set_title(App.t("peoples"))
		_para(App.t("peoples_none"), 17, true)
		return
	set_title(d["name"])
	title_label.add_theme_color_override("font_color", (d["colour"] as Color).darkened(0.35))
	_back_row()
	_para(App.t("peoples_bench_short"), 14, true).add_theme_color_override("font_color", Atlas.VERMILION)
	_para(d["summary"], 18)
	field(body, App.t("people_species"), d["species"])
	field(body, App.t("people_born"), d["born"] + ("" if d["ended"] == "" else " — " + App.t("people_ended") + " " + d["ended"]))
	field(body, App.t("population"), "%s · %d %s" % [d["population"], int(d["groups"]), App.t("people_groups")])
	if int(d["parent"]) >= 0:
		_links(App.t("people_parent"), [{"id": d["parent"], "name": d["parent_name"]}], open_people)
	_links(App.t("people_children"), d["children"], open_people)
	if d["language"] != "":
		field(body, App.t("language"), d["language"])
		var words: Array = d["words"]
		if not words.is_empty():
			var parts := PackedStringArray()
			for w in words:
				parts.append("%s « %s »" % [w["word"], w["meaning"]])
			_para(" · ".join(parts), 15, true)
	body.add_child(Atlas.hsep())
	_traits(App.t("techniques"), d["techniques"])
	_traits(App.t("customs"), d["customs"])
	var myths: Array = d["myths"]
	if not myths.is_empty():
		var parts := PackedStringArray()
		for m in myths:
			parts.append(m["title"])
		body.add_child(Atlas.text(App.t("myths"), 17, true))
		_para(" · ".join(parts), 16, true)
	_links(App.t("notables"), d["notables"], open_notable)
	var go := Atlas.button(App.t("go_see"), func(): go_to_cell.emit(int(d["cell"])))
	go.size_flags_horizontal = Control.SIZE_SHRINK_BEGIN
	body.add_child(go)

## Techniques ou coutumes : part des membres qui la portent (barre) et
## maîtrise moyenne.
func _traits(title: String, lines: Array) -> void:
	if lines.is_empty():
		return
	body.add_child(Atlas.text(title, 17, true))
	for t in lines:
		var h := HBoxContainer.new()
		var n := Atlas.text(t["name"], 16)
		n.autowrap_mode = TextServer.AUTOWRAP_OFF
		n.custom_minimum_size.x = 300
		n.clip_text = true
		n.tooltip_text = t["kind"]
		n.mouse_filter = Control.MOUSE_FILTER_PASS
		h.add_child(n)
		var bar := Control.new()
		bar.custom_minimum_size = Vector2(160, 14)
		var share := float(t["share"])
		bar.draw.connect(func():
			bar.draw_rect(Rect2(0, 2, 160, 10), Atlas.INK, false, 1.0)
			bar.draw_rect(Rect2(1, 3, 158 * clampf(share, 0.0, 1.0), 8), Color(Atlas.OCHRE, 0.85)))
		h.add_child(bar)
		var m := Atlas.text("%d %% · %s %d %%" % [roundi(share * 100.0), App.t("mastery"), roundi(float(t["mastery"]) * 100.0)], 14, true)
		m.autowrap_mode = TextServer.AUTOWRAP_OFF
		h.add_child(m)
		body.add_child(h)

func open_notable(id: int) -> void:
	var d: Dictionary = App.session.notable_info(id)
	_clear()
	history.append(["notable", id])
	if d.is_empty():
		set_title(App.t("notables"))
		return
	set_title(d["name"])
	title_label.remove_theme_color_override("font_color")
	_back_row()
	_para("%s · %s" % [d["role"], d["species"]], 17, true)
	_para(d["story"], 18)
	field(body, App.t("trait"), d["trait"])
	field(body, App.t("notable_born"), d["born"])
	if not bool(d["alive"]):
		field(body, App.t("death"), d["death"])
	_links(App.t("people"), [{"id": d["people"], "name": d["people_name"]}], open_people)
	var go := Atlas.button(App.t("go_see"), func(): go_to_cell.emit(int(d["cell"])))
	go.size_flags_horizontal = Control.SIZE_SHRINK_BEGIN
	body.add_child(go)
