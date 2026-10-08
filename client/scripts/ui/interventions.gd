extends "res://scripts/ui/fiche.gd"
## Interventions (document Fonctionnalités, « Interventions ») : le joueur
## agit sur l'environnement, jamais sur les gènes, autour de la cellule
## choisie sur le globe. Choix, réglages, coût en influence, aperçu des
## effets attendus, confirmation ; un point de sauvegarde propre à
## l'intervention est écrit juste avant, d'où partent le retour en arrière et
## la comparaison « avec et sans ».
##
## Étape 4 : impact météoritique, isolement d'un groupe (bras de mer ou
## chaîne de montagnes) et poussée climatique s'ajoutent aux apports de
## nutriments et aux éruptions.

signal intervened(kind: String, order: int)

## Réglages de chaque intervention : clé, libellé, bornes, valeur par
## défaut, échelle logarithmique (la valeur est alors un exposant de 10),
## unité affichée.
const KINDS := {
	"phosphate": [
		{"key": "moles", "label": "amount", "min": 12.0, "max": 16.0, "default": 13.0, "log": true, "unit": "mol P"},
		{"key": "radius_km", "label": "radius", "min": 200.0, "max": 5000.0, "default": 1500.0, "step": 100.0, "unit": "km"},
	],
	"eruption": [
		{"key": "moles", "label": "amount", "min": 15.0, "max": 18.5, "default": 16.0, "log": true, "unit": "mol CO₂"},
	],
	"methane": [
		{"key": "moles", "label": "amount", "min": 13.0, "max": 17.0, "default": 15.0, "log": true, "unit": "mol CH₄"},
	],
	"impact": [
		{"key": "diameter_km", "label": "diameter", "min": -0.5, "max": 1.5, "default": 0.7, "log": true, "unit": "km"},
	],
	"bras_de_mer": [
		{"key": "length_km", "label": "length", "min": 300.0, "max": 5000.0, "default": 1500.0, "step": 100.0, "unit": "km"},
		{"key": "azimuth_deg", "label": "orientation", "min": 0.0, "max": 175.0, "default": 0.0, "step": 5.0, "unit": "°"},
		{"key": "duration_years", "label": "duration", "min": 4.0, "max": 7.5, "default": 6.0, "log": true, "unit": "years"},
	],
	"montagnes": [
		{"key": "length_km", "label": "length", "min": 300.0, "max": 5000.0, "default": 1500.0, "step": 100.0, "unit": "km"},
		{"key": "azimuth_deg", "label": "orientation", "min": 0.0, "max": 175.0, "default": 0.0, "step": 5.0, "unit": "°"},
		{"key": "duration_years", "label": "duration", "min": 4.0, "max": 7.5, "default": 6.5, "log": true, "unit": "years"},
	],
	"climat": [
		{"key": "delta_k", "label": "delta_t", "min": -15.0, "max": 15.0, "default": -5.0, "step": 0.5, "unit": "K"},
		{"key": "rain_factor", "label": "rain_factor", "min": -0.6, "max": 0.5, "default": 0.0, "log": true, "unit": "×"},
		{"key": "radius_km", "label": "radius", "min": 300.0, "max": 5000.0, "default": 1500.0, "step": 100.0, "unit": "km"},
		{"key": "duration_years", "label": "duration", "min": 2.0, "max": 7.0, "default": 5.0, "log": true, "unit": "years"},
	],
}

var kind := "phosphate"
var params_box: VBoxContainer
var sliders := {}
var value_labels := {}
var preview: Label
var reserve_label: Label
var place_label: Label
var cost_label: Label
var confirm_button: Button
var cell := -1

func _ready() -> void:
	set_title(App.t("interventions"))
	custom_minimum_size = Vector2(600, 0)
	var intro := Atlas.text(App.t("intervene_text"), 16, true)
	intro.custom_minimum_size.x = 560
	content.add_child(intro)
	reserve_label = Atlas.text("", 16)
	reserve_label.custom_minimum_size.x = 560
	content.add_child(reserve_label)
	cell = App.globe.selected_cell
	place_label = Atlas.text("", 16, true)
	place_label.custom_minimum_size.x = 560
	content.add_child(place_label)
	if cell < 0:
		place_label.text = App.t("intervene_pick")
	else:
		var info: Dictionary = App.session.cell_info(cell)
		place_label.text = "%s : %s, %s (%s)" % [App.t("intervene_where"), str(info.get("region", "")), str(info.get("place", "")), str(info.get("medium", ""))]
	content.add_child(Atlas.hsep())
	# Deux colonnes de choix : monde microbien, puis monde multicellulaire.
	var cols := HBoxContainer.new()
	cols.add_theme_constant_override("separation", 24)
	content.add_child(cols)
	var group := ButtonGroup.new()
	var left := VBoxContainer.new()
	var right := VBoxContainer.new()
	cols.add_child(left)
	cols.add_child(right)
	for k in KINDS.keys():
		var c := CheckBox.new()
		c.text = App.t(k)
		c.button_group = group
		c.button_pressed = k == kind
		c.toggled.connect(func(on): if on: _choose(k))
		(left if k in ["phosphate", "eruption", "methane"] else right).add_child(c)
	params_box = VBoxContainer.new()
	content.add_child(params_box)
	cost_label = Atlas.text("", 17)
	cost_label.custom_minimum_size.x = 560
	content.add_child(cost_label)
	content.add_child(Atlas.title(App.t("preview"), 20))
	preview = Atlas.text("", 16)
	preview.custom_minimum_size.x = 560
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
	for c in params_box.get_children():
		c.queue_free()
	sliders.clear()
	value_labels.clear()
	for p in KINDS[k]:
		var h := HBoxContainer.new()
		params_box.add_child(h)
		var l := Atlas.text(App.t(p["label"]), 17, true)
		l.autowrap_mode = TextServer.AUTOWRAP_OFF
		l.custom_minimum_size.x = 130
		h.add_child(l)
		var s := HSlider.new()
		s.min_value = p["min"]
		s.max_value = p["max"]
		s.step = p.get("step", 0.05)
		s.value = p["default"]
		s.custom_minimum_size.x = 260
		s.value_changed.connect(func(_x): _update())
		h.add_child(s)
		var v := Atlas.text("", 17)
		v.autowrap_mode = TextServer.AUTOWRAP_OFF
		h.add_child(v)
		sliders[p["key"]] = s
		value_labels[p["key"]] = v
	_update()

## Réglages en valeurs physiques.
func params() -> Dictionary:
	var d := {}
	for p in KINDS[kind]:
		var x: float = sliders[p["key"]].value
		d[p["key"]] = pow(10.0, x) if p.get("log", false) else x
	d["sea"] = kind == "bras_de_mer"
	return d

## Type envoyé au moteur.
func engine_kind() -> String:
	if kind == "bras_de_mer" or kind == "montagnes":
		return "isolement"
	return kind

func cost() -> float:
	return App.session.intervention_cost_ex(engine_kind(), params())

func _format(p: Dictionary, value: float) -> String:
	var unit: String = p["unit"]
	if unit == "years":
		return App.session.format_duration(value)
	if unit == "×":
		return "×%.2f" % value
	if p.get("log", false) and value >= 1.0e4:
		return "%s %s" % [App.session.format_power(value), unit]
	if p.get("log", false):
		return "%.2f %s" % [value, unit]
	return "%d %s" % [int(round(value)), unit] if absf(value) >= 10.0 else "%.1f %s" % [value, unit]

func _update() -> void:
	var d := params()
	for p in KINDS[kind]:
		value_labels[p["key"]].text = _format(p, d[p["key"]])
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
	var text: String = App.session.intervention_preview(engine_kind(), d)
	preview.text = text if text != "" else App.t("preview_" + kind)
	confirm_button.disabled = cell < 0 or not affordable

func _confirm() -> void:
	# Point de sauvegarde propre à l'intervention, juste avant : elle se
	# défait en le rechargeant, et « avec et sans » en part.
	var name := "%s %d" % [App.t("before_intervention"), int(Time.get_unix_time_from_system())]
	var order: int = App.session.intervene_ex(engine_kind(), params(), cell, App.save_path(name), name)
	if order >= 0:
		intervened.emit(kind, order)
	close()
