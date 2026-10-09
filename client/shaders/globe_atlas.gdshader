// Globe de l'Atlas naturaliste (document Direction artistique, « Globe ») :
// océans en lavis bleu-gris selon la profondeur avec isobathes, terres à
// l'aquarelle, littoral à l'encre, graticule tous les 30°, côté nuit en
// hachures croisées, calques de données posés en lavis sous l'encre.
//
// Les variables simulées arrivent dans trois textures de données (une case
// par cellule, voir evo-view/src/layers.rs). Les grandeurs continues sont
// interpolées entre le pas précédent et le pas courant ; les continents, eux,
// glissent : chaque sommet est tourné autour du pôle de sa plaque d'une
// fraction du mouvement du pas (document Globe, « Interpolation »).
shader_type spatial;
render_mode unshaded, cull_back, depth_draw_opaque;

uniform sampler2D data0 : filter_nearest;
uniform sampler2D data1 : filter_nearest;
uniform sampler2D data2 : filter_nearest;
uniform sampler2D data3 : filter_nearest;
// Comparaison « avec et sans » (étape 4, voir evo-view/src/compare.rs).
uniform sampler2D data4 : filter_nearest;
uniform sampler2D prev0 : filter_nearest;
uniform sampler2D prev2 : filter_nearest;
uniform sampler2D palette : filter_linear, repeat_disable;
uniform sampler2D palette_cat : filter_nearest, repeat_disable;

uniform float blend = 1.0;
uniform float step_myr = 0.0;
// Rayon de la planète, m (glissement des plaques).
uniform float radius_m = 6.371e6;
uniform float relief_scale = 0.0;

uniform bool layer_on = false;
uniform int layer_tex = 0;
uniform int layer_channel = 0;
// 0 linéaire, 1 log10 à calculer, 2 déjà en log10, 3 catégorie
uniform int layer_transform = 0;
uniform float layer_min = 0.0;
uniform float layer_max = 1.0;
uniform float layer_opacity = 0.6;

uniform bool natural_life = true;
uniform bool show_graticule = true;
uniform bool show_terminator = false;
uniform bool show_isolines = true;
uniform vec3 light_dir = vec3(-0.5, 0.6, 0.62);
// Couleurs passées brutes (sRGB) : tout le mélange se fait en sRGB, comme
// sur le papier, puis la couleur finale est convertie en linéaire.
uniform vec3 star_tint = vec3(1.0);
uniform vec3 selected_pos = vec3(0.0);
uniform float selected_radius = 0.0;
uniform vec3 hover_pos = vec3(0.0);
uniform float hover_radius = 0.0;
uniform float ink_strength = 1.0;
uniform bool show_focus = true;
// Incrément G3 : le maillage d'une région subdivisée porte l'altitude fine
// de chaque sommet (UV.x, m) ; le globe s'efface sous la calotte qu'elle
// couvre (direction, cosinus du rayon ; 2 : pas de région).
instance uniform float regional = 0.0;
uniform vec4 region_cap = vec4(0.0, 1.0, 0.0, 2.0);
// Fondu des hachures de pente : 0 au globe entier, 1 aux paysages.
uniform float landscape = 0.0;
// Sources hydrothermales cerclées de vermillon (écran d'ensemencement).

const vec3 PAPER = vec3(0.925, 0.886, 0.788);
const vec3 WATER = vec3(0.725, 0.776, 0.749);
const vec3 OCHRE = vec3(0.780, 0.604, 0.333);
const vec3 INK = vec3(0.290, 0.200, 0.133);
const vec3 VERMILION = vec3(0.604, 0.231, 0.133);
const vec3 ICE = vec3(0.95, 0.965, 0.975);

varying vec3 v_dir;
varying float v_height;
varying float v_temp;
varying float v_bio;
varying float v_ice;
varying float v_layer;
varying float v_share;
varying vec3 v_pig;
varying float v_pig_w;
varying float v_ocean;
varying float v_focus;
varying flat float v_cat;
varying vec3 v_world_n;
varying vec3 v_grad;

vec3 to_linear(vec3 c) {
	return mix(c / 12.92, pow((c + 0.055) / 1.055, vec3(2.4)), step(0.04045, c));
}

vec4 fetch(sampler2D t, vec2 uv) {
	ivec2 s = textureSize(t, 0);
	return texelFetch(t, ivec2(uv * vec2(s)), 0);
}

float hash(vec3 p) {
	p = fract(p * 0.3183099 + 0.1);
	p *= 17.0;
	return fract(p.x * p.y * p.z * (p.x + p.y + p.z));
}

float noise3(vec3 x) {
	vec3 i = floor(x);
	vec3 f = fract(x);
	f = f * f * (3.0 - 2.0 * f);
	return mix(mix(mix(hash(i), hash(i + vec3(1, 0, 0)), f.x), mix(hash(i + vec3(0, 1, 0)), hash(i + vec3(1, 1, 0)), f.x), f.y),
		mix(mix(hash(i + vec3(0, 0, 1)), hash(i + vec3(1, 0, 1)), f.x), mix(hash(i + vec3(0, 1, 1)), hash(i + vec3(1, 1, 1)), f.x), f.y), f.z);
}

float fbm(vec3 p) {
	float s = 0.0;
	float a = 0.5;
	for (int i = 0; i < 4; i++) {
		s += a * noise3(p);
		p *= 2.03;
		a *= 0.5;
	}
	return s / 0.9375;
}

// Trait antialiasé sur les isovaleurs entières de `x`.
float isoline(float x, float width) {
	float d = abs(fract(x - 0.5) - 0.5);
	float w = fwidth(x) * width;
	return 1.0 - smoothstep(0.0, w, d);
}

float channel(vec4 v, int c) {
	if (c == 0) return v.r;
	if (c == 1) return v.g;
	if (c == 2) return v.b;
	return v.a;
}

void vertex() {
	vec2 uv = UV2;
	vec4 d0 = fetch(data0, uv);
	vec4 d1 = fetch(data1, uv);
	vec4 d2 = fetch(data2, uv);
	vec4 p0 = fetch(prev0, uv);
	vec4 p2 = fetch(prev2, uv);
	vec4 d3 = fetch(data3, uv);
	vec4 d4 = layer_tex == 4 ? fetch(data4, uv) : vec4(0.0);
	vec3 dir = normalize(VERTEX);
	// Glissement des plaques : la position courante, reculée de la part du
	// pas qui n'est pas encore jouée, le long de la vitesse de la plaque
	// (cm/an vers l'est et vers le nord ; le haut de Godot est le nord).
	if (step_myr > 0.0 && abs(dir.y) < 0.999) {
		vec3 east = normalize(cross(vec3(0.0, 1.0, 0.0), dir));
		vec3 north = cross(dir, east);
		float k = 0.01 * step_myr * 1.0e6 * (1.0 - blend) / radius_m;
		dir = normalize(dir - (east * d3.r + north * d3.g) * k);
	}
	v_height = regional > 0.5 ? UV.x : d0.r;
	v_temp = mix(p0.g, d0.g, blend);
	v_bio = mix(p0.b, d0.b, blend);
	v_ice = mix(p2.b, d2.b, blend);
	if (regional > 0.5) {
		// Région : les données des trois cellules qui entourent le sommet,
		// pondérées, plutôt que celles de la seule plus proche.
		vec3 w = CUSTOM1.xyz;
		vec4 d0b = fetch(data0, CUSTOM0.xy);
		vec4 d0c = fetch(data0, CUSTOM0.zw);
		vec4 p0b = fetch(prev0, CUSTOM0.xy);
		vec4 p0c = fetch(prev0, CUSTOM0.zw);
		vec4 d2b = fetch(data2, CUSTOM0.xy);
		vec4 d2c = fetch(data2, CUSTOM0.zw);
		vec4 p2b = fetch(prev2, CUSTOM0.xy);
		vec4 p2c = fetch(prev2, CUSTOM0.zw);
		v_temp = w.x * v_temp + w.y * mix(p0b.g, d0b.g, blend) + w.z * mix(p0c.g, d0c.g, blend);
		v_bio = w.x * v_bio + w.y * mix(p0b.b, d0b.b, blend) + w.z * mix(p0c.b, d0c.b, blend);
		v_ice = w.x * v_ice + w.y * mix(p2b.b, d2b.b, blend) + w.z * mix(p2c.b, d2c.b, blend);
	}
	v_share = d2.a;
	v_pig = d1.rgb;
	int flags = int(d1.a + 0.5);
	v_ocean = float((flags & 1) != 0);
	v_pig_w = float((flags & 4) != 0);
	v_focus = float((flags & 16) != 0);
	vec4 lt = layer_tex == 0 ? mix(p0, d0, blend) : (layer_tex == 1 ? d1 : (layer_tex == 2 ? mix(p2, d2, blend) : (layer_tex == 3 ? d3 : d4)));
	float raw = channel(lt, layer_channel);
	if (layer_transform == 1) {
		raw = log(max(raw, 1e-30)) / log(10.0);
	}
	v_layer = raw;
	v_cat = channel(layer_tex == 2 ? d2 : d1, layer_channel);
	v_dir = dir;
	VERTEX = dir * (1.0 + relief_scale * max(v_height, 0.0));
	v_grad = regional > 0.5 ? CUSTOM2.xyz : vec3(0.0);
	v_world_n = normalize((MODEL_MATRIX * vec4(dir, 0.0)).xyz);
}

void fragment() {
	vec3 n = normalize(v_dir);
	if (regional < 0.5 && dot(n, region_cap.xyz) > region_cap.w) {
		discard;
	}
	// Grain du papier (une octave suffit) et taches du lavis.
	float grain = noise3(n * 180.0);
	float bloom = fbm(n * 22.0 + 3.1);
	// Le trait de côte suit la hauteur, troublée d'un bruit de quelques
	// centaines de mètres : des rivages dessinés à main levée plutôt que les
	// arêtes de la grille. Ce n'est qu'un dessin : l'inspecteur dit le vrai.
	float height = v_height + (fbm(n * 64.0 + 7.3) - 0.5) * 700.0 * (1.0 - regional);
	vec3 col = PAPER * (0.96 + 0.06 * grain);
	// La mer est là où la surface est sous le niveau de la mer : le même
	// critère que le trait de côte.
	float ocean = 1.0 - step(0.0, height);
	// Mer : lavis superposé, une passe de plus par 1 500 m de profondeur.
	float depth = max(-height, 0.0);
	float passes = 1.0 + floor(clamp(depth / 1500.0, 0.0, 4.0)) + 0.25 * bloom;
	vec3 sea = PAPER * mix(vec3(1.0), WATER, clamp(0.32 * passes, 0.0, 1.0));
	// Terres : aquarelle ocre plus pâle en altitude, bords de lavis irréguliers.
	float h = max(height, 0.0);
	vec3 land = mix(PAPER * mix(vec3(1.0), OCHRE, 0.55 + 0.15 * bloom), PAPER * mix(vec3(1.0), OCHRE, 0.25), clamp(h / 4000.0, 0.0, 1.0));
	col = mix(land, sea, ocean);
	// Le vivant colore la planète : tapis et efflorescences du pigment réel.
	if (natural_life && v_pig_w > 0.5) {
		float cover = clamp((v_bio - 3.0) / 7.0, 0.0, 1.0) * (0.75 + 0.25 * bloom);
		col *= mix(vec3(1.0), v_pig, 0.45 * cover);
	}
	// Glace : papier laissé blanc, bleuté.
	float ice_edge = v_ice + regional * (fbm(n * 260.0) - 0.5) * 0.4;
	col = mix(col, ICE * (0.97 + 0.03 * grain), smoothstep(0.35, 0.65, ice_edge));
	// La lumière de l'étoile teinte le lavis (naine rouge plus chaude).
	col *= mix(vec3(1.0), star_tint, 0.18);
	// Calque de données en lavis sous l'encre.
	if (layer_on) {
		vec3 pc;
		if (layer_transform == 3) {
			float idx = floor(v_cat + 0.5);
			pc = texture(palette_cat, vec2((mod(idx, 255.0) + 0.5) / 256.0, 0.5)).rgb;
			if (idx < 0.5) pc = PAPER;
		} else {
			float t = clamp((v_layer - layer_min) / (layer_max - layer_min), 0.0, 1.0);
			pc = texture(palette, vec2(t * 0.996 + 0.002, 0.5)).rgb;
		}
		col = mix(col, pc * (0.96 + 0.06 * grain), layer_opacity);
	}
	// Aire de l'espèce choisie : hachures vermillon.
	if (show_focus && v_focus > 0.5) {
		float hatch = step(fract((FRAGCOORD.x - FRAGCOORD.y) / 7.0), 0.28);
		col = mix(col, VERMILION, 0.5 * hatch);
	}
	// Ombrage léger du relief et lumière d'atelier ou terminateur.
	float ndl = dot(normalize(v_world_n), normalize(light_dir));
	col *= 0.9 + 0.1 * clamp(ndl, 0.0, 1.0);
	float ink = 0.0;
	// Paysages (G3) : ombrage des versants depuis la normale de la facette,
	// hachures d'encre dans les ombres, plus serrées sur les pentes fortes.
	if (regional > 0.5) {
		// Normale lissée du relief exagéré : la sphère inclinée par le
		// gradient de l'altitude.
		vec3 g = v_grad - n * dot(v_grad, n);
		vec3 fn = normalize(n - relief_scale * g);
		// Estompage de cartographe : lumière du nord-ouest à 45°, dans le
		// repère local du sol.
		vec3 east = normalize(cross(vec3(0.0, 1.0, 0.0), n) + vec3(1e-5, 0.0, 0.0));
		vec3 north = cross(n, east);
		vec3 sun = normalize(n + 0.75 * (north - east));
		float lam = dot(fn, sun);
		float flat_lam = dot(n, sun);
		// Pour le contour d'encre : la normale du relief, pas celle de la
		// sphère.
		NORMAL = normalize((VIEW_MATRIX * vec4(fn, 0.0)).xyz);
		float land = 1.0 - ocean;
		col *= mix(1.0, clamp(0.2 + 0.92 * lam / flat_lam, 0.5, 1.15), landscape * land);
		float shadow = clamp((flat_lam - 0.12 - lam) * 3.0, 0.0, 1.0) * land * landscape;
		float h1 = step(fract((FRAGCOORD.x + FRAGCOORD.y) / 5.0), 0.12 + 0.3 * shadow);
		float h2 = step(fract((FRAGCOORD.x - FRAGCOORD.y) / 5.0), 0.25 * shadow) * step(0.6, shadow);
		ink = max(ink, 0.5 * max(h1, h2) * smoothstep(0.05, 0.4, shadow));
	}
	// Isobathes tous les 1 000 m, courbes de niveau tous les 1 000 m.
	if (show_isolines) {
		ink = max(ink, 0.35 * isoline(depth / 1000.0, 0.8) * ocean * step(200.0, depth));
		ink = max(ink, 0.25 * isoline(h / 1000.0, 0.8) * (1.0 - ocean) * step(200.0, h));
	}
	// Littoral à l'encre : là où la hauteur change de signe.
	float coast = 1.0 - smoothstep(0.0, fwidth(height) * 1.6, abs(height));
	ink = max(ink, coast);
	// Graticule tous les 30°.
	if (show_graticule) {
		float lat = degrees(asin(clamp(n.y, -1.0, 1.0)));
		float lon = degrees(atan(n.z, n.x));
		float g = max(isoline(lat / 30.0, 0.7), isoline(lon / 30.0, 0.7) * step(abs(lat), 85.0));
		ink = max(ink, 0.3 * g);
	}
	// Côté nuit : hachures croisées qui s'épaississent derrière le terminateur.
	if (show_terminator && ndl < 0.05) {
		float night = clamp((0.05 - ndl) * 2.5, 0.0, 1.0);
		float h1 = step(fract((FRAGCOORD.x + FRAGCOORD.y) / 6.0), 0.15 + 0.35 * night);
		float h2 = step(fract((FRAGCOORD.x - FRAGCOORD.y) / 6.0), 0.35 * night) * step(0.45, night);
		ink = max(ink, 0.55 * max(h1, h2) * smoothstep(0.0, 0.3, night));
	}
	col = mix(col, INK, clamp(ink * ink_strength, 0.0, 1.0));
	// Cellule survolée et cellule choisie : cercles au vermillon.
	if (hover_radius > 0.0) {
		float d = acos(clamp(dot(n, normalize(hover_pos)), -1.0, 1.0));
		float ring = 1.0 - smoothstep(0.0, fwidth(d) * 1.5, abs(d - hover_radius));
		col = mix(col, VERMILION, 0.6 * ring);
	}
	if (selected_radius > 0.0) {
		float d = acos(clamp(dot(n, normalize(selected_pos)), -1.0, 1.0));
		float ring = 1.0 - smoothstep(0.0, fwidth(d) * 2.2, abs(d - selected_radius));
		float ring2 = 1.0 - smoothstep(0.0, fwidth(d) * 1.2, abs(d - selected_radius * 1.35));
		col = mix(col, VERMILION, max(ring, 0.7 * ring2));
	}
	ALBEDO = to_linear(col);
}
