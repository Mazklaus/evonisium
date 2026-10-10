extends Node
## Son du jeu (livraison 1 du doc « Design sonore ») : musique des ères
## microbiennes, souffle du globe, monde liquide de la loupe, bruits de
## papier et d'encre. Toute la synthèse est en Rust (classe EvoSon, crate
## evo-son) ; ce script ne fait que dire où est le joueur et ce qu'il ouvre.
## Il observe l'interface sans la modifier. Ctrl+M coupe ou rend le son.

const CONFIG := "user://son.cfg"

var son: EvoSon
var muet := false
## Réglages de 0 à 1, gardés dans user://son.cfg.
var volumes := {"general": 0.8, "musique": 0.7, "ambiances": 0.7, "interface": 0.6}

var _inspecteur: WeakRef
var _sol: WeakRef
var _saves := 0

func _ready() -> void:
	process_mode = Node.PROCESS_MODE_ALWAYS
	# Les portes de la CI tournent sans fenêtre ni carte son : rien à faire
	# (sauf demande explicite, pour enregistrer le son avec --write-movie).
	if DisplayServer.get_name() == "headless" and not "--avec-son" in OS.get_cmdline_user_args():
		return
	son = EvoSon.new()
	son.name = "EvoSon"
	add_child(son)
	_charger()
	App.screen_requested.connect(_on_screen)
	get_tree().node_added.connect(_on_node_added)

func _charger() -> void:
	var cfg := ConfigFile.new()
	if cfg.load(CONFIG) == OK:
		for k in volumes:
			volumes[k] = clampf(float(cfg.get_value("volumes", k, volumes[k])), 0.0, 1.0)
		muet = bool(cfg.get_value("son", "muet", false))
	_appliquer()

func _enregistrer() -> void:
	var cfg := ConfigFile.new()
	for k in volumes:
		cfg.set_value("volumes", k, volumes[k])
	cfg.set_value("son", "muet", muet)
	cfg.save(CONFIG)

func _appliquer() -> void:
	son.volumes(volumes["general"], volumes["musique"], volumes["ambiances"], volumes["interface"])
	son.muet(muet)

## Règle un volume (« general », « musique », « ambiances », « interface »).
func regler(nom: String, valeur: float) -> void:
	if son == null or not volumes.has(nom):
		return
	volumes[nom] = clampf(valeur, 0.0, 1.0)
	_appliquer()
	_enregistrer()

func basculer_muet() -> void:
	if son == null:
		return
	muet = not muet
	_appliquer()
	_enregistrer()

func _unhandled_key_input(event: InputEvent) -> void:
	var k := event as InputEventKey
	if k and k.pressed and not k.echo and k.keycode == KEY_M and k.ctrl_pressed:
		basculer_muet()
		get_viewport().set_input_as_handled()

func _on_screen(_name: String, _params: Dictionary) -> void:
	son.bruit("page")
	son.oublier()

func _on_node_added(node: Node) -> void:
	var s: Script = node.get_script()
	if s == null:
		return
	var path := s.resource_path
	if path.ends_with("ui/inspecteur.gd"):
		_inspecteur = weakref(node)
	elif path.ends_with("screens/sol.gd"):
		_sol = weakref(node)
	elif node.has_signal("closed") and node is PanelContainer:
		# Une fiche de l'Atlas s'ouvre : la plume gratte.
		son.bruit("plume")

func _visible(ref: WeakRef) -> Node:
	if ref == null:
		return null
	var n = ref.get_ref()
	if n is CanvasItem and n.is_inside_tree() and n.is_visible_in_tree():
		return n
	return null

func _vue() -> String:
	if _visible(_sol):
		return "sol"
	var insp = _visible(_inspecteur)
	if insp:
		var loupe = insp.get("loupe")
		if is_instance_valid(loupe) and loupe is TextureRect and loupe.is_visible_in_tree():
			return "microscope"
	if App.current_screen == "jeu" or App.current_screen == "ensemencement":
		return "globe"
	return "menu"

func _process(_delta: float) -> void:
	if son == null:
		return
	son.vue(_vue())
	var s: EvoSession = App.session
	if s == null or not s.is_running():
		return
	son.ecouter(s)
	# Un point de sauvegarde vient d'être écrit : coup de tampon.
	var pending := s.saves_pending()
	if pending < _saves:
		son.bruit("tampon")
	_saves = pending
