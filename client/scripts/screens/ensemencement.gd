extends Control
## Ensemencement : la partie commence avec une cellule minimale ; le joueur
## choisit de la déposer près des sources chaudes ou dans toutes les mers.

var choice := "sources"
## Première partie guidée : le narrateur repart du début.
var guided := false

func setup(params: Dictionary) -> void:
	guided = params.get("guided", false)
	var g = App.globe
	g.interactive = true
	g.set_layer({})
	g.set_show_vents(true)
	g.target_distance = 4.4
	g.view_offset = 0.8
	var p := Atlas.panel()
	p.position = Vector2(40, 40)
	p.custom_minimum_size = Vector2(470, 0)
	add_child(p)
	var v := VBoxContainer.new()
	v.add_theme_constant_override("separation", 10)
	p.add_child(v)
	v.add_child(Atlas.title(App.t("seeding_title"), 36))
	v.add_child(Atlas.hsep())
	var t := Atlas.text(App.t("seeding_text"))
	t.custom_minimum_size.x = 430
	v.add_child(t)
	var group := ButtonGroup.new()
	var a := CheckBox.new()
	a.text = App.t("seed_vents")
	a.button_group = group
	a.button_pressed = true
	a.toggled.connect(func(on): if on: _choose("sources"))
	v.add_child(a)
	var b := CheckBox.new()
	b.text = App.t("seed_all")
	b.button_group = group
	b.toggled.connect(func(on): if on: _choose("mers"))
	v.add_child(b)
	var hint := Atlas.text(App.t("vents_shown"), 17, true)
	hint.custom_minimum_size.x = 430
	v.add_child(hint)
	v.add_child(Atlas.hsep())
	var h := HBoxContainer.new()
	v.add_child(h)
	h.add_child(Atlas.button(App.t("back"), func(): App.goto("accueil" if guided else "creation")))
	var go := Atlas.button(App.t("start_life"), _begin)
	h.add_child(go)
	go.grab_focus.call_deferred()

func _choose(kind: String) -> void:
	choice = kind
	App.globe.set_show_vents(kind == "sources")

func _process(_delta: float) -> void:
	App.globe.refresh_frame()

func begin() -> void:
	_begin()

func _begin() -> void:
	# La planète d'une partie guidée se génère encore.
	if not App.session.has_frame():
		return
	App.session.set_seeding(choice)
	App.session.seed_life()
	App.session.set_rules_profile(App.settings["stop_profile"])
	App.session.resume()
	App.globe.set_show_vents(false)
	App.goto("jeu", {"new": true, "guided": guided})
