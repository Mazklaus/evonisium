shader_type spatial;
// Organes de la vue anatomie : lavis franc de la couleur conventionnelle de
// leur appareil, ombré simplement.
render_mode diffuse_toon, specular_disabled;

uniform vec3 paper = vec3(0.925, 0.886, 0.788);

void fragment() {
	ALBEDO = mix(paper, COLOR.rgb, 0.9);
}
