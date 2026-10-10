shader_type spatial;
// Terrain de la scène au sol : couleurs du milieu en lavis, ombrées par le
// soleil, hachures d'encre sur les pentes fortes.
render_mode unshaded;

uniform vec3 ink = vec3(0.290, 0.200, 0.133);
uniform vec3 sun_dir = vec3(0.4, 0.8, 0.45);
uniform float hatch_spacing = 7.0;

varying vec3 colour;
varying vec3 world_normal;

void vertex() {
	colour = COLOR.rgb;
	world_normal = (MODEL_MATRIX * vec4(NORMAL, 0.0)).xyz;
}

void fragment() {
	vec3 n = normalize(world_normal);
	float lit = smoothstep(-0.2, 0.9, dot(n, normalize(sun_dir)));
	vec3 base = colour * mix(0.7, 1.04, lit);
	float slope = 1.0 - n.y;
	vec2 p = FRAGCOORD.xy / hatch_spacing;
	float line = step(abs(fract(p.x + p.y) - 0.5), 0.1);
	base = mix(base, ink, line * smoothstep(0.25, 0.5, slope) * 0.35);
	ALBEDO = pow(base, vec3(2.2));
}
