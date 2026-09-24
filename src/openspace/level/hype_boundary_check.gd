extends SceneTree
## Component-boundary proof, not initialization or integrated movement acceptance.
## Source turn rule and native character run automatically in normal physics.
var actor: CharacterBody3D
var brain: Node
var audit: Node
var ordinal := 0
var received := 0
var trace: Array[Dictionary] = []
var started_us := 0
var failed := false
var stall_ms := 0
var last_rotation := 0.0
var last_render := -1
var catchup_count := 0
var acknowledgments: Array[Dictionary] = []
var warmup_ticks := 0
var setup_finished_us := 0
var fault := ""
var recording_sha256 := ""
var recorded_events: Dictionary = {}
# Deliberate consecutive one-update press/release edges and a physical chord.
const EVENTS := {3: [[KEY_LEFT, true]], 4: [[KEY_LEFT, false]],
    5: [[KEY_LEFT, true]], 6: [[KEY_LEFT, false]],
    8: [[KEY_SHIFT, true], [KEY_LEFT, true]],
    13: [[KEY_LEFT, false], [KEY_SHIFT, false]]}

class Driver:
    extends Node
    func _ready() -> void:
        process_physics_priority = 1000
    func _input(event: InputEvent) -> void:
        if event is InputEventKey: get_tree().received += 1
    func _physics_process(delta: float) -> void:
        get_tree().after_consumer(delta)
    func _process(_delta: float) -> void:
        # A render/main-loop stall must produce real native catch-up physics.
        if get_tree().stall_ms > 0: OS.delay_msec(get_tree().stall_ms)

func _initialize() -> void:
    var args := OS.get_cmdline_user_args()
    Engine.physics_ticks_per_second = int(args[1])
    Engine.max_physics_steps_per_frame = 8
    stall_ms = int(args[2])
    if args.size() > 3: fault = args[3]
    recorded_events = EVENTS.duplicate(true)
    if args.size() > 4 and not args[4].is_empty():
        if not load_recording(args[4]):
            push_error("invalid or incompatible shared component recording")
            quit(1)
            return
    Input.use_accumulated_input = false
    call_deferred("setup")

func load_recording(path: String) -> bool:
    # This fixture proves the established pulse/chord boundary. Broader gameplay
    # recordings need their own expectations, not a different meaning here.
    if not FileAccess.file_exists(path): return false
    var value = JSON.parse_string(FileAccess.get_file_as_string(path))
    if not value is Dictionary or value.get("schema") != 1: return false
    if value.get("initial_held") != []: return false
    var updates = value.get("updates")
    if not updates is Array or updates.size() != 18: return false
    var physical := {"ArrowLeft": KEY_LEFT, "ArrowRight": KEY_RIGHT,
        "ArrowUp": KEY_UP, "ArrowDown": KEY_DOWN, "LeftShift": KEY_SHIFT}
    var supplied := {}
    for index in updates.size():
        var update = updates[index]
        if not update is Dictionary or update.get("ordinal") != index + 1: return false
        var edges = update.get("events")
        if not edges is Array: return false
        var converted := []
        for edge in edges:
            if not edge is Dictionary or edge.get("keyboard") != 0: return false
            if not physical.has(edge.get("key")): return false
            if edge.get("edge") not in ["press", "release"]: return false
            converted.append([physical[edge.key], edge.edge == "press"])
        if converted != EVENTS.get(index + 1, []): return false
        if not converted.is_empty(): supplied[index + 1] = converted
    recorded_events = supplied
    recording_sha256 = FileAccess.get_sha256(path)
    return recording_sha256.length() == 64

func require(ok: bool, message: String) -> bool:
    if not ok and not failed:
        failed = true
        push_error(message)
        finish()
    return ok

func setup() -> void:
    actor = load("res://" + OS.get_cmdline_user_args()[0]).instantiate()
    actor.autoplay = false
    root.add_child(actor)
    # This isolated input-boundary probe has no floor. Enable native physics
    # after _ready so it tests turning without the gameplay gravity default.
    actor.motion_enabled = true
    actor.select_state(47)
    # Explicit source-rule Given fixture, matching hype_turn_check.gd. No test
    # method advances gameplay, chooses an outcome action or forces movement.
    brain = load("res://rules.gd").new()
    brain.automatic = false
    brain.current_normal = 4
    actor.add_child(brain)
    brain.main_actor = actor
    brain.sound_emitter = actor.get_node("SoundEmitter")
    brain.@v10@ = false
    brain.@v19@ = false
    brain.@v52@ = true
    brain.@v59@ = false
    brain.@v61@ = false
    if not require(brain.start(), "source-rule startup failed"): return
    brain.automatic = fault != "no-op"
    audit = load("res://input-observer.gd").new()
    audit.automatic = false
    audit.current_normal = 0
    actor.add_child(audit)
    audit.main_actor = actor
    audit.sound_emitter = actor.get_node("SoundEmitter")
    if not require(audit.start(), "compiled input observer startup failed"): return
    audit.automatic = true
    root.add_child(Driver.new())
    setup_finished_us = Time.get_ticks_usec()

func after_consumer(delta: float) -> void:
    if failed: return
    # Asset loading can leave several native catch-up ticks in the first loop.
    # Ready state is established before the scenario's time/input origin.
    if warmup_ticks < 30:
        warmup_ticks += 1
        started_us = Time.get_ticks_usec()
        last_rotation = actor.rotation.y
        return
    ordinal += 1
    var render := Engine.get_process_frames()
    if render == last_render: catchup_count += 1
    last_render = render
    var held := ordinal in [3, 5] or (ordinal >= 8 and ordinal < 13)
    var press := ordinal in [3, 5, 8]
    var release := ordinal in [4, 6, 13]
    var expected_received := 0
    for at in recorded_events:
        if at <= ordinal: expected_received += recorded_events[at].size()
    var queries := [audit.dsg_0, audit.dsg_1, audit.dsg_2]
    var now := Time.get_ticks_usec()
    trace.append({"ordinal": ordinal, "physics_frame": Engine.get_physics_frames(),
        "render_frame": render, "delta_seconds": delta, "elapsed_us": now - started_us,
        "queries": queries, "rotation_y": actor.rotation.y, "action": actor.current_state,
        "normal": brain.current_normal, "input_callbacks": received,
        "native_animation_seconds": actor.animation_player.current_animation_position})
    if not require(queries == [held, press, release], "compiled input edge mismatch at %s: %s" % [ordinal, queries]): return
    if not require(received == expected_received, "physical events did not pass normal input dispatch"): return
    var expected_turn := deg_to_rad((240.0 if ordinal >= 8 and ordinal < 13 else 160.0) * delta) if held else 0.0
    if not require(absf(actor.rotation.y - last_rotation - expected_turn) < 0.00001,
        "source turn was not consumed in intended physics tick %s" % ordinal): return
    last_rotation = actor.rotation.y
    if not require(actor.position.length() < 0.0001, "turn input moved actor position"): return
    if ordinal == 18:
        if not require(stall_ms == 0 or catchup_count >= 4, "stall failed to exercise consecutive native catch-up ticks"): return
        finish()
        return
    # Input stamps its physics edge for the NEXT physics update. Dispatch after
    # this actor has completed, even when the next update precedes idle/render.
    var scheduled := ordinal if fault == "late-input" else ordinal + 1
    for event in recorded_events.get(scheduled, []):
        dispatch(event[0], event[1])
        acknowledgments.append({"requested_ordinal": ordinal + 1,
            "dispatch_after_physics": Engine.get_physics_frames(),
            "dispatch_us": Time.get_ticks_usec() - started_us,
            "physical_key": event[0], "pressed": event[1]})

func dispatch(code: int, pressed: bool) -> void:
    var event := InputEventKey.new()
    event.physical_keycode = code
    event.pressed = pressed
    if code == KEY_SHIFT: event.location = KEY_LOCATION_LEFT
    event.shift_pressed = pressed if code == KEY_SHIFT else Input.is_physical_key_pressed(KEY_SHIFT)
    Input.parse_input_event(event)
    Input.flush_buffered_events()

func finish() -> void:
    dispatch(KEY_LEFT, false)
    dispatch(KEY_SHIFT, false)
    if is_instance_valid(brain): brain.stop()
    if is_instance_valid(audit): audit.stop()
    print("HYPE_INPUT_BOUNDARY_TRACE ", JSON.stringify({"ticks_per_second": Engine.physics_ticks_per_second,
        "stall_ms": stall_ms, "catchup_ticks": catchup_count, "samples": trace,
        "dispatch": acknowledgments, "elapsed_us": Time.get_ticks_usec() - started_us,
        "recording_sha256": recording_sha256,
        "warmup_ticks": warmup_ticks, "warmup_us": started_us - setup_finished_us,
        "scope": "source turn rule; full initialization/reflex and movement not accepted"}))
    if not failed: print("HYPE_INPUT_BOUNDARY_OK")
    quit(1 if failed else 0)
