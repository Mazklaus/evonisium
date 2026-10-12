extends "res://scripts/ui/fiche.gd"
## Arbre du vivant fractal, à la manière d'OneZoom (document Fonctionnalités,
## « Arbre du vivant ») : chaque clade se divise en deux branches dont la
## taille suit le nombre de lignées vivantes qu'elles portent ; les petits
## clades s'enroulent en spirales sur le côté des grands. On zoome à la
## molette autour du pointeur, on déplace l'arbre en le tirant, un double
## clic zoome sur un clade et « Vue d'ensemble » revient à l'arbre entier.
## Les feuilles sont les lignées vivantes : un clic ouvre la fiche de
## l'espèce. La forme ne dit que la parenté ; les dates sont dans
## l'infobulle. La forme est calculée par evo_view::fractal.

signal species_requested(species: int)

## Couleurs des grands clades, à l'aquarelle sur le papier.
const CLADE_COLORS := [Color("#4A3322"), Color("#9A3B22"), Color("#4E6E86"), Color("#6F8445"), Color("#B07D33"), Color("#7A4E6A"), Color("#3F7A72"), Color("#8C5A3A")]
## Épaisseur d'une branche, en part de sa longueur.
const WIDTH := 0.08
const LEAF_RADIUS := 0.38
## Tailles à l'écran (px) à partir desquelles on écrit un nom.
const LEAF_LABEL_PX := 9.0
const CLADE_LABEL_PX := 150.0

var canvas: Control
var tip: Label
var data := {}
var count := 0
var from: PackedVector2Array
var to: PackedVector2Array
var length: PackedFloat32Array
var bbox: PackedFloat32Array
var ids: PackedInt32Array
var leaves: PackedInt32Array
var next_of: PackedInt32Array
var flags: PackedByteArray
var clade: PackedByteArray
## Vue : centre (coordonnées de l'arbre) et zoom (px par unité).
var centre := Vector2.ZERO
var zoom := 300.0
var fitted := false
var dragging := false
var drag_moved := false
var hover := -1
var names := {}
var refresh_timer := 0.0
var idle := 0.0
var tween: Tween

func _ready() -> void:
	set_title(App.t("tree"))
	custom_minimum_size = Vector2(1080, 0)
	var bar := HBoxContainer.new()
	content.add_child(bar)
	tip = Atlas.text(App.t("tree_hint"), 15, true)
	tip.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	tip.custom_minimum_size.x = 700
	bar.add_child(tip)
	bar.add_child(Atlas.button(App.t("tree_overview"), _overview))
	canvas = Control.new()
	canvas.custom_minimum_size = Vector2(1050, 640)
	canvas.clip_contents = true
	canvas.mouse_filter = Control.MOUSE_FILTER_STOP
	canvas.draw.connect(_draw_tree)
	canvas.gui_input.connect(_input_tree)
	canvas.resized.connect(func():
		if not fitted:
			_overview(false))
	content.add_child(canvas)
	_reload()

func _reload() -> void:
	data = App.session.tree_fractal()
	count = int(data.get("count", 0))
	if count == 0:
		canvas.queue_redraw()
		return
	from = data["from"]
	to = data["to"]
	length = data["length"]
	bbox = data["bbox"]
	ids = data["ids"]
	leaves = data["leaves"]
	next_of = data["next"]
	flags = data["flags"]
	clade = data["clade"]
	if hover >= count:
		hover = -1
	if not fitted:
		_overview(false)
	canvas.queue_redraw()

func _process(delta: float) -> void:
	# L'arbre suit la partie, sauf pendant que le joueur le manipule.
	idle += delta
	refresh_timer -= delta
	if refresh_timer <= 0.0 and idle > 2.0 and not dragging:
		refresh_timer = 4.0
		_reload()

# ----------------------------------------------------------------------
# Vue

func _screen(p: Vector2) -> Vector2:
	return (p - centre) * zoom + canvas.size / 2.0

func _world(s: Vector2) -> Vector2:
	return (s - canvas.size / 2.0) / zoom + centre

## Cadre la vue sur le sous-arbre `i` (la racine par défaut).
func _frame(i: int, animate := true) -> void:
	if count == 0 or canvas.size.x < 2.0:
		return
	var b := Rect2(Vector2(bbox[4 * i], bbox[4 * i + 1]), Vector2(bbox[4 * i + 2] - bbox[4 * i], bbox[4 * i + 3] - bbox[4 * i + 1]))
	var z: float = 0.92 * min(canvas.size.x / max(b.size.x, 1e-9), canvas.size.y / max(b.size.y, 1e-9))
	_go(b.get_center(), z, animate)

func _overview(animate := true) -> void:
	if count == 0 or canvas.size.x < 2.0:
		return
	fitted = true
	_frame(0, animate)

func _go(c: Vector2, z: float, animate: bool) -> void:
	if tween:
		tween.kill()
	if not animate or bool(App.settings["reduce_motion"]):
		centre = c
		zoom = z
		canvas.queue_redraw()
		return
	var c0 := centre
	var z0 := zoom
	tween = create_tween()
	# Zoom géométrique : le mouvement paraît régulier à toutes les échelles.
	tween.tween_method(func(t: float):
		zoom = z0 * pow(z / z0, t)
		centre = c0.lerp(c, t)
		canvas.queue_redraw(), 0.0, 1.0, 0.45).set_trans(Tween.TRANS_SINE).set_ease(Tween.EASE_IN_OUT)

## Zoom de `factor` en gardant fixe le point sous le pointeur.
func _zoom_at(s: Vector2, factor: float) -> void:
	if tween:
		tween.kill()
	var w := _world(s)
	zoom = clampf(zoom * factor, 20.0, 1.0e8)
	centre = w - (s - canvas.size / 2.0) / zoom
	canvas.queue_redraw()

# ----------------------------------------------------------------------
# Dessin

func _visible_rect(i: int) -> Rect2:
	var a := _screen(Vector2(bbox[4 * i], bbox[4 * i + 1]))
	var b := _screen(Vector2(bbox[4 * i + 2], bbox[4 * i + 3]))
	return Rect2(a, b - a)

func _draw_tree() -> void:
	var font := Atlas.body_font()
	if count == 0:
		canvas.draw_string(font, Vector2(20, 40), App.t("no_species"), HORIZONTAL_ALIGNMENT_LEFT, -1, int(18 * App.text_scale()), Atlas.INK)
		return
	var view := Rect2(Vector2.ZERO, canvas.size).grow(4.0)
	var fs := int(13 * App.text_scale())
	var labels := []
	var i := 0
	while i < count:
		var r := _visible_rect(i)
		# Clade hors champ ou plus petit qu'un point : sauté en entier.
		if not view.intersects(r) or max(r.size.x, r.size.y) < 1.2:
			if max(r.size.x, r.size.y) >= 0.3 and view.intersects(r):
				canvas.draw_rect(Rect2(r.get_center() - Vector2(0.6, 0.6), Vector2(1.2, 1.2)), _colour(i))
			i = next_of[i]
			continue
		var a := _screen(from[i])
		var b := _screen(to[i])
		var px := length[i] * zoom
		var c := _colour(i)
		if i == hover:
			c = Atlas.VERMILION
		canvas.draw_line(a, b, c, clampf(px * WIDTH, 0.7, 14.0), px > 6.0)
		var f := flags[i]
		if f & 1:
			var rad := px * LEAF_RADIUS
			if rad >= 0.7:
				canvas.draw_circle(b, rad, c)
				if rad > 4.0:
					canvas.draw_arc(b, rad, 0.0, TAU, 24, Color(Atlas.PAPER, 0.6), 1.0, true)
			if rad >= LEAF_LABEL_PX:
				var right := b.x >= a.x
				labels.append([i, b + Vector2(rad + 5.0 if right else -rad - 5.0, 0.0), 1000.0 + rad, rad, right])
		elif f & 4 and px >= CLADE_LABEL_PX:
			var mid := (a + b) / 2.0
			labels.append([i, mid + Vector2(clampf(px * WIDTH, 0.7, 14.0) / 2.0 + 6.0, 0.0), px, -1.0, true])
		i += 1
	# Les noms par-dessus les branches, les plus grands d'abord ; un nom qui
	# en chevaucherait un autre attend qu'on zoome.
	labels.sort_custom(func(x, y): return float(x[2]) > float(y[2]))
	var placed: Array[Rect2] = []
	for l in labels:
		var k: int = l[0]
		var at: Vector2 = l[1]
		var clade_label := float(l[3]) < 0.0
		var font_px := fs if clade_label else int(clampf(float(l[3]) * 0.7, 12.0, 18.0) * App.text_scale())
		var text := _name(ids[k])
		if clade_label:
			text = "%s · %d %s" % [text, leaves[k], App.t("tree_living")]
		var w := font.get_string_size(text, HORIZONTAL_ALIGNMENT_LEFT, -1, font_px).x
		# À droite de la feuille quand la branche part vers la droite, sinon à gauche.
		var x := at.x if bool(l[4]) else at.x - w
		var box := Rect2(Vector2(x, at.y - font_px * 0.75), Vector2(w, font_px * 1.1))
		var free := true
		for r in placed:
			if r.intersects(box):
				free = false
				break
		if not free:
			continue
		placed.append(box)
		var pos := Vector2(x, at.y + font_px * 0.3)
		canvas.draw_string_outline(font, pos, text, HORIZONTAL_ALIGNMENT_LEFT, -1, font_px, 4, Color(Atlas.PAPER, 0.9))
		canvas.draw_string(font, pos, text, HORIZONTAL_ALIGNMENT_LEFT, -1, font_px, Atlas.INK)

func _colour(i: int) -> Color:
	var c: Color = CLADE_COLORS[clade[i] % CLADE_COLORS.size()]
	return c if flags[i] & 2 else Color(c, 0.45)

func _name(lineage: int) -> String:
	if not names.has(lineage):
		var d: Dictionary = App.session.lineage_label(lineage)
		names[lineage] = str(d.get("name", "n° %d" % lineage))
	return names[lineage]

# ----------------------------------------------------------------------
# Souris

## Noeud sous le point `s` (écran) : une feuille, sinon la branche la plus
## proche à quelques pixels.
func _node_at(s: Vector2) -> int:
	var best := -1
	var best_d := 6.0
	var i := 0
	while i < count:
		var r := _visible_rect(i).grow(8.0)
		if not r.has_point(s) or max(r.size.x, r.size.y) < 17.0:
			i = next_of[i]
			continue
		var a := _screen(from[i])
		var b := _screen(to[i])
		var d := s.distance_to(Geometry2D.get_closest_point_to_segment(s, a, b))
		if flags[i] & 1:
			d = min(d, max(0.0, s.distance_to(b) - length[i] * zoom * LEAF_RADIUS))
		if d <= best_d:
			best_d = d
			best = i
		i += 1
	return best

func _input_tree(event: InputEvent) -> void:
	idle = 0.0
	if event is InputEventMouseButton:
		var mb := event as InputEventMouseButton
		if mb.button_index == MOUSE_BUTTON_WHEEL_UP and mb.pressed:
			_zoom_at(mb.position, 1.2)
		elif mb.button_index == MOUSE_BUTTON_WHEEL_DOWN and mb.pressed:
			_zoom_at(mb.position, 1.0 / 1.2)
		elif mb.button_index == MOUSE_BUTTON_LEFT:
			if mb.double_click:
				var n := _node_at(mb.position)
				if n >= 0:
					_frame(n)
			elif mb.pressed:
				dragging = true
				drag_moved = false
			else:
				dragging = false
				if not drag_moved:
					var n := _node_at(mb.position)
					if n >= 0 and flags[n] & 1:
						var d: Dictionary = App.session.lineage_label(ids[n])
						if not d.is_empty():
							species_requested.emit(int(d["species"]))
		canvas.accept_event()
	elif event is InputEventMouseMotion:
		var mm := event as InputEventMouseMotion
		if dragging:
			if mm.relative.length() > 0.0:
				drag_moved = true
				if tween:
					tween.kill()
				centre -= mm.relative / zoom
				canvas.queue_redraw()
		else:
			var n := _node_at(mm.position)
			if n != hover:
				hover = n
				canvas.queue_redraw()
				tip.text = App.t("tree_hint") if n < 0 else _describe(n)
	elif event is InputEventMagnifyGesture:
		_zoom_at((event as InputEventMagnifyGesture).position, (event as InputEventMagnifyGesture).factor)

func _describe(n: int) -> String:
	var d: Dictionary = App.session.lineage_label(ids[n])
	if d.is_empty():
		return ""
	var since: String = App.session.format_duration(float(App.session.frame_info().get("years", 0.0)) - float(d["born"]))
	if flags[n] & 1:
		return "%s — %s %s" % [d["name"], App.t("born"), since]
	return "%s — %s %s · %d %s" % [d["name"], App.t("born"), since, leaves[n], App.t("tree_living")]
