extends PanelContainer
## Inspecteur de cellule (document Fonctionnalités, « Inspecteur ») : le
## milieu en clair, les populations présentes avec leur part de biomasse, et
## la loupe sur la vie microscopique de la cellule.

signal species_requested(species: int)
## Réseau trophique ou colonne stratigraphique de la cellule (étape 4).
signal tool_requested(tool: String, cell: int)

var cell := -1
var body: VBoxContainer
var title_label: Label
var loupe: TextureRect
var loupe_key := ""
var loupe_caption: Label
var refresh_timer := 0.0

func _ready() -> void:
	custom_minimum_size = Vector2(380, 0)
	var v := VBoxContainer.new()
	add_child(v)
	var h := HBoxContainer.new()
	v.add_child(h)
	title_label = Atlas.title(App.t("inspector"), 24)
	title_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	h.add_child(title_label)
	h.add_child(Atlas.button("×", func(): show_cell(-1), App.t("close")))
	v.add_child(Atlas.hsep())
	var scroll := ScrollContainer.new()
	scroll.custom_minimum_size = Vector2(360, 480)
	scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	v.add_child(scroll)
	body = VBoxContainer.new()
	body.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	scroll.add_child(body)
	visible = false

func show_cell(c: int) -> void:
	cell = c
	visible = c >= 0
	loupe = null
	loupe_key = ""
	if c >= 0:
		_fill()
	else:
		App.globe.select_cell(-1)

func _process(delta: float) -> void:
	if not visible:
		return
	if loupe_key != "" and loupe:
		var img: Image = App.session.take_image(loupe_key)
		if img:
			loupe.texture = ImageTexture.create_from_image(img)
			loupe_caption.text = "%s — %s %s µm" % [App.t("microscope"), "barre d'échelle :" if App.settings["lang"] == "fr" else "scale bar:", str(int(App.session.last_scale_bar_um()))]
			loupe_key = ""
	refresh_timer -= delta
	if refresh_timer <= 0.0:
		refresh_timer = 2.0
		if cell >= 0 and loupe_key == "":
			_fill(false)

func _fill(with_loupe: bool = true) -> void:
	var d: Dictionary = App.session.cell_info(cell)
	if d.is_empty():
		return
	# Le détail du moteur arrive un peu après le reste : on repasse vite.
	var pending := bool(d.get("pending", false))
	if pending:
		refresh_timer = 0.2
	var keep_loupe := loupe if (not with_loupe and (pending or not d["populations"].is_empty())) else null
	for c in body.get_children():
		if keep_loupe and (c == keep_loupe or c == loupe_caption):
			body.remove_child(c)
			continue
		c.queue_free()
	title_label.text = "%s" % d["region"]
	var place := Atlas.text("%s — %s %d" % [d["place"], App.t("cell"), cell], 15, true)
	place.custom_minimum_size.x = 320
	body.add_child(place)
	_field(App.t("habitat"), str(d["medium"]).capitalize() if App.settings["lang"] == "en" else str(d["medium"]))
	_field("", d["height"])
	_field(App.t("temperature"), d["temperature"])
	_field("Lumière" if App.settings["lang"] == "fr" else "Light", d["light"])
	_field(App.t("oxygen"), d["oxygen"])
	_field("pH", d.get("ph", "…"))
	if bool(d["is_ocean"]):
		_field("Salinité" if App.settings["lang"] == "fr" else "Salinity", d.get("salinity", "…"))
	else:
		_field("Pluie" if App.settings["lang"] == "fr" else "Rain", d["rain"])
	_field(App.t("biomass"), d["biomass"])
	if bool(d.get("vent", false)):
		var vl := Atlas.text(("Source hydrothermale : " if App.settings["lang"] == "fr" else "Hydrothermal vent: ") + str(d.get("vent_text", "")), 16, true)
		vl.custom_minimum_size.x = 320
		vl.add_theme_color_override("font_color", Atlas.VERMILION)
		body.add_child(vl)
	body.add_child(Atlas.hsep())
	body.add_child(Atlas.title(App.t("populations"), 20))
	var pops: Array = d["populations"]
	if pops.is_empty():
		body.add_child(Atlas.text("…" if pending else App.t("none"), 16, true))
	for p in pops:
		var row := HBoxContainer.new()
		var sw := ColorRect.new()
		sw.color = p["colour"]
		sw.custom_minimum_size = Vector2(12, 12)
		sw.size_flags_vertical = Control.SIZE_SHRINK_CENTER
		row.add_child(sw)
		var b := Atlas.button(str(p["name"]), species_requested.emit.bind(int(p["species"])), str(p["guild"]))
		b.alignment = HORIZONTAL_ALIGNMENT_LEFT
		b.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		b.clip_text = true
		b.add_theme_font_size_override("font_size", int(16 * App.text_scale()))
		row.add_child(b)
		var share := Atlas.text("%d %%" % int(round(float(p["share"]) * 100.0)), 16)
		share.autowrap_mode = TextServer.AUTOWRAP_OFF
		share.custom_minimum_size.x = 48
		row.add_child(share)
		body.add_child(row)
	var tools := HBoxContainer.new()
	body.add_child(tools)
	tools.add_child(Atlas.button(App.t("food_web"), tool_requested.emit.bind("reseau", cell)))
	tools.add_child(Atlas.button(App.t("strata"), tool_requested.emit.bind("strates", cell)))
	tools.add_child(Atlas.button(App.t("ground_down"), tool_requested.emit.bind("sol", cell)))
	if keep_loupe:
		body.add_child(Atlas.hsep())
		body.add_child(loupe_caption)
		body.add_child(keep_loupe)
	elif not pops.is_empty() and with_loupe:
		body.add_child(Atlas.hsep())
		loupe_caption = Atlas.text(App.t("microscope"), 15, true)
		loupe_caption.custom_minimum_size.x = 320
		body.add_child(loupe_caption)
		loupe = TextureRect.new()
		loupe.custom_minimum_size = Vector2(320, 320)
		loupe.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
		loupe.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_CENTERED
		body.add_child(loupe)
		loupe_key = App.session.request_microscope(cell, 512, 512)

func _field(label: String, value: String) -> void:
	var h := HBoxContainer.new()
	var l := Atlas.text(label, 15, true)
	l.autowrap_mode = TextServer.AUTOWRAP_OFF
	l.custom_minimum_size.x = 120
	h.add_child(l)
	var v := Atlas.text(value, 17)
	v.custom_minimum_size.x = 190
	h.add_child(v)
	body.add_child(h)
