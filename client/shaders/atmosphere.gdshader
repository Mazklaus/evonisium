// Limbe de l'atmosphère et contour du globe en double filet (DA Atlas :
// « contour du globe en double filet »). La coque est un peu plus grande que
// la planète ; chaque fragment calcule la distance de son rayon de vue au
// centre, en rayons planétaires : 1 au bord du globe.
shader_type spatial;
render_mode unshaded, blend_mix, depth_draw_never, cull_back, shadows_disabled;

uniform vec3 atmosphere_colour = vec3(0.8, 0.75, 0.65);
uniform float shell_radius = 1.06;
uniform float thickness = 0.035;
uniform float ink_strength = 1.0;

const vec3 INK = vec3(0.290, 0.200, 0.133);

varying vec3 v_obj;

vec3 to_linear(vec3 c) {
	return mix(c / 12.92, pow((c + 0.055) / 1.055, vec3(2.4)), step(0.04045, c));
}

void vertex() {
	v_obj = VERTEX;
}

void fragment() {
	// Rayon de vue en coordonnées de l'objet (globe de rayon 1).
	vec3 cam = (inverse(MODEL_MATRIX) * vec4(CAMERA_POSITION_WORLD, 1.0)).xyz;
	vec3 dir = normalize(v_obj * shell_radius - cam);
	float t = -dot(cam, dir);
	float d = length(cam + dir * t);
	float w = fwidth(d);
	// Le disque de la planète reste visible.
	float outside = smoothstep(1.0 - w, 1.0 + w, d);
	// Lavis de l'atmosphère, dense au bord, qui s'efface vers l'espace.
	float halo = outside * exp(-(d - 1.0) / thickness) * 0.55;
	float line1 = 1.0 - smoothstep(0.0, w * 1.4, abs(d - 1.0 - w));
	float line2 = 1.0 - smoothstep(0.0, w * 0.9, abs(d - 1.0 - 0.012 - 4.0 * w));
	float ink = max(line1, 0.8 * line2) * ink_strength;
	vec3 col = mix(atmosphere_colour, INK, clamp(ink / max(halo + ink, 1e-3), 0.0, 1.0));
	ALBEDO = to_linear(col);
	ALPHA = clamp(halo + ink, 0.0, 1.0);
}
