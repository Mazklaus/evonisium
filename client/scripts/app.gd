extends Node
## État global du client : la session (pont vers le moteur Rust), les
## réglages du joueur, les textes de l'interface en français et en anglais,
## et le passage d'un écran à l'autre.

signal settings_changed
signal screen_requested(name: String, params: Dictionary)

const SETTINGS_PATH := "user://reglages.cfg"
const SAVE_DIR := "user://sauvegardes"

var session: EvoSession
var globe: Node3D
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
	"ground_down": ["Descendre au sol", "Go down to the ground"],
	"ground_title": ["Au sol", "On the ground"],
	"ground_extras": ["Figurants d'essai : le moteur ne publie pas encore d'individus. Ces corps viennent de plans de construction de banc d'essai et leurs gestes ne changent pas l'histoire.", "Test extras: the engine does not publish individuals yet. These bodies come from bench body plans and what they do never changes history."],
	"ground_wait": ["Préparation de la scène…", "Preparing the scene…"],
	"ground_up": ["Remonter", "Back up"],
	"ground_free": ["Survol libre", "Free flight"],
	"ground_follow": ["Suivre", "Follow"],
	"ground_next": ["Espèce suivante", "Next species"],
	"ground_action": ["Action :", "Action:"],
	"ground_free_behaviour": ["Comportement libre", "Free behaviour"],
	"ground_free_text": ["Survol libre : flèches pour avancer, clic sur un individu pour le suivre.", "Free flight: arrows to move, click an individual to follow it."],
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
	"intervene_pick": ["Choisissez d'abord un lieu sur le globe (clic sur une cellule), puis rouvrez cette fiche.", "First pick a place on the globe (click a cell), then reopen this sheet."],
	"lineages_short": ["Lignées", "Lineages"],
	"influence_short": ["Influence", "Influence"],
	"ecotypes": ["écotypes", "ecotypes"],
	"intervene_where": ["Lieu", "Place"],
	"radius": ["Rayon", "Radius"],
	"cost": ["Coût", "Cost"],
	"points": ["points", "points"],
	"per_myr": ["par Ma", "per Myr"],
	"not_enough": ["réserve insuffisante", "not enough influence"],
	"sandbox": ["bac à sable, sans limite", "sandbox, unlimited"],
	"refused": ["Intervention refusée", "Intervention refused"],
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
	"preset_terre": ["Terre archéenne", "Archean Earth"],
	"preset_ocean": ["Monde océan", "Ocean world"],
	"preset_desert": ["Monde désertique", "Desert world"],
	"preset_super-terre": ["Super-Terre", "Super-Earth"],
	"preset_petite": ["Petite planète", "Small planet"],
	"preset_sans-lune": ["Monde sans lune", "Moonless world"],
	"preset_star": ["Étoile du préréglage", "Preset star"],
	"code": ["Code de la planète", "Planet code"],
	"code_hint": ["À partager : il recrée la même planète.", "Share it: it recreates the same planet."],
	"level_4": ["2 562 cellules (rapide)", "2,562 cells (fast)"],
	"level_5": ["10 242 cellules (recommandé)", "10,242 cells (recommended)"],
	"level_6": ["40 962 cellules (lent)", "40,962 cells (slow)"],
	"generating": ["Formation de la planète…", "Forming the planet…"],
	"ocean_share": ["Océan", "Ocean"],
	"ice": ["Glace", "Ice"],
	"co2": ["CO₂", "CO₂"],
	"methane": ["Méthane", "Methane"],
	"speed": ["Vitesse", "Speed"],
	"paused": ["En pause", "Paused"],
	"auto_paused": ["Le temps s'est arrêté sur un événement", "Time stopped on an event"],
	"seeking": ["Recherche du prochain événement…", "Looking for the next event…"],
	"history": ["Frise", "Timeline"],
	"living": ["vivante", "living"],
	"extinct": ["éteinte", "extinct"],
	"metabolism": ["Métabolisme", "Metabolism"],
	"age": ["Âge", "Age"],
	"born": ["Apparue il y a", "Appeared"],
	"marked": ["Lignée suivie : son aire est hachurée de vermillon.", "Followed lineage: its range is hatched in vermilion."],
	"name_save": ["Nom du point de sauvegarde", "Save point name"],
	"autosave_name": ["sauvegarde automatique", "autosave"],
	"saves": ["Points de sauvegarde", "Save points"],
	"open": ["Ouvrir", "Open"],
	"profile": ["Profil", "Profile"],
	"profile_contemplatif": ["Contemplatif : rien ne m'arrête", "Contemplative: nothing stops me"],
	"profile_naturaliste": ["Naturaliste : les grands événements", "Naturalist: major events"],
	"profile_tout": ["Tout voir : chaque nouveauté", "See everything: every novelty"],
	"action_0": ["Ignorer", "Ignore"],
	"action_1": ["Noter", "Note"],
	"action_2": ["Alerter", "Alert"],
	"action_3": ["Ralentir", "Slow down"],
	"action_4": ["Pause", "Pause"],
	"intervene_text": ["Le joueur agit sur l'environnement, jamais sur les gènes. Chaque intervention entre dans la file d'ordres et dans la chronique.", "The player acts on the environment, never on genes. Each intervention enters the order queue and the chronicle."],
	"sent": ["Intervention transmise au moteur", "Intervention sent to the engine"],
	"preview_phosphate": ["Les mers reçoivent du phosphate : les producteurs, limités par ce nutriment, devraient croître, et avec eux l'oxygène s'ils photosynthétisent.", "The seas receive phosphate: producers limited by this nutrient should grow, and oxygen with them if they photosynthesise."],
	"preview_eruption": ["Un volcan rejette du CO₂ : effet de serre plus fort, planète plus chaude ; le carbone nourrit aussi les autotrophes.", "A volcano releases CO₂: stronger greenhouse, warmer planet; the carbon also feeds autotrophs."],
	"preview_methane": ["Du méthane s'échappe : effet de serre puissant mais bref, brume orangée si l'air en est riche.", "Methane escapes: a strong but short-lived greenhouse, orange haze if the air is rich in it."],
	# Étape 4 : interventions du monde multicellulaire.
	"impact": ["Impact météoritique", "Meteorite impact"],
	"bras_de_mer": ["Isoler par un bras de mer", "Isolate with a sea strait"],
	"montagnes": ["Isoler par une chaîne de montagnes", "Isolate with a mountain range"],
	"climat": ["Poussée climatique", "Climate pulse"],
	"diameter": ["Diamètre", "Diameter"],
	"length": ["Longueur", "Length"],
	"orientation": ["Orientation", "Bearing"],
	"duration": ["Durée", "Duration"],
	"delta_t": ["Écart de température", "Temperature shift"],
	"rain_factor": ["Pluie", "Rain"],
	"before_intervention": ["avant intervention", "before intervention"],
	"preview_impact": ["", ""],
	"preview_bras_de_mer": ["", ""],
	"preview_montagnes": ["", ""],
	"preview_climat": ["", ""],
	# Étape 4 : avec et sans, réseau trophique, colonne stratigraphique.
	"with_without": ["Avec et sans", "With and without"],
	"with_without_text": ["Rejouez la planète depuis le point de sauvegarde écrit juste avant une intervention, sur la même graine, sans elle, puis comparez les deux histoires. Sans intervention retirée, le rejeu redonnerait exactement la même partie.", "Replay the planet from the save point written just before an intervention, on the same seed, without it, then compare the two histories. Without anything removed, the replay would give exactly the same game."],
	"branch_pauses_only": ["Rejouer seulement pendant les pauses (sinon, en même temps que la partie, qui ralentit)", "Replay only while paused (otherwise alongside the game, which slows down)"],
	"and_without": ["Et sans ?", "And without?"],
	"no_intervention": ["Aucune intervention pour l'instant.", "No intervention yet."],
	"branch_none": ["Aucune branche en cours.", "No branch running."],
	"branch_stop": ["Arrêter la branche", "Stop the branch"],
	"branch_running": ["Sans « %s » : rejeu en cours, %s sur %s.", "Without “%s”: replaying, %s of %s."],
	"branch_done": ["Sans « %s » : à jour au %s.", "Without “%s”: up to date at %s."],
	"branch_map": ["Voir l'écart de biomasse sur le globe", "Show the biomass difference on the globe"],
	"quantity": ["Grandeur", "Quantity"],
	"with": ["Avec", "With"],
	"without": ["Sans", "Without"],
	"difference": ["Écart", "Difference"],
	"milestone": ["Jalon", "Milestone"],
	"map_changed": ["Carte : espèce dominante changée", "Map: dominant species changed"],
	"curve_o2": ["O₂ de l'air (log)", "Air O₂ (log)"],
	"curve_biomass": ["Biomasse (log)", "Biomass (log)"],
	"food_web": ["Réseau trophique", "Food web"],
	"food_web_text": ["Qui vit de qui dans cette cellule : les producteurs en bas, ceux qui vivent de leurs produits au-dessus. Trait plein : matière organique ; trait bleu : échange d'un produit (O₂, CH₄, sulfure) ; tireté vermillon : compétition. Les flux sont estimés depuis la production de chaque population.", "Who lives on whom in this cell: producers at the bottom, those living on their products above. Solid line: organic matter; blue line: exchange of a product (O₂, CH₄, sulphide); dashed vermilion: competition. Fluxes are estimated from each population's production."],
	"food_web_empty": ["Pas de vie dans cette cellule.", "No life in this cell."],
	"food_web_legend": ["Cliquez une espèce pour ouvrir sa fiche.", "Click a species to open its sheet."],
	"strata": ["Colonne stratigraphique", "Rock column"],
	"strata_text": ["Les roches que la partie a déposées dans cette région, des plus anciennes (en bas) aux plus récentes. Roche, épaisseur et isotopes sont déduits des conditions simulées : ce que les sciences des peuples pourront un jour lire.", "The rocks this game laid down in this region, oldest at the bottom. Rock type, thickness and isotopes are derived from simulated conditions: what the sciences of future peoples will be able to read."],
	"strata_empty": ["L'historique de la région est encore vide.", "The region's history is still empty."],
	"strata_hint": ["Survolez une couche pour la décrire.", "Hover a layer to describe it."],
	"thickness": ["Épaisseur", "Thickness"],
	"dominant": ["Espèce dominante", "Dominant species"],
	"stromatolites": ["Stromatolithes : tapis microbiens fossilisés.", "Stromatolites: fossil microbial mats."],
	"iridium": ["Couche à iridium : trace d'un impact.", "Iridium layer: the mark of an impact."],
	"anatomy": ["Anatomie", "Anatomy"],
	"anatomy_text": ["Le corps tiré du plan de construction de l'espèce, en vraie grandeur. La vue anatomie voile la peau et montre les organes internes, colorés par appareil.", "The body drawn from the species' body plan, true to scale. Anatomy view veils the skin and shows the internal organs, coloured by system."],
	"test_body": ["plan d'essai", "test plan"],
	"show_organs": ["Vue anatomie (organes internes)", "Anatomy view (internal organs)"],
	"drag_to_turn": ["Glisser pour tourner le corps.", "Drag to turn the body."],
	"compare": ["Comparer", "Compare"],
	"size": ["Taille", "Size"],
	"symmetry": ["Symétrie", "Symmetry"],
	"covering": ["Revêtement", "Covering"],
	"appendages": ["Appendices", "Appendages"],
	"eyes": ["Yeux", "Eyes"],
	"cell_types": ["Types cellulaires", "Cell types"],
	"organs": ["Organes internes", "Internal organs"],
	"no_organs": ["Aucun organe interne visible.", "No visible internal organ."],
	"comparator": ["Comparateur d'espèces", "Species comparator"],
	"compare_with": ["Comparer avec ", "Compare with "],
	"common_ancestor": ["Ancêtre commun", "Common ancestor"],
	"diverged": ["séparées depuis %s", "diverged %s ago"],
	"no_common_ancestor": ["Pas d'ancêtre commun connu dans l'arbre.", "No known common ancestor in the tree."],
	"tree_hint": ["Les lignées éteintes sans descendance sont repliées.", "Extinct lineages without descendants are folded."],
	"no_species": ["Aucune espèce n'est encore apparue.", "No species has appeared yet."],
	"clear_focus": ["Effacer l'aire", "Clear range"],
	"layer_none": ["Aucun calque : vue naturelle", "No layer: natural view"],
	"go_menu": ["Revenir au menu (la partie est sauvegardée)", "Back to menu (the game is saved)"],
	"real_speed_long": ["vitesse tenue", "speed held"],
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
