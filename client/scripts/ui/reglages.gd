extends PanelContainer
## Réglages et accessibilité de base (document Fonctionnalités, « Réglages »
## et « Accessibilité ») : langue, taille de l'interface et du texte, vision
## des couleurs, police très lisible, unités, mouvements réduits, sauvegarde
## automatique, rendu du globe, narrateur.

var draft := {}

func _ready() -> void:
	draft = App.settings.duplicate()
	custom_minimum_size = Vector2(560, 0)
	var scroll := ScrollContainer.new()
	scroll.custom_minimum_size = Vector2(540, 620)
	scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	add_child(scroll)
	var v := VBoxContainer.new()
	v.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	scroll.add_child(v)
	v.add_child(Atlas.title(App.t("settings"), 32))
	v.add_child(Atlas.hsep())

	_option(v, "lang", ["Français", "English"], ["fr", "en"])
	_slider(v, "ui_scale", 0.75, 1.75, 0.05)
	_slider(v, "text_scale", 0.8, 1.8, 0.05)
	var modes := []
	for m in App.VISION_MODES:
		modes.append(m[App.lang_index()])
	_option(v, "vision", modes, [0, 1, 2, 3, 4])
	_check(v, "readable_font")
	_check(v, "celsius")
	_check(v, "reduce_motion")
	_slider(v, "autosave", 0, 60, 5, "autosave_minutes")
	v.add_child(Atlas.hsep())
	_slider(v, "relief", 0.0, 3.0, 0.1)
	_check(v, "graticule")
	_check(v, "terminator")
	_check(v, "narrator")
	_option(v, "profile", [App.t("profile_contemplatif"), App.t("profile_naturaliste"), App.t("profile_tout")], ["contemplatif", "naturaliste", "tout"], "stop_profile")
	v.add_child(Atlas.hsep())
	var h := HBoxContainer.new()
	v.add_child(h)
	h.add_child(Atlas.button(App.t("accept"), _apply))
	h.add_child(Atlas.button(App.t("close"), queue_free))

func _row(parent: Control, key: String) -> HBoxContainer:
	var h := HBoxContainer.new()
	var l := Atlas.text(App.t(key))
	l.custom_minimum_size.x = 250
	l.autowrap_mode = TextServer.AUTOWRAP_OFF
	h.add_child(l)
	parent.add_child(h)
	return h

func _option(parent: Control, key: String, labels: Array, values: Array, setting: String = "") -> void:
	var s := setting if setting != "" else key
	var h := _row(parent, key)
	var o := OptionButton.new()
	for i in labels.size():
		o.add_item(labels[i], i)
	o.selected = max(0, values.find(draft[s]))
	o.item_selected.connect(func(i): draft[s] = values[i])
	o.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	h.add_child(o)

func _slider(parent: Control, key: String, lo: float, hi: float, step: float, setting: String = "") -> void:
	var s := setting if setting != "" else key
	var h := _row(parent, key)
	var sl := HSlider.new()
	sl.min_value = lo
	sl.max_value = hi
	sl.step = step
	sl.value = float(draft[s])
	sl.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	sl.custom_minimum_size.x = 160
	var value := Atlas.text(_fmt(sl.value))
	value.custom_minimum_size.x = 50
	value.autowrap_mode = TextServer.AUTOWRAP_OFF
	sl.value_changed.connect(func(x):
		draft[s] = int(x) if step >= 1.0 else x
		value.text = _fmt(x))
	h.add_child(sl)
	h.add_child(value)

func _fmt(x: float) -> String:
	if is_equal_approx(x, round(x)):
		return str(int(x))
	return ("%.2f" % x).replace(".", "," if App.settings["lang"] == "fr" else ".")

func _check(parent: Control, key: String) -> void:
	var c := CheckBox.new()
	c.text = App.t(key)
	c.button_pressed = bool(draft[key])
	c.toggled.connect(func(on): draft[key] = on)
	parent.add_child(c)

func _apply() -> void:
	for k in draft.keys():
		App.settings[k] = draft[k]
	App.save_settings()
	App.apply_settings()
	queue_free()
