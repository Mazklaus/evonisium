extends Node3D
## Le globe de l'Atlas (document Globe 3D, incréments G1 et G2) : la planète
## entière puis le zoom continental, caméra en orbite, calques de données,
## glissement des plaques et interpolation entre deux pas, survol et choix
## d'une cellule. La caméra envoie sa zone d'intérêt au moteur par le canal
## d'observation, qui ne change jamais l'histoire.

signal cell_selected(cell: int)
signal cell_hovered(cell: int)

const MIN_DISTANCE := 1.18
const MAX_DISTANCE := 9.0
const ATLAS_SHADER := preload("res://shaders/globe_atlas.gdshader")
const ATMOSPHERE_SHADER := preload("res://shaders/atmosphere.gdshader")

var planet: MeshInstance3D
var atmosphere: MeshInstance3D
var arrows: MeshInstance3D
## Barrières et anomalies climatiques posées par le joueur (étape 4).
var marks: MeshInstance3D
var marks_key := ""
## Calque « avec et sans » : écart à la branche sans intervention.
var comparison := false
var comparison_tex: ImageTexture
var layer_before_comparison := {}
var camera: Camera3D
var material: ShaderMaterial
var arrow_material: StandardMaterial3D

var yaw := 0.6
var pitch := 0.35
var distance := 4.2
var target_yaw := 0.6
var target_pitch := 0.35
var target_distance := 4.2
## Décalage horizontal de la vue (le globe à droite d'un panneau).
var view_offset := 0.0
var dragging := false
var drag_moved := 0.0
var interactive := true

var textures: Array = [null, null, null]
var prev_textures: Array = [null, null]
var last_images: Array = []
var palette_tex: ImageTexture
var palette_cat_tex: ImageTexture
var blend := 1.0
var blend_duration := 0.25
var last_frame_time := 0.0
var mesh_cells := -1
var mesh_tiles := false
var tiles := false
## Cellules brutes demandées par le joueur ; un calque de catégories les
## montre aussi (une couleur par cellule, sans fondu trompeur).
var raw_tiles := false
var selected_cell := -1
var hover_cell := -1
var layer := {}
var show_plates := false
var observe_timer := 0.0
var last_observation := Vector4.ZERO
var radius_m := 6.371e6
var frames_seen := 0

func _ready() -> void:
	var env := WorldEnvironment.new()
	var e := Environment.new()
	e.background_mode = Environment.BG_COLOR
	e.background_color = Atlas.PAPER
	e.tonemap_mode = Environment.TONE_MAPPER_LINEAR
	env.environment = e
	add_child(env)

	camera = Camera3D.new()
	camera.fov = 38.0
	camera.near = 0.01
	camera.far = 50.0
	add_child(camera)

	material = ShaderMaterial.new()
	material.shader = ATLAS_SHADER
	planet = MeshInstance3D.new()
	planet.material_override = material
	planet.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	add_child(planet)

	var shell := SphereMesh.new()
	shell.radius = 1.06
	shell.height = 2.12
	shell.radial_segments = 96
	shell.rings = 48
	var am := ShaderMaterial.new()
	am.shader = ATMOSPHERE_SHADER
	am.render_priority = 1
	atmosphere = MeshInstance3D.new()
	atmosphere.mesh = shell
	atmosphere.material_override = am
	atmosphere.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	add_child(atmosphere)

	arrow_material = StandardMaterial3D.new()
	arrow_material.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	arrow_material.albedo_color = Atlas.INK
	arrows = MeshInstance3D.new()
	arrows.material_override = arrow_material
	arrows.visible = false
	add_child(arrows)

	var mark_material := StandardMaterial3D.new()
	mark_material.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	mark_material.vertex_color_use_as_albedo = true
	marks = MeshInstance3D.new()
	marks.material_override = mark_material
	add_child(marks)

	App.settings_changed.connect(_apply_settings)
	_apply_settings()
	_place_camera()

func _apply_settings() -> void:
	var mode := int(App.settings["vision"])
	palette_tex = ImageTexture.create_from_image(App.session.palette_image(int(layer.get("palette", 0)), mode))
	palette_cat_tex = ImageTexture.create_from_image(App.session.palette_image(2, mode))
	material.set_shader_parameter("palette", palette_tex)
	material.set_shader_parameter("palette_cat", palette_cat_tex)
	material.set_shader_parameter("show_graticule", bool(App.settings["graticule"]))
	material.set_shader_parameter("show_terminator", bool(App.settings["terminator"]))
	material.set_shader_parameter("ink_strength", 1.25 if mode == 4 else 1.0)
	_update_relief()

func _update_relief() -> void:
	# Relief exagéré quinze fois au réglage 1 (le relief réel est invisible à
	# l'échelle du globe).
	material.set_shader_parameter("relief_scale", float(App.settings["relief"]) * 15.0 / radius_m)

## Prépare le maillage à la taille de la grille de la partie.
func ensure_mesh() -> void:
	var info: Dictionary = App.session.frame_info()
	if info.is_empty():
		return
	var cells := int(info["cells"])
	if cells == mesh_cells and tiles == mesh_tiles:
		return
	var m: Dictionary = App.session.globe_mesh(tiles)
	if m.is_empty():
		return
	var arrays := []
	arrays.resize(Mesh.ARRAY_MAX)
	var verts: PackedVector3Array = m["vertices"]
	arrays[Mesh.ARRAY_VERTEX] = verts
	arrays[Mesh.ARRAY_NORMAL] = verts
	arrays[Mesh.ARRAY_TEX_UV2] = m["uv2"]
	arrays[Mesh.ARRAY_INDEX] = m["indices"]
	var mesh := ArrayMesh.new()
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	# Le relief déplace les sommets dans le shader : boîte élargie.
	mesh.custom_aabb = AABB(Vector3(-1.2, -1.2, -1.2), Vector3(2.4, 2.4, 2.4))
	planet.mesh = mesh
	mesh_cells = cells
	mesh_tiles = tiles
	radius_m = float(info["radius_m"])
	material.set_shader_parameter("radius_m", radius_m)
	_update_relief()
	textures = [null, null, null, null]
	prev_textures = [null, null]
	last_images = []

func reset() -> void:
	mesh_cells = -1
	selected_cell = -1
	hover_cell = -1
	frames_seen = 0
	material.set_shader_parameter("selected_radius", 0.0)
	material.set_shader_parameter("hover_radius", 0.0)

func _set_texture(slot: Array, i: int, img: Image) -> ImageTexture:
	var t: ImageTexture = slot[i]
	if t != null and t.get_width() == img.get_width() and t.get_height() == img.get_height():
		t.update(img)
	else:
		t = ImageTexture.create_from_image(img)
		slot[i] = t
	return t

## Lit le dernier pas publié : textures de données, plaques, atmosphère.
func refresh_frame(force: bool = false) -> void:
	if not App.session.has_frame():
		return
	ensure_mesh()
	if not force and not App.session.has_new_frame():
		return
	var imgs: Array = App.session.frame_textures(false)
	if imgs.size() < 4 or imgs[0] == null:
		return
	# Le pas précédent devient la source de l'interpolation.
	if not last_images.is_empty():
		_set_texture(prev_textures, 0, last_images[0])
		_set_texture(prev_textures, 1, last_images[2])
	else:
		_set_texture(prev_textures, 0, imgs[0])
		_set_texture(prev_textures, 1, imgs[2])
	for i in 4:
		_set_texture(textures, i, imgs[i])
	last_images = imgs
	material.set_shader_parameter("data0", textures[0])
	material.set_shader_parameter("data1", textures[1])
	material.set_shader_parameter("data2", textures[2])
	material.set_shader_parameter("data3", textures[3])
	material.set_shader_parameter("prev0", prev_textures[0])
	material.set_shader_parameter("prev2", prev_textures[1])
	var info: Dictionary = App.session.frame_info()
	material.set_shader_parameter("step_myr", float(info.get("step_years", 0.0)) / 1.0e6)
	var star: Color = info.get("star_colour", Color.WHITE)
	material.set_shader_parameter("star_tint", Vector3(star.r, star.g, star.b))
	var atm: Color = App.session.atmosphere_colour()
	atmosphere.material_override.set_shader_parameter("atmosphere_colour", Vector3(atm.r, atm.g, atm.b))
	# Interpolation sur la durée réelle qui sépare deux pas.
	var now := Time.get_ticks_msec() / 1000.0
	if last_frame_time > 0.0:
		blend_duration = clamp(now - last_frame_time, 0.05, 1.5)
	last_frame_time = now
	frames_seen += 1
	blend = 1.0 if (frames_seen <= 1 or bool(App.settings["reduce_motion"]) or bool(info.get("paused", false))) else 0.0
	material.set_shader_parameter("blend", blend)
	if show_plates:
		_build_arrows()
	_build_marks()
	if comparison:
		_update_comparison()

## Barrières en trait épais (bleu-gris pour un bras de mer, sépia pour des
## montagnes) et calottes des poussées climatiques en tireté vermillon ou
## lavis d'eau.
func _build_marks() -> void:
	var d: Dictionary = App.session.disturbances()
	var barriers: Array = d.get("barriers", [])
	var anomalies: Array = d.get("anomalies", [])
	var key := "%d-%d" % [barriers.size(), anomalies.size()]
	if key == marks_key:
		return
	marks_key = key
	if barriers.is_empty() and anomalies.is_empty():
		marks.mesh = null
		return
	var im := ImmediateMesh.new()
	im.surface_begin(Mesh.PRIMITIVE_LINES)
	for b in barriers:
		var pts: PackedVector3Array = b["points"]
		var c := Atlas.WATER.darkened(0.35) if bool(b["sea"]) else Atlas.INK
		for lift in [1.004, 1.006, 1.008]:
			for i in pts.size() - 1:
				im.surface_set_color(c)
				im.surface_add_vertex(pts[i] * lift)
				im.surface_set_color(c)
				im.surface_add_vertex(pts[i + 1] * lift)
	for a in anomalies:
		if bool(a["global"]):
			continue
		var centre: Vector3 = a["centre"]
		var r: float = a["radius"]
		var c := Atlas.VERMILION if float(a["delta_k"]) > 0.0 else Atlas.WATER.darkened(0.45)
		var side := centre.cross(Vector3.UP if absf(centre.y) < 0.9 else Vector3.RIGHT).normalized()
		var up := centre.cross(side).normalized()
		var n := 72
		for i in n:
			if i % 2 == 1:
				continue
			for j in [i, i + 1]:
				var t := TAU * float(j) / float(n)
				var p := (centre * cos(r) + (side * cos(t) + up * sin(t)) * sin(r)).normalized() * 1.005
				im.surface_set_color(c)
				im.surface_add_vertex(p)
	im.surface_end()
	marks.mesh = im

## Calque « avec et sans » : log₁₀ du rapport des biomasses avec et sans
## l'intervention, en palette divergente.
func set_comparison(on: bool) -> void:
	if on == comparison:
		if on:
			_update_comparison()
		return
	comparison = on
	if on:
		layer_before_comparison = layer
		_update_comparison()
		set_layer({"key": "avec-sans", "texture": 4, "channel": 0, "transform": 0, "min": -2.0, "max": 2.0, "palette": 1})
	else:
		set_layer(layer_before_comparison)

func _update_comparison() -> void:
	var img: Image = App.session.comparison_texture()
	if img == null:
		return
	if comparison_tex != null and comparison_tex.get_width() == img.get_width() and comparison_tex.get_height() == img.get_height():
		comparison_tex.update(img)
	else:
		comparison_tex = ImageTexture.create_from_image(img)
	material.set_shader_parameter("data4", comparison_tex)

## Calque de données (dictionnaire de session.layers()) ou {} : vue naturelle.
func set_layer(l: Dictionary) -> void:
	layer = l
	show_plates = l.get("key", "") == "plaques"
	arrows.visible = show_plates
	if show_plates:
		_build_arrows()
	_update_tiles()
	if l.is_empty():
		material.set_shader_parameter("layer_on", false)
		material.set_shader_parameter("natural_life", true)
		return
	material.set_shader_parameter("layer_on", true)
	material.set_shader_parameter("natural_life", false)
	material.set_shader_parameter("layer_tex", int(l["texture"]))
	material.set_shader_parameter("layer_channel", int(l["channel"]))
	material.set_shader_parameter("layer_transform", int(l["transform"]))
	material.set_shader_parameter("layer_min", float(l["min"]))
	material.set_shader_parameter("layer_max", float(l["max"]))
	material.set_shader_parameter("layer_opacity", 0.6)
	palette_tex = ImageTexture.create_from_image(App.session.palette_image(int(l["palette"]), int(App.settings["vision"])))
	material.set_shader_parameter("palette", palette_tex)

func set_tiles(on: bool) -> void:
	raw_tiles = on
	_update_tiles()

func _update_tiles() -> void:
	tiles = raw_tiles or int(layer.get("transform", 0)) == 3
	ensure_mesh()
	refresh_frame(true)

func set_show_vents(on: bool) -> void:
	material.set_shader_parameter("show_vents", on)

func set_focus_visible(on: bool) -> void:
	material.set_shader_parameter("show_focus", on)

## Flèches des plaques à l'encre, longueur proportionnelle à la vitesse.
func _build_arrows() -> void:
	var d: Dictionary = App.session.plate_arrows()
	if d.is_empty():
		return
	var pos: PackedVector3Array = d["positions"]
	var vel: PackedVector3Array = d["velocities"]
	var im := ImmediateMesh.new()
	im.surface_begin(Mesh.PRIMITIVE_LINES)
	for i in pos.size():
		var v: Vector3 = vel[i]
		var speed := v.length()
		if speed < 0.05:
			continue
		var p: Vector3 = pos[i] * 1.004
		var dir := v / speed
		var length: float = clamp(speed * 0.008, 0.012, 0.07)
		var tip := (p + dir * length).normalized() * 1.004
		var side := p.cross(dir).normalized() * length * 0.3
		im.surface_add_vertex(p)
		im.surface_add_vertex(tip)
		im.surface_add_vertex(tip)
		im.surface_add_vertex(tip - dir * length * 0.35 + side)
		im.surface_add_vertex(tip)
		im.surface_add_vertex(tip - dir * length * 0.35 - side)
	im.surface_end()
	arrows.mesh = im

# ----------------------------------------------------------------------
# Caméra

func _place_camera() -> void:
	var dir := Vector3(cos(pitch) * sin(yaw), sin(pitch), cos(pitch) * cos(yaw))
	camera.position = dir * distance
	camera.look_at(Vector3.ZERO, Vector3.UP)
	camera.h_offset = lerp(camera.h_offset, -view_offset * distance * 0.25, 0.2)
	# Lumière d'atelier venue d'en haut à gauche de la vue, ou étoile fixe.
	if bool(App.settings["terminator"]):
		material.set_shader_parameter("light_dir", Vector3(-0.6, 0.35, 0.72).normalized())
	else:
		var b := camera.global_transform.basis
		material.set_shader_parameter("light_dir", (b.z * 0.8 + b.y * 0.5 - b.x * 0.4).normalized())

## Rayon angulaire de la calotte vue, et bande de zoom (1 : planète entière,
## 6 : au plus près).
func view_cap() -> Vector2:
	var cap := acos(clamp(1.0 / distance, -1.0, 1.0))
	var visible_half: float = min(cap, deg_to_rad(camera.fov) * 0.5 * distance)
	var band: int = clamp(1 + int(floor(log(PI / 2.0 / max(visible_half, 0.01)) / log(2.0) * 1.5)), 1, 6)
	return Vector2(visible_half, band)

func look_at_direction(dir: Vector3, zoom: float = -1.0) -> void:
	dir = dir.normalized()
	target_yaw = atan2(dir.x, dir.z)
	target_pitch = asin(clamp(dir.y, -1.0, 1.0))
	# Chemin le plus court autour du globe.
	while target_yaw - yaw > PI:
		target_yaw -= TAU
	while target_yaw - yaw < -PI:
		target_yaw += TAU
	if zoom > 0.0:
		target_distance = clamp(zoom, MIN_DISTANCE, MAX_DISTANCE)
	if bool(App.settings["reduce_motion"]):
		yaw = target_yaw
		pitch = target_pitch
		distance = target_distance

func go_to_cell(cell: int, zoom: float = 1.9) -> void:
	if cell < 0:
		return
	look_at_direction(App.session.cell_centre(cell), zoom)
	select_cell(cell)

func select_cell(cell: int) -> void:
	selected_cell = cell
	if cell < 0:
		material.set_shader_parameter("selected_radius", 0.0)
	else:
		material.set_shader_parameter("selected_pos", App.session.cell_centre(cell))
		material.set_shader_parameter("selected_radius", _cell_radius() * 1.1)

func _cell_radius() -> float:
	# Rayon angulaire moyen d'une cellule : surface 4π / N.
	return sqrt(4.0 / max(mesh_cells, 12))

func zoom_by(factor: float) -> void:
	target_distance = clamp(target_distance * factor, MIN_DISTANCE, MAX_DISTANCE)

func rotate_by(dyaw: float, dpitch: float) -> void:
	var k := (distance - 1.0) * 0.5
	target_yaw += dyaw * k
	target_pitch = clamp(target_pitch + dpitch * k, -1.45, 1.45)

func _process(delta: float) -> void:
	var k: float = 1.0 if bool(App.settings["reduce_motion"]) else 1.0 - exp(-delta * 9.0)
	yaw = lerp(yaw, target_yaw, k)
	pitch = lerp(pitch, target_pitch, k)
	distance = lerp(distance, target_distance, k)
	if interactive:
		var keys := Vector2(Input.get_axis("ui_left", "ui_right"), Input.get_axis("ui_down", "ui_up"))
		if keys != Vector2.ZERO and get_viewport().gui_get_focus_owner() == null:
			rotate_by(keys.x * delta * 1.6, keys.y * delta * 1.6)
	_place_camera()
	if blend < 1.0:
		blend = min(1.0, blend + delta / blend_duration)
		material.set_shader_parameter("blend", blend)
	observe_timer -= delta
	if observe_timer <= 0.0:
		observe_timer = 0.5
		_send_observation()

func _send_observation() -> void:
	if not App.session.is_running():
		return
	var centre := camera.position.normalized()
	var cap := view_cap()
	var o := Vector4(centre.x, centre.y, centre.z, cap.y)
	if o.distance_to(last_observation) > 0.02:
		last_observation = o
		App.session.observe(centre, cap.x, int(cap.y))

# ----------------------------------------------------------------------
# Souris

func pick(screen_pos: Vector2) -> int:
	var origin := camera.project_ray_origin(screen_pos)
	var dir := camera.project_ray_normal(screen_pos)
	var b := origin.dot(dir)
	var c := origin.dot(origin) - 1.0
	var disc := b * b - c
	if disc < 0.0:
		return -1
	var t := -b - sqrt(disc)
	if t < 0.0:
		return -1
	return App.session.cell_at(origin + dir * t)

func _unhandled_input(event: InputEvent) -> void:
	if not interactive:
		return
	if event is InputEventMouseButton:
		var mb := event as InputEventMouseButton
		if mb.button_index == MOUSE_BUTTON_WHEEL_UP and mb.pressed:
			zoom_by(0.9)
		elif mb.button_index == MOUSE_BUTTON_WHEEL_DOWN and mb.pressed:
			zoom_by(1.11)
		elif mb.button_index == MOUSE_BUTTON_LEFT:
			if mb.pressed:
				dragging = true
				drag_moved = 0.0
				if mb.double_click:
					var cell := pick(mb.position)
					if cell >= 0:
						look_at_direction(App.session.cell_centre(cell), max(MIN_DISTANCE, target_distance * 0.6))
			else:
				dragging = false
				if drag_moved < 6.0:
					var cell := pick(mb.position)
					select_cell(cell)
					cell_selected.emit(cell)
	elif event is InputEventMouseMotion:
		var mm := event as InputEventMouseMotion
		if dragging and (mm.button_mask & MOUSE_BUTTON_MASK_LEFT):
			drag_moved += mm.relative.length()
			rotate_by(-mm.relative.x * 0.006, mm.relative.y * 0.006)
		else:
			var cell := pick(mm.position)
			if cell != hover_cell:
				hover_cell = cell
				if cell >= 0:
					material.set_shader_parameter("hover_pos", App.session.cell_centre(cell))
					material.set_shader_parameter("hover_radius", _cell_radius() * 0.9)
				else:
					material.set_shader_parameter("hover_radius", 0.0)
				cell_hovered.emit(cell)
	elif event is InputEventMagnifyGesture:
		zoom_by(1.0 / (event as InputEventMagnifyGesture).factor)
	elif event is InputEventKey and event.pressed:
		var key := event as InputEventKey
		if key.keycode == KEY_EQUAL or key.keycode == KEY_KP_ADD or key.keycode == KEY_PLUS:
			zoom_by(0.85)
		elif key.keycode == KEY_MINUS or key.keycode == KEY_KP_SUBTRACT:
			zoom_by(1.18)
