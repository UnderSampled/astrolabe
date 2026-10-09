extends Node
## Source fade levels are gameplay state; native buses hold user preferences.
signal source_volume_changed(module: StringName)
signal preference_volume_changed(module: StringName)

const BUSES := {
    &"music": &"Astrolabe Music",
    &"effects": &"Astrolabe Effects",
    &"voice": &"Astrolabe Voice",
}
var source_levels: Dictionary = {&"music": 127, &"effects": 127, &"voice": 127}
var preference_levels: Dictionary = {&"music": 127, &"effects": 127, &"voice": 127}

static func shared(requester: Node) -> Node:
    var root := requester.get_tree().root
    var mixer: Node = root.get_meta(&"astrolabe_audio_mixer", null) as Node
    if not is_instance_valid(mixer):
        mixer = load("res://addons/astrolabe/audio/mixer.gd").new()
        mixer.name = "AstrolabeMixer"
        # Preferences can create the component in their autoload _ready, while
        # Root is setting up children. Publish the binding immediately so the
        # three preference groups share one component before deferred insertion.
        root.set_meta(&"astrolabe_audio_mixer", mixer)
        root.call_deferred("add_child", mixer)
    return mixer

static func module_for_bus(bus_name: StringName) -> StringName:
    for module in BUSES:
        if BUSES[module] == bus_name: return module
    return &""

func _init() -> void:
    for module in BUSES:
        var bus := AudioServer.get_bus_index(BUSES[module])
        if bus < 0:
            AudioServer.add_bus()
            bus = AudioServer.bus_count - 1
            AudioServer.set_bus_name(bus, BUSES[module])
            AudioServer.set_bus_send(bus, &"Master")

## 004ed2f0/004ec660 republish every request using this preference, including
## static requests. The preference remains independent of source+38 fades.
func set_preference(module: StringName, value: int) -> void:
    assert(BUSES.has(module))
    var level := clampi(value, 0, 127)
    preference_levels[module] = level
    var bus := AudioServer.get_bus_index(BUSES[module])
    AudioServer.set_bus_volume_db(bus, linear_to_db(maxf(float(level) / 127.0, 0.000001)))
    preference_volume_changed.emit(module)

func source_volume_word(module: StringName) -> int:
    assert(source_levels.has(module))
    return int(source_levels[module])

func set_source_volume_word(module: StringName, value: int) -> void:
    assert(source_levels.has(module))
    source_levels[module] = value & 0xffffffff
    # 004ec710 updates dynamic requests; emitters retain static captured gain.
    source_volume_changed.emit(module)

## DWORD IMUL can overflow before the signed shift. Split the multiplication
## so Godot's signed64-bit arithmetic also handles two full unsigned DWORDs.
static func low32_product(left: int, right: int) -> int:
    var low := (left & 0xffff) * (right & 0xffff)
    var high := ((left >> 16) & 0xffff) * (right & 0xffff) + (left & 0xffff) * ((right >> 16) & 0xffff)
    return (low + (high << 16)) & 0xffffffff

## Clean004ec090: exact127/127 bypasses scaling. Every other pair uses
## low32(object*source*preference), signed SAR14, then AL without a clamp.
static func parameter_volume_byte(preference: int, source_word: int, object_word: int) -> int:
    var byte := preference & 0xff
    var source := source_word & 0xffffffff
    var object := object_word & 0xffffffff
    if source == 127 and object == 127: return byte
    var product := low32_product(low32_product(object, source), byte)
    if product >= 0x80000000: product -= 0x100000000
    return (product >> 14) & 0xff

func request_volume_byte(module: StringName, object_word: int) -> int:
    return parameter_volume_byte(int(preference_levels[module]), source_volume_word(module), object_word)

## The user bus already supplies its preference gain. Compensate at the native
## player so quantization/wrap are applied once, rather than multiplying the
## same preference twice. Authored alternate buses receive the complete gain.
## Byte-to-native amplitude remains /127 pending the external driver's law.
func native_request_gain(module: StringName, object_word: int, native_bus: StringName) -> float:
    var byte := request_volume_byte(module, object_word)
    if native_bus == BUSES[module]:
        var preference := int(preference_levels[module])
        return float(byte) / float(preference) if preference > 0 else 0.0
    return float(byte) / 127.0
