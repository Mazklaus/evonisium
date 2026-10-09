shader_type spatial;
// Contour à la plume : la coque retournée du corps, gonflée le long des
// normales lissées, peinte à l'encre.
render_mode unshaded, cull_front;

uniform vec3 ink = vec3(0.290, 0.200, 0.133);
uniform float width = 0.012;
uniform float opacity = 1.0;

void vertex() {
	VERTEX += NORMAL * width;
}

void fragment() {
	ALBEDO = ink;
	ALPHA = opacity;
}
