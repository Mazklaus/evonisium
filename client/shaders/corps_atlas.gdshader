shader_type spatial;
// Corps du vivant à la manière de l'Atlas naturaliste (DA retenue) : lavis
// du pigment réel posé sur le papier, plus clair dans la lumière, hachures
// d'encre dans les ombres, liseré sombre sur les bords. Le contour d'encre
// est une seconde passe (corps_contour.gdshader).
render_mode diffuse_toon, specular_disabled, cull_back, depth_draw_always;

uniform vec3 paper = vec3(0.925, 0.886, 0.788);
uniform vec3 ink = vec3(0.290, 0.200, 0.133);
uniform float gloss = 0.3;
uniform float iridescence = 0.0;
// Vue anatomie : la peau devient un voile.
uniform float opacity : hint_range(0.0, 1.0) = 1.0;
uniform float hatch_spacing = 6.0;

varying vec3 colour;
varying float pattern;

void vertex() {
	colour = COLOR.rgb;
	pattern = COLOR.a;
}

void fragment() {
	// Lavis : le pigment se mêle au papier, sans saturation de rendu 3D.
	vec3 wash = mix(paper, colour, 0.85);
	float rim = 1.0 - clamp(dot(NORMAL, VIEW), 0.0, 1.0);
	// Irisation (couleur structurale) : une teinte qui tourne avec l'angle.
	vec3 iri = 0.5 + 0.5 * cos(6.2831 * (rim * 1.5 + vec3(0.0, 0.33, 0.67)));
	wash = mix(wash, iri, iridescence * rim * 0.6);
	ALBEDO = wash;
	ROUGHNESS = 1.0 - gloss * 0.6;
	ALPHA = opacity;
}

void light() {
	float ndl = dot(NORMAL, LIGHT);
	float lit = smoothstep(-0.1, 0.25, ndl);
	// Hachures diagonales à l'écran, plus denses dans l'ombre.
	vec2 p = FRAGCOORD.xy / hatch_spacing;
	float line1 = abs(fract(p.x + p.y) - 0.5);
	float line2 = abs(fract(p.x - p.y) - 0.5);
	float shade = 1.0 - lit;
	float hatch = step(line1, 0.12) * step(0.35, shade) + step(line2, 0.12) * step(0.75, shade);
	vec3 base = ALBEDO * mix(0.72, 1.05, lit);
	base = mix(base, ink, clamp(hatch, 0.0, 1.0) * 0.55);
	// Reflet doux sur les revêtements luisants (mucus, cuticule).
	float spec = pow(max(dot(reflect(-LIGHT, NORMAL), VIEW), 0.0), 24.0) * gloss;
	DIFFUSE_LIGHT += (base + vec3(spec) * 0.35) * ATTENUATION;
}
