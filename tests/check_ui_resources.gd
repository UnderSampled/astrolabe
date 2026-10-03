extends SceneTree
const Emitter = preload("res://addons/astrolabe/audio/emitter.gd")
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
    for record in [["gogood_texte1_manoir", 3, 3, 25, 4], ["gogood_master_whatif01", 1, 21, 20, 17]]:
        var item: Resource = load("res://generated/hype/levels/manoir/dialogue/en/" + str(record[0]) + ".tres")
        if not require(item != null and item.translation is Translation and item.locale == "en", "native Translation/dialogue load"): return
        if not require(item.box == record[1] and item.lines.size() == record[2] and item.completion_normal == record[4], "native box/line/callback source meaning"): return
        if not require(item.voice != null and str(item.voice.identity).ends_with("#" + str(record[3])), "localized voice binding"): return
        var voice: Resource = item.voice.select(emitter)
        if not require(voice != null and voice.stream is AudioStreamWAV and voice.stream.get_length() > 1.0, "dialogue event selects native waveform"): return
        var span: Resource = item.lines[0].spans[0]
        if not require(item.translation.get_message(span.key) != &"" and not str(span.key).contains("0x"), "stable line/span translation key"): return
        facts.append({"dialogue":item.identity,"lines":item.lines.size(),"voice":item.voice.identity,"seconds":voice.stream.get_length(),"normal":item.completion_normal})
    print("UI_MANOR_RESOURCES_OK " + JSON.stringify(facts))
    emitter.queue_free()
    await process_frame
    quit(0)
