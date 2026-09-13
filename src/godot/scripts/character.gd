# Temporary conversion-validation player. Gameplay/AI remain downstream concerns.
extends Node3D
signal cue_triggered(cue: Dictionary, action: Dictionary)
@export var definition_path: String
static var data_cache: Dictionary = {}
var definition: Dictionary
var family: Dictionary
var configuration: Array = []
var reset_configuration: Variant = null
var channels: Array[Node3D] = []
var nodes_by_id: Dictionary = {}
var clip: Dictionary = {}
var current_state: int = -1
var frame_index: int = 0
var playing: bool = true
var elapsed: float = 0.0
var sample_rate: float = 0.0
var playback_revision: int = 0
var sound_player: Node3D
var current_mechanics: Dictionary = {}
var bounds_debug: Node3D

static func data(path: String) -> Dictionary:
    if not data_cache.has(path):
        var parsed = JSON.parse_string(FileAccess.get_file_as_string(path))
        assert(parsed is Dictionary, "Invalid generated character data: " + path)
        data_cache[path] = parsed
    return data_cache[path]

func _ready():
    definition = data(definition_path)
    family = data(definition.family)
    if definition.configuration != null:
        var tables = data(definition.configuration)
        configuration = tables.parts.duplicate(true)
        reset_configuration = tables.reset_parts
    if definition.state != null:
        select_state(int(definition.state))

# Restore only the object table, as FUN_00461ac0 does. Full actor/AI reset is separate.
func restore_configuration() -> bool:
    if reset_configuration == null: return false
    configuration = reset_configuration.duplicate(true)
    for channel in nodes_by_id.values():
        channel.set_meta("object", -2)
    if not clip.is_empty(): apply_frame(frame_index)
    return true

func set_bounds_visible(show_bounds: bool):
    if bounds_debug == null and show_bounds and family.bounds != null:
        bounds_debug = Node3D.new()
        bounds_debug.name = "FamilyBounds"
        add_child(bounds_debug)
        var material = StandardMaterial3D.new()
        material.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
        material.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
        material.albedo_color = Color(0.1, 1.0, 0.4, 0.18)
        material.cull_mode = BaseMaterial3D.CULL_DISABLED
        for shape in data(family.bounds).shapes:
            var mesh: Mesh
            if shape.kind == "sphere":
                var sphere = SphereMesh.new()
                sphere.radius = float(shape.radius)
                sphere.height = float(shape.radius) * 2.0
                mesh = sphere
            elif shape.kind == "box":
                var box = BoxMesh.new()
                box.size = Vector3(shape.size[0], shape.size[1], shape.size[2])
                mesh = box
            else: continue
            var instance = MeshInstance3D.new()
            instance.mesh = mesh
            instance.material_override = material
            instance.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
            instance.position = Vector3(shape.center[0], shape.center[1], shape.center[2])
            instance.set_meta("family_bounds_debug", true)
            bounds_debug.add_child(instance)
    if bounds_debug != null: bounds_debug.visible = show_bounds

func select_state(id: int) -> bool:
    var state: Dictionary = {}
    for candidate in family.states:
        if int(candidate.id) == id:
            state = candidate
            break
    if state.is_empty():
        return false
    for channel in nodes_by_id.values():
        if is_instance_valid(channel):
            channel.reparent(self, false)
    for channel in nodes_by_id.values():
        channel.free()
    channels.clear()
    nodes_by_id.clear()
    playback_revision += 1
    current_state = id
    current_mechanics = {} if state.mechanics == null else data(state.mechanics)
    elapsed = 0.0
    frame_index = 0
    clip = {} if state.animation == null else data(state.animation)
    sample_rate = float(state.speed)
    if clip.is_empty():
        return true
    channels.resize(int(clip.channel_count))
    for sample in clip.samples:
        if sample.channel == null or nodes_by_id.has(int(sample.channel)): continue
        var channel_id = int(sample.channel)
        var channel = Node3D.new()
        channel.name = "channel_%d" % channel_id
        add_child(channel)
        nodes_by_id[channel_id] = channel
    apply_frame(0)
    dispatch_cues(0)
    return true

func _process(delta: float):
    if playing:
        advance(delta)

func advance(seconds: float):
    if clip.is_empty() or sample_rate <= 0.0:
        return
    assert(is_finite(seconds) and seconds >= 0.0, "Invalid animation time step")
    var revision = playback_revision
    var count = clip.frames.size()
    var from_step = int(floor(elapsed * sample_rate + 0.000001))
    var to_step = int(floor((elapsed + seconds) * sample_rate + 0.000001))
    # Every crossed frame dispatches once, including complete loops during a long tick.
    for step in range(from_step + 1, to_step + 1):
        dispatch_cues(step % count)
        if revision != playback_revision: return
    elapsed = fposmod(elapsed + seconds, float(count) / sample_rate)
    var next_frame = to_step % count
    if next_frame != frame_index:
        apply_frame(next_frame)

func dispatch_cues(index: int):
    var revision = playback_revision
    var pose_applied = false
    for cue in clip.cues:
        if int(cue.frame) != index: continue
        if not pose_applied:
            apply_frame(index)
            pose_applied = true
        var action = data(cue.action)
        if action.sound != null:
            if sound_player == null:
                sound_player = Node3D.new()
                sound_player.name = "SoundEmitter"
                sound_player.set_script(load("res://generated/hype/runtime/sound.gd"))
                add_child(sound_player)
            sound_player.execute(action.sound)
        cue_triggered.emit(cue, action)
        if revision != playback_revision: return

static func source_transform(values: Array) -> Transform3D:
    return Transform3D(Basis(Vector3(values[0],values[1],values[2]),Vector3(values[3],values[4],values[5]),Vector3(values[6],values[7],values[8])),Vector3(values[9],values[10],values[11]))

func apply_frame(index: int):
    assert(index >= 0 and index < clip.frames.size())
    frame_index = index
    var frame: Dictionary = clip.frames[index]
    # Remove the previous frame's hierarchy first, so reparenting cannot form a transient cycle.
    for channel in nodes_by_id.values():
        if channel.get_parent() != self:
            channel.reparent(self, false)
        var previous = channel.get_node_or_null("Part")
        if previous != null: previous.visible = false
    for i in channels.size():
        var sample = clip.samples[int(frame.samples[i])]
        channels[i] = null if sample.channel == null else nodes_by_id[int(sample.channel)]
    for pair in frame.hierarchy:
        var child = channels[int(pair[0])]
        var parent = channels[int(pair[1])]
        if child != null and parent != null and child != parent: child.reparent(parent, false)
    for i in channels.size():
        var sample: Dictionary = clip.samples[int(frame.samples[i])]
        var channel = channels[i]
        if channel == null: continue
        if sample.transform == null:
            channel.transform = Transform3D.IDENTITY
        else:
            channel.transform = source_transform(data(sample.transform.pool).entries[str(int(sample.transform.entry))])
        var object: int = -1 if sample.object == null else int(sample.object)
        if int(channel.get_meta("object", -1)) != object:
            var old = channel.get_node_or_null("Part")
            if old != null:
                old.free()
            if object >= 0 and object < configuration.size():
                var part: Dictionary = configuration[object]
                if part.scene != null:
                    var model = load(part.scene).instantiate()
                    model.name = "Part"
                    channel.add_child(model)
                    if part.scale != null:
                        model.scale = Vector3(part.scale[0],part.scale[1],part.scale[2])
            channel.set_meta("object", object)
        var active = channel.get_node_or_null("Part")
        if active != null:
            active.visible = int(sample.visibility) != 0 and int(sample.opacity) > 0
            var geometries = active.find_children("*", "GeometryInstance3D", true, false)
            if active is GeometryInstance3D: geometries.append(active)
            for geometry in geometries: geometry.transparency = 1.0 - float(sample.opacity) / 255.0
        # Invisible parts do not hide descendant channels.
