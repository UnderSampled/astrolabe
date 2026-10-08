extends Control
## Proc421: timed catalogue artwork at a source 640x480 center/size, linear alpha.
## Source 0041df20/0041e020; native Tweens own lifetime and pause behavior.

static func display(host: CanvasLayer, artwork: Resource, item: int, milliseconds: int, from: float, to: float, x: int, y: int, width: int, height: int) -> bool:
    if artwork == null or not is_finite(from) or not is_finite(to): return false
    var images: Variant = artwork.get("images")
    if not images is Dictionary or not images.has(item) or not images[item] is Array: return false
    for texture in images[item]:
        if not texture is Texture2D: return false
    if milliseconds <= 0: return true
    var popup := Control.new()
    popup.name = "PickupFeedback"
    popup.process_mode = Node.PROCESS_MODE_PAUSABLE
    popup.mouse_filter = Control.MOUSE_FILTER_IGNORE
    popup.modulate.a = from
    var viewport_size := host.get_viewport().get_visible_rect().size
    var scale := viewport_size / Vector2(640.0, 480.0)
    popup.position = (Vector2(x, y) - Vector2(width, height) * 0.5) * scale
    popup.size = Vector2(width, height) * scale
    for texture in images[item]:
        var layer := TextureRect.new()
        layer.mouse_filter = Control.MOUSE_FILTER_IGNORE
        layer.texture = texture
        layer.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
        layer.size = popup.size
        popup.add_child(layer)
    host.add_child(popup)
    var tween := popup.create_tween()
    tween.tween_property(popup, "modulate:a", to, float(milliseconds) / 1000.0)
    tween.tween_callback(popup.queue_free)
    return true
