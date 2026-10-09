shader_type spatial;
// Contour à l'encre en post-traitement (document Globe 3D, incrément G3) :
// un quadrilatère plein écran lit la profondeur et les normales de la scène
// et trace un trait là où elles changent brusquement (crêtes, falaises,
// silhouettes du relief contre le ciel). Atténué au globe entier, plein aux
// paysages.
render_mode unshaded, cull_disabled, depth_test_disabled, depth_draw_never, blend_mix;

uniform sampler2D depth_tex : hint_depth_texture, filter_nearest;
uniform sampler2D normal_tex : hint_normal_roughness_texture, filter_nearest;
uniform vec3 ink = vec3(0.290, 0.200, 0.133);
uniform float strength = 0.0;
uniform float depth_threshold = 0.04;
uniform float normal_threshold = 0.5;

void vertex() {
	POSITION = vec4(VERTEX.xy, 1.0, 1.0);
}

float linear_depth(vec2 uv, mat4 inv_proj) {
	float d = texture(depth_tex, uv).r;
	vec4 v = inv_proj * vec4(uv * 2.0 - 1.0, d, 1.0);
	return -v.z / v.w;
}

vec3 normal_at(vec2 uv) {
	return texture(normal_tex, uv).xyz * 2.0 - 1.0;
}

void fragment() {
	if (strength <= 0.0) {
		discard;
	}
	vec2 px = 1.0 / VIEWPORT_SIZE;
	vec2 uv = SCREEN_UV;
	float c = linear_depth(uv, INV_PROJECTION_MATRIX);
	float e = 0.0;
	vec3 nc = normal_at(uv);
	for (int k = 0; k < 4; k++) {
		vec2 o = k == 0 ? vec2(px.x, 0.0) : (k == 1 ? vec2(-px.x, 0.0) : (k == 2 ? vec2(0.0, px.y) : vec2(0.0, -px.y)));
		float d = linear_depth(uv + o, INV_PROJECTION_MATRIX);
		// Saut de profondeur relatif : le trait ne dépend pas de la distance.
		e = max(e, smoothstep(depth_threshold, depth_threshold * 2.0, abs(d - c) / max(c, 1e-4)));
		vec3 n = normal_at(uv + o);
		e = max(e, smoothstep(normal_threshold, normal_threshold + 0.3, 1.0 - dot(nc, n)));
	}
	if (e <= 0.01) {
		discard;
	}
	ALBEDO = ink;
	ALPHA = e * strength;
}
