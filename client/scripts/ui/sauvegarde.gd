extends "res://scripts/ui/fiche.gd"
## Sauvegarder (document Fonctionnalités, « Sauvegardes ») : un point de
## sauvegarde nommé, qui garde la graine, la planète et la file d'ordres ;
## la partie se reprend par rejeu, à l'identique.

var name_edit: LineEdit

func _ready() -> void:
	set_title(App.t("save"))
	custom_minimum_size = Vector2(520, 0)
	var info: Dictionary = App.session.frame_info()
	var h := HBoxContainer.new()
	content.add_child(h)
	var l := Atlas.text(App.t("name_save"), 17, true)
	l.autowrap_mode = TextServer.AUTOWRAP_OFF
	h.add_child(l)
	name_edit = LineEdit.new()
	name_edit.text = "%s, %s" % [info.get("planet", "Evonisium"), info.get("date", "")]
	name_edit.custom_minimum_size.x = 260
	name_edit.text_submitted.connect(func(_t): _save())
	h.add_child(name_edit)
	var code := Atlas.text("%s : %s" % [App.t("code"), App.session.planet_code()], 15, true)
	code.custom_minimum_size.x = 480
	content.add_child(code)
	var actions := HBoxContainer.new()
	content.add_child(actions)
	actions.add_child(Atlas.button(App.t("save"), _save))
	actions.add_child(Atlas.button(App.t("cancel"), close))
	name_edit.grab_focus.call_deferred()

func _save() -> void:
	var n := name_edit.text.strip_edges()
	if n == "":
		return
	App.session.save(App.save_path(n), n)
	close()
