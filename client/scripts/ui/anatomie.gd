extends "res://scripts/ui/fiche.gd"
## Anatomie d'une espèce (document Fonctionnalités, « Anatomie » ; Rendu du
## vivant, palier 2) : le corps tiré du plan de construction, en vraie
## grandeur avec sa barre d'échelle ; la vue anatomie voile la peau et montre
## les organes internes, colorés par appareil, avec leur légende.

signal compare_requested(species: int)

const Portrait := preload("res://scripts/ui/portrait.gd")

var species := -1
var portrait
var overlay: Control
var anatomy_button: CheckButton
var legend: VBoxContainer
var traits_box: VBoxContainer

func open(s: int) -> void:
	species = s
	var info: Dictionary = App.session.species_info(s)
	set_title("%s — %s" % [App.t("anatomy"), str(info.get("common", ""))])
	portrait.show_body(App.session.request_body(s))

## Banc d'essai : un plan tiré au hasard.
func open_test(seed: int) -> void:
	set_title("%s — %s %d" % [App.t("anatomy"), App.t("test_body"), seed])
	portrait.show_body(App.session.request_test_body(seed))

func _ready() -> void:
	custom_minimum_size = Vector2(820, 0)
	var intro := Atlas.text(App.t("anatomy_text"), 15, true)
	intro.custom_minimum_size.x = 780
	content.add_child(intro)
	var h := HBoxContainer.new()
	h.add_theme_constant_override("separation", 18)
	content.add_child(h)
	var left := VBoxContainer.new()
	h.add_child(left)
	portrait = Portrait.new()
	portrait.custom_minimum_size = Vector2(520, 380)
	portrait.body_ready.connect(_on_ready)
	left.add_child(portrait)
	overlay = Control.new()
	overlay.custom_minimum_size = Vector2(520, 30)
	overlay.draw.connect(func(): portrait.draw_scale_bar(overlay, Vector2(16, 14)))
	left.add_child(overlay)
	var right := VBoxContainer.new()
	right.custom_minimum_size.x = 260
	h.add_child(right)
	anatomy_button = CheckButton.new()
	anatomy_button.text = App.t("show_organs")
	anatomy_button.toggled.connect(func(on): portrait.set_anatomy(on))
	right.add_child(anatomy_button)
	traits_box = VBoxContainer.new()
	right.add_child(traits_box)
	right.add_child(Atlas.hsep())
	legend = VBoxContainer.new()
	right.add_child(legend)
	right.add_child(Atlas.text(App.t("drag_to_turn"), 14, true))
	var cmp := Atlas.button(App.t("compare"), func(): compare_requested.emit(species))
	right.add_child(cmp)

func _on_ready(d: Dictionary) -> void:
	overlay.queue_redraw()
	for c in traits_box.get_children():
		c.queue_free()
	for c in legend.get_children():
		c.queue_free()
	var t: Dictionary = d["traits"]
	_line(traits_box, App.t("size"), t["size"])
	_line(traits_box, App.t("symmetry"), t["symmetry"])
	_line(traits_box, App.t("covering"), t["covering"])
	_line(traits_box, App.t("appendages"), str(t["appendages"]))
	_line(traits_box, App.t("eyes"), str(t["eyes"]))
	_line(traits_box, App.t("cell_types"), str(t["cell_types"]))
	legend.add_child(Atlas.text(App.t("organs"), 16, true))
	var systems: Array = t["systems"]
	if systems.is_empty():
		legend.add_child(Atlas.text(App.t("no_organs"), 14, true))
	for s in systems:
		var row := HBoxContainer.new()
		var sw := ColorRect.new()
		sw.color = s["colour"]
		sw.custom_minimum_size = Vector2(16, 16)
		row.add_child(sw)
		var sl := Atlas.text(s["label"], 15)
		sl.autowrap_mode = TextServer.AUTOWRAP_OFF
		row.add_child(sl)
		legend.add_child(row)

func _line(parent: Control, label: String, value: String) -> void:
	var h := HBoxContainer.new()
	var l := Atlas.text(label, 15, true)
	l.custom_minimum_size.x = 120
	l.autowrap_mode = TextServer.AUTOWRAP_OFF
	h.add_child(l)
	var v := Atlas.text(value, 15)
	v.autowrap_mode = TextServer.AUTOWRAP_OFF
	h.add_child(v)
	parent.add_child(h)
