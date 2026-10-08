extends Control
## Chargement d'un point de sauvegarde : la partie est rejouée depuis sa
## graine et sa file d'ordres jusqu'au pas sauvegardé.

var bar: ProgressBar
var label: Label

func setup(params: Dictionary) -> void:
	App.globe.interactive = false
	var p := Atlas.panel()
	p.position = Vector2(40, 40)
	p.custom_minimum_size = Vector2(470, 0)
	add_child(p)
	var v := VBoxContainer.new()
	p.add_child(v)
	v.add_child(Atlas.title(App.t("loading"), 32))
	bar = ProgressBar.new()
	bar.custom_minimum_size = Vector2(430, 24)
	v.add_child(bar)
	label = Atlas.text("", 17, true)
	v.add_child(label)
	var err: String = App.session.load_save(params.get("path", ""))
	App.globe.reset()
	if err != "":
		label.text = err
		v.add_child(Atlas.button(App.t("back"), func(): App.goto("accueil")))
		set_process(false)

func _process(_delta: float) -> void:
	var p: Vector2 = App.session.loading_progress()
	if p.y > 0.0:
		bar.value = 100.0 * p.x / p.y
		label.text = "%d / %d" % [int(p.x), int(p.y)]
	elif App.session.has_frame():
		App.session.set_rules_profile(App.settings["stop_profile"])
		App.goto("jeu", {"new": false})
