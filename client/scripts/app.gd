extends Node
## État global du client : la session (pont vers le moteur Rust), les
## réglages du joueur, les textes de l'interface en français et en anglais,
## et le passage d'un écran à l'autre.

signal settings_changed
signal screen_requested(name: String, params: Dictionary)

const SETTINGS_PATH := "user://reglages.cfg"
const SAVE_DIR := "user://sauvegardes"

var session: EvoSession
var settings := {
	"lang": "fr",
	"ui_scale": 1.0,
	"text_scale": 1.0,
	"vision": 0,
	"readable_font": false,
	"celsius": true,
	"reduce_motion": false,
	"stop_profile": "naturaliste",
	"autosave_minutes": 10,
	"level": 5,
	"relief": 1.0,
	"graticule": true,
	"terminator": false,
	"narrator": true,
}
var current_screen := ""
## Partie demandée par l'écran de création, en attente d'ensemencement.
var pending := {}

const TEXT := {
	"title": ["Evonisium", "Evonisium"],
	"subtitle": ["Atlas d'un monde qui s'invente", "Atlas of a self-inventing world"],
	"continue": ["Continuer la dernière partie", "Continue last game"],
	"new_game": ["Nouvelle partie", "New game"],
	"load": ["Charger", "Load"],
	"settings": ["Réglages", "Settings"],
	"quit": ["Quitter", "Quit"],
	"back": ["Retour", "Back"],
	"create_title": ["Création de la planète", "Planet creation"],
	"preset": ["Préréglage", "Preset"],
	"seed": ["Graine", "Seed"],
	"random": ["Au hasard", "Random"],
	"orbit": ["Distance à l'étoile", "Distance to star"],
	"water": ["Part d'eau", "Water inventory"],
	"star": ["Température de l'étoile", "Star temperature"],
	"grid": ["Grille de simulation", "Simulation grid"],
	"generate": ["Générer", "Generate"],
	"to_seeding": ["Vers l'ensemencement", "To seeding"],
	"seeding_title": ["Ensemencement", "Seeding"],
	"seeding_text": ["La partie commence avec une cellule minimale : une chimioautotrophe qui tire son énergie de l'hydrogène des sources chaudes. Choisissez où la déposer.", "The game starts with a minimal cell: a chemoautotroph living on hydrogen from hot springs. Choose where to place it."],
	"seed_vents": ["Près des sources hydrothermales (recommandé)", "Near hydrothermal vents (recommended)"],
	"seed_all": ["Dans toutes les mers", "In every sea"],
	"seed_engine": ["Laisser le moteur choisir", "Let the engine choose"],
	"start_life": ["Déposer la vie et commencer", "Seed life and begin"],
	"vents_shown": ["Les sources chaudes sont cerclées de vermillon.", "Hot springs are circled in vermilion."],
	"pause": ["Pause", "Pause"],
	"play": ["Lecture", "Play"],
	"next_event": ["Aller au prochain", "Go to next"],
	"layers": ["Calques", "Layers"],
	"natural": ["Vue naturelle", "Natural view"],
	"raw_cells": ["Données brutes (cellules)", "Raw data (cells)"],
	"inspector": ["Inspecteur", "Inspector"],
	"species": ["Fiche d'espèce", "Species sheet"],
	"tree": ["Arbre du vivant", "Tree of life"],
	"chronicle": ["Chronique", "Chronicle"],
	"interventions": ["Interventions", "Interventions"],
	"save": ["Sauvegarder", "Save"],
	"saved": ["Point de sauvegarde écrit", "Save point written"],
	"menu": ["Menu", "Menu"],
	"populations": ["Populations", "Populations"],
	"follow": ["Suivre la lignée", "Follow lineage"],
	"followed": ["Lignée suivie", "Lineage followed"],
	"show_range": ["Montrer l'aire", "Show range"],
	"microscope": ["Loupe", "Magnifier"],
	"habitat": ["Milieu de vie", "Habitat"],
	"traits": ["Traits", "Traits"],
	"status": ["Statut", "Status"],
	"origin": ["Origine", "Origin"],
	"ancestor": ["Ancêtre", "Ancestor"],
	"range": ["Aire", "Range"],
	"biomass": ["Biomasse", "Biomass"],
	"oxygen": ["Oxygène", "Oxygen"],
	"temperature": ["Température", "Temperature"],
	"lineages": ["Lignées vivantes", "Living lineages"],
	"climate": ["Climat", "Climate"],
	"real_speed": ["tenue", "held"],
	"filter_all": ["Tout", "All"],
	"search": ["Rechercher…", "Search…"],
	"go_see": ["Aller voir", "Go see"],
	"explain": ["Expliquer", "Explain"],
	"ignore_type": ["Ignorer ce type", "Ignore this type"],
	"stop_rules": ["Règles d'arrêt", "Stop rules"],
	"intervene": ["Intervenir", "Intervene"],
	"phosphate": ["Apport de nutriments (phosphate)", "Nutrient supply (phosphate)"],
	"eruption": ["Éruption volcanique (CO₂)", "Volcanic eruption (CO₂)"],
	"amount": ["Ampleur", "Magnitude"],
	"preview": ["Aperçu", "Preview"],
	"confirm": ["Confirmer", "Confirm"],
	"cancel": ["Annuler", "Cancel"],
	"influence": ["Réserve d'influence", "Influence reserve"],
	"influence_pending": ["La réserve d'influence arrive avec le volet moteur de l'étape 3 : en attendant, les interventions sont libres.", "The influence reserve arrives with the engine side of step 3: until then interventions are free."],
	"loading": ["Rejeu de la partie…", "Replaying the game…"],
	"lang": ["Langue", "Language"],
	"ui_scale": ["Taille de l'interface", "Interface size"],
	"text_scale": ["Taille du texte", "Text size"],
	"vision": ["Vision des couleurs", "Colour vision"],
	"readable_font": ["Police très lisible", "Highly legible font"],
	"celsius": ["Degrés Celsius (sinon kelvins)", "Degrees Celsius (otherwise kelvins)"],
	"reduce_motion": ["Réduire les mouvements", "Reduce motion"],
	"autosave": ["Sauvegarde automatique (minutes)", "Autosave (minutes)"],
	"relief": ["Relief exagéré", "Relief exaggeration"],
	"graticule": ["Graticule", "Graticule"],
	"terminator": ["Jour et nuit (sinon lumière d'atelier)", "Day and night (otherwise studio light)"],
	"narrator": ["Conseils du narrateur", "Narrator hints"],
	"accept": ["Appliquer", "Apply"],
	"no_save": ["Aucune partie sauvegardée.", "No saved game."],
	"cell": ["Cellule", "Cell"],
	"none": ["aucune", "none"],
	"close": ["Fermer", "Close"],
	"major": ["Événement majeur", "Major event"],
}

const VISION_MODES := [["Standard", "Standard"], ["Protanopie", "Protanopia"], ["Deutéranopie", "Deuteranopia"], ["Tritanopie", "Tritanopia"], ["Contraste élevé", "High contrast"]]

func _ready() -> void:
	DirAccess.make_dir_recursive_absolute(SAVE_DIR)
	load_settings()
	session = EvoSession.new()
	apply_settings()

func t(key: String) -> String:
	var row = TEXT.get(key)
	if row == null:
		return key
	return row[1] if settings["lang"] == "en" else row[0]

func lang_index() -> int:
	return 1 if settings["lang"] == "en" else 0

func text_scale() -> float:
	return float(settings["text_scale"])

func load_settings() -> void:
	var cfg := ConfigFile.new()
	if cfg.load(SETTINGS_PATH) != OK:
		return
	for k in settings.keys():
		settings[k] = cfg.get_value("reglages", k, settings[k])

func save_settings() -> void:
	var cfg := ConfigFile.new()
	for k in settings.keys():
		cfg.set_value("reglages", k, settings[k])
	cfg.save(SETTINGS_PATH)

func apply_settings() -> void:
	Atlas.rebuild_theme(float(settings["text_scale"]), bool(settings["readable_font"]), int(settings["vision"]) == 4)
	get_tree().root.content_scale_factor = float(settings["ui_scale"])
	if session:
		session.set_language(settings["lang"])
		session.set_celsius(bool(settings["celsius"]))
		session.set_rules_profile(settings["stop_profile"])
	settings_changed.emit()

func goto(screen: String, params: Dictionary = {}) -> void:
	screen_requested.emit(screen, params)

## Points de sauvegarde, du plus récent au plus ancien.
func list_saves() -> Array:
	var out := []
	var dir := DirAccess.open(SAVE_DIR)
	if dir == null:
		return out
	for f in dir.get_files():
		if f.ends_with(".evo"):
			var path := SAVE_DIR + "/" + f
			out.append({"path": path, "file": f, "time": FileAccess.get_modified_time(path)})
	out.sort_custom(func(a, b): return a["time"] > b["time"])
	return out

func save_path(name: String) -> String:
	var clean := name.to_lower().replace(" ", "-").validate_filename()
	return ProjectSettings.globalize_path("%s/%s.evo" % [SAVE_DIR, clean])
