extends "res://scripts/ui/fiche.gd"
## Galerie de rejeux (étape 7, document Fonctionnalités) : les sauvegardes
## et les rejeux exportés, avec leur planète, leur code à partager et les
## jalons de leur histoire. On reprend une sauvegarde, on rejoue un rejeu
## depuis sa graine, on compare les jalons de deux histoires.

var entries: Array = []  # [{path, kind, info, check}]
var detail: VBoxContainer
var selected := -1
## Ouverte en partie : la partie en cours est sauvegardée (sauvegarde
## automatique) avant d'en reprendre ou d'en rejouer une autre.
var autosave := false

func _ready() -> void:
	set_title(App.t("gallery_title"))
	custom_minimum_size = Vector2(980, 560)
	var h := HBoxContainer.new()
	h.add_theme_constant_override("separation", 14)
	h.size_flags_vertical = Control.SIZE_EXPAND_FILL
	content.add_child(h)
	var left := VBoxContainer.new()
	left.custom_minimum_size.x = 400
	h.add_child(left)
	var hint := Atlas.text(App.t("gallery_hint"), 15, true)
	hint.custom_minimum_size.x = 390
	left.add_child(hint)
	var scroll := ScrollContainer.new()
	scroll.size_flags_vertical = Control.SIZE_EXPAND_FILL
	scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	left.add_child(scroll)
	var list := VBoxContainer.new()
	list.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	scroll.add_child(list)
	var compare := Atlas.button(App.t("gallery_compare"), _compare)
	left.add_child(compare)
	var dscroll := ScrollContainer.new()
	dscroll.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	dscroll.size_flags_vertical = Control.SIZE_EXPAND_FILL
	dscroll.custom_minimum_size = Vector2(540, 470)
	dscroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	h.add_child(dscroll)
	detail = VBoxContainer.new()
	detail.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	dscroll.add_child(detail)
	_collect()
	if entries.is_empty():
		var e := Atlas.text(App.t("gallery_empty"), 17, true)
		e.custom_minimum_size.x = 380
		list.add_child(e)
	for k in entries.size():
		var en: Dictionary = entries[k]
		var row := HBoxContainer.new()
		list.add_child(row)
		var c := CheckBox.new()
		c.tooltip_text = App.t("gallery_compare")
		row.add_child(c)
		en["check"] = c
		var info: Dictionary = en["info"]
		var tag := App.t("gallery_replay") if en["kind"] == "replay" else App.t("gallery_save")
		var b := Atlas.button("%s · %s\n%s, %s" % [info.get("name", en["file"]), tag, info.get("planet", ""), info.get("date", "")], select.bind(k))
		b.alignment = HORIZONTAL_ALIGNMENT_LEFT
		b.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		b.clip_text = true
		row.add_child(b)
	if not entries.is_empty():
		select(0)

## Sauvegardes et rejeux, du plus récent au plus ancien.
func _collect() -> void:
	for s in App.list_saves():
		var d: Dictionary = App.session.describe_save(s["path"])
		if not d.is_empty():
			entries.append({"path": s["path"], "file": s["file"], "time": s["time"], "kind": "save", "info": d})
	for r in App.list_replays():
		var d: Dictionary = App.session.describe_replay(r["path"])
		if not d.has("error"):
			entries.append({"path": r["path"], "file": r["file"], "time": r["time"], "kind": "replay", "info": d})
	entries.sort_custom(func(a, b): return a["time"] > b["time"])

func _clear() -> void:
	for c in detail.get_children():
		c.queue_free()

func select(k: int) -> void:
	selected = k
	_clear()
	var en: Dictionary = entries[k]
	var info: Dictionary = en["info"]
	var replay: bool = en["kind"] == "replay"
	var t := Atlas.title(str(info.get("name", en["file"])), 26)
	t.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	t.custom_minimum_size.x = 500
	detail.add_child(t)
	field(detail, App.t("gallery_replay") if replay else App.t("gallery_save"), str(info.get("planet", "")))
	field(detail, App.t("gallery_reached"), str(info.get("date", "")))
	field(detail, App.t("gallery_interventions"), str(info.get("interventions", 0)))
	var code_row := HBoxContainer.new()
	detail.add_child(code_row)
	var code := Atlas.text("%s : %s" % [App.t("code"), info.get("code", "")], 15, true)
	code.custom_minimum_size.x = 340
	code_row.add_child(code)
	code_row.add_child(Atlas.button(App.t("gallery_copy"), func():
		DisplayServer.clipboard_set(str(info.get("code", "")))
		code.text = App.t("gallery_copied")))
	var actions := HBoxContainer.new()
	detail.add_child(actions)
	if replay:
		actions.add_child(Atlas.button(App.t("gallery_play"), _play.bind(en["path"])))
	else:
		actions.add_child(Atlas.button(App.t("gallery_resume"), _resume.bind(en["path"])))
	if replay:
		var note := Atlas.text(App.t("gallery_replay_note"), 14, true)
		note.custom_minimum_size.x = 500
		detail.add_child(note)
	detail.add_child(Atlas.hsep())
	detail.add_child(Atlas.title(App.t("gallery_milestones"), 22))
	var ms: Array = info.get("milestones", [])
	if ms.is_empty():
		var e := Atlas.text(App.t("gallery_no_milestone"), 15, true)
		e.custom_minimum_size.x = 500
		detail.add_child(e)
	for m in ms:
		field(detail, str(m["date"]), str(m["title"]))

## Les jalons de deux histoires côte à côte, dans l'ordre du premier atteint ;
## une date qui diffère de plus de 1 % passe au vermillon.
func _compare() -> void:
	var picked := []
	for en in entries:
		if (en["check"] as CheckBox).button_pressed:
			picked.append(en)
	if picked.size() != 2:
		return
	_clear()
	var a: Dictionary = picked[0]["info"]
	var b: Dictionary = picked[1]["info"]
	var t := Atlas.title("%s · %s" % [a.get("name", ""), b.get("name", "")], 24)
	t.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	t.custom_minimum_size.x = 500
	detail.add_child(t)
	var rows := {}
	for side in 2:
		for m in (a if side == 0 else b).get("milestones", []):
			var key := str(m["title"])
			if not rows.has(key):
				rows[key] = [null, null]
			if rows[key][side] == null:
				rows[key][side] = m
	var keys := rows.keys()
	var first := func(k) -> float:
		var r: Array = rows[k]
		return float((r[0] if r[0] != null else r[1])["years"])
	keys.sort_custom(func(x, y): return first.call(x) < first.call(y))
	var grid := GridContainer.new()
	grid.columns = 3
	grid.add_theme_constant_override("h_separation", 16)
	detail.add_child(grid)
	for head in [App.t("gallery_milestones"), str(a.get("name", "")), str(b.get("name", ""))]:
		var l := Atlas.text(head, 16, true)
		l.autowrap_mode = TextServer.AUTOWRAP_OFF
		l.clip_text = true
		l.custom_minimum_size.x = 160
		grid.add_child(l)
	for k in keys:
		var r: Array = rows[k]
		var title_cell := Atlas.text(str(k), 16)
		title_cell.custom_minimum_size.x = 220
		grid.add_child(title_cell)
		var differs: bool = r[0] == null or r[1] == null or absf(float(r[0]["years"]) - float(r[1]["years"])) > 0.01 * maxf(absf(float(r[0]["years"])), 1.0)
		for side in 2:
			var cell := Atlas.text(str(r[side]["date"]) if r[side] != null else App.t("gallery_not_reached"), 16)
			cell.autowrap_mode = TextServer.AUTOWRAP_OFF
			cell.custom_minimum_size.x = 140
			if differs:
				cell.add_theme_color_override("font_color", Atlas.VERMILION)
			grid.add_child(cell)

func _leave() -> void:
	if autosave:
		App.session.save(App.save_path(App.t("autosave_name")), App.t("autosave_name"))

func _resume(path: String) -> void:
	_leave()
	App.session.stop()
	App.goto("chargement", {"path": path})

func _play(path: String) -> void:
	_leave()
	if App.session.start_replay(path):
		App.globe.reset()
		App.goto("jeu", {"new": false})
