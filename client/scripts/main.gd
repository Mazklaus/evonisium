extends Node
## Scène principale : le globe en 3D, toujours présent, et par-dessus
## l'écran courant (accueil, création, ensemencement, partie, chargement).

const SCREENS := {
	"accueil": preload("res://scripts/screens/accueil.gd"),
	"creation": preload("res://scripts/screens/creation.gd"),
	"ensemencement": preload("res://scripts/screens/ensemencement.gd"),
	"jeu": preload("res://scripts/screens/jeu.gd"),
	"chargement": preload("res://scripts/screens/chargement.gd"),
}

var globe: Node3D
var ui: CanvasLayer
var screen: Control

func _ready() -> void:
	get_window().title = "Evonisium"
	globe = preload("res://scripts/globe.gd").new()
	globe.name = "Globe"
	add_child(globe)
	App.globe = globe
	ui = CanvasLayer.new()
	ui.name = "Interface"
	add_child(ui)
	App.screen_requested.connect(_show)
	var first := "accueil"
	var args := OS.get_cmdline_user_args()
	# Le scénario de la porte pilote le client depuis la ligne de commande.
	if "--porte4" in args:
		var porte4 = load("res://tests/porte4.gd").new()
		porte4.name = "Porte4"
		add_child(porte4)
		return
	elif "--porte" in args:
		var porte = load("res://tests/porte.gd").new()
		porte.name = "Porte"
		add_child(porte)
		return
	_show(first, {})

func _show(name: String, params: Dictionary) -> void:
	if screen:
		screen.queue_free()
		screen = null
	App.current_screen = name
	screen = SCREENS[name].new()
	screen.name = name.capitalize()
	screen.set_anchors_preset(Control.PRESET_FULL_RECT)
	screen.mouse_filter = Control.MOUSE_FILTER_IGNORE
	screen.theme = Atlas.theme
	ui.add_child(screen)
	if screen.has_method("setup"):
		screen.setup(params)

func _notification(what: int) -> void:
	if what == NOTIFICATION_WM_CLOSE_REQUEST:
		App.session.stop()
