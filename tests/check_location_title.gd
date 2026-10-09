extends SceneTree
const ScreenScene = preload("res://addons/astrolabe/ui/screen.tscn")
const Item = preload("res://addons/astrolabe/ui/dialogue_item.gd")
const Line = preload("res://addons/astrolabe/ui/dialogue_line.gd")
const Span = preload("res://addons/astrolabe/ui/dialogue_span.gd")
const Presentation = preload("res://addons/astrolabe/ui/dialogue_presentation.gd")

func require(condition: bool, message: String) -> bool:
    if condition: return true
    push_error(message)
    quit(1)
    return false

func _initialize() -> void:
    check.call_deferred()

static func fixture_texture(width: int = 32, height: int = 32) -> ImageTexture:
    var image := Image.create(width, height, false, Image.FORMAT_RGBA8)
    image.fill(Color(0.2, 0.3, 0.4, 1.0))
    return ImageTexture.create_from_image(image)

static func fixture_item() -> Resource:
    var style: Resource = Presentation.new()
    var font := FontFile.new()
    font.fixed_size = 21
    font.allow_system_fallback = false
    font.set_cache_ascent(0, 21, 17.2)
    font.set_cache_descent(0, 21, 3.4)
    for code in range(32, 127):
        font.set_glyph_advance(0, 21, code, Vector2(8.0, 0.0))
        font.set_glyph_size(0, Vector2i(21, 0), code, Vector2(8.0, 16.0))
    style.font = font
    style.corner = fixture_texture()
    style.border = fixture_texture()
    style.side = fixture_texture()
    style.center = fixture_texture()
    style.continuation = fixture_texture()
    var value: Resource = Item.new()
    value.identity = "synthetic_title"
    value.box = 4
    value.presentation = style
    value.translation = Translation.new()
    value.translation.locale = "en"
    for index in range(2):
        var span: Resource = Span.new()
        span.key = StringName("synthetic.title.%d" % index)
        span.source_offset = 0.25 * index
        value.translation.add_message(span.key, "Place [literal]" if index == 0 else "Synthetic era")
        var line: Resource = Line.new()
        line.spans.append(span)
        value.lines.append(line)
    return value

func check() -> void:
    # Native save/reopen includes the Translation, FontFile and texture inputs.
    var value: Resource = fixture_item()
    for argument in OS.get_cmdline_user_args():
        if argument.begins_with("--presentation="):
            value.presentation = load(argument.trim_prefix("--presentation="))
            if not require(value.presentation != null and value.presentation.font.get_font_name() == "Playmo_Font2_0", "converted source font resource"): return
    var aligned: Vector2 = value.presentation.line_origin("AB", Vector2i(1, 0), Vector2(1.0, 2.0))
    var expected_width := 0.0
    for character in [65, 66]:
        var glyph: int = value.presentation.font.get_glyph_index(21, character, 0)
        expected_width += value.presentation.font.get_glyph_size(0, Vector2i(21, 0), glyph).x * 0.5
    var a_glyph: int = value.presentation.font.get_glyph_index(21, 65, 0)
    expected_width += value.presentation.font.get_glyph_advance(0, 21, a_glyph).x - value.presentation.font.get_glyph_size(0, Vector2i(21, 0), a_glyph).x
    if not require(aligned.is_equal_approx(Vector2(value.presentation.SOURCE_SCALE - expected_width * 0.5, -2.0 * value.presentation.SOURCE_SCALE)), "promoted source line center/offset using native metrics"): return
    if not require(ResourceSaver.save(value, "res://synthetic-title.tres") == OK, "save native item"): return
    value = load("res://synthetic-title.tres")
    root.size = Vector2i(640, 480)
    var host := Node3D.new()
    root.add_child(host)
    var screen: Node = ScreenScene.instantiate()
    screen.adopt_parent_screen = false
    host.add_child(screen)
    var title: Control = screen.location_title
    title.set_process(false)
    if not require(not title.visible and title.item == null, "no popup on scene entry"): return
    title.position = Vector2(160, 70)
    if not require(title.run(value, 1), "style4 run with native source inputs"): return
    if not require(title.panel.texture == value.presentation.center, "shared center texture"): return
    if not require(title.panel.position == Vector2(16, 16) and title.panel.size == Vector2(288, 96), "source style4 center rectangle"): return
    if not require(title.source_text.get_child_count() == 2 and not title.source_text.visible, "two lines held during frame opening"): return
    if not require(title.source_text.get_child(0).text == "Place [literal]", "literal localized text"): return
    if not require(title.source_text.get_child(0).get_theme_font("font") == value.presentation.font, "shared native font"): return
    title.request_select()
    if not require(title.item == value, "select ignored while opening"): return
    var origin: Vector2 = value.presentation.text_origin(title.position, 4)
    if not require(title.source_text.get_child(0).position.is_equal_approx(origin + Vector2(value.presentation.span_origin("Place [literal]", 0.0), 0.0)), "native source span position"): return
    title._process(0.010)
    if not require(title.panel.modulate.a == 0.0, "strict >10ms open threshold"): return
    title._process(0.001)
    if not require(is_equal_approx(title.panel.modulate.a, 0.1), "first alpha step"): return
    title._process(1.0)
    if not require(is_equal_approx(title.panel.modulate.a, 0.2), "one alpha step per slow draw"): return
    for index in range(12): title._process(0.011)
    if not require(title.source_text.visible and not title.is_over(), "text after frame opens; untimed title held"): return
    if not require(screen.dialogue.item == null and not screen.completion_pending, "title component leaves gameplay callback owner idle"): return
    var labels: Array[Node] = title.source_text.get_children()
    if not require(labels[0].visible_characters > 0 and labels[0].visible_characters < labels[0].text.length() and labels[1].visible_characters == 0, "first line prefix precedes second line"): return
    if not require(is_equal_approx(labels[1].position.y - labels[0].position.y, value.presentation.line_spacing()), "source line spacing"): return
    # The cue's 200/3000ms waits belong to compiled AI, never this component.
    title._process(4.0)
    if not require(title.item == value, "no invented 3-second UI timer"): return
    var prefix: int = labels[0].visible_characters
    title._process(0.0)
    if not require(labels[0].visible_characters <= prefix + 1, "no reveal catchup after slow draw"): return
    for index in range(40): title._process(0.003)
    if not require(labels[0].visible_characters == -1 and labels[1].visible_characters == -1, "source sequential reveal completes untimed page"): return
    for argument in OS.get_cmdline_user_args():
        if argument.begins_with("--capture="):
            await RenderingServer.frame_post_draw
            var capture := root.get_texture().get_image()
            if not require(capture != null and not capture.is_empty(), "rendered UI capture"): return
            if not require(capture.save_png(argument.trim_prefix("--capture=")) == OK, "save owned UI capture"): return
    title.end()
    if not require(title.item == null and not title.source_text.visible and title.panel.modulate.a == 1.0, "text clears before close; panel starts opaque"): return
    title._process(0.0)
    if not require(is_equal_approx(title.panel.modulate.a, 0.9) and not title.is_over(), "close decrements once per draw without timer gate"): return
    for index in range(12): title._process(0.0)
    if not require(title.is_over() and not title.visible, "close completes"): return
    # Restart during opening/closing and cancellation must not retain text.
    if not require(title.run(value, 1), "repeat entry"): return
    title._process(0.011)
    title.cancel()
    if not require(title.item == null and not title.visible and title.source_text.get_child_count() == 0, "map-change cancellation"): return
    if not require(title.run(value, 1), "entry after cancellation"): return
    title.end(false)
    if not require(title.is_over(), "immediate end"): return
    if not require(title.run(value, 1), "entry before viewport resize"): return
    root.size = Vector2i(1280, 960)
    await process_frame
    await process_frame
    screen._layout()
    if not require(screen.canvas.scale.is_equal_approx(root.get_visible_rect().size / Vector2(640, 480)), "existing viewport scaling owner"): return
    var expected: Vector2 = (title.position + title.panel.position) * screen.canvas.scale
    if not require(title.panel.global_position.is_equal_approx(expected), "scaled screen-space rectangle"): return
    var label_count: int = title.source_text.get_child_count()
    title.position = Vector2(80, 35)
    title.refresh_layout()
    if not require(title.source_text.get_child_count() == label_count, "position refresh replaces labels"): return
    value.box = 3
    if not require(not title.run(value, 1) and title.item != null, "cinematic item rejected without cancelling live title"): return
    value.box = 4
    if not require(not title.run(value, 0), "unsupported run mode rejected"): return
    var odd: Resource = fixture_item()
    odd.presentation.corner = fixture_texture(31, 33)
    odd.presentation.border = fixture_texture(63, 32)
    odd.presentation.side = fixture_texture(32, 63)
    if not require(title.run(odd, 1), "alternate native texture dimensions"): return
    if not require(title.panel.position == Vector2(15, 16) and title.panel.size == Vector2(287.5, 97.5), "source integer halves/ratios before float layout"): return
    # A fresh opening uses actual Godot draw callbacks, not just manual steps.
    title.cancel()
    if not require(title.run(value, 1), "actual draw playback entry"): return
    title.set_process(true)
    var open_deadline := Time.get_ticks_msec() + 1000
    while not title.source_text.visible and Time.get_ticks_msec() < open_deadline:
        await process_frame
    if not require(title.source_text.visible, "actual native draw playback opens text"): return
    title.request_select()
    if not require(title.item == value and title.source_text.get_child(0).visible_characters == -1, "first select completes reveal"): return
    title.request_select()
    if not require(title.item == null and not title.source_text.visible, "single untimed page select ends text"): return
    # Restart during the native close; no deferred callback may hide the new item.
    if not require(title.run(value, 1), "replace closing title"): return
    open_deadline = Time.get_ticks_msec() + 1000
    while not title.source_text.visible and Time.get_ticks_msec() < open_deadline:
        await process_frame
    if not require(title.item == value and title.source_text.visible, "new title survives old close"): return
    title.cancel()
    host.remove_child(screen)
    screen.free()
    host.queue_free()
    await process_frame
    print("LOCATION_TITLE_COMPONENT_OK native resources; style4 layout; alpha order; cancellation; repeat; viewport scaling")
    quit(0)
