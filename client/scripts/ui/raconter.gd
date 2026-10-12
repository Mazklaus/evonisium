extends Button
## Bouton « Raconter » : le narrateur raconte ce que montre l'écran (une
## espèce, ce qui vit dans une cellule), à la demande du joueur, même quand la
## lecture automatique est décochée dans les réglages. Pendant le récit, le
## bouton devient « Se taire ».

## Lance le récit et rend le groupe de phrases du module son, ou -1.
var tell: Callable
var group := -1
var note_left := 0.0

func _init(what: Callable) -> void:
	tell = what
	text = App.t("narrate")
	tooltip_text = App.t("narrate_hint")
	pressed.connect(_pressed)
	if Son.has_signal("phrase_finie"):
		Son.connect("phrase_finie", _done)

func _pressed() -> void:
	if group >= 0:
		Son.taire()
		_done(group)
		return
	group = int(tell.call()) if Son.has_method("raconter_lieu") else -1
	if group >= 0:
		text = App.t("narrate_stop")
	else:
		# Rien ne sera dit : pas de voix installée, son coupé ou anglais.
		text = App.t("narrate_none")
		note_left = 4.0

func _done(id: int) -> void:
	if id == group:
		group = -1
		text = App.t("narrate")

func _process(delta: float) -> void:
	if note_left > 0.0:
		note_left -= delta
		if note_left <= 0.0:
			text = App.t("narrate")
