extends "res://scripts/ui/fiche.gd"
## Interventions (document Fonctionnalités, « Interventions ») : le joueur
## agit sur l'environnement, jamais sur les gènes, autour de la cellule
## choisie sur le globe. Choix, ampleur, coût en influence, aperçu des effets
## attendus, confirmation ; un point de sauvegarde est écrit juste avant,
## pour pouvoir revenir en arrière.

signal intervened(kind: String, moles: float)

const KINDS := {
	"phosphate": {"min": 12.0, "max": 16.0, "default": 14.0, "unit": "mol P"},
	"eruption": {"min": 15.0, "max": 18.5, "default": 16.0, "unit": "mol CO₂"},
	"methane": {"min": 13.0, "max": 17.0, "default": 15.0, "unit": "mol CH₄"},
}

var kind := "phosphate"
var slider: HSlider
var amount_label: Label
var preview: Label
var reserve_label: Label
var place_label: Label
var cost_label: Label
var radius_box: HBoxContainer
var radius_slider: HSlider
var radius_label: Label
var confirm_button: Button
var cell := -1

func _ready() -> void:
	set_title(App.t("interventions"))
	custom_minimum_size = Vector2(560, 0)
	var intro := Atlas.text(App.t("intervene_text"), 16, true)
	intro.custom_minimum_size.x = 520
	content.add_child(intro)
	reserve_label = Atlas.text("", 16)
	reserve_label.custom_minimum_size.x = 520
	content.add_child(reserve_label)
	cell = App.globe.selected_cell
	place_label = Atlas.text("", 16, true)
	place_label.custom_minimum_size.x = 520
	content.add_child(place_label)
	if cell < 0:
		place_label.text = App.t("intervene_pick")
	else:
		var info: Dictionary = App.session.cell_info(cell)
		place_label.text = "%s : %s, %s (%s)" % [App.t("intervene_where"), str(info.get("region", "")), str(info.get("place", "")), str(info.get("medium", ""))]
	content.add_child(Atlas.hsep())
	var group := ButtonGroup.new()
	for k in KINDS.keys():
		var c := CheckBox.new()
		c.text = App.t(k)
		c.button_group = group
		c.button_pressed = k == kind
		c.toggled.connect(func(on): if on: _choose(k))
		content.add_child(c)
	var h := HBoxContainer.new()
	content.add_child(h)
	var l := Atlas.text(App.t("amount"), 17, true)
	l.autowrap_mode = TextServer.AUTOWRAP_OFF
	l.custom_minimum_size.x = 110
	h.add_child(l)
	slider = HSlider.new()
	slider.step = 0.25
	slider.custom_minimum_size.x = 250
	slider.value_changed.connect(func(_x): _update_amount())
	h.add_child(slider)
	amount_label = Atlas.text("", 17)
	amount_label.autowrap_mode = TextServer.AUTOWRAP_OFF
	h.add_child(amount_label)
	radius_box = HBoxContainer.new()
	content.add_child(radius_box)
	var rl := Atlas.text(App.t("radius"), 17, true)
	rl.autowrap_mode = TextServer.AUTOWRAP_OFF
	rl.custom_minimum_size.x = 110
	radius_box.add_child(rl)
	radius_slider = HSlider.new()
	radius_slider.min_value = 200.0
	radius_slider.max_value = 5000.0
	radius_slider.step = 100.0
	radius_slider.value = 1500.0
	radius_slider.custom_minimum_size.x = 250
	radius_slider.value_changed.connect(func(_x): _update_amount())
	radius_box.add_child(radius_slider)
	radius_label = Atlas.text("", 17)
	radius_label.autowrap_mode = TextServer.AUTOWRAP_OFF
	radius_box.add_child(radius_label)
	cost_label = Atlas.text("", 17)
	cost_label.custom_minimum_size.x = 520
	content.add_child(cost_label)
	content.add_child(Atlas.title(App.t("preview"), 20))
	preview = Atlas.text("", 16)
	preview.custom_minimum_size.x = 520
	content.add_child(preview)
	content.add_child(Atlas.hsep())
	var actions := HBoxContainer.new()
	content.add_child(actions)
	confirm_button = Atlas.button(App.t("confirm"), _confirm)
	actions.add_child(confirm_button)
	actions.add_child(Atlas.button(App.t("cancel"), close))
	_choose(kind)

func _choose(k: String) -> void:
	kind = k
	var d: Dictionary = KINDS[k]
	slider.min_value = d["min"]
	slider.max_value = d["max"]
	slider.value = d["default"]
	preview.text = App.t("preview_" + k)
	_update_amount()

func moles() -> float:
	return pow(10.0, slider.value)

func cost() -> float:
	return App.session.intervention_cost(kind, moles())

func _update_amount() -> void:
	amount_label.text = "%s %s" % [App.session.format_power(moles()), KINDS[kind]["unit"]]
	radius_box.visible = kind == "phosphate"
	radius_label.text = "%d km" % int(radius_slider.value)
	var inf: Dictionary = App.session.influence()
	var points := float(inf.get("points", 0.0))
	var sandbox := bool(inf.get("sandbox", false))
	if sandbox:
		reserve_label.text = "%s : %s" % [App.t("influence"), App.t("sandbox")]
	else:
		reserve_label.text = "%s : %d / %d (+%d %s)" % [App.t("influence"), int(points), int(inf.get("max", 0.0)), int(inf.get("recharge_per_myr", 0.0)), App.t("per_myr")]
	var affordable := sandbox or points >= cost()
	cost_label.text = "%s : %d %s" % [App.t("cost"), int(ceil(cost())), App.t("points")]
	if not affordable:
		cost_label.text += " · " + App.t("not_enough")
	cost_label.add_theme_color_override("font_color", Atlas.INK if affordable else Atlas.VERMILION)
	confirm_button.disabled = cell < 0 or not affordable

func _confirm() -> void:
	# Point de sauvegarde juste avant : l'intervention se défait en le
	# rechargeant.
	App.session.save(App.save_path(App.t("autosave_name")), App.t("autosave_name"))
	if App.session.intervene(kind, moles(), cell, radius_slider.value):
		intervened.emit(kind, moles())
	close()
