extends Control
## Création de la planète : préréglage, graine, distance à l'étoile, part
## d'eau, étoile, taille de grille. Chaque réglage reforme la planète, vue en
## direct sur le globe. Le code de la planète la recrée à l'identique.

const PRESETS := ["terre", "ocean", "desert", "super-terre", "petite", "sans-lune"]
const LEVELS := [4, 5, 6]

var spec := {"preset": "terre", "seed": 0, "orbit": 1.0, "water": 1.0, "star_k": 0.0, "level": 5}
var info_label: RichTextLabel
var code_edit: LineEdit
var status_label: Label
var seed_edit: LineEdit
var star_slider: HSlider
var star_check: CheckBox
var orbit_slider: HSlider
var water_slider: HSlider
var preset_option: OptionButton
var level_option: OptionButton
var debounce := -1.0
var waiting := false
var updating := false

func setup(_params: Dictionary) -> void:
	spec["seed"] = randi() % 1000000
	spec["level"] = int(App.settings["level"])
	var g = App.globe
	g.interactive = true
	g.set_layer({})
	g.set_show_vents(false)
	g.target_distance = 4.4
	g.view_offset = 0.8
	_build()
	_regenerate()

func _build() -> void:
	var p := Atlas.panel()
	p.position = Vector2(40, 40)
	p.custom_minimum_size = Vector2(470, 0)
	add_child(p)
	var v := VBoxContainer.new()
	v.add_theme_constant_override("separation", 9)
	p.add_child(v)
	v.add_child(Atlas.title(App.t("create_title"), 36))
	v.add_child(Atlas.hsep())

	preset_option = OptionButton.new()
	for k in PRESETS:
		preset_option.add_item(App.t("preset_" + k))
	preset_option.item_selected.connect(func(i):
		spec["preset"] = PRESETS[i]
		_changed())
	_row(v, App.t("preset"), preset_option)

	var sh := HBoxContainer.new()
	seed_edit = LineEdit.new()
	seed_edit.text = str(spec["seed"])
	seed_edit.custom_minimum_size.x = 120
	seed_edit.text_submitted.connect(func(t):
		spec["seed"] = int(t)
		_changed())
	sh.add_child(seed_edit)
	sh.add_child(Atlas.button(App.t("random"), func():
		spec["seed"] = randi() % 1000000
		seed_edit.text = str(spec["seed"])
		_changed()))
	_row(v, App.t("seed"), sh)

	orbit_slider = _slider(0.8, 1.25, 0.01, spec["orbit"], func(x):
		spec["orbit"] = x
		_changed())
	_row(v, App.t("orbit"), orbit_slider)
	water_slider = _slider(0.2, 3.0, 0.05, spec["water"], func(x):
		spec["water"] = x
		_changed())
	_row(v, App.t("water"), water_slider)

	var stb := VBoxContainer.new()
	star_check = CheckBox.new()
	star_check.text = App.t("preset_star")
	star_check.button_pressed = true
	star_slider = _slider(2600.0, 7500.0, 50.0, 5772.0, func(x):
		if not star_check.button_pressed:
			spec["star_k"] = x
			_changed())
	star_slider.editable = false
	star_check.toggled.connect(func(on):
		star_slider.editable = not on
		spec["star_k"] = 0.0 if on else star_slider.value
		_changed())
	stb.add_child(star_check)
	stb.add_child(star_slider)
	_row(v, App.t("star"), stb)

	level_option = OptionButton.new()
	for l in LEVELS:
		level_option.add_item(App.t("level_%d" % l))
	level_option.selected = max(0, LEVELS.find(spec["level"]))
	level_option.item_selected.connect(func(i):
		spec["level"] = LEVELS[i]
		_changed())
	_row(v, App.t("grid"), level_option)

	v.add_child(Atlas.hsep())
	code_edit = LineEdit.new()
	code_edit.tooltip_text = App.t("code_hint")
	code_edit.text_submitted.connect(_apply_code)
	_row(v, App.t("code"), code_edit)
	status_label = Atlas.text("", 17, true)
	status_label.custom_minimum_size.x = 430
	v.add_child(status_label)
	info_label = RichTextLabel.new()
	info_label.fit_content = true
	info_label.bbcode_enabled = true
	info_label.custom_minimum_size = Vector2(430, 0)
	v.add_child(info_label)
	v.add_child(Atlas.hsep())
	var h := HBoxContainer.new()
	v.add_child(h)
	h.add_child(Atlas.button(App.t("back"), func(): App.goto("accueil")))
	var go := Atlas.button(App.t("to_seeding"), _to_seeding)
	h.add_child(go)

func _row(parent: Control, label: String, control: Control) -> void:
	var h := HBoxContainer.new()
	var l := Atlas.text(label, 18)
	l.autowrap_mode = TextServer.AUTOWRAP_OFF
	l.custom_minimum_size.x = 170
	h.add_child(l)
	control.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	h.add_child(control)
	parent.add_child(h)

func _slider(lo: float, hi: float, step: float, value: float, cb: Callable) -> HSlider:
	var s := HSlider.new()
	s.min_value = lo
	s.max_value = hi
	s.step = step
	s.value = value
	s.custom_minimum_size.x = 200
	s.value_changed.connect(cb)
	return s

func _changed() -> void:
	if updating:
		return
	debounce = 0.6

func _regenerate() -> void:
	App.session.start(spec["preset"], int(spec["seed"]), int(spec["level"]), float(spec["orbit"]), float(spec["water"]), float(spec["star_k"]))
	App.globe.reset()
	waiting = true
	status_label.text = App.t("generating")
	info_label.text = ""
	code_edit.text = App.session.planet_code()

## Code « préréglage-graine-niveau-orbite-eau-étoile » : recrée la planète.
func _apply_code(code: String) -> void:
	var parts := code.strip_edges().rsplit("-", true, 5)
	if parts.size() < 6:
		return
	var preset := parts[0]
	if not preset in PRESETS:
		return
	updating = true
	spec["preset"] = preset
	spec["seed"] = int(parts[1])
	spec["level"] = int(parts[2])
	spec["orbit"] = float(parts[3])
	spec["water"] = float(parts[4])
	spec["star_k"] = float(parts[5])
	preset_option.selected = PRESETS.find(preset)
	seed_edit.text = parts[1]
	orbit_slider.value = spec["orbit"]
	water_slider.value = spec["water"]
	star_check.button_pressed = spec["star_k"] <= 0.0
	if spec["star_k"] > 0.0:
		star_slider.value = spec["star_k"]
	level_option.selected = max(0, LEVELS.find(spec["level"]))
	updating = false
	_regenerate()

func _process(delta: float) -> void:
	if debounce > 0.0:
		debounce -= delta
		if debounce <= 0.0:
			_regenerate()
	App.globe.refresh_frame()
	if waiting and App.session.has_frame():
		waiting = false
		status_label.text = ""
		_show_info()

func _show_info() -> void:
	var i: Dictionary = App.session.frame_info()
	var radius_km := float(i["radius_m"]) / 1000.0
	info_label.text = "[i]%s[/i]\n%s : %s km   %s : %s\n%s : %s   %s : %s %%\n%s : %s Pa" % [
		i["planet"],
		App.t("radius"), str(int(round(radius_km))), App.t("temperature"), i["temperature_text"],
		App.t("ocean_share"), str(int(round(float(i["ocean"]) * 100.0))) + " %", App.t("ice"), str(int(round(float(i["ice"]) * 100.0))),
		App.t("co2"), App.session.format_power(float(i["co2_pa"])),
	]

func _to_seeding() -> void:
	if not App.session.has_frame():
		return
	App.settings["level"] = spec["level"]
	App.save_settings()
	App.goto("ensemencement")
