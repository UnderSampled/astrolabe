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
    for record in RECORDS:
        var item: Resource = load("res://generated/hype/levels/manoir/dialogue/en/" + str(record[0]) + ".tres")
        if not require(item != null and item.translation is Translation and item.locale == "en", "native Translation/dialogue load"): return
        if not require(item.box == record[1] and item.lines.size() == record[2] and item.completion_normal == record[4], "native box/line/callback source meaning"): return
        if not require(item.completion_model == "level:/ai/models/" + str(record[5]), "native callback model identity"): return
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
    print("UI_MANOR_RESOURCES_OK " + JSON.stringify(facts))
    emitter.queue_free()
    await process_frame
    quit(0)
