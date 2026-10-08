extends "res://scripts/ui/fiche.gd"
## Chronique (document Fonctionnalités, « Alertes et chronique ») : tous les
## événements, du plus récent au plus ancien, filtrés par famille et par
## recherche ; chaque ligne mène au lieu. Onglet des règles d'arrêt.

signal go_to(event: Dictionary)

const PAGE := 60

var family := ""
var search := ""
var list: VBoxContainer
var offset := 0
var count_label: Label
var rules_box: VBoxContainer

func _ready() -> void:
	set_title(App.t("chronicle"))
	custom_minimum_size = Vector2(760, 0)
	var tabs := TabContainer.new()
	tabs.custom_minimum_size = Vector2(730, 600)
	content.add_child(tabs)
	var page := VBoxContainer.new()
	page.name = App.t("chronicle")
	tabs.add_child(page)
	var filters := HFlowContainer.new()
	page.add_child(filters)
	var group := ButtonGroup.new()
	var all := Atlas.button(App.t("filter_all"), _filter.bind(""))
	all.toggle_mode = true
	all.button_group = group
	all.button_pressed = true
	filters.add_child(all)
	for r in App.session.rules():
		var b := Atlas.button(r["label"], _filter.bind(r["key"]))
		b.toggle_mode = true
		b.button_group = group
		filters.add_child(b)
	var s := LineEdit.new()
	s.placeholder_text = App.t("search")
	s.custom_minimum_size.x = 300
	s.text_changed.connect(func(t):
		search = t
		_reload())
	page.add_child(s)
	count_label = Atlas.text("", 14, true)
	page.add_child(count_label)
	var scroll := ScrollContainer.new()
	scroll.size_flags_vertical = Control.SIZE_EXPAND_FILL
	scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	page.add_child(scroll)
	list = VBoxContainer.new()
	list.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	scroll.add_child(list)
	var nav := HBoxContainer.new()
	page.add_child(nav)
	nav.add_child(Atlas.button("‹", func():
		offset = max(0, offset - PAGE)
		_reload()))
	nav.add_child(Atlas.button("›", func():
		offset += PAGE
		_reload()))

	rules_box = VBoxContainer.new()
	rules_box.name = App.t("stop_rules")
	tabs.add_child(rules_box)
	_fill_rules()
	_reload()

func _filter(f: String) -> void:
	family = f
	offset = 0
	_reload()

func _reload() -> void:
	for c in list.get_children():
		c.queue_free()
	var events: Array = App.session.events_page(offset, PAGE, family, search)
	count_label.text = "%d – %d / %d" % [offset + 1, offset + events.size(), App.session.events_count()]
	for e in events:
		var row := HBoxContainer.new()
		var date := Atlas.text(e["date"], 15, true)
		date.autowrap_mode = TextServer.AUTOWRAP_OFF
		date.custom_minimum_size.x = 110
		row.add_child(date)
		var fam := Atlas.text(e["family_label"], 14, true)
		fam.autowrap_mode = TextServer.AUTOWRAP_OFF
		fam.custom_minimum_size.x = 120
		row.add_child(fam)
		var text := Atlas.text(e["text"], 16)
		text.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		text.custom_minimum_size.x = 340
		if int(e["level"]) >= 2:
			text.add_theme_color_override("font_color", Atlas.VERMILION)
		row.add_child(text)
		if int(e["cell"]) >= 0 or int(e["lineage"]) >= 0:
			row.add_child(Atlas.button(App.t("go_see"), go_to.emit.bind(e)))
		list.add_child(row)

func _fill_rules() -> void:
	for c in rules_box.get_children():
		c.queue_free()
	var intro := Atlas.text("Ce que fait le temps quand un événement survient. Un événement majeur n'est jamais seulement noté." if App.settings["lang"] == "fr" else "What time does when an event happens. A major event is never merely noted.", 16, true)
	intro.custom_minimum_size.x = 680
	rules_box.add_child(intro)
	for r in App.session.rules():
		var h := HBoxContainer.new()
		var l := Atlas.text(r["label"], 17)
		l.autowrap_mode = TextServer.AUTOWRAP_OFF
		l.custom_minimum_size.x = 220
		h.add_child(l)
		var o := OptionButton.new()
		for a in 5:
			o.add_item(App.t("action_%d" % a), a)
		o.selected = int(r["action"])
		var key: String = r["key"]
		o.item_selected.connect(func(i): App.session.set_rule(key, i))
		h.add_child(o)
		rules_box.add_child(h)
