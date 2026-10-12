extends RefCounted
## Fenêtres déplaçables : le joueur attrape une fenêtre par sa poignée (son
## titre) et la pose où il veut ; un double clic sur la poignée la remet à sa
## place d'origine. La place est gardée par fenêtre d'une partie à l'autre
## (réglage « fenetres ») et la fenêtre reste toujours dans l'écran.
##
## `home` pose la fenêtre à sa place d'origine (centrée, ancrée dans un
## coin) ; elle est rappelée tant que le joueur ne l'a pas déplacée, à
## chaque changement de taille de la fenêtre ou de l'écran.

static func attach(target: Control, handle: Control, key: String, home: Callable) -> void:
	var st := {"drag": false, "grab": Vector2.ZERO, "moved": false}
	handle.mouse_filter = Control.MOUSE_FILTER_STOP
	handle.mouse_default_cursor_shape = Control.CURSOR_MOVE
	handle.tooltip_text = App.t("window_drag")
	var fit := func():
		if not target.is_inside_tree():
			return
		if st["moved"]:
			target.global_position = _clamped(target, target.global_position)
		else:
			home.call()
	var saved: Dictionary = App.settings.get("fenetres", {})
	var start := func():
		target.get_viewport().size_changed.connect(fit)
		if saved.has(key):
			var p = saved[key]
			st["moved"] = true
			_unanchor(target)
			target.global_position = Vector2(float(p[0]), float(p[1]))
		fit.call_deferred()
	if target.is_node_ready():
		start.call()
	else:
		target.ready.connect(start, CONNECT_ONE_SHOT)
	target.resized.connect(fit)
	handle.gui_input.connect(func(event: InputEvent):
		if event is InputEventMouseButton and event.button_index == MOUSE_BUTTON_LEFT:
			var mb := event as InputEventMouseButton
			if mb.double_click:
				st["drag"] = false
				st["moved"] = false
				_store(key, null)
				home.call()
			elif mb.pressed:
				st["drag"] = true
				st["grab"] = target.get_global_mouse_position() - target.global_position
			elif st["drag"]:
				st["drag"] = false
				if st["moved"]:
					_store(key, [target.global_position.x, target.global_position.y])
			handle.accept_event()
		elif event is InputEventMouseMotion and st["drag"]:
			if not st["moved"]:
				st["moved"] = true
				_unanchor(target)
			target.global_position = _clamped(target, target.get_global_mouse_position() - st["grab"])
			handle.accept_event())

static func _store(key: String, value) -> void:
	var all: Dictionary = App.settings.get("fenetres", {})
	if value == null:
		all.erase(key)
	else:
		all[key] = value
	App.settings["fenetres"] = all
	App.save_settings()

## Passe d'un ancrage (coin, centre) à une place libre, sans bouger.
static func _unanchor(target: Control) -> void:
	var gp := target.global_position
	target.set_anchors_preset(Control.PRESET_TOP_LEFT)
	target.grow_horizontal = Control.GROW_DIRECTION_END
	target.grow_vertical = Control.GROW_DIRECTION_END
	target.global_position = gp

## La fenêtre reste dans l'écran ; plus grande que lui, son titre reste visible.
static func _clamped(target: Control, p: Vector2) -> Vector2:
	var area := target.get_viewport_rect().size
	var room := area - target.size
	return Vector2(clampf(p.x, min(0.0, room.x), max(0.0, room.x)), clampf(p.y, 0.0, max(0.0, room.y)))

## Place d'origine des fiches : au centre de leur parent.
static func centre(target: Control) -> void:
	var parent := target.get_parent() as Control
	if parent:
		target.position = ((parent.size - target.size) / 2.0).floor()
