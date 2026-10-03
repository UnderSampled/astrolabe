extends SceneTree
const Emitter = preload("res://addons/astrolabe/audio/emitter.gd")
const RECORDS = [
    ["gogood_texte1_manoir", 3, 3, 25, 4, 210, {1:1.4, 2:4.14}],
    ["gogood_texte1a_manoir", 3, 3, 2, 5, 210, {1:3.06, 2:5.31}],
    ["gogood_texte1b_manoir", 3, 4, 3, 6, 210, {1:5.34, 3:14.04}],
    ["gogood_texte1c_manoir", 3, 4, 4, 1, 210, {1:3.43, 3:9.28}],
    ["gogood_texte2_manoir", 3, 4, 5, 11, 210, {1:4.18, 3:9.87}],
    ["gogood_texte2a_manoir", 3, 9, 6, 12, 210, {1:4.31, 3:10.62, 5:17.29, 7:21.13, 8:23.6}],
    ["hype_texte1_manoir", 3, 2, 26, 5, 294, {1:3.88}],
    ["hype_texte2_manoir", 3, 8, 27, 3, 294, {1:1.9, 3:11.12, 5:19.99, 7:26.83}],
    ["gogood_master_whatif01", 1, 21, 20, 17, 991, {3:3.75, 8:8.45, 15:16.25}],
]
func require(condition: bool, message: String) -> bool:
    if condition: return true
    push_error(message)
    quit(1)
    return false
func _initialize() -> void:
    check.call_deferred()
func check() -> void:
    var emitter := Node3D.new()
    emitter.set_script(Emitter)
    root.add_child(emitter)
    var facts: Array[Dictionary] = []
    var shared_style: Resource
    var dialogue: Control = load("res://addons/astrolabe/ui/dialogue.tscn").instantiate()
    root.add_child(dialogue)
    dialogue.set_process(false)
    for record in RECORDS:
        var item: Resource = load("res://generated/hype/levels/manoir/dialogue/en/" + str(record[0]) + ".tres")
        if not require(item != null and item.translation is Translation and item.locale == "en", "native Translation/dialogue load"): return
        if not require(item.box == record[1] and item.lines.size() == record[2] and item.completion_normal == record[4], "native box/line/callback source meaning"): return
        if not require(item.completion_model == "level:/ai/models/" + str(record[5]), "native callback model identity"): return
        if not require(item.presentation != null and item.presentation.font is FontFile, "native source presentation/FontFile"): return
        var style: Resource = item.presentation
        if shared_style == null: shared_style = style
        if not require(style == shared_style, "all nine items share one presentation resource"): return
        var font: FontFile = style.font
        var scale: float = 640.0 / 9.3
        if not require(font.get_font_name() == "Playmo_Font2_0" and not font.allow_system_fallback and font.fixed_size == 21, "native bitmap font identity and fallback policy"): return
        if not require(font.get_glyph_list(0,Vector2i(21,0)).size() == 133 and font.get_texture_count(0,Vector2i(21,1)) == 1, "native glyph and outline caches"): return
        var glyph_a: int = font.get_glyph_index(21,65,0)
        if not require(glyph_a == 65 and font.has_char(233) and is_equal_approx(font.get_glyph_advance(0,21,glyph_a).x,(0.22588237+0.01)*scale), "Unicode mapping/fractional source advances"): return
        if not require(font.get_glyph_offset(0,Vector2i(21,0),glyph_a).is_equal_approx(Vector2(0.0,(-2.0*0.12705882+0.01)*scale)), "source glyph baseline bearing"): return
        if not require(style.normal_color == Color.WHITE and style.emphasis_color == Color.RED and is_equal_approx(style.line_spacing(),0.3*scale), "source white text/red emphasis/line spacing"): return
        if not require(style.corner.get_size() == Vector2(32,32) and style.border.get_size() == Vector2(32,32) and style.center.get_size() == Vector2(32,32) and style.side.get_size() == Vector2(32,32), "source frame texture sizes"): return
        for index in range(item.lines.size()):
            var expected: float = record[6].get(index, 3.4e38)
            var actual: float = item.lines[index].end_seconds
            if not require(item.lines[index].flags == 0 and (actual > 3.0e38 if expected > 3.0e38 else is_equal_approx(actual, expected)), "native finite/untimed deadline at " + item.identity + " line " + str(index)): return
        if not require(item.voice != null and str(item.voice.identity).ends_with("#" + str(record[3])), "localized voice binding"): return
        var voice: Resource = item.voice.select(emitter)
        if not require(voice != null and voice.stream is AudioStreamWAV and voice.stream.get_length() > 1.0, "dialogue event selects native waveform"): return
        var span: Resource = item.lines[0].spans[0]
        if not require(item.translation.get_message(span.key) != &"" and not str(span.key).contains("0x"), "stable line/span translation key"): return
        facts.append({"dialogue":item.identity,"lines":item.lines.size(),"voice":item.voice.identity,"seconds":voice.stream.get_length(),"model":item.completion_model,"normal":item.completion_normal})
        if item.box == 1 or item.identity == "gogood_texte1_manoir":
            dialogue.position = Vector2(305,10) if item.box == 1 else Vector2(0,405)
            if not require(dialogue.run(item,1), "actual source item adopts native presentation"): return
            if not require(not dialogue.text.visible and dialogue.panel.visible == (item.box == 1), "source labels and frame visibility"): return
            var labels: Array[Node] = dialogue.source_text.get_children()
            if not require(not labels.is_empty() and labels[0] is Label, "native span controls"): return
            var label: Label = labels[0]
            var expected_origin: Vector2 = style.text_origin(dialogue.position,item.box) + Vector2(style.span_origin(item.message(span.key),span.source_offset),0.0)
            if not require(label.position.is_equal_approx(expected_origin) and label.text == item.message(span.key) and label.get_theme_font("font") == font, "source span offsets and literal native text"): return
            var panel: Control = dialogue.panel
            if not require(panel.size == Vector2(320,192) and panel.get_node("Top").stretch_mode == TextureRect.STRETCH_TILE and panel.get_node("Left").stretch_mode == TextureRect.STRETCH_TILE and panel.get_node("Center").stretch_mode == TextureRect.STRETCH_SCALE and not panel.get_node("Fallback").visible, "authored frame repeats edges/stretch center"): return
            dialogue.cancel()
    print("UI_MANOR_RESOURCES_OK " + JSON.stringify(facts))
    print("UI_NATIVE_PRESENTATION_OK glyphs=133 source_span_labels=1 frame=320x192")
    dialogue.queue_free()
    emitter.queue_free()
    await process_frame
    quit(0)
