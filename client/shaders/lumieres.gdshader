shader_type spatial;
// Lumières des villes, côté nuit (étape 7) : une lueur dorée et douce par
// cellule habitée, posée à plat sur le globe, qui s'allume derrière le
// terminateur. Sans terminateur (lumière d'atelier), rien ne s'allume.
render_mode unshaded, blend_add, depth_draw_never, cull_disabled;

uniform vec3 light_dir = vec3(-0.6, 0.35, 0.72);
uniform bool night_on = true;
uniform float size = 0.02;

varying float v_night;
varying float v_glow;

void vertex() {
	VERTEX.xy *= size * (0.6 + 0.8 * INSTANCE_CUSTOM.r);
	vec3 up = normalize((MODEL_MATRIX * vec4(0.0, 0.0, 1.0, 0.0)).xyz);
	float ndl = dot(up, normalize(light_dir));
	v_night = night_on ? smoothstep(0.02, -0.12, ndl) : 0.0;
	v_glow = INSTANCE_CUSTOM.r;
}

void fragment() {
	float r = length(UV - vec2(0.5)) * 2.0;
	float core = exp(-r * r * 6.0);
	float a = core * v_night * (0.35 + 0.65 * v_glow);
	if (a < 0.003) {
		discard;
	}
	ALBEDO = vec3(1.0, 0.78, 0.42) * a * 1.4;
}
