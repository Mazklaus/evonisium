extends SubViewportContainer
## Portrait 3D d'une espèce (document Rendu du vivant, palier 2) : le corps
## généré par evo-morph depuis le plan de construction, de profil comme sur
## une planche, à l'encre et au lavis, sur fond transparent posé sur le
## papier. Le glisser le fait tourner. En vue anatomie, la peau devient un
## voile et les organes internes apparaissent, colorés par appareil.

signal body_ready(info: Dictionary)

const BODY_SHADER := preload("res://shaders/corps_atlas.gdshader")
const CONTOUR_SHADER := preload("res://shaders/corps_contour.gdshader")
const ORGAN_SHADER := preload("res://shaders/organe.gdshader")

var viewport: SubViewport
var pivot: Node3D
var skin: MeshInstance3D
var organs: MeshInstance3D
var camera: Camera3D
var body_material: ShaderMaterial
var contour_material: ShaderMaterial
var key := ""
var info := {}
var meshes: Array = []
var lod := -1
var anatomy := false
## Taille relative (comparateur : la plus grande des deux espèces vaut 1).
var relative := 1.0
var yaw := 0.0
var pitch := -0.12
var spin := true
var dragging := false

func _init() -> void:
	stretch = true
	mouse_filter = Control.MOUSE_FILTER_STOP
	viewport = SubViewport.new()
	viewport.transparent_bg = true
	viewport.own_world_3d = true
	viewport.msaa_3d = Viewport.MSAA_4X
	add_child(viewport)
	var env := WorldEnvironment.new()
	var e := Environment.new()
	e.background_mode = Environment.BG_CLEAR_COLOR
	e.ambient_light_source = Environment.AMBIENT_SOURCE_COLOR
	e.ambient_light_color = Color("#ECE2C9")
	e.ambient_light_energy = 0.35
	env.environment = e
	viewport.add_child(env)
	var light := DirectionalLight3D.new()
	light.rotation_degrees = Vector3(-50, 35, 0)
	light.light_energy = 1.1
	viewport.add_child(light)
	camera = Camera3D.new()
	camera.fov = 30.0
	camera.near = 0.01
	camera.transform = Transform3D(Basis(), Vector3(0, 0.12, 2.25)).looking_at(Vector3.ZERO)
	viewport.add_child(camera)
	pivot = Node3D.new()
	viewport.add_child(pivot)
	skin = MeshInstance3D.new()
	pivot.add_child(skin)
	organs = MeshInstance3D.new()
	organs.visible = false
	pivot.add_child(organs)
	body_material = ShaderMaterial.new()
	body_material.shader = BODY_SHADER
	contour_material = ShaderMaterial.new()
	contour_material.shader = CONTOUR_SHADER
	body_material.next_pass = contour_material
	var om := ShaderMaterial.new()
	om.shader = ORGAN_SHADER
	organs.material_override = om

## Montre le corps d'une clé rendue par `request_body` ou `request_test_body`.
func show_body(k: String, rel := 1.0) -> void:
	key = k
	relative = rel
	info = {}
	meshes = []
	lod = -1
	skin.mesh = null
	organs.mesh = null

func set_relative(rel: float) -> void:
	relative = rel
	pivot.scale = Vector3.ONE * relative
	_choose_lod()

func set_anatomy(on: bool) -> void:
	anatomy = on
	body_material.set_shader_parameter("opacity", 0.22 if on else 1.0)
	contour_material.set_shader_parameter("width", 0.0 if on else 0.012 / max(relative, 0.05))
	organs.visible = on

static func array_mesh(m: Dictionary) -> ArrayMesh:
	var mesh := ArrayMesh.new()
	var verts: PackedVector3Array = m.get("vertices", PackedVector3Array())
	if verts.is_empty():
		return mesh
	var arrays := []
	arrays.resize(Mesh.ARRAY_MAX)
	arrays[Mesh.ARRAY_VERTEX] = verts
	arrays[Mesh.ARRAY_NORMAL] = m["normals"]
	arrays[Mesh.ARRAY_COLOR] = m["colors"]
	arrays[Mesh.ARRAY_BONES] = m["bones"]
	arrays[Mesh.ARRAY_WEIGHTS] = m["weights"]
	arrays[Mesh.ARRAY_INDEX] = m["indices"]
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	return mesh

func _process(delta: float) -> void:
	if key != "" and info.is_empty():
		var d: Dictionary = App.session.body(key)
		if bool(d.get("ready", false)):
			_build(d)
	if spin and not dragging:
		yaw += delta * 0.35
	pivot.rotation = Vector3(pitch, yaw, 0.0)

func _build(d: Dictionary) -> void:
	info = d
	meshes = []
	for m in d["lods"]:
		meshes.append(array_mesh(m))
	organs.mesh = array_mesh(d["organs"])
	body_material.set_shader_parameter("gloss", float(d["gloss"]))
	body_material.set_shader_parameter("iridescence", float(d["iridescence"]))
	skin.material_override = body_material
	set_relative(relative)
	set_anatomy(anatomy)
	body_ready.emit(d)

## Niveau de détail selon la taille à l'écran.
func _choose_lod() -> void:
	if meshes.is_empty():
		return
	var px := size.y * relative
	var want := 0 if px > 260.0 else (1 if px > 110.0 else 2)
	want = min(want, meshes.size() - 1)
	if want != lod:
		lod = want
		skin.mesh = meshes[lod]

func _notification(what: int) -> void:
	if what == NOTIFICATION_RESIZED:
		_choose_lod()

func _gui_input(ev: InputEvent) -> void:
	if ev is InputEventMouseButton and ev.button_index == MOUSE_BUTTON_LEFT:
		dragging = ev.pressed
		if ev.pressed:
			spin = false
	elif ev is InputEventMouseMotion and dragging:
		yaw += ev.relative.x * 0.01
		pitch = clampf(pitch + ev.relative.y * 0.01, -1.3, 1.3)

## Pixels par mètre vrai dans le portrait (barre d'échelle).
func pixels_per_metre() -> float:
	var extent := float(info.get("extent_m", 0.0))
	if extent <= 0.0:
		return 0.0
	var visible_h := 2.0 * camera.position.length() * tan(deg_to_rad(camera.fov / 2.0))
	return size.y / visible_h * relative / extent

## Longueur ronde (1, 2 ou 5 × 10ⁿ) proche du tiers de la largeur.
static func nice_length(target: float) -> float:
	if target <= 0.0:
		return 0.0
	var p := pow(10.0, floor(log(target) / log(10.0)))
	for k in [5.0, 2.0, 1.0]:
		if k * p <= target:
			return k * p
	return p

static func length_text(m: float) -> String:
	var fr: bool = App.settings["lang"] == "fr"
	var v := m
	var unit := "m"
	if m < 1.0e-3:
		v = m * 1.0e6
		unit = "µm"
	elif m < 1.0e-2:
		v = m * 1.0e3
		unit = "mm"
	elif m < 1.0:
		v = m * 1.0e2
		unit = "cm"
	var s := str(int(round(v))) if v >= 1.0 else ("%.1f" % v)
	if fr:
		s = s.replace(".", ",")
	return "%s %s" % [s, unit]

## Barre d'échelle dessinée sous le portrait par un Control parent.
func draw_scale_bar(on: Control, origin: Vector2) -> void:
	var ppm := pixels_per_metre()
	if ppm <= 0.0:
		return
	var metres := nice_length(size.x * 0.33 / ppm)
	var px := metres * ppm
	on.draw_line(origin, origin + Vector2(px, 0), Atlas.INK, 3.0)
	on.draw_line(origin + Vector2(0, -5), origin + Vector2(0, 5), Atlas.INK, 1.5)
	on.draw_line(origin + Vector2(px, -5), origin + Vector2(px, 5), Atlas.INK, 1.5)
	on.draw_string(Atlas.body_font(), origin + Vector2(px + 8, 5), length_text(metres), HORIZONTAL_ALIGNMENT_LEFT, -1, 14, Atlas.INK)
