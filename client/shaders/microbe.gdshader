shader_type spatial;
// Microbes de la lame en 3D, à la manière de l'Atlas : vus en fond clair,
// les cellules sont translucides (le cœur pâle, le bord plus dense), lavées
// de leur pigment, cernées d'un trait d'encre ; hachures légères à l'ombre.
render_mode unshaded, cull_disabled;

uniform vec3 paper = vec3(0.949, 0.922, 0.847);
uniform vec3 ink = vec3(0.290, 0.200, 0.133);
uniform vec3 light_dir = vec3(0.3, 0.9, 0.3);
uniform float hatch_spacing = 4.0;

varying vec3 colour;

void vertex() {
	colour = COLOR.rgb;
}

void fragment() {
	vec3 n = normalize(NORMAL);
	vec3 v = normalize(VIEW);
	float facing = abs(dot(n, v));
	// Translucide : le pigment se lit sur les bords, où la cellule est épaisse.
	vec3 wash = mix(paper, colour, mix(0.95, 0.45, facing));
	vec3 nw = normalize((INV_VIEW_MATRIX * vec4(n * sign(dot(n, v) + 1e-4), 0.0)).xyz);
	float lit = smoothstep(-0.3, 0.7, dot(nw, normalize(light_dir)));
	wash *= mix(0.8, 1.03, lit);
	vec2 p = FRAGCOORD.xy / hatch_spacing;
	float line = step(abs(fract(p.x + p.y) - 0.5), 0.12);
	wash = mix(wash, ink, line * (1.0 - lit) * 0.25);
	// Cerne d'encre au bord de la silhouette.
	float rim = smoothstep(0.32, 0.08, facing);
	wash = mix(wash, ink, rim * 0.85);
	ALBEDO = pow(wash, vec3(2.2));
}
