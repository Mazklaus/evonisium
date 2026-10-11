extends Node
## Son du jeu (livraison 1 du doc « Design sonore ») : musique des ères
## microbiennes, souffle du globe, monde liquide de la loupe, bruits de
## papier et d'encre. Toute la synthèse est en Rust (classe EvoSon, crate
## evo-son) ; ce script ne fait que dire où est le joueur et ce qu'il ouvre.
## Il observe l'interface sans la modifier. Ctrl+M coupe ou rend le son.
##
## Narrateur vocal : une voix de synthèse installée depuis les réglages
## (voix.gd) raconte les moments clés de la chronique, et, au choix, ce que
## montre la loupe (« scène ») et l'espèce dont on ouvre la fiche (« fiche »).
## Les sous-titres s'affichent en bas de l'écran.

const CONFIG := "user://son.cfg"

## Une phrase du narrateur vient de finir (ou a été interrompue).
signal phrase_finie(id: int)
## Texte de la phrase qui commence, pour un sous-titre.
signal sous_titre(texte: String, id: int)

var son: EvoSon
var muet := false
## Réglages de 0 à 1, gardés dans user://son.cfg.
var volumes := {"general": 0.8, "musique": 0.7, "ambiances": 0.7, "interface": 0.6, "narrateur": 0.9}
## Options du narrateur vocal, gardées dans user://son.cfg.
var narrateur := {"voix": "gilles", "moments": true, "scene": false, "fiche": false, "sous_titres": true}
var installeur: Node

var _sous_titre: Label
var _sous_titre_panneau: PanelContainer
var _loupe_cellule := -1
var _fiche_lue := -1
## Groupes dont l'appelant affiche lui-même le texte (le guide du client).
var _sans_bandeau := {}

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
	installeur = preload("res://scripts/son/voix.gd").new()
	installeur.name = "Voix"
	add_child(installeur)
	installeur.installee.connect(func(): brancher_voix())
	_charger()
	brancher_voix()
	_poser_sous_titres()
	App.screen_requested.connect(_on_screen)
	get_tree().node_added.connect(_on_node_added)

func _charger() -> void:
	var cfg := ConfigFile.new()
	if cfg.load(CONFIG) == OK:
		for k in volumes:
			volumes[k] = clampf(float(cfg.get_value("volumes", k, volumes[k])), 0.0, 1.0)
		muet = bool(cfg.get_value("son", "muet", false))
		for k in narrateur:
			narrateur[k] = cfg.get_value("narrateur", k, narrateur[k])
	if installeur and not installeur.VOIX.has(narrateur["voix"]):
		narrateur["voix"] = installeur.VOIX.keys()[0]
	_appliquer()

func enregistrer() -> void:
	var cfg := ConfigFile.new()
	for k in volumes:
		cfg.set_value("volumes", k, volumes[k])
	cfg.set_value("son", "muet", muet)
	for k in narrateur:
		cfg.set_value("narrateur", k, narrateur[k])
	cfg.save(CONFIG)

func _appliquer() -> void:
	if son == null:
		return
	son.volumes(volumes["general"], volumes["musique"], volumes["ambiances"], volumes["interface"], volumes["narrateur"])
	son.muet(muet)
	son.raconter_moments(bool(narrateur["moments"]))

## Règle un volume (« general », « musique », « ambiances », « interface »)
## et l'entend aussitôt ; `enregistrer()` le garde ensuite.
func regler(nom: String, valeur: float) -> void:
	if not volumes.has(nom):
		return
	volumes[nom] = clampf(valeur, 0.0, 1.0)
	_appliquer()

## Règle une option du narrateur (« voix », « moments », « scene »,
## « fiche », « sous_titres ») et la garde.
func regler_narrateur(nom: String, valeur) -> void:
	if not narrateur.has(nom):
		return
	narrateur[nom] = valeur
	if nom == "voix":
		brancher_voix()
	_appliquer()
	enregistrer()

## Donne au son la voix choisie si elle est installée.
func brancher_voix() -> bool:
	if son == null or installeur == null:
		return false
	var v: Dictionary = installeur.chemins(narrateur["voix"])
	if v.is_empty():
		son.voix("", "", 0)
		return false
	return son.voix(v["exe"], v["modele"], v["locuteur"])

func a_une_voix() -> bool:
	return son != null and son.a_une_voix()

## Narrateur vocal. Priorités : 1 pour le guide et les moments clés (dits
## l'un après l'autre), 2 pour une demande du joueur (fiche, scène) ; une
## priorité plus haute interrompt, une plus basse est ignorée pendant qu'il
## parle. `bandeau` : afficher le texte en sous-titre (faux quand l'appelant
## l'affiche lui-même). Renvoie -1 quand rien ne sera dit (pas de voix
## installée, pas de son, ou langue autre que le français).
func dire(texte: String, priorite: int = 1, bandeau: bool = false) -> int:
	if son == null or App.settings["lang"] != "fr":
		return -1
	var g: int = son.dire(texte, priorite)
	if g >= 0 and not bandeau:
		_sans_bandeau[g] = true
	return g

## Raconte une espèce comme dans un documentaire (option « fiche »).
func raconter_espece(espece: int) -> int:
	if son == null or App.session == null or App.settings["lang"] != "fr":
		return -1
	return son.raconter_espece(App.session, espece)

## Raconte ce qui vit dans une cellule (option « scène »).
func raconter_lieu(cellule: int) -> int:
	if son == null or App.session == null or App.settings["lang"] != "fr":
		return -1
	return son.raconter_lieu(App.session, cellule)

## Coupe la phrase en cours et vide la file.
func taire() -> void:
	if son:
		son.taire()

func parle() -> bool:
	return son != null and son.parle()

func couper(on: bool) -> void:
	muet = on
	_appliquer()
	enregistrer()

func basculer_muet() -> void:
	couper(not muet)

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
		if path.ends_with("ui/fiche_espece.gd"):
			_lire_fiche.call_deferred(weakref(node))

## Option « fiche » : la fiche d'espèce qui s'ouvre est lue.
func _lire_fiche(ref: WeakRef) -> void:
	var n = ref.get_ref()
	if n == null or not bool(narrateur["fiche"]):
		return
	var sp := int(n.get("species"))
	if sp >= 0 and sp != _fiche_lue:
		_fiche_lue = sp
		raconter_espece(sp)

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
	var vue := _vue()
	son.vue(vue)
	_suivre_narrateur(vue)
	var s: EvoSession = App.session
	if s == null or not s.is_running():
		return
	son.ecouter(s)
	# Un point de sauvegarde vient d'être écrit : coup de tampon.
	var pending := s.saves_pending()
	if pending < _saves:
		son.bruit("tampon")
	_saves = pending

func _suivre_narrateur(vue: String) -> void:
	for n in son.nouvelles():
		var g: int = n["groupe"]
		if n["debut"]:
			_montrer_sous_titre("" if _sans_bandeau.has(g) else n["texte"])
			sous_titre.emit(n["texte"], g)
		elif n["fin_groupe"]:
			_montrer_sous_titre("")
			_sans_bandeau.erase(g)
			phrase_finie.emit(g)
	# Option « scène » : la loupe ouverte sur une cellule est racontée une fois.
	if vue != "microscope":
		_loupe_cellule = -1
		return
	var insp = _visible(_inspecteur)
	var c := int(insp.get("cell")) if insp else -1
	if c >= 0 and c != _loupe_cellule and bool(narrateur["scene"]):
		_loupe_cellule = c
		raconter_lieu(c)

func _poser_sous_titres() -> void:
	var calque := CanvasLayer.new()
	calque.layer = 50
	add_child(calque)
	_sous_titre_panneau = PanelContainer.new()
	_sous_titre_panneau.add_theme_stylebox_override("panel", Atlas.cartouche(Color(Atlas.INK, 0.82), Color(Atlas.PAPER, 0.5), 1, false))
	_sous_titre_panneau.set_anchors_preset(Control.PRESET_CENTER_BOTTOM)
	_sous_titre_panneau.grow_horizontal = Control.GROW_DIRECTION_BOTH
	_sous_titre_panneau.grow_vertical = Control.GROW_DIRECTION_BEGIN
	_sous_titre_panneau.offset_bottom = -64
	_sous_titre_panneau.mouse_filter = Control.MOUSE_FILTER_IGNORE
	calque.add_child(_sous_titre_panneau)
	_sous_titre = Atlas.text("", 18, true)
	_sous_titre.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	_sous_titre.custom_minimum_size.x = 640
	_sous_titre.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	_sous_titre_panneau.add_child(_sous_titre)
	_sous_titre_panneau.visible = false

func _montrer_sous_titre(texte: String) -> void:
	if _sous_titre == null:
		return
	_sous_titre.text = texte
	_sous_titre_panneau.visible = texte != "" and bool(narrateur["sous_titres"])
	_sous_titre_panneau.reset_size()
