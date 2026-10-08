extends PanelContainer
## Barre des calques (document Fonctionnalités, « Calques ») : vue naturelle
## ou une donnée à la fois en lavis, avec sa légende graduée ; bascule des
## cellules brutes.

signal layer_changed(layer: Dictionary)
signal raw_toggled(on: bool)

var layers: Array = []
var buttons: Array = []
var legend: Control
var legend_title: Label
var legend_box: VBoxContainer
var current := {}
var palette_img: Image
var guild_labels: VBoxContainer

func _ready() -> void:
	custom_minimum_size = Vector2(232, 0)
	var v := VBoxContainer.new()
	v.add_theme_constant_override("separation", 4)
	add_child(v)
	v.add_child(Atlas.title(App.t("layers"), 24))
	var group := ButtonGroup.new()
	var nat := Atlas.button(App.t("natural"), _select.bind(-1), App.t("layer_none") + " (Alt+0)")
	nat.toggle_mode = true
	nat.button_group = group
	nat.button_pressed = true
	nat.alignment = HORIZONTAL_ALIGNMENT_LEFT
	v.add_child(nat)
	buttons.append(nat)
	layers = App.session.layers()
	for i in layers.size():
		var l: Dictionary = layers[i]
		var b := Atlas.button(l["name"], _select.bind(i), "%s (Alt+%d)" % [l["name"], i + 1])
		b.toggle_mode = true
		b.button_group = group
		b.alignment = HORIZONTAL_ALIGNMENT_LEFT
		v.add_child(b)
		buttons.append(b)
	var raw := CheckBox.new()
	raw.text = App.t("raw_cells")
	raw.toggled.connect(func(on): raw_toggled.emit(on))
	v.add_child(raw)
	v.add_child(Atlas.hsep())
	legend_box = VBoxContainer.new()
	legend_box.add_theme_constant_override("separation", 2)
	v.add_child(legend_box)
	legend_title = Atlas.text("", 15, true)
	legend_title.custom_minimum_size.x = 210
	legend_box.add_child(legend_title)
	legend = Control.new()
	legend.custom_minimum_size = Vector2(210, 46)
	legend.draw.connect(_draw_legend)
	legend_box.add_child(legend)
	guild_labels = VBoxContainer.new()
	guild_labels.add_theme_constant_override("separation", 0)
	legend_box.add_child(guild_labels)
	legend_box.visible = false

func select_index(i: int) -> void:
	if i + 1 < buttons.size():
		buttons[i + 1].button_pressed = true
		_select(i)

func select_key(key: String) -> void:
	for i in layers.size():
		if layers[i]["key"] == key:
			select_index(i)
			return

func _select(i: int) -> void:
	current = {} if i < 0 else layers[i]
	legend_box.visible = not current.is_empty()
	if not current.is_empty():
		palette_img = App.session.palette_image(int(current["palette"]), int(App.settings["vision"]))
		legend_title.text = current["name"] + ("" if str(current["unit"]) == "" else " (" + str(current["unit"]) + ")")
		_fill_categories()
	legend.queue_redraw()
	layer_changed.emit(current)

func _fill_categories() -> void:
	for c in guild_labels.get_children():
		c.queue_free()
	if int(current.get("transform", 0)) != 3 or current.get("key", "") != "guildes":
		return
	var names: PackedStringArray = App.session.guild_legend()
	var pal := App.session.palette_image(2, int(App.settings["vision"]))
	for i in names.size():
		var h := HBoxContainer.new()
		var sw := ColorRect.new()
		sw.color = pal.get_pixel(i + 1, 0)
		sw.custom_minimum_size = Vector2(14, 14)
		h.add_child(sw)
		var name: String = names[i]
		if name.length() > 64:
			name = name.substr(0, 62) + "…"
		var l := Atlas.text(name, 14)
		l.custom_minimum_size.x = 180
		h.add_child(l)
		guild_labels.add_child(h)

func refresh_categories() -> void:
	if current.get("key", "") == "guildes":
		_fill_categories()

func _draw_legend() -> void:
	if current.is_empty() or palette_img == null:
		return
	var w := legend.size.x
	var transform := int(current["transform"])
	if transform == 3:
		legend.custom_minimum_size.y = 0
		return
	legend.custom_minimum_size.y = 46
	var steps := 64
	for k in steps:
		var t := float(k) / steps
		var c := palette_img.get_pixel(int(t * 255.0), 0)
		legend.draw_rect(Rect2(t * w, 0, w / steps + 1.0, 16), c)
	legend.draw_rect(Rect2(0, 0, w, 16), Atlas.INK, false, 1.0)
	var pos: PackedFloat32Array = current["tick_positions"]
	var labels: PackedStringArray = current["tick_labels"]
	var font := Atlas.body_font()
	var fs := int(13 * App.text_scale())
	for i in pos.size():
		var x: float = pos[i] * w
		legend.draw_line(Vector2(x, 16), Vector2(x, 21), Atlas.INK, 1.0)
		var s: String = labels[i]
		var sw := font.get_string_size(s, HORIZONTAL_ALIGNMENT_LEFT, -1, fs).x
		legend.draw_string(font, Vector2(clamp(x - sw / 2.0, 0.0, w - sw), 36), s, HORIZONTAL_ALIGNMENT_LEFT, -1, fs, Atlas.INK)
