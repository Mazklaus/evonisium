extends Node3D
## Sociétés sur le globe (étape 7, sur sociétés d'essai) : villes au
## vermillon (un carré, doublé de son nom pour les plus grandes), routes
## d'échange en pointillés, lumières des villes côté nuit. Les territoires
## et les champs sont peints par le globe lui-même (texture data5).
## Tout est relu quand l'histoire d'essai publie un nouvel état.

const LIGHT_SHADER := preload("res://shaders/lumieres.gdshader")
## Rayon où se posent les marques, au-dessus du relief.
const LIFT := 1.012
## Villes nommées au plus.
const NAMED_CITIES := 24

var globe: Node3D
var serial := -1
var refresh_timer := 0.0
var cities: MultiMeshInstance3D
var routes: MeshInstance3D
var lights: MultiMeshInstance3D
var labels: Node3D
var light_material: ShaderMaterial

func _init(g: Node3D) -> void:
	globe = g

func _ready() -> void:
	var square := BoxMesh.new()
	square.size = Vector3(1, 1, 0.05)
	var city_mat := StandardMaterial3D.new()
	city_mat.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	city_mat.albedo_color = Atlas.VERMILION
	city_mat.no_depth_test = false
	cities = MultiMeshInstance3D.new()
	cities.multimesh = MultiMesh.new()
	cities.multimesh.transform_format = MultiMesh.TRANSFORM_3D
	cities.multimesh.mesh = square
	cities.material_override = city_mat
	cities.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	add_child(cities)

	var route_mat := StandardMaterial3D.new()
	route_mat.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	route_mat.vertex_color_use_as_albedo = true
	route_mat.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	routes = MeshInstance3D.new()
	routes.material_override = route_mat
	routes.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	add_child(routes)

	var quad := QuadMesh.new()
	quad.size = Vector2(1, 1)
	light_material = ShaderMaterial.new()
	light_material.shader = LIGHT_SHADER
	quad.material = light_material
	lights = MultiMeshInstance3D.new()
	lights.multimesh = MultiMesh.new()
	lights.multimesh.transform_format = MultiMesh.TRANSFORM_3D
	lights.multimesh.use_custom_data = true
	lights.multimesh.mesh = quad
	lights.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	add_child(lights)

	labels = Node3D.new()
	add_child(labels)

## Relit l'état publié s'il a changé (au plus deux fois par seconde).
func update(delta: float) -> void:
	refresh_timer -= delta
	if refresh_timer <= 0.0:
		refresh_timer = 0.5
		var s: int = App.session.society_serial()
		if s != serial:
			serial = s
			_rebuild()
	# Taille constante à l'écran : les marques suivent la distance.
	var d: float = globe.distance - 1.0
	var size := clampf(d * 0.008, 0.001, 0.02)
	_scale_cities(size)
	light_material.set_shader_parameter("light_dir", globe.material.get_shader_parameter("light_dir"))
	light_material.set_shader_parameter("night_on", bool(App.settings["terminator"]))
	light_material.set_shader_parameter("size", clampf(d * 0.03, 0.006, 0.08))
	var eye: Vector3 = globe.camera.global_position.normalized()
	for l in labels.get_children():
		var lb := l as Label3D
		var facing := lb.position.normalized().dot(eye)
		lb.visible = facing > 0.25 and (d < 1.6 or int(lb.get_meta("rank")) < 8)
		lb.modulate.a = clampf((facing - 0.25) / 0.2, 0.0, 1.0)

## Écarte les noms qui se chevauchent à l'écran : on parcourt les noms du
## plus important au moins important, et un nom qui tomberait sur un nom
## déjà posé se tait. `taken` garde les rectangles posés d'un appel à
## l'autre (noms des sociétés d'abord, puis des villes).
static func declutter(nodes: Array, camera: Camera3D, taken: Array) -> void:
	var vp := camera.get_viewport().get_visible_rect().size
	for n in nodes:
		var lb := n as Label3D
		if not lb.visible or lb.is_queued_for_deletion():
			continue
		var at := camera.unproject_position(lb.global_position)
		# Taille à l'écran d'un nom à taille fixe : pixel_size rapporté à
		# la hauteur de la vue et à l'ouverture de la caméra.
		var k := lb.pixel_size * vp.y / (2.0 * tan(deg_to_rad(camera.fov) * 0.5))
		var w := lb.font.get_string_size(lb.text, HORIZONTAL_ALIGNMENT_LEFT, -1, lb.font_size).x * k
		var h := lb.font_size * k
		var r := Rect2(at + Vector2(-w * 0.5, -lb.offset.y * k - h * 0.5), Vector2(w, h)).grow(4.0)
		var clash := false
		for t in taken:
			if (t as Rect2).intersects(r):
				clash = true
				break
		if clash:
			lb.visible = false
		else:
			taken.append(r)

var city_points: Array = []
var last_size := -1.0

func _scale_cities(size: float) -> void:
	if absf(size - last_size) < last_size * 0.03:
		return
	last_size = size
	var mm := cities.multimesh
	for i in city_points.size():
		var c: Dictionary = city_points[i]
		var p: Vector3 = c["position"].normalized()
		var s: float = size * (1.4 if c["capital"] else 1.0) * (0.5 + 0.3 * clampf(log(maxf(float(c["population"]), 1.0)) / log(10.0) - 3.0, 0.0, 3.0))
		var up := Vector3.UP if absf(p.y) < 0.95 else Vector3.RIGHT
		var frame := Basis.looking_at(-p, up).scaled(Vector3(s, s, s))
		mm.set_instance_transform(i, Transform3D(frame, p * LIFT))

func _rebuild() -> void:
	city_points = App.session.society_cities()
	cities.multimesh.instance_count = city_points.size()
	last_size = -1.0
	for c in labels.get_children():
		c.queue_free()
	var rank := 0
	for c in city_points:
		if rank >= NAMED_CITIES:
			break
		var lb := Label3D.new()
		lb.text = str(c["name"])
		lb.font = Atlas.body_font()
		lb.font_size = 30 if c["capital"] else 24
		lb.pixel_size = 0.0008
		lb.fixed_size = true
		lb.billboard = BaseMaterial3D.BILLBOARD_ENABLED
		lb.no_depth_test = true
		lb.modulate = Atlas.VERMILION
		lb.outline_modulate = Color(Atlas.PAPER, 0.85)
		lb.outline_size = 8
		lb.render_priority = 3
		lb.offset = Vector2(0, 22)
		lb.position = (c["position"] as Vector3).normalized() * 1.02
		lb.set_meta("rank", rank)
		labels.add_child(lb)
		rank += 1
	_build_routes()
	_build_lights()

## Routes en pointillés le long du grand cercle : encre pour la terre, bleu
## de Prusse pour la mer, plus épaisses (plus opaques) quand on y échange
## beaucoup.
func _build_routes() -> void:
	var r: Dictionary = App.session.society_routes()
	var pts: PackedVector3Array = r["points"]
	var vals: PackedVector2Array = r["values"]
	if pts.is_empty():
		routes.mesh = null
		return
	var verts := PackedVector3Array()
	var cols := PackedColorArray()
	for i in vals.size():
		var a: Vector3 = pts[2 * i].normalized()
		var b: Vector3 = pts[2 * i + 1].normalized()
		var sea := vals[i].y > 0.5
		var alpha := clampf((vals[i].x - 1.0) / 4.0, 0.25, 0.9)
		var col := Color(0.2, 0.33, 0.45, alpha) if sea else Color(Atlas.VERMILION, alpha)
		# Quatre tirets par route.
		for k in 4:
			var t0 := (k + 0.15) / 4.0
			var t1 := (k + 0.65) / 4.0
			verts.append(a.slerp(b, t0) * (LIFT + 0.001))
			verts.append(a.slerp(b, t1) * (LIFT + 0.001))
			cols.append(col)
			cols.append(col)
	var arr := []
	arr.resize(Mesh.ARRAY_MAX)
	arr[Mesh.ARRAY_VERTEX] = verts
	arr[Mesh.ARRAY_COLOR] = cols
	var m := ArrayMesh.new()
	m.add_surface_from_arrays(Mesh.PRIMITIVE_LINES, arr)
	routes.mesh = m

## Lumières nocturnes : une lueur par cellule habitée assez dense, posée à
## plat sur le globe ; le shader ne les montre que côté nuit.
func _build_lights() -> void:
	var l: PackedVector4Array = App.session.society_lights()
	var mm := lights.multimesh
	mm.instance_count = l.size()
	for i in l.size():
		var p := Vector3(l[i].x, l[i].y, l[i].z).normalized()
		var up := Vector3.UP if absf(p.y) < 0.95 else Vector3.RIGHT
		mm.set_instance_transform(i, Transform3D(Basis.looking_at(-p, up), p * (LIFT + 0.004)))
		mm.set_instance_custom_data(i, Color(l[i].w, 0, 0, 0))
