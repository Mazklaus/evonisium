extends PanelContainer
## Narrateur discret (document Fonctionnalités, « Prise en main » ; choix de
## l'utilisateur du 8 octobre 2026) : une phrase à la fois, en bas de
## l'écran, jamais bloquante, désactivable dans les réglages.
##
## Il présente d'abord un outil à la fois, quand il devient utile (phrases
## et conditions dans evo_view::guide), et souligne l'outil à l'écran. Les
## phrases vues sont gardées d'une partie à l'autre ; la première partie
## guidée les remet à zéro. Le guide épuisé, il devient le conseiller : il
## signale un événement qui vaut d'être regardé, sans dire quoi faire.
## La voix passe par le module son (autoload Son), qui décide de parler.

signal highlight(tool: String)
signal go_to(event: Dictionary)

## Durée d'affichage d'une phrase, s, et silence entre deux phrases.
const SHOW_SECONDS := 18.0
const GAP_SECONDS := 10.0
## Silence minimal entre deux conseils, s.
const ADVICE_GAP := 75.0
## Priorité de la voix du guide dans le module son : celle des moments clés
## du documentaire ; les deux se suivent sans se couper.
const VOICE_PRIORITY := 1
## Au-delà, la phrase s'en va même si la voix n'a pas dit qu'elle a fini.
const VOICE_MAX_SECONDS := 60.0

var label: Label
var see_button: Button
var current := ""
var current_tool := ""
var current_event := {}
var clock := 0.0
var quiet := 0.0
var poll := 0.0
var last_advice := -1000.0
## Outils ouverts par le joueur dans cette partie.
var opened: PackedStringArray = []
var cell_selected := false
## Groupe de phrases que la voix est en train de dire, ou -1.
var voice_id := -1

func _ready() -> void:
	custom_minimum_size = Vector2(640, 0)
	add_theme_stylebox_override("panel", Atlas.cartouche(Color(Atlas.PAPER, 0.94), Color(Atlas.INK, 0.7), 1, false))
	var h := HBoxContainer.new()
	add_child(h)
	label = Atlas.text("", 17, true)
	label.custom_minimum_size.x = 520
	label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	h.add_child(label)
	see_button = Atlas.button(App.t("go_see"), func():
		if not current_event.is_empty():
			go_to.emit(current_event)
		dismiss())
	h.add_child(see_button)
	h.add_child(Atlas.button("×", dismiss, App.t("close")))
	visible = false
	if Son.has_signal("phrase_finie"):
		Son.connect("phrase_finie", _voice_done)

func _voice_done(id: int) -> void:
	if id == voice_id:
		voice_id = -1

func enabled() -> bool:
	return bool(App.settings["narrator"])

func seen() -> PackedStringArray:
	return PackedStringArray(App.settings.get("guide_vu", []))

func _mark_seen(ids: PackedStringArray) -> void:
	var s := seen()
	var changed := false
	for id in ids:
		if not id in s:
			s.append(id)
			changed = true
	if changed:
		App.settings["guide_vu"] = Array(s)
		App.save_settings()

## Première partie guidée : le guide recommence au début.
func restart() -> void:
	App.settings["guide_vu"] = []
	App.settings["narrator"] = true
	App.save_settings()

## Le joueur a ouvert un outil : sa présentation devient inutile.
func tool_opened(tool: String) -> void:
	if not tool in opened:
		opened.append(tool)
	if tool == current_tool and current != "":
		dismiss()

func _process(delta: float) -> void:
	if visible:
		clock += delta
		# La phrase reste à l'écran tant que la voix la dit.
		if clock >= SHOW_SECONDS and (voice_id < 0 or clock >= VOICE_MAX_SECONDS):
			dismiss()
		return
	quiet -= delta
	poll -= delta
	if quiet > 0.0 or poll > 0.0 or not enabled():
		return
	poll = 1.0
	var d: Dictionary = App.session.guide_next(seen(), opened, cell_selected)
	if d.is_empty():
		return
	var known: PackedStringArray = d.get("known", PackedStringArray())
	if not known.is_empty():
		_mark_seen(known)
	if d.has("id"):
		_show(d["text"], d["tool"], {})
		current = d["id"]
		_mark_seen(PackedStringArray([current]))

## Un événement vient d'arriver : le conseiller le signale peut-être.
func consider(e: Dictionary) -> void:
	if not enabled() or visible or quiet > 0.0:
		return
	var now := Time.get_ticks_msec() / 1000.0
	if now - last_advice < ADVICE_GAP:
		return
	var text: String = App.session.advice_for(e)
	if text == "":
		return
	last_advice = now
	current = "conseil"
	_show(text, "", e)

func _show(text: String, tool: String, event: Dictionary) -> void:
	label.text = text
	current_tool = tool
	current_event = event
	see_button.visible = not event.is_empty()
	clock = 0.0
	visible = true
	highlight.emit(tool)
	# La voix, si le module son sait parler ; le texte reste dans ce
	# cartouche (pas de bandeau du module son).
	voice_id = -1
	if Son.has_method("dire"):
		voice_id = int(Son.call("dire", text, VOICE_PRIORITY))

func dismiss() -> void:
	if not visible:
		return
	visible = false
	voice_id = -1
	current = ""
	current_tool = ""
	current_event = {}
	quiet = GAP_SECONDS
	highlight.emit("")
