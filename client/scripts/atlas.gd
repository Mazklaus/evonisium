extends Node
## Direction artistique retenue : l'Atlas naturaliste (document DA).
## Couleurs, polices et thème de l'interface : carnet de terrain, fiches en
## cartouches à filet d'encre et ombre portée décalée, boutons en filet
## d'encre sans remplissage, survol par un léger lavis.

const PAPER := Color("#ECE2C9")
const WATER := Color("#B9C6BF")
const OCHRE := Color("#C79A55")
const PLANT := Color("#8A9A5B")
const VERMILION := Color("#9A3B22")
const INK := Color("#4A3322")
const PAPER_DARK := Color("#E2D5B5")
const SHADOW := Color(0.29, 0.2, 0.133, 0.28)

var title_font: FontFile
var title_italic: FontFile
var text_font: FontVariation
var text_italic: FontVariation
var text_bold: FontVariation
var readable_font: FontFile
var theme: Theme

func _ready() -> void:
	title_font = _load_font("res://fonts/IMFellEnglish-Regular.ttf")
	title_italic = _load_font("res://fonts/IMFellEnglish-Italic.ttf")
	var crimson := _load_font("res://fonts/CrimsonPro-Variable.ttf")
	var crimson_it := _load_font("res://fonts/CrimsonPro-Italic-Variable.ttf")
	readable_font = _load_font("res://fonts/AtkinsonHyperlegible-Regular.ttf")
	text_font = FontVariation.new()
	text_font.base_font = crimson
	text_font.variation_opentype = {"wght": 450}
	# Chiffres tabulaires (DA : « Crimson Pro en chiffres tabulaires »).
	text_font.opentype_features = {"tnum": 1}
	text_italic = FontVariation.new()
	text_italic.base_font = crimson_it
	text_bold = FontVariation.new()
	text_bold.base_font = crimson
	text_bold.variation_opentype = {"wght": 650}
	text_bold.opentype_features = {"tnum": 1}
	rebuild_theme(1.0, false, false)

## Les polices sont lues directement : le projet tourne sans étape
## d'import préalable (intégration continue, première ouverture).
func _load_font(path: String) -> FontFile:
	var f := FontFile.new()
	var bytes := FileAccess.get_file_as_bytes(path)
	if bytes.is_empty():
		push_warning("police introuvable : " + path)
		return f
	f.data = bytes
	return f

func heading_font() -> Font:
	return readable_font if App.settings.get("readable_font", false) else title_font

func body_font() -> Font:
	return readable_font if App.settings.get("readable_font", false) else text_font

## Cartouche : papier, filet d'encre, ombre portée décalée.
func cartouche(bg: Color = PAPER, border: Color = INK, width: int = 2, shadow: bool = true) -> StyleBoxFlat:
	var s := StyleBoxFlat.new()
	s.bg_color = bg
	s.border_color = border
	s.set_border_width_all(width)
	s.set_corner_radius_all(1)
	s.set_content_margin_all(12)
	if shadow:
		s.shadow_color = SHADOW
		s.shadow_size = 1
		s.shadow_offset = Vector2(4, 4)
	return s

func _button_box(fill: Color, border: Color, width: int) -> StyleBoxFlat:
	var s := StyleBoxFlat.new()
	s.bg_color = fill
	s.draw_center = fill.a > 0.0
	s.border_color = border
	s.set_border_width_all(width)
	s.set_corner_radius_all(1)
	s.content_margin_left = 12
	s.content_margin_right = 12
	s.content_margin_top = 5
	s.content_margin_bottom = 5
	return s

func rebuild_theme(text_scale: float, readable: bool, high_contrast: bool) -> void:
	var ink := Color.BLACK if high_contrast else INK
	var paper := Color("#FFF8E8") if high_contrast else PAPER
	var t := Theme.new()
	var body: Font = readable_font if readable else text_font
	t.default_font = body
	t.default_font_size = int(round(19 * text_scale))
	for cls in ["Label", "Button", "CheckBox", "CheckButton", "OptionButton", "LineEdit", "RichTextLabel", "ItemList", "SpinBox", "MenuButton", "PopupMenu", "TabBar", "TooltipLabel"]:
		t.set_color("font_color", cls, ink)
	for cls in ["Button", "OptionButton", "CheckBox", "CheckButton", "MenuButton"]:
		t.set_color("font_hover_color", cls, ink)
		t.set_color("font_pressed_color", cls, VERMILION)
		t.set_color("font_focus_color", cls, ink)
		t.set_color("font_disabled_color", cls, Color(ink, 0.45))
		t.set_color("font_hover_pressed_color", cls, VERMILION)
	var w := 2 if high_contrast else 1
	t.set_stylebox("normal", "Button", _button_box(Color(0, 0, 0, 0), ink, w))
	t.set_stylebox("hover", "Button", _button_box(Color(WATER, 0.45), ink, w))
	t.set_stylebox("pressed", "Button", _button_box(Color(OCHRE, 0.35), VERMILION, w + 1))
	t.set_stylebox("hover_pressed", "Button", _button_box(Color(OCHRE, 0.45), VERMILION, w + 1))
	t.set_stylebox("disabled", "Button", _button_box(Color(0, 0, 0, 0), Color(ink, 0.35), w))
	var focus := _button_box(Color(0, 0, 0, 0), VERMILION, 2)
	focus.expand_margin_left = 2
	focus.expand_margin_right = 2
	focus.expand_margin_top = 2
	focus.expand_margin_bottom = 2
	t.set_stylebox("focus", "Button", focus)
	for st in ["normal", "hover", "pressed", "disabled", "focus"]:
		t.set_stylebox(st, "OptionButton", t.get_stylebox(st, "Button"))
		t.set_stylebox(st, "MenuButton", t.get_stylebox(st, "Button"))
	t.set_stylebox("panel", "PanelContainer", cartouche(paper, ink, w + 1))
	t.set_stylebox("panel", "Panel", cartouche(paper, ink, w + 1))
	t.set_stylebox("panel", "PopupMenu", cartouche(paper, ink, w + 1))
	t.set_stylebox("panel", "PopupPanel", cartouche(paper, ink, w + 1))
	t.set_stylebox("panel", "TooltipPanel", cartouche(paper, ink, 1, false))
	t.set_color("font_color", "TooltipLabel", ink)
	var edit := _button_box(Color(PAPER_DARK, 0.6), ink, 1)
	t.set_stylebox("normal", "LineEdit", edit)
	t.set_stylebox("focus", "LineEdit", _button_box(Color(PAPER_DARK, 0.8), VERMILION, 1))
	t.set_color("caret_color", "LineEdit", ink)
	t.set_color("selection_color", "LineEdit", Color(OCHRE, 0.5))
	t.set_color("font_placeholder_color", "LineEdit", Color(ink, 0.5))
	# Réglettes : filet d'encre, curseur en triangle vermillon.
	var slider := StyleBoxLine.new()
	slider.color = ink
	slider.thickness = 2
	t.set_stylebox("slider", "HSlider", slider)
	var area := StyleBoxLine.new()
	area.color = VERMILION
	area.thickness = 3
	t.set_stylebox("grabber_area", "HSlider", area)
	t.set_stylebox("grabber_area_highlight", "HSlider", area)
	t.set_icon("grabber", "HSlider", _triangle_icon(VERMILION, 18))
	t.set_icon("grabber_highlight", "HSlider", _triangle_icon(ink, 18))
	t.set_color("font_color", "RichTextLabel", ink)
	t.set_color("default_color", "RichTextLabel", ink)
	t.set_font("normal_font", "RichTextLabel", body)
	t.set_font("italics_font", "RichTextLabel", readable_font if readable else text_italic)
	t.set_font("bold_font", "RichTextLabel", readable_font if readable else text_bold)
	t.set_font_size("normal_font_size", "RichTextLabel", int(round(19 * text_scale)))
	t.set_font_size("italics_font_size", "RichTextLabel", int(round(19 * text_scale)))
	t.set_font_size("bold_font_size", "RichTextLabel", int(round(19 * text_scale)))
	var sep := StyleBoxLine.new()
	sep.color = Color(ink, 0.6)
	sep.thickness = 1
	t.set_stylebox("separator", "HSeparator", sep)
	var vsep := StyleBoxLine.new()
	vsep.color = Color(ink, 0.6)
	vsep.thickness = 1
	vsep.vertical = true
	t.set_stylebox("separator", "VSeparator", vsep)
	t.set_stylebox("panel", "ItemList", _button_box(Color(PAPER_DARK, 0.35), Color(ink, 0.6), 1))
	t.set_stylebox("selected", "ItemList", _button_box(Color(OCHRE, 0.35), VERMILION, 1))
	t.set_stylebox("selected_focus", "ItemList", _button_box(Color(OCHRE, 0.45), VERMILION, 1))
	t.set_stylebox("hovered", "ItemList", _button_box(Color(WATER, 0.4), Color(0, 0, 0, 0), 0))
	t.set_color("font_selected_color", "ItemList", ink)
	t.set_color("font_hovered_color", "ItemList", ink)
	var scroll := StyleBoxFlat.new()
	scroll.bg_color = Color(ink, 0.5)
	scroll.set_corner_radius_all(2)
	scroll.content_margin_left = 3
	scroll.content_margin_right = 3
	t.set_stylebox("grabber", "VScrollBar", scroll)
	t.set_stylebox("grabber_highlight", "VScrollBar", scroll)
	t.set_stylebox("grabber_pressed", "VScrollBar", scroll)
	var track := StyleBoxFlat.new()
	track.bg_color = Color(ink, 0.08)
	track.content_margin_left = 3
	track.content_margin_right = 3
	t.set_stylebox("scroll", "VScrollBar", track)
	t.set_constant("separation", "VBoxContainer", 6)
	t.set_constant("separation", "HBoxContainer", 8)
	theme = t

func _triangle_icon(c: Color, size: int) -> ImageTexture:
	var img := Image.create(size, size, false, Image.FORMAT_RGBA8)
	img.fill(Color(0, 0, 0, 0))
	for y in size:
		var half := float(size - y) / 2.0
		for x in size:
			if absf(x - size / 2.0 + 0.5) <= half * 0.9:
				img.set_pixel(x, y, c)
	return ImageTexture.create_from_image(img)

## Titre à la plume (IM Fell English).
func title(text: String, size: int = 30) -> Label:
	var l := Label.new()
	l.text = text
	l.add_theme_font_override("font", heading_font())
	l.add_theme_font_size_override("font_size", int(round(size * App.text_scale())))
	return l

## Texte courant.
func text(t: String, size: int = 19, italic: bool = false) -> Label:
	var l := Label.new()
	l.text = t
	if italic:
		l.add_theme_font_override("font", text_italic if not App.settings.get("readable_font", false) else readable_font)
	l.add_theme_font_size_override("font_size", int(round(size * App.text_scale())))
	l.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	return l

func button(t: String, callback: Callable = Callable(), tooltip: String = "") -> Button:
	var b := Button.new()
	b.text = t
	b.tooltip_text = tooltip
	if callback.is_valid():
		b.pressed.connect(callback)
	return b

func panel() -> PanelContainer:
	return PanelContainer.new()

func hsep() -> HSeparator:
	return HSeparator.new()
