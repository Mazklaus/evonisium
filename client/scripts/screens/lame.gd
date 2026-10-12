extends Control
## Lame de microscope en 3D (vue microscope, sous 1 mm) : une goutte d'eau
## entre lame et lamelle où nagent les microbes d'une cellule, tirés de ses
## populations dans les proportions de leur biomasse (evo_view::slide). On
## tourne autour en tirant, on se déplace au clic droit ou aux flèches, on
## zoome à la molette ; un clic sur une cellule ouvre la fiche de son espèce.
## La barre d'échelle se lit au plan de mise au point. Rien de ce que font
## ces cellules ne change l'histoire.

signal closed
signal species_requested(species: int)

const MicrobeShader := preload("res://shaders/microbe.gdshader")
const Raconter := preload("res://scripts/ui/raconter.gd")
## Longueurs possibles de la barre d'échelle, µm.
const BAR_STEPS := [0.5, 1.0, 2.0, 5.0, 10.0, 20.0, 50.0, 100.0]

var slide: EvoSlide
var cell := -1
var ready_done := false
var viewport: SubViewport
var world: Node3D
var camera: Camera3D
var env: Environment
var meshes: Array = []  # MultiMesh par espèce
var field := Vector3(180, 24, 120)

var target := Vector3.ZERO
var yaw := 0.3
var pitch := 1.05
var distance := 120.0
var target_distance := 120.0
var dragging := 0
var drag_moved := false

var title_label: Label
var info_label: Label
var hover_label: Label
var waiting_label: Label
var legend: VBoxContainer
var scale_bar: Control
var names := {}

func _ready() -> void:
	set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	mouse_filter = Control.MOUSE_FILTER_STOP
	var bg := ColorRect.new()
	bg.color = Color(0.949, 0.922, 0.847)
	bg.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	add_child(bg)
	var container := SubViewportContainer.new()
	container.stretch = true
	container.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	container.mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_child(container)
	viewport = SubViewport.new()
	viewport.own_world_3d = true
	viewport.msaa_3d = Viewport.MSAA_4X
	container.add_child(viewport)
	world = Node3D.new()
	viewport.add_child(world)
	camera = Camera3D.new()
	camera.near = 0.5
	camera.far = 3000.0
	camera.fov = 40.0
	world.add_child(camera)
	env = Environment.new()
	env.background_mode = Environment.BG_COLOR
	# Fond clair de l'oculaire, un peu d'eau dans la profondeur.
	env.background_color = Color(0.949, 0.922, 0.847)
	env.fog_enabled = true
	env.fog_light_color = Color(0.90, 0.91, 0.86)
	env.fog_density = 0.0025
	var we := WorldEnvironment.new()
	we.environment = env
	world.add_child(we)
	_build_ui()

## Ouvre la lame de `cell` (la construction se fait en tâche de fond).
func open(s: EvoSlide, c: int) -> void:
	slide = s
	cell = c

func _build_ui() -> void:
	var top := Atlas.panel()
	top.position = Vector2(12, 12)
	add_child(top)
	var v := VBoxContainer.new()
	top.add_child(v)
	title_label = Atlas.title(App.t("slide_title"), 24)
	v.add_child(title_label)
	var note := Atlas.text(App.t("slide_note"), 14, true)
	note.custom_minimum_size.x = 380
	note.add_theme_color_override("font_color", Atlas.VERMILION)
	v.add_child(note)
	info_label = Atlas.text("", 15)
	info_label.custom_minimum_size.x = 380
	v.add_child(info_label)
	legend = VBoxContainer.new()
	v.add_child(legend)
	var row := HBoxContainer.new()
	v.add_child(row)
	row.add_child(Raconter.new(func(): return Son.raconter_lieu(cell) if cell >= 0 else -1))
	row.add_child(Atlas.button(App.t("slide_reset"), _reset_view))
	hover_label = Atlas.text("", 15, true)
	v.add_child(hover_label)
	var up := Atlas.button(App.t("ground_up"), func(): closed.emit())
	up.set_anchors_preset(Control.PRESET_TOP_RIGHT)
	up.position = Vector2(-180, 14)
	up.grow_horizontal = Control.GROW_DIRECTION_BEGIN
	add_child(up)
	waiting_label = Atlas.title(App.t("slide_wait"), 22)
	waiting_label.set_anchors_preset(Control.PRESET_CENTER)
	waiting_label.grow_horizontal = Control.GROW_DIRECTION_BOTH
	waiting_label.grow_vertical = Control.GROW_DIRECTION_BOTH
	add_child(waiting_label)
	scale_bar = Control.new()
	scale_bar.set_anchors_preset(Control.PRESET_BOTTOM_LEFT)
	scale_bar.position = Vector2(24, -70)
	scale_bar.custom_minimum_size = Vector2(260, 50)
	scale_bar.mouse_filter = Control.MOUSE_FILTER_IGNORE
	scale_bar.draw.connect(_draw_scale)
	add_child(scale_bar)

func _setup() -> void:
	ready_done = true
	waiting_label.visible = false
	field = slide.field()
	# Lame et lamelle : deux verres pâles qui bornent le film d'eau.
	for y in [-field.y / 2.0 - 0.5, field.y / 2.0 + 0.5]:
		var glass := MeshInstance3D.new()
		var pm := PlaneMesh.new()
		pm.size = Vector2(field.x + 40.0, field.z + 40.0)
		glass.mesh = pm
		glass.position = Vector3(0, y, 0)
		var gm := StandardMaterial3D.new()
		gm.albedo_color = Color(0.80, 0.86, 0.84, 0.10 if y > 0 else 0.35)
		gm.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
		gm.cull_mode = BaseMaterial3D.CULL_DISABLED
		gm.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
		glass.material_override = gm
		world.add_child(glass)
	var total := 0
	for k in slide.species_count():
		var md: Dictionary = slide.mesh(k)
		var a := []
		a.resize(Mesh.ARRAY_MAX)
		a[Mesh.ARRAY_VERTEX] = md["vertices"]
		a[Mesh.ARRAY_NORMAL] = md["normals"]
		a[Mesh.ARRAY_COLOR] = md["colors"]
		a[Mesh.ARRAY_INDEX] = md["indices"]
		var am := ArrayMesh.new()
		am.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, a)
		var mat := ShaderMaterial.new()
		mat.shader = MicrobeShader
		var mm := MultiMesh.new()
		mm.transform_format = MultiMesh.TRANSFORM_3D
		mm.mesh = am
		var info: Dictionary = slide.species_info(k)
		mm.instance_count = int(info["cells"])
		total += int(info["cells"])
		var mi := MultiMeshInstance3D.new()
		mi.multimesh = mm
		mi.material_override = mat
		mi.custom_aabb = AABB(-field, field * 2.0)
		world.add_child(mi)
		meshes.append(mm)
		_legend_row(k, info)
	info_label.text = App.t("slide_info") % [slide.species_count(), total, int(field.x), int(field.z), int(field.y)]
	if slide.species_count() == 0:
		info_label.text = App.t("slide_empty")
	_update_meshes()

func _legend_row(k: int, info: Dictionary) -> void:
	var h := HBoxContainer.new()
	var sw := ColorRect.new()
	sw.color = info["colour"]
	sw.custom_minimum_size = Vector2(16, 16)
	sw.size_flags_vertical = Control.SIZE_SHRINK_CENTER
	h.add_child(sw)
	var b := Atlas.button(_name(k), func(): species_requested.emit(int(slide.species_info(k)["species"])), App.t("slide_open_card"))
	b.alignment = HORIZONTAL_ALIGNMENT_LEFT
	b.clip_text = true
	b.custom_minimum_size.x = 290
	b.add_theme_font_size_override("font_size", int(15 * App.text_scale()))
	h.add_child(b)
	var share := Atlas.text("%d %%" % int(round(float(info["share"]) * 100.0)), 15)
	share.autowrap_mode = TextServer.AUTOWRAP_OFF
	h.add_child(share)
	legend.add_child(h)

## Nom de l'espèce `k`, celui que donne l'inspecteur à ses populations.
func _name(k: int) -> String:
	if not names.has(k):
		var sp := int(slide.species_info(k)["species"])
		for p in App.session.cell_info(cell).get("populations", []):
			if int(p["species"]) == sp:
				names[k] = str(p["name"])
		if not names.has(k):
			names[k] = str(App.session.lineage_label(int(slide.species_info(k)["lineage"])).get("name", "?"))
	return names[k]

func _update_meshes() -> void:
	for k in meshes.size():
		var buf: PackedFloat32Array = slide.transforms(k)
		if buf.size() > 0:
			(meshes[k] as MultiMesh).buffer = buf

func _reset_view() -> void:
	target = Vector3.ZERO
	yaw = 0.3
	pitch = 1.05
	target_distance = 120.0

func _process(delta: float) -> void:
	if slide == null:
		return
	if not ready_done:
		if slide.is_ready():
			_setup()
		return
	slide.step(delta)
	_update_meshes()
	var mv := Vector2(Input.get_axis("ui_left", "ui_right"), Input.get_axis("ui_up", "ui_down"))
	if mv != Vector2.ZERO:
		var fwd := Vector3(-sin(yaw), 0, -cos(yaw))
		var right := Vector3(cos(yaw), 0, -sin(yaw))
		target += (right * mv.x + fwd * -mv.y) * delta * distance * 0.8
	target = target.clamp(-field / 2.0, field / 2.0)
	distance = lerp(distance, target_distance, 1.0 - exp(-delta * 6.0))
	var off := Vector3(sin(yaw) * cos(pitch), sin(pitch), cos(yaw) * cos(pitch)) * distance
	camera.look_at_from_position(target + off, target, Vector3.UP)
	scale_bar.queue_redraw()

## Barre d'échelle au plan de mise au point (la distance de la caméra à la
## cible) : une longueur ronde d'environ 120 pixels.
func _draw_scale() -> void:
	var h := float(viewport.size.y)
	if h <= 0.0:
		return
	var px_per_um := h / (2.0 * distance * tan(deg_to_rad(camera.fov) / 2.0)) * (size.y / h)
	var um: float = BAR_STEPS[0]
	for s in BAR_STEPS:
		if s * px_per_um <= 160.0:
			um = s
	var w := um * px_per_um
	var font := Atlas.body_font()
	scale_bar.draw_line(Vector2(0, 30), Vector2(w, 30), Atlas.INK, 3.0)
	scale_bar.draw_line(Vector2(0, 24), Vector2(0, 36), Atlas.INK, 2.0)
	scale_bar.draw_line(Vector2(w, 24), Vector2(w, 36), Atlas.INK, 2.0)
	var label := ("%s µm" % ("%.1f" % um if um < 1.0 else str(int(um))))
	scale_bar.draw_string(font, Vector2(0, 18), label, HORIZONTAL_ALIGNMENT_LEFT, -1, int(15 * App.text_scale()), Atlas.INK)

func _ray(pos: Vector2) -> Array:
	# Les coordonnées de l'écran et celles du viewport coïncident (étirement).
	var p := pos * (Vector2(viewport.size) / size)
	return [camera.project_ray_origin(p), camera.project_ray_normal(p)]

func _gui_input(event: InputEvent) -> void:
	if event is InputEventMouseButton:
		var mb := event as InputEventMouseButton
		if mb.button_index == MOUSE_BUTTON_WHEEL_UP and mb.pressed:
			target_distance = clampf(target_distance / 1.15, 4.0, 400.0)
		elif mb.button_index == MOUSE_BUTTON_WHEEL_DOWN and mb.pressed:
			target_distance = clampf(target_distance * 1.15, 4.0, 400.0)
		elif mb.button_index in [MOUSE_BUTTON_LEFT, MOUSE_BUTTON_RIGHT]:
			if mb.pressed:
				dragging = mb.button_index
				drag_moved = false
			else:
				if mb.button_index == MOUSE_BUTTON_LEFT and not drag_moved and ready_done:
					var r := _ray(mb.position)
					var k := slide.pick(r[0], r[1])
					if k >= 0:
						species_requested.emit(int(slide.species_info(k)["species"]))
				dragging = 0
		accept_event()
	elif event is InputEventMouseMotion:
		var mm := event as InputEventMouseMotion
		if dragging != 0 and mm.relative.length() > 0.5:
			drag_moved = true
		if dragging == MOUSE_BUTTON_LEFT:
			yaw -= mm.relative.x * 0.006
			pitch = clampf(pitch + mm.relative.y * 0.006, 0.15, 1.5)
		elif dragging == MOUSE_BUTTON_RIGHT:
			var right := Vector3(cos(yaw), 0, -sin(yaw))
			var fwd := Vector3(-sin(yaw), 0, -cos(yaw))
			target += (right * -mm.relative.x + fwd * mm.relative.y) * distance * 0.0015
		elif ready_done:
			var r := _ray(mm.position)
			var k := slide.pick(r[0], r[1])
			hover_label.text = "" if k < 0 else "%s — %s" % [_name(k), App.t("slide_open_card")]

func _unhandled_key_input(event: InputEvent) -> void:
	if event.is_action_pressed("ui_cancel"):
		get_viewport().set_input_as_handled()
		closed.emit()
