shader_type spatial;
// Animaux de la scène au sol (étape 5), à la manière de l'Atlas naturaliste
// comme corps_atlas.gdshader : lavis du pigment, hachures dans les ombres.
// Le squelette se lit dans une texture des poses : une ligne par instance
// (INSTANCE_ID), trois texels RGBAF par os, les trois lignes de sa
// transformation. Chaque sommet suit deux os (UV : os, UV2 : poids).
render_mode unshaded, cull_back;

uniform sampler2D poses : filter_nearest;
uniform vec3 paper = vec3(0.925, 0.886, 0.788);
uniform vec3 ink = vec3(0.290, 0.200, 0.133);
uniform float gloss = 0.3;
uniform float hatch_spacing = 5.0;
// Direction de la lumière (vers le soleil), repère du monde.
uniform vec3 sun_dir = vec3(0.4, 0.8, 0.45);

varying vec3 colour;

vec3 bone_point(int b, int row, vec3 p) {
	vec4 r0 = texelFetch(poses, ivec2(b * 3, row), 0);
	vec4 r1 = texelFetch(poses, ivec2(b * 3 + 1, row), 0);
	vec4 r2 = texelFetch(poses, ivec2(b * 3 + 2, row), 0);
	vec4 q = vec4(p, 1.0);
	return vec3(dot(r0, q), dot(r1, q), dot(r2, q));
}

vec3 bone_normal(int b, int row, vec3 n) {
	vec3 r0 = texelFetch(poses, ivec2(b * 3, row), 0).xyz;
	vec3 r1 = texelFetch(poses, ivec2(b * 3 + 1, row), 0).xyz;
	vec3 r2 = texelFetch(poses, ivec2(b * 3 + 2, row), 0).xyz;
	return vec3(dot(r0, n), dot(r1, n), dot(r2, n));
}

void vertex() {
	int a = int(UV.x + 0.5);
	int b = int(UV.y + 0.5);
	float wa = UV2.x;
	float wb = UV2.y;
	float sum = max(wa + wb, 1e-5);
	vec3 p = (bone_point(a, INSTANCE_ID, VERTEX) * wa + bone_point(b, INSTANCE_ID, VERTEX) * wb) / sum;
	vec3 n = bone_normal(a, INSTANCE_ID, NORMAL) * wa + bone_normal(b, INSTANCE_ID, NORMAL) * wb;
	VERTEX = p;
	NORMAL = normalize(n);
	colour = COLOR.rgb;
}

vec3 to_linear(vec3 c) {
	return pow(c, vec3(2.2));
}

void fragment() {
	// Lavis : le pigment se mêle au papier ; lumière et hachures calculées
	// ici (rendu de l'Atlas, indépendant de l'éclairage de la scène).
	vec3 wash = mix(paper, colour, 0.85);
	vec3 n = normalize((INV_VIEW_MATRIX * vec4(NORMAL, 0.0)).xyz);
	float lit = smoothstep(-0.1, 0.5, dot(n, normalize(sun_dir)));
	vec2 p = FRAGCOORD.xy / hatch_spacing;
	float line1 = abs(fract(p.x + p.y) - 0.5);
	float line2 = abs(fract(p.x - p.y) - 0.5);
	float shade = 1.0 - lit;
	float hatch = step(line1, 0.12) * step(0.45, shade) + step(line2, 0.12) * step(0.8, shade);
	vec3 base = wash * mix(0.62, 1.0, lit);
	base = mix(base, ink, clamp(hatch, 0.0, 1.0) * 0.5);
	// Liseré d'encre sur le pourtour.
	float rim = 1.0 - clamp(dot(NORMAL, VIEW), 0.0, 1.0);
	base = mix(base, ink, smoothstep(0.75, 0.95, rim) * 0.7);
	ALBEDO = to_linear(base);
}
