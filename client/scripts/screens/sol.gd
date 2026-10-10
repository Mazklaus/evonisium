extends Control
## Scène au sol (étape 5, descente au sol du globe) : un carré de terrain
## autour du point visé, ses habitants animés près de la caméra et en
## imposteurs au loin, une caméra qui suit un individu ou survole librement.
##
## Les espèces vraies de la cellule assez grandes pour se voir sont des
## agents du moteur (individus tirés de leur population, comportements tirés
## de leurs traits) ; les autres sont des figurants de banc d'essai. Rien de
## ce qu'ils font ne change l'histoire.

signal closed

const AnimalShader := preload("res://shaders/animal.gdshader")
const ImpostorShader := preload("res://shaders/imposteur.gdshader")
const TerrainShader := preload("res://shaders/sol_terrain.gdshader")

## Individus animés (les plus proches de la caméra).
var animated := 1000
## Niveau de détail des maillages animés (0 : le plus fin).
var lod := 2
## Les imposteurs se rafraîchissent une image sur `impostor_every`.
var impostor_every := 2

var ground: EvoGround
var ready_done := false
var viewport: SubViewport
var world: Node3D
var camera: Camera3D
var sun: DirectionalLight3D
var env: Environment
var water: MeshInstance3D
var species_nodes: Array = []  # [{mm: MultiMesh, mat: ShaderMaterial, tex: ImageTexture, rows: int}]
var impostor_mm: MultiMesh
var impostor_frame := 0

var followed := -1
var free_target := Vector3.ZERO
var yaw := 0.6
var pitch := 0.35
var distance := 8.0
var target_distance := 8.0
var dragging := false
var sky := Color(0.82, 0.86, 0.88)
var is_water := false

var title_label: Label
var info_label: Label
var notice_label: Label
var waiting_label: Label
var actions_box: OptionButton
var follow_button: Button
var fps_label: Label
var keys: Array = []

func _ready() -> void:
	set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	mouse_filter = Control.MOUSE_FILTER_STOP
	var bg := ColorRect.new()
	bg.color = Atlas.PAPER
	bg.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	add_child(bg)
	var container := SubViewportContainer.new()
	container.stretch = true
	container.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	container.mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_child(container)
	viewport = SubViewport.new()
	viewport.own_world_3d = true
	viewport.msaa_3d = Viewport.MSAA_2X
	container.add_child(viewport)
	world = Node3D.new()
	viewport.add_child(world)
	camera = Camera3D.new()
	camera.near = 0.02
	camera.far = 6000.0
	camera.fov = 55.0
	world.add_child(camera)
	sun = DirectionalLight3D.new()
	sun.rotation = Vector3(deg_to_rad(-50.0), deg_to_rad(-35.0), 0.0)
	sun.light_energy = 1.1
	world.add_child(sun)
	env = Environment.new()
	env.background_mode = Environment.BG_COLOR
	env.ambient_light_source = Environment.AMBIENT_SOURCE_COLOR
	env.ambient_light_color = Color(0.85, 0.82, 0.74)
	env.ambient_light_energy = 0.55
	env.fog_enabled = true
	env.fog_density = 0.0009
	var we := WorldEnvironment.new()
	we.environment = env
	world.add_child(we)
	_build_ui()

func _build_ui() -> void:
	var top := Atlas.panel()
	top.position = Vector2(12, 12)
	add_child(top)
	var v := VBoxContainer.new()
	top.add_child(v)
	title_label = Atlas.title(App.t("ground_title"), 24)
	v.add_child(title_label)
	notice_label = Atlas.text(App.t("ground_extras"), 14, true)
	notice_label.custom_minimum_size.x = 380
	notice_label.add_theme_color_override("font_color", Atlas.VERMILION)
	v.add_child(notice_label)
	info_label = Atlas.text("", 15)
	info_label.custom_minimum_size.x = 380
	v.add_child(info_label)
	var row := HBoxContainer.new()
	v.add_child(row)
	follow_button = Atlas.button(App.t("ground_free"), _toggle_follow)
	row.add_child(follow_button)
	row.add_child(Atlas.button(App.t("ground_next"), _follow_next))
	var row2 := HBoxContainer.new()
	v.add_child(row2)
	var al := Atlas.text(App.t("ground_action"), 15)
	al.autowrap_mode = TextServer.AUTOWRAP_OFF
	row2.add_child(al)
	actions_box = OptionButton.new()
	actions_box.item_selected.connect(_on_action)
	row2.add_child(actions_box)
	fps_label = Atlas.text("", 13, true)
	v.add_child(fps_label)
	var up := Atlas.button(App.t("ground_up"), func(): closed.emit())
	up.set_anchors_preset(Control.PRESET_TOP_RIGHT)
	up.position = Vector2(-180, 14)
	up.grow_horizontal = Control.GROW_DIRECTION_BEGIN
	add_child(up)
	waiting_label = Atlas.title(App.t("ground_wait"), 22)
	waiting_label.set_anchors_preset(Control.PRESET_CENTER)
	waiting_label.grow_horizontal = Control.GROW_DIRECTION_BOTH
	waiting_label.grow_vertical = Control.GROW_DIRECTION_BOTH
	add_child(waiting_label)

func open(g: EvoGround) -> void:
	ground = g

func _setup_scene() -> void:
	ready_done = true
	waiting_label.visible = false
	var t: Dictionary = ground.terrain()
	sky = t["sky"]
	is_water = bool(t["water"])
	env.background_color = sky
	env.fog_light_color = sky
	var arrays := []
	arrays.resize(Mesh.ARRAY_MAX)
	arrays[Mesh.ARRAY_VERTEX] = t["vertices"]
	arrays[Mesh.ARRAY_NORMAL] = t["normals"]
	arrays[Mesh.ARRAY_COLOR] = t["colors"]
	arrays[Mesh.ARRAY_INDEX] = t["indices"]
	var am := ArrayMesh.new()
	am.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	var tm := ShaderMaterial.new()
	tm.shader = TerrainShader
	tm.set_shader_parameter("sun_dir", sun_dir())
	var terrain := MeshInstance3D.new()
	terrain.mesh = am
	terrain.material_override = tm
	world.add_child(terrain)
	if is_water:
		var pm := PlaneMesh.new()
		pm.size = Vector2(float(t["size"]), float(t["size"]))
		var wm := StandardMaterial3D.new()
		wm.albedo_color = Color(Atlas.WATER.r, Atlas.WATER.g, Atlas.WATER.b, 0.55)
		wm.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
		wm.cull_mode = BaseMaterial3D.CULL_DISABLED
		wm.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
		water = MeshInstance3D.new()
		water.mesh = pm
		water.material_override = wm
		world.add_child(water)
	# Une espèce : un MultiMesh animé (squelette lu dans la texture des poses).
	var n := ground.species_count()
	for k in n:
		var md: Dictionary = ground.species_mesh(k, lod)
		var sa := []
		sa.resize(Mesh.ARRAY_MAX)
		sa[Mesh.ARRAY_VERTEX] = md["vertices"]
		sa[Mesh.ARRAY_NORMAL] = md["normals"]
		sa[Mesh.ARRAY_COLOR] = md["colors"]
		sa[Mesh.ARRAY_TEX_UV] = md["uv"]
		sa[Mesh.ARRAY_TEX_UV2] = md["uv2"]
		sa[Mesh.ARRAY_INDEX] = md["indices"]
		var sm := ArrayMesh.new()
		sm.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, sa)
		var mat := ShaderMaterial.new()
		mat.shader = AnimalShader
		mat.set_shader_parameter("sun_dir", sun_dir())
		var mm := MultiMesh.new()
		mm.transform_format = MultiMesh.TRANSFORM_3D
		mm.mesh = sm
		mm.instance_count = 0
		var mi := MultiMeshInstance3D.new()
		mi.multimesh = mm
		mi.material_override = mat
		mi.custom_aabb = AABB(Vector3(-3000, -3000, -3000), Vector3(6000, 6000, 6000))
		world.add_child(mi)
		species_nodes.append({"mm": mm, "mat": mat, "tex": null, "rows": 0, "tex_size": Vector2i.ZERO})
	# Imposteurs : un seul MultiMesh pour toute la foule.
	var atlas_img: Image = ground.impostor_atlas()
	var layout: Vector2i = ground.impostor_layout()
	var im := ShaderMaterial.new()
	im.shader = ImpostorShader
	im.set_shader_parameter("atlas", ImageTexture.create_from_image(atlas_img))
	im.set_shader_parameter("views", float(layout.x))
	im.set_shader_parameter("rows", float(max(n, 1)))
	var quad := QuadMesh.new()
	quad.size = Vector2(1, 1)
	impostor_mm = MultiMesh.new()
	impostor_mm.transform_format = MultiMesh.TRANSFORM_3D
	impostor_mm.use_custom_data = true
	impostor_mm.mesh = quad
	impostor_mm.instance_count = ground.individual_count()
	var imi := MultiMeshInstance3D.new()
	imi.multimesh = impostor_mm
	imi.material_override = im
	imi.custom_aabb = AABB(Vector3(-3000, -3000, -3000), Vector3(6000, 6000, 6000))
	world.add_child(imi)
	# Actions du lexique.
	keys = [""]
	actions_box.add_item(App.t("ground_free_behaviour"))
	for a in ground.actions():
		keys.append(a["key"])
		actions_box.add_item(a["label"])
	follow_species(0)

## Direction vers le soleil, repère du monde.
func sun_dir() -> Vector3:
	return sun.global_transform.basis.z.normalized()

func follow_species(k: int) -> void:
	followed = ground.first_of(k)
	if followed >= 0:
		var d: Dictionary = ground.individual(followed)
		target_distance = clampf(float(d["size_m"]) * 4.5, 0.3, 300.0)
		distance = target_distance
	follow_button.text = App.t("ground_free")

func _follow_next() -> void:
	if followed < 0:
		follow_species(0)
		return
	var s := int(ground.individual(followed)["species"])
	follow_species((s + 1) % max(ground.species_count(), 1))

func _toggle_follow() -> void:
	if followed >= 0:
		free_target = ground.individual(followed)["position"]
		followed = -1
		follow_button.text = App.t("ground_follow")
	else:
		var i := ground.pick(camera.global_position, (free_target - camera.global_position).normalized())
		if i >= 0:
			followed = i
		else:
			follow_species(0)
		follow_button.text = App.t("ground_free")

func _on_action(idx: int) -> void:
	if followed < 0:
		return
	var s := int(ground.individual(followed)["species"])
	ground.force_action(s, keys[idx])

## Impose une action à l'espèce suivie (captures de la porte).
func force(key: String) -> void:
	if followed >= 0:
		ground.force_action(int(ground.individual(followed)["species"]), key)

func _process(delta: float) -> void:
	if ground == null:
		return
	if not ready_done:
		if ground.is_ready():
			_setup_scene()
		return
	ground.step(delta, camera.global_position, animated)
	var target := free_target
	if followed >= 0:
		var d: Dictionary = ground.individual(followed)
		target = d["position"]
		var sp: Dictionary = ground.species_info(int(d["species"]))
		info_label.text = "%s\n%s · %s · %s" % [sp["name"], d["action_label"], _length(float(d["size_m"])), sp["locomotion"]]
	else:
		var mv := Vector2(Input.get_axis("ui_left", "ui_right"), Input.get_axis("ui_up", "ui_down"))
		if mv != Vector2.ZERO:
			var fwd := Vector3(-sin(yaw), 0, -cos(yaw))
			var right := Vector3(cos(yaw), 0, -sin(yaw))
			free_target += (right * mv.x + fwd * -mv.y) * delta * distance * 0.8
		info_label.text = App.t("ground_free_text")
	var k := 1.0 - exp(-delta * 6.0)
	distance = lerp(distance, target_distance, k)
	var dir := Vector3(cos(pitch) * sin(yaw), sin(pitch), cos(pitch) * cos(yaw))
	var want := target + dir * distance
	camera.global_position = camera.global_position.lerp(want, k) if followed >= 0 and camera.global_position != Vector3.ZERO else want
	camera.look_at(target, Vector3.UP)
	# Sous l'eau : brume bleutée.
	if is_water and camera.global_position.y < 0.0:
		env.fog_light_color = Atlas.WATER.darkened(0.2)
		env.fog_density = 0.02
		env.background_color = Atlas.WATER.darkened(0.25)
	else:
		env.fog_light_color = sky
		env.fog_density = 0.0009
		env.background_color = sky
	_update_animated()
	impostor_frame += 1
	if impostor_frame % impostor_every == 0:
		impostor_mm.buffer = ground.impostors()
	var st: Dictionary = ground.stats()
	fps_label.text = "%d img/s · %d animés · %d individus · foule %.1f ms" % [Engine.get_frames_per_second(), int(st.get("animated", 0)), int(st.get("individuals", 0)), float(st.get("step_ms", 0.0))]

func _length(m: float) -> String:
	if m < 0.01:
		return "%.1f mm" % (m * 1000.0)
	if m < 1.0:
		return "%.0f cm" % (m * 100.0)
	return "%.1f m" % m

func _update_animated() -> void:
	for k in species_nodes.size():
		var s: Dictionary = species_nodes[k]
		var ids: PackedInt32Array = ground.animated_of(k)
		var mm: MultiMesh = s["mm"]
		var rows := int(s["rows"])
		if ids.size() > rows or rows == 0:
			rows = max(16, nearest_po2(ids.size()))
			mm.instance_count = rows
			s["rows"] = rows
		mm.visible_instance_count = ids.size()
		if ids.is_empty():
			continue
		mm.buffer = ground.transforms(ids, rows)
		var img: Image = ground.poses(k, ids, rows)
		if img == null:
			continue
		var size := Vector2i(img.get_width(), img.get_height())
		if s["tex"] == null or s["tex_size"] != size:
			s["tex"] = ImageTexture.create_from_image(img)
			s["tex_size"] = size
			(s["mat"] as ShaderMaterial).set_shader_parameter("poses", s["tex"])
		else:
			(s["tex"] as ImageTexture).update(img)

func _gui_input(event: InputEvent) -> void:
	if event is InputEventMouseButton:
		var mb := event as InputEventMouseButton
		if mb.button_index == MOUSE_BUTTON_WHEEL_UP and mb.pressed:
			target_distance = max(0.15, target_distance * 0.88)
		elif mb.button_index == MOUSE_BUTTON_WHEEL_DOWN and mb.pressed:
			target_distance = min(1500.0, target_distance * 1.14)
		elif mb.button_index == MOUSE_BUTTON_RIGHT:
			dragging = mb.pressed
		elif mb.button_index == MOUSE_BUTTON_LEFT and mb.pressed and ready_done:
			var o := camera.project_ray_origin(mb.position)
			var d := camera.project_ray_normal(mb.position)
			var i := ground.pick(o, d)
			if i >= 0:
				followed = i
				follow_button.text = App.t("ground_free")
	elif event is InputEventMouseMotion and dragging:
		var mm := event as InputEventMouseMotion
		yaw -= mm.relative.x * 0.006
		pitch = clampf(pitch + mm.relative.y * 0.006, -0.3, 1.45)

func _unhandled_key_input(event: InputEvent) -> void:
	if event.pressed and not event.echo and (event as InputEventKey).keycode == KEY_ESCAPE:
		closed.emit()
		get_viewport().set_input_as_handled()
