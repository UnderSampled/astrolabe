extends SceneTree
const Emitter = preload("res://addons/astrolabe/audio/emitter.gd")
class SpeechHolder extends Node:
    var source_dialogue_record: Resource
    func stop_dialogue_speech() -> void:
        source_dialogue_record = null
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
    if not audit_font_api(): return
    if "--font-api-only" in OS.get_cmdline_user_args():
        print("UI_FONT_API_OK " + Engine.get_version_info().string)
        quit(0)
        return
    check.call_deferred()
func audit_font_api() -> bool:
    for constant in ["FIXED_SIZE_SCALE_DISABLE","SUBPIXEL_POSITIONING_ONE_QUARTER"]:
        if not require(ClassDB.class_has_integer_constant("TextServer",constant), "pinned bitmap enum namespace/name: " + constant): return false
    var font := FontFile.new()
    var expected := {"set_font_name":1,"set_cache_ascent":3,"set_cache_descent":3,"set_cache_scale":3,"set_texture_image":4,"set_glyph_advance":4,"set_glyph_offset":4,"set_glyph_size":4,"set_glyph_texture_idx":4,"set_glyph_uv_rect":4,"get_font_name":0,"get_glyph_index":3,"get_glyph_list":2,"get_size_cache_list":1,"get_texture_count":2,"get_glyph_advance":3,"get_glyph_offset":3,"get_glyph_size":3,"has_char":1,"get_ascent":1}
    for method in font.get_method_list():
        var name: String = str(method.name)
        if expected.has(name):
            if not require(method.args.size() == expected[name], "pinned FontFile argument count: " + name): return false
            expected.erase(name)
    if not require(expected.is_empty(), "pinned FontFile missing APIs: " + str(expected.keys())): return false
    var properties := {"fixed_size":true,"fixed_size_scale_mode":true,"subpixel_positioning":true,"allow_system_fallback":true}
    for property in font.get_property_list(): properties.erase(str(property.name))
    if not require(properties.is_empty(), "pinned FontFile properties: " + str(properties.keys())): return false
    # Cheap native serialization/metric check before the original-data import.
    font.fixed_size = 21
    font.fixed_size_scale_mode = TextServer.FIXED_SIZE_SCALE_DISABLE
    font.subpixel_positioning = TextServer.SUBPIXEL_POSITIONING_ONE_QUARTER
    font.allow_system_fallback = false
    font.set_cache_ascent(0,21,17.2)
    font.set_cache_descent(0,21,3.4)
    for code in [65,233]:
        font.set_glyph_advance(0,21,code,Vector2(16.2338,0.0))
    if not require(ResourceSaver.save(font,"res://ui-font-api.tres") == OK, "native bitmap metric preflight save"): return false
    var saved: FontFile = load("res://ui-font-api.tres")
    var advance := saved.get_glyph_advance(0,21,65).x
    # Native string bounds are ceiled; inspect the shaped glyph advances instead.
    var bounds := saved.get_string_size("AA",HORIZONTAL_ALIGNMENT_LEFT,-1,21).x
    var line := TextLine.new()
    if not require(line.add_string("AA",saved,21), "native bitmap metric preflight shaping"): return false
    line.get_size()
    var glyphs: Array = TextServerManager.get_primary_interface().shaped_text_get_glyphs(line.get_rid())
    var shaped: float = 0.0
    var advances: Array[float] = []
    for glyph in glyphs:
        advances.append(float(glyph.advance))
        shaped += float(glyph.advance) * int(glyph.repeat)
    print("UI_FONT_METRIC_PREFLIGHT " + JSON.stringify({"glyph_a":saved.get_glyph_index(21,65,0),"has_e_acute":saved.has_char(233),"advance_a":advance,"shaped_aa":shaped,"glyph_advances":advances,"bounds_aa":bounds}))
    if not require(saved.get_glyph_index(21,65,0) == 65 and saved.has_char(233), "native bitmap metric preflight Unicode"): return false
    if not require(is_equal_approx(advance,16.2338), "native bitmap metric preflight fractional advance"): return false
    if not require(glyphs.size() == 2 and absf(shaped-2.0*16.2338) <= 1.0/32.0, "native bitmap metric preflight shaped advance"): return false
    if not require(is_equal_approx(bounds,ceilf(shaped)), "native bitmap metric preflight integer bounds"): return false
    # A named native resource also exercises metadata loading before glyph caches.
    # fixed_size is a prerequisite of font_name, as in normalized exporter output.
    var named_file := FileAccess.open("res://ui-font-api-named.tres",FileAccess.WRITE)
    named_file.store_string("""[gd_resource type="FontFile" format=3]
[resource]
fixed_size = 21
font_name = "UI_Named_Bitmap"
allow_system_fallback = false
cache/0/21/0/glyphs/65/advance = Vector2(16.2338, 0)
cache/0/21/0/glyphs/233/advance = Vector2(9.432005, 0)
""")
    named_file.close()
    var named: FontFile = load("res://ui-font-api-named.tres")
    var sizes := named.get_size_cache_list(0)
    print("UI_NAMED_FONT_PREFLIGHT " + JSON.stringify({"has_a":named.has_char(65),"has_e_acute":named.has_char(233),"size_caches":str(sizes)}))
    return require(named.get_font_name() == "UI_Named_Bitmap" and named.has_char(65) and named.has_char(233) and not sizes.has(Vector2i(16,0)), "named native bitmap font Unicode/cache binding")
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
        if not require(font.get_font_name() == "Playmo_Font2_0" and not font.allow_system_fallback and font.fixed_size == 21 and font.subpixel_positioning == TextServer.SUBPIXEL_POSITIONING_ONE_QUARTER, "native bitmap font identity and fallback/positioning policy"): return
        if not require(font.get_glyph_list(0,Vector2i(21,0)).size() == 133 and font.get_texture_count(0,Vector2i(21,1)) == 1, "native glyph and outline caches"): return
        var glyph_a: int = font.get_glyph_index(21,65,0)
        var missing_codes: Array[int] = []
        for code in font.get_glyph_list(0,Vector2i(21,0)):
            if not font.has_char(code): missing_codes.append(code)
        if facts.is_empty():
            print("UI_NATIVE_FONT_METRICS " + JSON.stringify({"glyph_a":glyph_a,"has_e_acute":font.has_char(233),"advance_a":font.get_glyph_advance(0,21,glyph_a),"bearing_a":font.get_glyph_offset(0,Vector2i(21,0),glyph_a),"size_caches":str(font.get_size_cache_list(0)),"missing_codes":missing_codes}))
        if not require(glyph_a == 65 and font.has_char(233) and missing_codes.is_empty(), "Unicode mapping/cache binding: " + str(missing_codes)): return
        if not require(is_equal_approx(font.get_glyph_advance(0,21,glyph_a).x,(0.22588237+0.01)*scale), "fractional source advances"): return
        if not require(font.get_glyph_offset(0,Vector2i(21,0),glyph_a).is_equal_approx(Vector2(0.0,(-2.0*0.12705882+0.01)*scale)), "source glyph baseline bearing"): return
        if not require(style.normal_color == Color.WHITE and style.emphasis_color == Color.RED and is_equal_approx(style.line_spacing(),0.3*scale), "source white text/red emphasis/line spacing"): return
        if not require(style.corner.get_size() == Vector2(32,32) and style.border.get_size() == Vector2(32,32) and style.center.get_size() == Vector2(32,32) and style.side.get_size() == Vector2(32,32), "source frame texture sizes"): return
        if not require(style.continuation is Texture2D and style.continuation.get_width() > 0 and style.continuation.get_height() > 0, "native continuation stencil"): return
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
            var expected_origin: Vector2 = style.text_origin(dialogue.position,item.box) + Vector2(style.chain_span_origin(span.source_offset),0.0)
            if not require(label.position.is_equal_approx(expected_origin) and label.text == item.message(span.key) and label.get_theme_font("font") == font, "source span offsets and literal native text"): return
            var panel: Control = dialogue.panel
            if not require(panel.size == Vector2(320,192) and panel.get_node("Top").stretch_mode == TextureRect.STRETCH_TILE and panel.get_node("Left").stretch_mode == TextureRect.STRETCH_TILE and panel.get_node("Center").stretch_mode == TextureRect.STRETCH_SCALE and not panel.get_node("Fallback").visible, "authored frame repeats edges/stretch center"): return
            if not require(not dialogue.continuation.visible, "continuation held while opening"): return
            for index in range(14): dialogue._process(0.011)
            if not require(dialogue.continuation.visible and dialogue.continuation.texture_normal == style.continuation, "completed nonfinal page shows source arrow"): return
            var expected_arrow: Vector2 = Vector2(266,138) if item.box == 1 else style.continuation_origin(dialogue.position,3,root.get_visible_rect().size)
            if not require(dialogue.continuation.position.is_equal_approx(expected_arrow), "source arrow uses border width/side height or viewport anchor"): return
            dialogue.continuation.pressed.emit()
            if not require(dialogue.page_start == item.lines_per_page() and not dialogue.continuation.visible, "native continue button advances one page then resets indicator"): return
            dialogue.cancel()
            if not require(not dialogue.continuation.visible, "cancellation hides continuation"): return
        if item.identity == "hype_texte1_manoir":
            # This final finite page must finish when speech clears, without an
            # arrow press. The actual mixer/holder path has its own audio check.
            if not require(dialogue.run(item,1), "Hype finite final page starts"): return
            var holder := SpeechHolder.new()
            root.add_child(holder)
            dialogue.voice_emitter = holder
            for index in range(14): dialogue._process(0.011)
            if not require(dialogue.closing and not dialogue.continuation.visible, "finite final speech page completes without continue input"): return
            dialogue.cancel()
            holder.queue_free()
    print("UI_MANOR_RESOURCES_OK " + JSON.stringify(facts))
    print("UI_NATIVE_PRESENTATION_OK glyphs=133 source_span_labels=1 frame=320x192")
    dialogue.queue_free()
    emitter.queue_free()
    await process_frame
    quit(0)
