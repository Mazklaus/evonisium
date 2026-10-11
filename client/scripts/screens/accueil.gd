extends Control
## Accueil : continuer, nouvelle partie, charger, réglages, quitter. Derrière
## le cartouche, une planète de démonstration tourne lentement.

var menu: VBoxContainer
var saves_box: PanelContainer
var settings_panel: Control

func setup(_params: Dictionary) -> void:
	var g = App.globe
	g.interactive = false
	g.set_layer({})
	g.set_show_vents(false)
	g.target_distance = 4.6
	g.view_offset = 0.9
	g.target_pitch = 0.25
	if not App.session.is_running():
		# Planète de démonstration, petite grille, sans vie et en pause.
		App.session.start("terre", 20261007, 4, 1.0, 1.0, 0.0)
		g.reset()
	_build()
	App.settings_changed.connect(_rebuild)

func _rebuild() -> void:
	for c in get_children():
		c.queue_free()
	settings_panel = null
	_build()

func _build() -> void:
	var p := Atlas.panel()
	p.position = Vector2(70, 110)
	p.custom_minimum_size = Vector2(430, 0)
	add_child(p)
	menu = VBoxContainer.new()
	menu.add_theme_constant_override("separation", 12)
	p.add_child(menu)
	var t := Atlas.title(App.t("title"), 58)
	menu.add_child(t)
	var st := Atlas.text(App.t("subtitle"), 22, true)
	st.custom_minimum_size.x = 400
	menu.add_child(st)
	menu.add_child(Atlas.hsep())
	var saves := App.list_saves()
	if not saves.is_empty():
		var b := Atlas.button(App.t("continue"), _continue.bind(saves[0]["path"]))
		var d: Dictionary = App.session.describe_save(saves[0]["path"])
		if not d.is_empty():
			b.tooltip_text = "%s — %s" % [d["name"], d["date"]]
		menu.add_child(b)
		b.grab_focus.call_deferred()
	# La première partie guidée vient en tête tant que le guide n'a pas été
	# suivi jusqu'au bout.
	var guided := Atlas.button(App.t("guided_game"), _guided, App.t("guided_tip"))
	var n := Atlas.button(App.t("new_game"), func(): App.goto("creation"))
	var fresh: bool = not App.session.guide_finished(PackedStringArray(App.settings.get("guide_vu", [])))
	if fresh:
		menu.add_child(guided)
	menu.add_child(n)
	if not fresh:
		menu.add_child(guided)
	if saves.is_empty():
		(guided if fresh else n).grab_focus.call_deferred()
	menu.add_child(Atlas.button(App.t("load"), _show_saves))
	menu.add_child(Atlas.button(App.t("settings"), _show_settings))
	menu.add_child(Atlas.button(App.t("quit"), func(): get_tree().quit()))

## Première partie guidée : la Terre, graine connue, ensemencée près des
## sources chaudes ; le narrateur repart du début.
const GUIDED_SEED := 2026

func _guided() -> void:
	App.session.start("terre", GUIDED_SEED, int(App.settings["level"]), 1.0, 1.0, 0.0)
	App.globe.reset()
	App.goto("ensemencement", {"guided": true})

func _process(delta: float) -> void:
	var g = App.globe
	if g and not bool(App.settings["reduce_motion"]):
		g.target_yaw += delta * 0.05
	if g:
		g.refresh_frame()

func _continue(path: String) -> void:
	App.goto("chargement", {"path": path})

func _show_saves() -> void:
	if saves_box:
		saves_box.queue_free()
	saves_box = Atlas.panel()
	saves_box.position = Vector2(530, 110)
	saves_box.custom_minimum_size = Vector2(520, 380)
	add_child(saves_box)
	var v := VBoxContainer.new()
	saves_box.add_child(v)
	v.add_child(Atlas.title(App.t("saves"), 30))
	var saves := App.list_saves()
	if saves.is_empty():
		v.add_child(Atlas.text(App.t("no_save"), 19, true))
	var list := ItemList.new()
	list.custom_minimum_size = Vector2(480, 260)
	list.size_flags_vertical = Control.SIZE_EXPAND_FILL
	# Arbre des branches : une sauvegarde faite après avoir rechargé une
	# autre se range sous elle.
	var info := {}
	var children := {}
	for s in saves:
		var d: Dictionary = App.session.describe_save(s["path"])
		info[s["path"]] = d
	var roots := []
	for s in saves:
		var parent: String = ProjectSettings.globalize_path(str(info[s["path"]].get("branch_of", "")))
		var found := ""
		for other in saves:
			if other["path"] != s["path"] and ProjectSettings.globalize_path(other["path"]) == parent:
				found = other["path"]
		if found == "":
			roots.append(s)
		else:
			if not children.has(found):
				children[found] = []
			children[found].append(s)
	var add := func(self_ref: Callable, s: Dictionary, depth: int) -> void:
		var d: Dictionary = info[s["path"]]
		var label: String = s["file"]
		if not d.is_empty():
			label = "%s — %s — %s" % [d["name"], d["planet"], d["date"]]
		if depth > 0:
			label = "    ".repeat(depth - 1) + "  ↳ " + label
		list.add_item(label)
		list.set_item_metadata(list.item_count - 1, s["path"])
		for c in children.get(s["path"], []):
			self_ref.call(self_ref, c, min(depth + 1, 6))
	for s in roots:
		add.call(add, s, 0)
	list.item_activated.connect(func(i): _continue(list.get_item_metadata(i)))
	v.add_child(list)
	var h := HBoxContainer.new()
	v.add_child(h)
	h.add_child(Atlas.button(App.t("open"), func():
		var sel := list.get_selected_items()
		if not sel.is_empty():
			_continue(list.get_item_metadata(sel[0]))))
	h.add_child(Atlas.button(App.t("close"), func(): saves_box.queue_free()))

func _show_settings() -> void:
	if settings_panel:
		settings_panel.queue_free()
	settings_panel = preload("res://scripts/ui/reglages.gd").new()
	settings_panel.position = Vector2(530, 60)
	add_child(settings_panel)
