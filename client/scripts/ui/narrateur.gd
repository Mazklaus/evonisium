extends PanelContainer
## Narrateur discret de la première partie (décision du 8 octobre 2026) :
## une phrase à la fois, en bas de l'écran, déclenchée par l'état de la
## partie ; jamais bloquant, désactivable dans les réglages.

const HINTS := [
	{"id": "temps", "fr": "La vie est déposée. Le temps file à 100 000 ans par seconde : la barre du haut l'accélère, l'arrête, ou saute au prochain événement.", "en": "Life is seeded. Time runs at 100,000 years per second: the top bar speeds it up, stops it, or jumps to the next event."},
	{"id": "cellule", "fr": "Cliquez sur la mer pour ouvrir l'inspecteur : le milieu, les populations, et la loupe sur les cellules.", "en": "Click the sea to open the inspector: the environment, the populations, and the magnifier on the cells."},
	{"id": "calque", "fr": "Les calques posent une donnée en lavis sur le globe. Essayez « Biomasse », puis revenez à la vue naturelle.", "en": "Layers lay one quantity as a wash on the globe. Try “Biomass”, then return to the natural view."},
	{"id": "espece", "fr": "Une nouvelle lignée est apparue. L'arbre du vivant (touche T) montre d'où elle vient.", "en": "A new lineage has appeared. The tree of life (T key) shows where it comes from."},
	{"id": "pigment", "fr": "Des cellules captent la lumière avec un pigment. Si l'une d'elles casse l'eau, l'oxygène va monter : suivez-le en haut, et sur la frise.", "en": "Cells now harvest light with a pigment. If one of them splits water, oxygen will rise: watch it at the top and on the timeline."},
	{"id": "oxygene", "fr": "L'oxygène s'accumule dans l'air. Vous pouvez aider ou gêner la vie : le bouton Interventions agit sur l'environnement, jamais sur les gènes.", "en": "Oxygen is building up in the air. You can help or hinder life: Interventions act on the environment, never on genes."},
]

var label: Label
var shown := {}
var current := ""

func _ready() -> void:
	custom_minimum_size = Vector2(620, 0)
	add_theme_stylebox_override("panel", Atlas.cartouche(Color(Atlas.PAPER, 0.94), Color(Atlas.INK, 0.7), 1, false))
	var h := HBoxContainer.new()
	add_child(h)
	label = Atlas.text("", 17, true)
	label.custom_minimum_size.x = 560
	label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	h.add_child(label)
	h.add_child(Atlas.button("×", dismiss, App.t("close")))
	visible = false

func trigger(id: String) -> void:
	if not bool(App.settings["narrator"]) or shown.has(id):
		return
	for hint in HINTS:
		if hint["id"] == id:
			shown[id] = true
			current = id
			label.text = hint["en"] if App.settings["lang"] == "en" else hint["fr"]
			visible = true
			var tw := create_tween()
			tw.tween_interval(16.0)
			tw.tween_callback(func(): if current == id: dismiss())
			return

func dismiss() -> void:
	visible = false
	current = ""
