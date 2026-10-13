extends "res://scripts/ui/fiche.gd"
## Sociétés (étape 7, sur sociétés d'essai ; documents Fonctionnalités et
## Vision, « Interface dans les étapes ») : la liste des sociétés vivantes,
## la chronique de leur histoire (filtrable, avec recherche), le récit d'une
## période écrit comme un chapitre d'histoire, et ce que les sociétés
## rendent à la planète. Un clic ouvre la fiche ; deux cases cochées
## ouvrent la comparaison.

signal society_requested(id: int)
signal compare_requested(a: int, b: int)
signal go_to_cell(cell: int)

var tabs: TabContainer
var status_label: Label
var list: VBoxContainer
var events: VBoxContainer
var search: LineEdit
var big_only: CheckBox
var story_box: VBoxContainer
var from_spin: SpinBox
var to_spin: SpinBox
var planet: VBoxContainer
var picked: Array = []
var serial := -1
var timer := 0.0

func _ready() -> void:
	set_title(App.t("societies"))
	custom_minimum_size = Vector2(880, 0)
	var warn := Atlas.text(App.t("societies_bench"), 15, true)
	warn.custom_minimum_size.x = 840
	warn.add_theme_color_override("font_color", Atlas.VERMILION)
	content.add_child(warn)
	status_label = Atlas.text("", 16, true)
	status_label.custom_minimum_size.x = 840
	content.add_child(status_label)
	tabs = TabContainer.new()
	tabs.custom_minimum_size = Vector2(850, 540)
	content.add_child(tabs)

	list = _scroll_tab(App.t("societies"))

	var cv := VBoxContainer.new()
	cv.name = App.t("society_chronicle")
	tabs.add_child(cv)
	var bar := HBoxContainer.new()
	cv.add_child(bar)
	search = LineEdit.new()
	search.placeholder_text = App.t("search")
	search.custom_minimum_size.x = 320
	search.text_changed.connect(func(_t): _fill_events())
	bar.add_child(search)
	big_only = CheckBox.new()
	big_only.text = App.t("society_big_only")
	big_only.button_pressed = true
	big_only.toggled.connect(func(_on): _fill_events())
	bar.add_child(big_only)
	var es := ScrollContainer.new()
	es.size_flags_vertical = Control.SIZE_EXPAND_FILL
	es.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	cv.add_child(es)
	events = VBoxContainer.new()
	events.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	es.add_child(events)

	var sv := VBoxContainer.new()
	sv.name = App.t("story")
	tabs.add_child(sv)
	var sb := HBoxContainer.new()
	sv.add_child(sb)
	sb.add_child(_inline(App.t("society_from")))
	from_spin = _year_spin()
	sb.add_child(from_spin)
	sb.add_child(_inline(App.t("society_to")))
	to_spin = _year_spin()
	sb.add_child(to_spin)
	sb.add_child(Atlas.button(App.t("society_tell"), _fill_story))
	var ss := ScrollContainer.new()
	ss.size_flags_vertical = Control.SIZE_EXPAND_FILL
	ss.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	sv.add_child(ss)
	story_box = VBoxContainer.new()
	story_box.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	story_box.add_theme_constant_override("separation", 10)
	ss.add_child(story_box)

	planet = _scroll_tab(App.t("society_planet"))
	_refresh(true)

func _scroll_tab(title: String) -> VBoxContainer:
	var s := ScrollContainer.new()
	s.name = title
	s.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	tabs.add_child(s)
	var v := VBoxContainer.new()
	v.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	s.add_child(v)
	return v

func _inline(t: String) -> Label:
	var l := Atlas.text(t, 16, true)
	l.autowrap_mode = TextServer.AUTOWRAP_OFF
	l.size_flags_vertical = Control.SIZE_SHRINK_CENTER
	return l

func _year_spin() -> SpinBox:
	var s := SpinBox.new()
	s.step = 100
	s.min_value = -20000
	s.max_value = 3000
	s.custom_minimum_size.x = 130
	return s

func _process(delta: float) -> void:
	timer -= delta
	if timer <= 0.0:
		timer = 1.0
		_refresh(false)

func _refresh(first: bool) -> void:
	var st: int = App.session.society_status()
	match st:
		0, 1:
			status_label.text = App.t("societies_waiting")
		-1:
			status_label.text = App.t("societies_none")
		_:
			status_label.text = App.t("societies_year") % [App.session.society_year(), App.t("societies_done") if st == 3 else App.t("societies_running")]
	var s: int = App.session.society_serial()
	if s == serial:
		return
	serial = s
	var span: Vector2i = App.session.society_span()
	if first or to_spin.value == to_spin.max_value or to_spin.value <= from_spin.value:
		from_spin.value = maxi(span.x, span.y - 2000)
		to_spin.value = span.y
		if first:
			_fill_story()
	_fill_list()
	if tabs.current_tab == 1 or first:
		_fill_events()
	_fill_planet()

func _clear(box: Container) -> void:
	for c in box.get_children():
		c.queue_free()

func _fill_list() -> void:
	_clear(list)
	var rows: Array = App.session.society_list()
	if rows.is_empty():
		list.add_child(Atlas.text(App.t("societies_none_yet"), 16, true))
		return
	var hint := Atlas.text(App.t("societies_compare_hint"), 15, true)
	hint.custom_minimum_size.x = 800
	list.add_child(hint)
	for r in rows.slice(0, 120):
		var h := HBoxContainer.new()
		var pick := CheckBox.new()
		pick.tooltip_text = App.t("compare")
		pick.button_pressed = picked.has(int(r["id"]))
		pick.toggled.connect(_pick.bind(int(r["id"])))
		h.add_child(pick)
		var sw := ColorRect.new()
		sw.color = r["colour"]
		sw.custom_minimum_size = Vector2(16, 16)
		sw.size_flags_vertical = Control.SIZE_SHRINK_CENTER
		h.add_child(sw)
		var b := Atlas.button(r["name"], society_requested.emit.bind(int(r["id"])))
		b.custom_minimum_size.x = 210
		b.alignment = HORIZONTAL_ALIGNMENT_LEFT
		b.clip_text = true
		h.add_child(b)
		var marks := ""
		if r["war"]:
			marks += " ⚔"
		if r["crisis"]:
			marks += " ‼"
		var t := Atlas.text(App.t("society_row") % [r["label"], r["population"], int(r["cells"]), int(r["techniques"])] + marks, 15)
		t.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		t.custom_minimum_size.x = 520
		h.add_child(t)
		list.add_child(h)

func _pick(on: bool, id: int) -> void:
	if on and not picked.has(id):
		picked.append(id)
	elif not on:
		picked.erase(id)
	if picked.size() >= 2:
		var a: int = picked[0]
		var b: int = picked[1]
		picked.clear()
		compare_requested.emit(a, b)

func _fill_events() -> void:
	_clear(events)
	var rows: Array = App.session.society_events(300, 0.5 if big_only.button_pressed else 0.0, search.text)
	if rows.is_empty():
		events.add_child(Atlas.text(App.t("none"), 16, true))
	for e in rows:
		var h := HBoxContainer.new()
		var y := Atlas.text(e["year"], 15, true)
		y.autowrap_mode = TextServer.AUTOWRAP_OFF
		y.custom_minimum_size.x = 110
		h.add_child(y)
		var t := Atlas.text(e["text"], 15)
		t.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		t.custom_minimum_size.x = 560
		h.add_child(t)
		if int(e["cell"]) >= 0:
			h.add_child(Atlas.button(App.t("go_see"), go_to_cell.emit.bind(int(e["cell"]))))
		events.add_child(h)

func _fill_story() -> void:
	_clear(story_box)
	var paras: PackedStringArray = App.session.society_story(int(from_spin.value), int(to_spin.value))
	for i in paras.size():
		var l := Atlas.text(paras[i], 17 if i > 0 else 18, i == 0)
		l.custom_minimum_size.x = 800
		story_box.add_child(l)

func _fill_planet() -> void:
	_clear(planet)
	for row in App.session.society_planet():
		field(planet, row[0], row[1])
