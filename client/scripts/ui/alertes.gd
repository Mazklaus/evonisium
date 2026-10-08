extends VBoxContainer
## Alertes (document Fonctionnalités, « Alertes et chronique ») : cartes
## brèves en haut à droite pour ce que les règles d'arrêt signalent ; un
## événement qui arrête le temps garde sa carte jusqu'à ce qu'on la ferme.

signal go_to(event: Dictionary)

const MAX_CARDS := 4
const LIFETIME := 9.0

func _ready() -> void:
	add_theme_constant_override("separation", 8)
	mouse_filter = Control.MOUSE_FILTER_IGNORE

func push(e: Dictionary, sticky: bool = false) -> void:
	var card := PanelContainer.new()
	card.custom_minimum_size = Vector2(380, 0)
	var major: bool = int(e["level"]) >= 2 or sticky
	if major:
		card.add_theme_stylebox_override("panel", Atlas.cartouche(Atlas.PAPER, Atlas.VERMILION, 2))
	var v := VBoxContainer.new()
	v.add_theme_constant_override("separation", 3)
	card.add_child(v)
	var head := HBoxContainer.new()
	v.add_child(head)
	var fam := Atlas.text("%s — %s" % [e["family_label"], e["date"]], 14, true)
	fam.autowrap_mode = TextServer.AUTOWRAP_OFF
	fam.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	if major:
		fam.add_theme_color_override("font_color", Atlas.VERMILION)
	head.add_child(fam)
	head.add_child(Atlas.button("×", card.queue_free, App.t("close")))
	var t := Atlas.text(e["text"], 17)
	t.custom_minimum_size.x = 350
	v.add_child(t)
	var actions := HBoxContainer.new()
	v.add_child(actions)
	if int(e["cell"]) >= 0 or int(e["lineage"]) >= 0:
		actions.add_child(Atlas.button(App.t("go_see"), func():
			go_to.emit(e)
			card.queue_free()))
	var fam_key: String = e["family"]
	actions.add_child(Atlas.button(App.t("ignore_type"), func():
		App.session.set_rule(fam_key, 0)
		card.queue_free()))
	add_child(card)
	move_child(card, 0)
	while get_child_count() > MAX_CARDS:
		var last := get_child(get_child_count() - 1)
		remove_child(last)
		last.queue_free()
	if not sticky:
		var tw := card.create_tween()
		tw.tween_interval(LIFETIME * (1.6 if major else 1.0))
		if bool(App.settings["reduce_motion"]):
			tw.tween_callback(card.queue_free)
		else:
			tw.tween_property(card, "modulate:a", 0.0, 0.8)
			tw.tween_callback(card.queue_free)
