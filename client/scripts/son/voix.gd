extends Node
## Installation de la voix du narrateur, depuis les réglages : le moteur de
## synthèse sherpa-onnx (licence Apache 2.0) et une voix Piper, téléchargés
## depuis leurs pages GitHub et dépliés avec `tar` (présent sous Windows 10
## et plus, comme sous Linux) dans user://voix/. Rien n'est dans le dépôt.

signal progression(texte: String, part: float)
signal installee
signal echec(texte: String)

const MOTEUR := "v1.12.14"
const MOTEUR_URL := "https://github.com/k2-fsa/sherpa-onnx/releases/download/%s/sherpa-onnx-%s-%s-shared.tar.bz2"
const VOIX_URL := "https://github.com/k2-fsa/sherpa-onnx/releases/download/tts-models/%s.tar.bz2"
## Voix proposées : archive, locuteur, taille annoncée et crédit à afficher.
## Choix de l'utilisateur du 11 octobre 2026 : la voix masculine du premier
## essai (« gilles »), préférée à « pierre » et à « tom ».
const VOIX := {
	"gilles": {"archive": "vits-piper-fr_FR-gilles-low", "locuteur": 0, "mo": 67, "nom": "Gilles",
		"credit": "Voix « gilles », modèle Piper, versée au domaine public (CC0)."},
}

var _http: HTTPRequest
var _etapes: Array = []
var _fichier := ""
var _pid := -1
var _en_cours := ""

func _ready() -> void:
	_http = HTTPRequest.new()
	_http.use_threads = true
	add_child(_http)
	_http.request_completed.connect(_telecharge)
	set_process(false)

static func _plateforme() -> String:
	return "win-x64" if OS.get_name() == "Windows" else "linux-x64"

static func dossier() -> String:
	return OS.get_user_data_dir().path_join("voix")

static func _moteur_dossier() -> String:
	return dossier().path_join("sherpa-onnx-%s-%s-shared" % [MOTEUR, _plateforme()])

static func _exe() -> String:
	var nom := "sherpa-onnx-offline-tts" + (".exe" if OS.get_name() == "Windows" else "")
	return _moteur_dossier().path_join("bin").path_join(nom)

## Chemins de la voix installée (`exe`, `modele`, `locuteur`), ou rien.
func chemins(voix: String) -> Dictionary:
	if not VOIX.has(voix):
		return {}
	var modele := dossier().path_join(VOIX[voix]["archive"])
	if not FileAccess.file_exists(_exe()) or not DirAccess.dir_exists_absolute(modele.path_join("espeak-ng-data")):
		return {}
	return {"exe": _exe(), "modele": modele, "locuteur": VOIX[voix]["locuteur"]}

func est_installee(voix: String) -> bool:
	return not chemins(voix).is_empty()

func en_cours() -> bool:
	return _en_cours != ""

## Taille à télécharger, en mégaoctets.
func taille(voix: String) -> int:
	var mo: int = VOIX[voix]["mo"] if VOIX.has(voix) else 0
	if not FileAccess.file_exists(_exe()):
		mo += 27
	return mo

func installer(voix: String) -> void:
	if en_cours() or not VOIX.has(voix):
		return
	DirAccess.make_dir_recursive_absolute(dossier())
	_etapes.clear()
	if not FileAccess.file_exists(_exe()):
		_etapes.append(MOTEUR_URL % [MOTEUR, MOTEUR, _plateforme()])
	if not DirAccess.dir_exists_absolute(dossier().path_join(VOIX[voix]["archive"]).path_join("espeak-ng-data")):
		_etapes.append(VOIX_URL % VOIX[voix]["archive"])
	_en_cours = voix
	_suivante()

func _suivante() -> void:
	if _etapes.is_empty():
		var voix := _en_cours
		_en_cours = ""
		set_process(false)
		if chemins(voix).is_empty():
			echec.emit("La voix n'a pas pu être installée.")
			return
		progression.emit("Voix installée.", 1.0)
		installee.emit()
		return
	var url: String = _etapes[0]
	_fichier = dossier().path_join(url.get_file())
	_http.download_file = _fichier
	if _http.request(url) != OK:
		_rater("Téléchargement impossible.")
		return
	set_process(true)

func _process(_delta: float) -> void:
	if _pid >= 0:
		if OS.is_process_running(_pid):
			progression.emit("Installation…", 1.0)
			return
		_pid = -1
		DirAccess.remove_absolute(_fichier)
		_etapes.pop_front()
		_suivante()
		return
	var total := _http.get_body_size()
	if total > 0:
		var n := _http.get_downloaded_bytes()
		progression.emit("Téléchargement : %d / %d Mo" % [n / 1000000, total / 1000000], float(n) / float(total))

func _telecharge(result: int, code: int, _h: PackedStringArray, _b: PackedByteArray) -> void:
	if result != HTTPRequest.RESULT_SUCCESS or code != 200:
		_rater("Téléchargement interrompu (%d, %d)." % [result, code])
		return
	# Dépliage en tâche de fond : l'interface ne gèle pas.
	_pid = OS.create_process("tar", ["-xjf", _fichier, "-C", dossier()])
	if _pid < 0:
		_rater("Impossible de déplier l'archive (tar manque).")

func _rater(texte: String) -> void:
	_en_cours = ""
	_etapes.clear()
	_pid = -1
	set_process(false)
	if _fichier != "":
		DirAccess.remove_absolute(_fichier)
	echec.emit(texte)

## Retire les fichiers de la voix (le moteur reste).
func desinstaller(voix: String) -> void:
	if not VOIX.has(voix):
		return
	_supprimer(dossier().path_join(VOIX[voix]["archive"]))

func _supprimer(chemin: String) -> void:
	var d := DirAccess.open(chemin)
	if d == null:
		return
	for f in d.get_files():
		d.remove(f)
	for sub in d.get_directories():
		_supprimer(chemin.path_join(sub))
	DirAccess.remove_absolute(chemin)
