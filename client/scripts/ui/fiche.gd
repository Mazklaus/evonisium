extends PanelContainer
## Fiche de l'Atlas : cartouche à filet d'encre, titre à la plume, bouton de
## fermeture. Les panneaux de la partie (espèce, arbre, chronique,
## interventions, sauvegarde) en héritent.

signal closed

var content: VBoxContainer
var title_label: Label
var header: HBoxContainer

func _init() -> void:
	var v := VBoxContainer.new()
	v.add_theme_constant_override("separation", 8)
	add_child(v)
	header = HBoxContainer.new()
	v.add_child(header)
	title_label = Atlas.title("", 30)
	title_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	title_label.clip_text = true
	header.add_child(title_label)
	var x := Atlas.button("×", close, App.t("close"))
	x.add_theme_font_size_override("font_size", 22)
	header.add_child(x)
	v.add_child(Atlas.hsep())
	content = VBoxContainer.new()
	content.size_flags_vertical = Control.SIZE_EXPAND_FILL
	v.add_child(content)

func set_title(t: String) -> void:
	title_label.text = t

func close() -> void:
	closed.emit()
	queue_free()

func _unhandled_key_input(event: InputEvent) -> void:
	if event.is_action_pressed("ui_cancel"):
		get_viewport().set_input_as_handled()
		close()

## Libellé et valeur sur une ligne.
func field(parent: Control, label: String, value: String) -> Label:
	var h := HBoxContainer.new()
	var l := Atlas.text(label, 17, true)
	l.autowrap_mode = TextServer.AUTOWRAP_OFF
	l.custom_minimum_size.x = 150
	h.add_child(l)
	var v := Atlas.text(value, 18)
	v.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	v.custom_minimum_size.x = 120
	h.add_child(v)
	parent.add_child(h)
	return v
