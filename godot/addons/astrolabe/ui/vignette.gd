extends CanvasLayer
## Converted source vignette artwork with a positive real millisecond hold.
## Proc307 cannot reach the original presenter's zero-duration input loop.
signal completed
@onready var picture: TextureRect = $Screen/Picture
var _generation := 0
var _duration_ms := 0
var _holding := false
var _held_ms := 0.0

func _ready() -> void:
    hide()
    set_process(false)

func present(image: Texture2D, milliseconds: int) -> bool:
    if image == null or milliseconds <= 0: return false
    _generation += 1
    _duration_ms = milliseconds
    _holding = false
    picture.texture = image
    show()
    set_process(false)
    call_deferred("_begin_hold", _generation)
    return true

func _begin_hold(generation: int) -> void:
    # Original display precedes the wait. Start the native hold only after
    # this frame has presented the full-screen image.
    await RenderingServer.frame_post_draw
    if generation != _generation or not is_inside_tree(): return
    _held_ms = 0.0
    _holding = true
    set_process(true)

func _process(delta: float) -> void:
    # Game time, so the hold follows pauses and fixed-step capture.
    if not _holding: return
    _held_ms += delta * 1000.0
    if _held_ms < _duration_ms: return
    _holding = false
    set_process(false)
    hide()
    picture.texture = null
    completed.emit()
