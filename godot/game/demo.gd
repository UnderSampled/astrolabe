extends Node3D
## Game session using the normally exported, already saved level.
## Hype's generated Brain owns gameplay choices. The provisional Load Game
## path restores the measured English-save state until general save/load lands.

const LEVEL := "res://generated/hype/levels/manoir/level.tscn"
const BRIDGE_GATE := "world/0/15/iMGrille_Manoir_Pont"
const GATE_LIFT := 5.2
const MANOIR_START = preload("res://game/manoir_start.gd")

@onready var demo_camera: Camera3D = $DemoCamera
@onready var status_label: Label = $Hud/Panel/Margin/Rows/Status
@onready var mana_label: Label = $Hud/Panel/Margin/Rows/Mana

var level: Node3D
var actor: CharacterBody3D
var brain: Node
var ready_to_play := false

func _ready() -> void:
    status_label.text = "Loading saved game..."
    mana_label.text = ""
    call_deferred("open_level")

func open_level() -> void:
    if not ResourceLoader.exists(LEVEL):
        fail("The manor has not been generated. See godot/README.md.")
        return
    status_label.text = "Loading the manor..."
    var packed := load(LEVEL) as PackedScene
    if packed == null:
        fail("The generated manor scene could not be opened.")
        return
    level = packed.instantiate() as Node3D
    if level == null:
        fail("The generated manor scene is invalid.")
        return
    actor = level.get_node_or_null("world/0/0/Hype") as CharacterBody3D
    brain = actor.get_node_or_null("Brain") if actor != null else null
    if actor == null or brain == null:
        fail("The generated manor is missing Hype or his Brain.")
        return
    var gate := level.get_node_or_null(BRIDGE_GATE) as CharacterBody3D
    var gate_brain := gate.get_node_or_null("Brain") if gate != null else null
    if gate == null or gate_brain == null:
        fail("The generated manor is missing the bridge gate.")
        return
    var fixture := MANOIR_START.load_fixture()
    if fixture.is_empty():
        fail("The English-save starting state is missing.")
        return
    brain.set("automatic", false)
    gate_brain.set("automatic", false)
    actor.set_physics_process(false)
    status_label.text = "Restoring saved game..."
    add_child(level)
    # Temporary gameplay workaround: retract the gate by its full height.
    # Its source opening AI is still missing; this is not restored save data.
    gate.position.y += GATE_LIFT
    actor.set_physics_process(false)
    var fixture_error: String = MANOIR_START.apply(actor, brain, fixture)
    if not fixture_error.is_empty():
        fail(fixture_error)
        return
    demo_camera.set("target", actor)
    demo_camera.call("follow")
    demo_camera.make_current()
    status_label.text = "Preparing to play..."
    # Match the gameplay harness: let first-use resources settle before Hype's
    # action clock starts, then give native contact one physics step.
    await get_tree().create_timer(5.0).timeout
    if not is_inside_tree():
        return
    actor.set_physics_process(true)
    await get_tree().create_timer(0.1).timeout
    if not is_inside_tree():
        return
    brain.set("automatic", true)
    if not bool(brain.call("start")):
        fail("Hype's compiled behavior could not start.")
        return
    ready_to_play = true
    status_label.text = "Explore the manor"

func _process(_delta: float) -> void:
    if not ready_to_play or not is_instance_valid(actor) or not is_instance_valid(brain):
        return
    var magick: Resource = actor.get("source_magick")
    if magick != null:
        mana_label.text = "Fire  %d / %d mana" % [int(magick.get("current_points")), int(magick.get("maximum_points"))]
    var failure := str(brain.get("unsupported_failure"))
    if not failure.is_empty():
        status_label.text = "This behavior is not supported yet. Press R to reload."

func _unhandled_key_input(event: InputEvent) -> void:
    var key := event as InputEventKey
    if key == null or not key.pressed or key.echo:
        return
    if key.physical_keycode == KEY_R:
        get_viewport().set_input_as_handled()
        get_tree().change_scene_to_file("res://game/demo.tscn")
    elif key.physical_keycode == KEY_ESCAPE:
        get_viewport().set_input_as_handled()
        get_tree().change_scene_to_file("res://game/bootstrap.tscn")

func fail(message: String) -> void:
    push_error(message)
    status_label.text = message
