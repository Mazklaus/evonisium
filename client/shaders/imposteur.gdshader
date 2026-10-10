shader_type spatial;
// Imposteurs de la scène au sol : un carré tourné vers la caméra, qui
// montre l'une des vues pré-rendues de l'espèce (atlas : une ligne par
// espèce, `views` vues autour du corps), choisie d'après l'angle sous lequel
// la caméra voit l'individu. INSTANCE_CUSTOM.x : ligne de l'atlas.
render_mode unshaded, cull_disabled;

uniform sampler2D atlas : source_color, filter_linear;
uniform float views = 8.0;
uniform float rows = 1.0;
uniform float half_size = 0.62;

varying vec2 atlas_uv;

void vertex() {
	vec3 centre = MODEL_MATRIX[3].xyz;
	float s = length(MODEL_MATRIX[0].xyz);
	vec3 cam = INV_VIEW_MATRIX[3].xyz;
	vec3 d = cam - centre;
	// Direction de la caméra dans le repère du corps (x vers l'avant).
	mat3 rot = mat3(MODEL_MATRIX) / max(s, 1e-6);
	vec3 l = transpose(rot) * d;
	float yaw = atan(l.z, l.x);
	float k = mod(round(yaw / (TAU / views)), views);
	vec3 right = INV_VIEW_MATRIX[0].xyz;
	vec3 up = INV_VIEW_MATRIX[1].xyz;
	vec3 world = centre + (right * VERTEX.x + up * VERTEX.y) * 2.0 * half_size * s;
	POSITION = PROJECTION_MATRIX * VIEW_MATRIX * vec4(world, 1.0);
	atlas_uv = vec2((k + UV.x) / views, (INSTANCE_CUSTOM.x + UV.y) / rows);
	if (s <= 0.0) {
		POSITION = vec4(0.0, 0.0, 2.0, 1.0);
	}
}

void fragment() {
	vec4 c = texture(atlas, atlas_uv);
	ALBEDO = c.rgb;
	ALPHA = c.a;
	ALPHA_SCISSOR_THRESHOLD = 0.5;
}
