extends "res://scripts/ui/fiche.gd"
## Fiche d'espèce simple (document Fonctionnalités, « Fiche d'espèce » ; DA
## Atlas) : la planche de l'organisme posée devant le décor de son milieu de
## vie, tiré de l'environnement simulé, puis les traits en clair.

signal species_requested(species: int)

var species := -1
var decor: TextureRect
var figure: TextureRect
var decor_key := ""
var figure_key := ""
var bar_label: Label
var info := {}
var follow_button: Button
var range_button: Button

func open(l: int) -> void:
	species = l
	custom_minimum_size = Vector2(800, 0)
	info = App.session.species_info(l)
	if info.is_empty():
		set_title(App.t("species"))
		content.add_child(Atlas.text(App.t("no_species"), 18, true))
		return
	set_title(info["common"])
	var sci := Atlas.title(info["scientific"], 22)
	sci.add_theme_font_override("font", Atlas.title_italic)
	content.add_child(sci)
	# Planche : décor de milieu en bandeau, figure en médaillon.
	var plate := Control.new()
	plate.custom_minimum_size = Vector2(760, 300)
	content.add_child(plate)
	decor = TextureRect.new()
	decor.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
	decor.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_COVERED
	decor.position = Vector2.ZERO
	decor.size = Vector2(760, 300)
	plate.add_child(decor)
	var frame := Panel.new()
	frame.position = Vector2(18, 26)
	frame.size = Vector2(250, 250)
	plate.add_child(frame)
	figure = TextureRect.new()
	figure.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
	figure.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_CENTERED
	figure.position = Vector2(26, 34)
	figure.size = Vector2(234, 234)
	plate.add_child(figure)
	bar_label = Atlas.text("", 14, true)
	bar_label.autowrap_mode = TextServer.AUTOWRAP_OFF
	bar_label.position = Vector2(30, 278)
	plate.add_child(bar_label)
	decor_key = App.session.request_decor(l, 1024, 404)
	figure_key = App.session.request_figure(l, 480, 480)

	var cols := HBoxContainer.new()
	cols.add_theme_constant_override("separation", 24)
	content.add_child(cols)
	var a := VBoxContainer.new()
	a.custom_minimum_size.x = 370
	cols.add_child(a)
	var b := VBoxContainer.new()
	b.custom_minimum_size.x = 370
	cols.add_child(b)
	field(a, App.t("status"), info["status"])
	field(a, App.t("metabolism"), ", ".join(info["metabolisms"]))
	field(a, "Pigment", info["pigment"])
	field(a, App.t("range"), "%s (%d)" % [info["range_share"], int(info["range_cells"])])
	field(b, App.t("biomass"), info["biomass"])
	field(b, App.t("age"), info["age"])
	field(b, App.t("origin"), info["origin_region"])
	field(b, App.t("lineages"), "%d · %d %s" % [int(info["lineages"]), int(info["ecotypes"]), App.t("ecotypes")])
	if int(info["parent"]) >= 0:
		var h := HBoxContainer.new()
		var lab := Atlas.text(App.t("ancestor"), 17, true)
		lab.custom_minimum_size.x = 150
		lab.autowrap_mode = TextServer.AUTOWRAP_OFF
		h.add_child(lab)
		var pb := Atlas.button(info["parent_name"], species_requested.emit.bind(int(info["parent"])))
		pb.clip_text = true
		pb.custom_minimum_size.x = 200
		h.add_child(pb)
		b.add_child(h)
	content.add_child(Atlas.hsep())
	var actions := HBoxContainer.new()
	content.add_child(actions)
	follow_button = Atlas.button(App.t("followed") if bool(info["marked"]) else App.t("follow"), _follow)
	follow_button.disabled = bool(info["marked"]) or int(info["founder"]) < 0
	actions.add_child(follow_button)
	range_button = Atlas.button(App.t("show_range"), _show_range)
	range_button.toggle_mode = true
	range_button.button_pressed = App.session.focus() == l
	actions.add_child(range_button)
	if bool(info["marked"]):
		var m := Atlas.text(App.t("marked"), 15, true)
		m.custom_minimum_size.x = 380
		actions.add_child(m)

func _follow() -> void:
	App.session.mark_lineage(int(info["founder"]))
	follow_button.text = App.t("followed")
	follow_button.disabled = true

func _show_range() -> void:
	App.session.set_focus(species if range_button.button_pressed else -1)
	App.globe.refresh_frame(true)
	if range_button.button_pressed:
		App.globe.target_distance = max(App.globe.target_distance, 4.0)

func _process(_delta: float) -> void:
	if decor_key != "":
		var img: Image = App.session.take_image(decor_key)
		if img:
			decor.texture = ImageTexture.create_from_image(img)
			decor_key = ""
	if figure_key != "":
		var img: Image = App.session.take_image(figure_key)
		if img:
			figure.texture = ImageTexture.create_from_image(img)
			var um := App.session.last_scale_bar_um()
			bar_label.text = ("barre d'échelle : %s µm" if App.settings["lang"] == "fr" else "scale bar: %s µm") % _num(um)
			figure_key = ""

func _num(x: float) -> String:
	if x >= 1.0:
		return str(int(round(x)))
	return ("%.1f" % x).replace(".", "," if App.settings["lang"] == "fr" else ".")
