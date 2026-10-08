extends Resource
## Generated source inputs. Authored scenes own layout and native presentation.
const SOURCE_SCALE: float = 640.0 / (2.0 * 4.65)
const POSITION_SCALE: float = SOURCE_SCALE * 0.0140625
const FONT_SIZE: int = 21
@export var font: FontFile
@export var normal_color: Color = Color.WHITE
@export var emphasis_color: Color = Color.RED
@export var corner: Texture2D
@export var border: Texture2D
@export var center: Texture2D
@export var side: Texture2D

func line_spacing() -> float:
    return SOURCE_SCALE * 0.3

func text_origin(dialog_position: Vector2, box: int) -> Vector2:
    # Source calibrates the baseline using corner height even for unframed text.
    var left := SOURCE_SCALE * 0.2 if box == 3 else SOURCE_SCALE * 0.15 + POSITION_SCALE * (corner.get_width() - 2.0 + dialog_position.x)
    var baseline := SOURCE_SCALE * 0.265 + POSITION_SCALE * (corner.get_height() + dialog_position.y)
    return Vector2(left, baseline - font.get_ascent(FONT_SIZE)) - dialog_position

func span_origin(message: String, offset: float) -> float:
    if message.is_empty(): return offset * SOURCE_SCALE
    # Source positions a span at the center of its first glyph. Native FontFile
    # uses left bearings, so only this span origin needs the half-width adjustment.
    var glyph := font.get_glyph_index(FONT_SIZE, message.unicode_at(0), 0)
    return offset * SOURCE_SCALE - font.get_glyph_size(0, Vector2i(FONT_SIZE, 0), glyph).x * 0.5

func line_origin(message: String, alignment: Vector2i, offset: Vector2) -> Vector2:
    var origin := offset * SOURCE_SCALE * Vector2(1.0, -1.0)
    # The source's FON width measures first-to-last glyph centers. Reuse the
    # native bitmap metrics, including source tracking, rather than an asset table.
    if alignment.x != 0 and message.length() > 1:
        var width := 0.0
        for index in range(message.length()):
            var glyph := font.get_glyph_index(FONT_SIZE, message.unicode_at(index), 0)
            var glyph_width := font.get_glyph_size(0, Vector2i(FONT_SIZE, 0), glyph).x
            if index == message.length() - 1: width += glyph_width * 0.5
            else:
                var advance := font.get_glyph_advance(0, FONT_SIZE, glyph).x
                width += advance - (glyph_width * 0.5 if index == 0 else 0.0)
        origin.x -= width * (0.5 if alignment.x == 1 else 1.0)
    if alignment.y != 0:
        var space := font.get_glyph_index(FONT_SIZE, 32, 0)
        var half_height := -font.get_glyph_size(0, Vector2i(FONT_SIZE, 0), space).y * 0.5
        origin.y += half_height * (0.5 if alignment.y == 1 else 1.0)
    return origin
