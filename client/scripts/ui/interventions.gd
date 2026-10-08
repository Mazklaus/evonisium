extends "res://scripts/ui/fiche.gd"
## Interventions (document Fonctionnalités, « Interventions ») : le joueur
## agit sur l'environnement, jamais sur les gènes. Choix, ampleur, aperçu des
## effets attendus, confirmation ; un point de sauvegarde est écrit juste
## avant, pour pouvoir revenir en arrière.

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

func _ready() -> void:
	set_title(App.t("interventions"))
	custom_minimum_size = Vector2(560, 0)
	var intro := Atlas.text(App.t("intervene_text"), 16, true)
	intro.custom_minimum_size.x = 520
	content.add_child(intro)
	var inf: Dictionary = App.session.influence()
	reserve_label = Atlas.text("", 16)
	reserve_label.custom_minimum_size.x = 520
	if bool(inf.get("available", false)):
		reserve_label.text = "%s : %s" % [App.t("influence"), str(inf.get("text", ""))]
	else:
		reserve_label.text = App.t("influence_pending")
		reserve_label.add_theme_color_override("font_color", Color(Atlas.INK, 0.7))
	content.add_child(reserve_label)
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
	content.add_child(Atlas.title(App.t("preview"), 20))
	preview = Atlas.text("", 16)
	preview.custom_minimum_size.x = 520
	content.add_child(preview)
	content.add_child(Atlas.hsep())
	var actions := HBoxContainer.new()
	content.add_child(actions)
	actions.add_child(Atlas.button(App.t("confirm"), _confirm))
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

func _update_amount() -> void:
	amount_label.text = "%s %s" % [App.session.format_power(moles()), KINDS[kind]["unit"]]

func _confirm() -> void:
	# Point de sauvegarde juste avant : l'intervention se défait en le
	# rechargeant.
	App.session.save(App.save_path(App.t("autosave_name")), App.t("autosave_name"))
	if App.session.intervene(kind, moles()):
		intervened.emit(kind, moles())
	close()
