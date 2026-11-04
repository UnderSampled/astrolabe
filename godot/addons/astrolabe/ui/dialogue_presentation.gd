extends Resource
## Generated source inputs. Authored scenes own layout and native presentation.
const SOURCE_SCALE: float = 640.0 / (2.0 * 4.65)
const POSITION_SCALE: float = SOURCE_SCALE * 0.0140625
const FONT_SIZE: int = 21
## Exact float32 constants corroborated in the source style5 geometry path.
const SOURCE_CALIBRATION: float = 0.01406249962747097
const SOURCE_LINE_PITCH: float = 0.30000001192092896
const SOURCE_EMPHASIS_OFFSET: float = 0.009999999776482582
@export var font: FontFile
@export var normal_color: Color = Color.WHITE
@export var emphasis_color: Color = Color.RED
@export var corner: Texture2D
@export var border: Texture2D
@export var center: Texture2D
@export var side: Texture2D
@export var continuation: Texture2D
## Third DLG root font: the four offset copies under a selected answer.
@export var selection_color: Color = Color(0.1, 0.1, 0.8)
## Stencil 5, beside each row of the selected answer.
@export var bullet: Texture2D
## SoundEffects:Dialog entries 0 (paid answer) and 2 (answer scroll).
@export var cash_sound: Resource
@export var scroll_sound: Resource
## 0056c8e0: answer rows are drawn 0.5 units right of the body text.
const SOURCE_ANSWER_INDENT: float = 0.5

func answer_indent() -> float:
    return SOURCE_ANSWER_INDENT * SOURCE_SCALE

## 0056c8e0 places the bullet sprite's top-left in screen pixels from the box
## origin, with its own 20.5 px row step.
func bullet_origin(row: int) -> Vector2:
    return Vector2(40.0, row * 20.5 + 35.0)

func continuation_origin(dialog_position: Vector2, box: int, viewport_size: Vector2) -> Vector2:
    # 0056d5c0 anchors cinematic arrows to the viewport, independently of
    # PositionDialog. The shared canvas owns scaling from 640x480 pixels.
    if box == 3:
        var x: int = ((int(viewport_size.x) * 608) & 0xffffffff) / 640
        var y: int = ((int(viewport_size.y) * 448) & 0xffffffff) / 480
        if x >= 0x80000000: x -= 0x100000000
        if y >= 0x80000000: y -= 0x100000000
        return Vector2(float(x), float(y)) * Vector2(640, 480) / viewport_size - dialog_position
    # The admitted framed selectors 1/5 use border width and side height.
    return Vector2(border.get_width() * 8 + 10, side.get_height() * 4 + 10)

func line_spacing() -> float:
    return SOURCE_SCALE * 0.3

func text_origin(dialog_position: Vector2, box: int) -> Vector2:
    if box == 5:
        var x_anchor := float(PackedFloat32Array([(corner.get_width() - 2.0 + dialog_position.x) * SOURCE_CALIBRATION - 4.5])[0])
        var y_anchor := float(PackedFloat32Array([3.375 - (corner.get_height() + dialog_position.y) * SOURCE_CALIBRATION])[0])
        # FON's text height is the space glyph's signed half-height. Read its
        # converted native size; each glyph's bearing remains owned by FontFile.
        var space := font.get_glyph_index(FONT_SIZE, 32, 0)
        var half_space_height := font.get_glyph_size(0, Vector2i(FONT_SIZE, 0), space).y * 0.5
        return Vector2((x_anchor + 4.65) * SOURCE_SCALE, (3.49 - y_anchor) * SOURCE_SCALE + half_space_height - font.get_ascent(FONT_SIZE)) - dialog_position
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

func chain_span_origin(offset: float) -> float:
    # Source font-record drawing adds the first glyph's half-width before
    # FON projects that glyph from its center. This maps to native left bearings;
    # the intervening source float stores do not prove bit-exact cancellation.
    return offset * SOURCE_SCALE

func chain_line_spacing() -> float:
    return SOURCE_LINE_PITCH * SOURCE_SCALE

func chain_emphasis_offset() -> float:
    return SOURCE_EMPHASIS_OFFSET * SOURCE_SCALE

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
