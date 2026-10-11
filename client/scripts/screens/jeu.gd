extends Control
## L'écran de la partie : le globe au centre, la barre du temps en haut, les
## calques à gauche, l'inspecteur à droite, la frise en bas, les alertes et
## le narrateur ; les fiches (espèce, arbre, chronique, interventions,
## sauvegarde, réglages) s'ouvrent au centre.

const BarreTemps := preload("res://scripts/ui/barre_temps.gd")
const Frise := preload("res://scripts/ui/frise.gd")
const Calques := preload("res://scripts/ui/calques.gd")
const Inspecteur := preload("res://scripts/ui/inspecteur.gd")
const Alertes := preload("res://scripts/ui/alertes.gd")
const Narrateur := preload("res://scripts/ui/narrateur.gd")
const FicheEspece := preload("res://scripts/ui/fiche_espece.gd")
const Arbre := preload("res://scripts/ui/arbre.gd")
const Chronique := preload("res://scripts/ui/chronique.gd")
const Interventions := preload("res://scripts/ui/interventions.gd")
const Sauvegarde := preload("res://scripts/ui/sauvegarde.gd")
const Reglages := preload("res://scripts/ui/reglages.gd")
const AvecSans := preload("res://scripts/ui/avec_sans.gd")
const Reseau := preload("res://scripts/ui/reseau.gd")
const Strates := preload("res://scripts/ui/strates.gd")
const Anatomie := preload("res://scripts/ui/anatomie.gd")
const Comparateur := preload("res://scripts/ui/comparateur.gd")
const Sol := preload("res://scripts/screens/sol.gd")

var bar: PanelContainer
var frise: PanelContainer
var calques: PanelContainer
var inspecteur: PanelContainer
var alertes: VBoxContainer
var narrateur: PanelContainer
var overlay: CenterContainer
var notice: Label
var fiche: Control

var seeking := false
var speed_before_seek := 1.0e5
var autosave_clock := 0.0
var info := {}
var recent: Array = []
var o2_seen := false
## Outil souligné par le narrateur, et son cadre.
var highlighted := ""
var highlight_frame: Panel
var menu_buttons := {}
## « Pendant votre absence » : dernière date regardée par le joueur.
var absence_card: PanelContainer
var absence_label: Label
var absence_events: Array = []
var idle := 0.0
var looked_years := -1.0
var sol: Control

func setup(params: Dictionary) -> void:
	var g = App.globe
	g.interactive = true
	g.set_show_vents(false)
	g.target_distance = 4.0
	g.view_offset = 0.0
	g.cell_selected.connect(_on_cell_selected)

	bar = BarreTemps.new()
	add_child(bar)
	bar.set_anchors_and_offsets_preset(Control.PRESET_TOP_WIDE, Control.PRESET_MODE_MINSIZE, 10)
	bar.next_requested.connect(go_next)

	calques = Calques.new()
	calques.position = Vector2(10, 96)
	add_child(calques)
	calques.layer_changed.connect(func(l):
		App.globe.set_layer(l)
		if not l.is_empty():
			narrateur.tool_opened("calques"))
	calques.raw_toggled.connect(func(on): App.globe.set_tiles(on))

	inspecteur = Inspecteur.new()
	inspecteur.set_anchors_preset(Control.PRESET_TOP_RIGHT)
	inspecteur.offset_left = -392
	inspecteur.offset_right = -10
	inspecteur.offset_top = 96
	inspecteur.grow_horizontal = Control.GROW_DIRECTION_BEGIN
	add_child(inspecteur)
	inspecteur.species_requested.connect(open_species)
	inspecteur.tool_requested.connect(open_tool)

	frise = Frise.new()
	add_child(frise)
	frise.set_anchors_and_offsets_preset(Control.PRESET_BOTTOM_WIDE, Control.PRESET_MODE_MINSIZE, 10)
	frise.event_clicked.connect(go_to_event)
	for m in [["chronicle", "C", open_chronicle], ["tree", "T", open_tree], ["interventions", "I", open_interventions], ["with_without", "A", open_with_without], ["save", "F5", open_save], ["settings", "", open_settings], ["menu", "", back_to_menu]]:
		var b := Atlas.button(App.t(m[0]), m[2], App.t(m[0]) + ("" if m[1] == "" else " (%s)" % m[1]))
		menu_buttons[m[0]] = b
		b.add_theme_font_size_override("font_size", int(17 * App.text_scale()))
		frise.menu.add_child(b)

	alertes = Alertes.new()
	alertes.set_anchors_preset(Control.PRESET_BOTTOM_LEFT)
	alertes.offset_left = 254
	alertes.offset_bottom = -150
	alertes.grow_vertical = Control.GROW_DIRECTION_BEGIN
	add_child(alertes)
	alertes.go_to.connect(go_to_event)

	narrateur = Narrateur.new()
	narrateur.set_anchors_preset(Control.PRESET_CENTER_BOTTOM)
	narrateur.offset_bottom = -150
	narrateur.offset_left = -310
	narrateur.grow_vertical = Control.GROW_DIRECTION_BEGIN
	narrateur.grow_horizontal = Control.GROW_DIRECTION_BOTH
	add_child(narrateur)
	narrateur.highlight.connect(_highlight)
	narrateur.go_to.connect(go_to_event)
	if params.get("guided", false):
		narrateur.restart()

	absence_card = _absence_card()
	add_child(absence_card)

	notice = Atlas.text("", 16, true)
	notice.set_anchors_preset(Control.PRESET_CENTER_TOP)
	notice.offset_top = 92
	notice.grow_horizontal = Control.GROW_DIRECTION_BOTH
	notice.autowrap_mode = TextServer.AUTOWRAP_OFF
	notice.add_theme_color_override("font_color", Atlas.VERMILION)
	add_child(notice)

	overlay = CenterContainer.new()
	overlay.set_anchors_preset(Control.PRESET_FULL_RECT)
	overlay.mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_child(overlay)

	App.session.set_rules_profile(App.settings["stop_profile"])
	App.settings_changed.connect(_settings_changed)

# ----------------------------------------------------------------------
# Narrateur : l'outil présenté est souligné d'un cadre vermillon.

func _highlight(tool: String) -> void:
	highlighted = tool
	if highlight_frame == null:
		highlight_frame = Panel.new()
		highlight_frame.mouse_filter = Control.MOUSE_FILTER_IGNORE
		var box := StyleBoxFlat.new()
		box.bg_color = Color(0, 0, 0, 0)
		box.border_color = Atlas.VERMILION
		box.set_border_width_all(3)
		box.set_corner_radius_all(4)
		highlight_frame.add_theme_stylebox_override("panel", box)
		add_child(highlight_frame)
	highlight_frame.visible = _highlight_target() != null

## Le contrôle qui porte l'outil présenté.
func _highlight_target() -> Control:
	match highlighted:
		"temps", "oxygene":
			return bar
		"frise":
			return frise
		"calques":
			return calques
		"inspecteur", "loupe", "reseau", "strates", "sol", "fiche", "anatomie":
			return inspecteur if inspecteur.visible else null
		"arbre":
			return menu_buttons.get("tree")
		"chronique":
			return menu_buttons.get("chronicle")
		"interventions":
			return menu_buttons.get("interventions")
		"avec_sans":
			return menu_buttons.get("with_without")
	return null

func _place_highlight() -> void:
	if highlight_frame == null or highlighted == "":
		if highlight_frame:
			highlight_frame.visible = false
		return
	var t := _highlight_target()
	if t == null or not t.is_visible_in_tree():
		highlight_frame.visible = false
		return
	var r := t.get_global_rect().grow(5.0)
	highlight_frame.visible = true
	highlight_frame.global_position = r.position
	highlight_frame.size = r.size
	# Une pulsation lente, discrète.
	var k := 0.55 + 0.45 * sin(Time.get_ticks_msec() / 260.0)
	highlight_frame.modulate.a = 1.0 if bool(App.settings["reduce_motion"]) else k

# ----------------------------------------------------------------------
# « Pendant votre absence » : après une longue avance sans que le joueur
# touche à rien, un résumé de ce qui a changé l'attend à son retour.

## Secondes sans geste du joueur avant de le tenir pour absent.
const ABSENT_AFTER := 45.0

func _absence_card() -> PanelContainer:
	var p := Atlas.panel()
	p.set_anchors_preset(Control.PRESET_CENTER_TOP)
	p.offset_top = 120
	p.offset_left = -330
	p.grow_horizontal = Control.GROW_DIRECTION_BOTH
	var v := VBoxContainer.new()
	p.add_child(v)
	v.add_child(Atlas.title(App.t("while_away"), 22))
	absence_label = Atlas.text("", 16)
	absence_label.custom_minimum_size.x = 620
	v.add_child(absence_label)
	var h := HBoxContainer.new()
	v.add_child(h)
	h.add_child(Atlas.button(App.t("go_see"), func():
		if not absence_events.is_empty():
			go_to_event(absence_events[0])
		absence_card.visible = false))
	h.add_child(Atlas.button(App.t("chronicle"), func():
		absence_card.visible = false
		open_chronicle()))
	h.add_child(Atlas.button("×", func(): absence_card.visible = false, App.t("close")))
	p.visible = false
	return p

func _watch_absence(delta: float) -> void:
	idle += delta
	if looked_years < 0.0:
		looked_years = float(info["years"])

func _input(event: InputEvent) -> void:
	if not (event is InputEventMouseButton or event is InputEventKey) or not event.is_pressed():
		return
	var back := idle >= ABSENT_AFTER
	idle = 0.0
	if info.is_empty():
		return
	var now := float(info["years"])
	if back and looked_years >= 0.0 and now > looked_years:
		var d: Dictionary = App.session.absence(looked_years)
		if not d.is_empty():
			absence_label.text = d["text"]
			absence_events = d["events"]
			absence_card.visible = true
	looked_years = now

func _settings_changed() -> void:
	# Les textes changent de langue ou de taille : on reconstruit l'écran.
	App.goto("jeu", {"new": false})

# ----------------------------------------------------------------------
# Boucle

func _process(delta: float) -> void:
	App.globe.refresh_frame()
	info = App.session.frame_info()
	bar.update_info(info, seeking)
	_handle_events()
	var n: String = App.session.take_notice()
	if n.begins_with("ok"):
		_flash(App.t("saved"))
	elif n != "":
		_flash(n.replace("\t", " "))
	if not info.is_empty() and not bool(info["paused"]):
		var minutes := int(App.settings["autosave_minutes"])
		autosave_clock += delta
		if minutes > 0 and autosave_clock >= minutes * 60.0:
			autosave_clock = 0.0
			App.session.save(App.save_path(App.t("autosave_name")), App.t("autosave_name"))
	if not info.is_empty():
		_watch_absence(delta)
	_place_highlight()

func _handle_events() -> void:
	var events: Array = App.session.poll_events()
	var slow := false
	for e in events:
		recent.append(e)
		if recent.size() > 300:
			recent.pop_front()
		var level := int(e["level"])
		var action := int(e["action"])
		if level >= 1:
			frise.add_mark(e)
			narrateur.consider(e)
		if bool(e.get("refused", false)):
			alertes.push(e, true)
			_flash(App.t("refused"))
		elif action == 2 or action == 3:
			alertes.push(e)
		if action == 3:
			slow = true
		if seeking and level >= 1 and not bool(e["player"]) and e["family"] != "intervention":
			_end_seek(e)
	if slow and not seeking:
		var s := App.session.speed()
		if s > 1.0e3:
			bar.set_speed(s / 10.0)
	var paused_by := App.session.take_auto_pause()
	if paused_by >= 0:
		for e in recent:
			if int(e["id"]) == paused_by:
				alertes.push(e, true)
				_flash(App.t("auto_paused"))
				if seeking:
					_end_seek(e)
				break

func _flash(text: String) -> void:
	notice.text = text
	var tw := notice.create_tween()
	notice.modulate.a = 1.0
	tw.tween_interval(3.0)
	tw.tween_property(notice, "modulate:a", 0.0, 0.6)

# ----------------------------------------------------------------------
# Aller au prochain événement

func go_next() -> void:
	if seeking:
		return
	seeking = true
	speed_before_seek = App.session.speed()
	App.session.set_speed(1.0e7)
	App.session.resume()

func _end_seek(e: Dictionary) -> void:
	seeking = false
	App.session.pause()
	App.session.set_speed(speed_before_seek)
	alertes.push(e, true)
	go_to_event(e)

func go_to_event(e: Dictionary) -> void:
	var cell := int(e.get("cell", -1))
	if cell >= 0:
		App.globe.go_to_cell(cell)
		inspecteur.show_cell(cell)
	var species := int(e.get("species", -1))
	if species >= 0:
		open_species(species)

func _on_cell_selected(cell: int) -> void:
	inspecteur.show_cell(cell)
	narrateur.cell_selected = cell >= 0
	narrateur.tool_opened("inspecteur")

# ----------------------------------------------------------------------
# Fiches

func _open(f: Control) -> Control:
	if fiche and is_instance_valid(fiche):
		fiche.queue_free()
	fiche = f
	overlay.add_child(f)
	return f

func open_species(species: int) -> void:
	narrateur.tool_opened("fiche")
	var f = FicheEspece.new()
	_open(f)
	f.open(species)
	f.species_requested.connect(open_species)
	f.anatomy_requested.connect(open_anatomy)
	f.compare_requested.connect(open_comparator)

func open_anatomy(species: int) -> void:
	narrateur.tool_opened("anatomie")
	var f = _open(Anatomie.new())
	f.open(species)
	f.compare_requested.connect(open_comparator)

func open_comparator(species: int, other := -1) -> void:
	var f = _open(Comparateur.new())
	f.open(species, other)
	f.species_requested.connect(open_species)

func open_tree() -> void:
	narrateur.tool_opened("arbre")
	var f = _open(Arbre.new())
	f.species_requested.connect(open_species)

func open_chronicle() -> void:
	narrateur.tool_opened("chronique")
	var f = _open(Chronique.new())
	f.go_to.connect(func(e):
		f.close()
		go_to_event(e))

func open_interventions() -> void:
	narrateur.tool_opened("interventions")
	var f = _open(Interventions.new())
	f.intervened.connect(func(_k, _m): _flash(App.t("sent")))

func open_with_without() -> void:
	narrateur.tool_opened("avec_sans")
	_open(AvecSans.new())

## Outils d'une cellule ouverts depuis l'inspecteur.
func open_tool(tool: String, cell: int) -> void:
	narrateur.tool_opened(tool)
	if tool == "sol":
		descend(cell)
	elif tool == "reseau":
		var f = _open(Reseau.new())
		f.open(cell)
		f.species_requested.connect(open_species)
	else:
		var f = _open(Strates.new())
		f.open(cell)

## Descente au sol : le globe plonge vers la cellule, puis la scène locale
## se pose par-dessus ; « Remonter » rend le globe.
func descend(cell: int) -> void:
	if cell < 0 or sol != null:
		return
	if fiche and is_instance_valid(fiche):
		fiche.queue_free()
	var g = App.session.open_ground(cell, 100000)
	if g == null:
		return
	App.globe.go_to_cell(cell, 1.03)
	var s = Sol.new()
	s.modulate.a = 0.0
	add_child(s)
	s.open(g)
	s.closed.connect(ascend)
	sol = s
	var tw := create_tween()
	tw.tween_interval(0.0 if bool(App.settings["reduce_motion"]) else 0.9)
	tw.tween_property(s, "modulate:a", 1.0, 0.0 if bool(App.settings["reduce_motion"]) else 0.6)

func ascend() -> void:
	if sol == null:
		return
	var s := sol
	sol = null
	App.globe.target_distance = 1.4
	var tw := create_tween()
	tw.tween_property(s, "modulate:a", 0.0, 0.0 if bool(App.settings["reduce_motion"]) else 0.4)
	tw.tween_callback(s.queue_free)

func open_save() -> void:
	_open(Sauvegarde.new())

func open_settings() -> void:
	_open(Reglages.new())

func back_to_menu() -> void:
	App.session.save(App.save_path(App.t("autosave_name")), App.t("autosave_name"))
	App.session.stop()
	App.goto("accueil")

func _unhandled_key_input(event: InputEvent) -> void:
	if not event.pressed or event.echo:
		return
	var k := event as InputEventKey
	if k.alt_pressed and k.keycode >= KEY_0 and k.keycode <= KEY_9:
		var i := k.keycode - KEY_1
		if k.keycode == KEY_0:
			calques.buttons[0].button_pressed = true
			calques._select(-1)
		else:
			calques.select_index(i)
		get_viewport().set_input_as_handled()
		return
	match k.keycode:
		KEY_SPACE:
			bar.toggle_pause()
		KEY_1, KEY_2, KEY_3, KEY_4, KEY_5:
			bar.speed_index(k.keycode - KEY_1)
		KEY_N:
			go_next()
		KEY_C:
			open_chronicle()
		KEY_T:
			open_tree()
		KEY_I:
			open_interventions()
		KEY_A:
			open_with_without()
		KEY_F5:
			open_save()
		_:
			return
	get_viewport().set_input_as_handled()
