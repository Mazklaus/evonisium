extends "res://scripts/ui/fiche.gd"
## Encyclopédie (étape 7 ; document Fonctionnalités, « Encyclopédie ») : le
## tableau des simplifications assumées du moteur (règle du doc
## d'architecture : chaque chantier y ajoute les siennes), et le catalogue
## des techniques avec leur équivalent sur Terre. Elle passe en plein écran
## comme l'arbre du vivant.

var tabs: TabContainer
var simplifications: VBoxContainer
var techniques: VBoxContainer
var search: LineEdit
var all_techniques: Array = []
var full := false

func _ready() -> void:
	set_title(App.t("encyclopedia"))
	custom_minimum_size = Vector2(900, 0)
	var x := Atlas.button(App.t("full_screen"), _toggle_full)
	header.add_child(x)
	header.move_child(x, header.get_child_count() - 2)
	tabs = TabContainer.new()
	tabs.custom_minimum_size = Vector2(870, 600)
	tabs.size_flags_vertical = Control.SIZE_EXPAND_FILL
	content.add_child(tabs)

	var ss := ScrollContainer.new()
	ss.name = App.t("simplifications")
	ss.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	tabs.add_child(ss)
	simplifications = VBoxContainer.new()
	simplifications.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	simplifications.add_theme_constant_override("separation", 6)
	ss.add_child(simplifications)
	var intro := Atlas.text(App.t("simplifications_intro"), 16, true)
	intro.custom_minimum_size.x = 820
	simplifications.add_child(intro)
	var source := ""
	for s in App.session.encyclopedia_simplifications():
		if s["source"] != source:
			source = s["source"]
			simplifications.add_child(Atlas.hsep())
			simplifications.add_child(Atlas.title(source, 20))
		var t := Atlas.text(s["title"], 16)
		t.custom_minimum_size.x = 820
		simplifications.add_child(t)
		for k in [["reality", "simpl_reality"], ["lost", "simpl_lost"], ["kept", "simpl_kept"]]:
			if str(s[k[0]]) != "":
				var l := Atlas.text("%s : %s" % [App.t(k[1]), s[k[0]]], 14, true)
				l.custom_minimum_size.x = 800
				simplifications.add_child(l)

	var tv := VBoxContainer.new()
	tv.name = App.t("techniques")
	tabs.add_child(tv)
	search = LineEdit.new()
	search.placeholder_text = App.t("search")
	search.text_changed.connect(func(_t): _fill_techniques())
	tv.add_child(search)
	var ts := ScrollContainer.new()
	ts.size_flags_vertical = Control.SIZE_EXPAND_FILL
	ts.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	tv.add_child(ts)
	techniques = VBoxContainer.new()
	techniques.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	ts.add_child(techniques)
	all_techniques = App.session.encyclopedia_techniques()
	_fill_techniques()

func _fill_techniques() -> void:
	for c in techniques.get_children():
		c.queue_free()
	var q := search.text.to_lower()
	var shown := 0
	for t in all_techniques:
		var hay := (str(t["name"]) + " " + str(t["on_earth"])).to_lower()
		if q != "" and not hay.contains(q):
			continue
		shown += 1
		var line := str(t["name"])
		if str(t["path"]) != "":
			line += " (%s)" % t["path"]
		var l := Atlas.text(line, 16)
		l.custom_minimum_size.x = 820
		techniques.add_child(l)
		var d := Atlas.text(App.t("on_earth") % t["on_earth"] + ("" if str(t["requires"]) == "" else "  ·  " + App.t("requires") % t["requires"]), 14, true)
		d.custom_minimum_size.x = 800
		techniques.add_child(d)
	if shown == 0:
		techniques.add_child(Atlas.text(App.t("none"), 16, true))

func _toggle_full() -> void:
	full = not full
	if full:
		set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT, Control.PRESET_MODE_MINSIZE, 20)
	else:
		set_anchors_preset(Control.PRESET_TOP_LEFT)
		size = custom_minimum_size
		Deplacable.centre(self)
