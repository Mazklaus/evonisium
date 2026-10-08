extends PanelContainer
## Barre du temps (document Fonctionnalités, « Contrôle du temps ») : date
## de la partie, pause et lecture, crans de vitesse, « aller au prochain »,
## vitesse réellement tenue, et les grandeurs globales en clair.

signal next_requested
signal panel_requested(name: String)

const SPEEDS := [1.0e3, 1.0e4, 1.0e5, 1.0e6, 1.0e7]

var date_label: Label
var state_label: Label
var play_button: Button
var speed_buttons: Array = []
var real_label: Label
var o2_label: Label
var temp_label: Label
var bio_label: Label
var lin_label: Label
var inf_label: Label
var paused := true

func _ready() -> void:
	var h := HBoxContainer.new()
	h.add_theme_constant_override("separation", 10)
	add_child(h)
	var dv := VBoxContainer.new()
	dv.add_theme_constant_override("separation", 0)
	date_label = Atlas.title("", 26)
	date_label.custom_minimum_size.x = 250
	dv.add_child(date_label)
	state_label = Atlas.text("", 15, true)
	state_label.autowrap_mode = TextServer.AUTOWRAP_OFF
	dv.add_child(state_label)
	h.add_child(dv)
	h.add_child(VSeparator.new())
	play_button = Atlas.button("", toggle_pause, App.t("pause") + " (Espace)")
	play_button.custom_minimum_size.x = 96
	h.add_child(play_button)
	var group := ButtonGroup.new()
	for i in SPEEDS.size():
		var b := Atlas.button(App.session.format_speed(SPEEDS[i]), set_speed.bind(SPEEDS[i]), "%s (%d)" % [App.t("speed"), i + 1])
		b.toggle_mode = true
		b.button_group = group
		speed_buttons.append(b)
		h.add_child(b)
	h.add_child(Atlas.button(App.t("next_event") + " ›", func(): next_requested.emit(), App.t("next_event") + " (N)"))
	real_label = Atlas.text("", 15, true)
	real_label.autowrap_mode = TextServer.AUTOWRAP_OFF
	real_label.custom_minimum_size.x = 110
	real_label.tooltip_text = App.t("real_speed_long")
	real_label.mouse_filter = Control.MOUSE_FILTER_PASS
	h.add_child(real_label)
	h.add_child(VSeparator.new())
	o2_label = _indicator(h, App.t("oxygen"))
	temp_label = _indicator(h, App.t("temperature"))
	bio_label = _indicator(h, App.t("biomass"))
	lin_label = _indicator(h, App.t("lineages_short"))
	inf_label = _indicator(h, App.t("influence_short"))
	var spacer := Control.new()
	spacer.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	h.add_child(spacer)
	_sync_speed()

func _indicator(parent: Control, label: String) -> Label:
	var v := VBoxContainer.new()
	v.add_theme_constant_override("separation", -2)
	var l := Atlas.text(label, 14, true)
	l.autowrap_mode = TextServer.AUTOWRAP_OFF
	v.add_child(l)
	var value := Atlas.text("—", 19)
	value.autowrap_mode = TextServer.AUTOWRAP_OFF
	value.custom_minimum_size.x = 76
	v.add_child(value)
	parent.add_child(v)
	return value

func _sync_speed() -> void:
	var s := App.session.speed()
	for i in SPEEDS.size():
		speed_buttons[i].set_pressed_no_signal(is_equal_approx(SPEEDS[i], s))

func set_speed(s: float) -> void:
	App.session.set_speed(s)
	_sync_speed()
	if paused:
		App.session.resume()

func speed_index(i: int) -> void:
	if i >= 0 and i < SPEEDS.size():
		set_speed(SPEEDS[i])

func toggle_pause() -> void:
	if paused:
		App.session.resume()
	else:
		App.session.pause()

func update_info(info: Dictionary, seeking: bool) -> void:
	if info.is_empty():
		return
	paused = bool(info["paused"])
	date_label.text = info["date"]
	var state: String = App.t("seeking") if seeking else (App.t("paused") if paused else App.t("climate") + " : " + str(info["climate"]))
	state_label.text = state
	play_button.text = "▶ " + App.t("play") if paused else "❚❚ " + App.t("pause")
	var real: float = float(info["real_speed"])
	real_label.text = "" if paused else "%s %s" % [App.t("real_speed"), info["real_speed_text"]]
	# La vitesse tenue est signalée quand elle reste loin de la demande.
	var short: bool = not paused and real > 0.0 and real < App.session.speed() * 0.7
	real_label.add_theme_color_override("font_color", Atlas.VERMILION if short else Atlas.INK)
	o2_label.text = info["o2_text"]
	temp_label.text = info["temperature_text"]
	bio_label.text = info["biomass_text"]
	lin_label.text = str(info["lineages"])
	inf_label.text = "∞" if bool(info.get("sandbox", false)) else "%d / %d" % [int(info.get("influence", 0.0)), int(info.get("influence_max", 0.0))]
	_sync_speed()
