extends SceneTree
## Independent session evidence harness for the authored Godot game. It starts
## the real main menu and uses native InputEventKey dispatch; it never builds a
## replacement level or interprets source asset tables.

var source_dir: String
var output_dir: String
var stream: FileAccess
var sequence := 0
var script_ticks_us := 0
var startup_offset_us := 0
var source_inputs: Array = []
var next_input := 0
var frames := 0
var states := 0
var frame_interval_us := 250000
var next_frame_us := 0
var next_state_us := 0
var duration_us := 0
var previous_iteration_us := -1

func _initialize() -> void:
    call_deferred("run")

func elapsed_us() -> int:
    return startup_offset_us + Time.get_ticks_usec() - script_ticks_us

func write_json(path: String, value: Dictionary) -> bool:
    var file := FileAccess.open(path, FileAccess.WRITE)
    if file == null:
        push_error("Cannot write Godot session evidence: " + path)
        return false
    file.store_string(JSON.stringify(value) + "\n")
    file.flush()
    file.close()
    return true

func append_event(kind: String, data: Dictionary, at_us: int = -1) -> void:
    if at_us < 0: at_us = elapsed_us()
    stream.store_string(JSON.stringify({
        "sequence": sequence,
        "elapsed_micros": at_us,
        "kind": kind,
        "data": data,
    }) + "\n")
    stream.flush()
    sequence += 1

func x_keysym_to_godot(keysym: int) -> int:
    match keysym:
        0xff51: return KEY_LEFT
        0xff52: return KEY_UP
        0xff53: return KEY_RIGHT
        0xff54: return KEY_DOWN
        0xffe1: return KEY_SHIFT
        0xffe3: return KEY_CTRL
        0xff0d: return KEY_ENTER
        0xff1b: return KEY_ESCAPE
        0xff09: return KEY_TAB
        0x20: return KEY_SPACE
        0x78, 0x58: return KEY_X
        0x7a, 0x5a: return KEY_Z
        _: return 0

func load_source() -> bool:
    var manifest = JSON.parse_string(FileAccess.get_file_as_string(source_dir.path_join("manifest.json")))
    if not manifest is Dictionary or int(manifest.get("schema", 0)) != 1:
        push_error("Unsupported source session manifest")
        return false
    duration_us = int(manifest.get("elapsed_micros", 0))
    if duration_us < 1:
        push_error("Source session has no duration")
        return false
    var file := FileAccess.open(source_dir.path_join("events.jsonl"), FileAccess.READ)
    if file == null:
        push_error("Source session has no event stream")
        return false
    while not file.eof_reached():
        var line := file.get_line()
        if line.is_empty(): continue
        var value = JSON.parse_string(line)
        if not value is Dictionary:
            push_error("Malformed source session event")
            return false
        if str(value.get("kind", "")) == "input":
            var data: Dictionary = value.get("data", {})
            source_inputs.append({
                "at_micros": int(data.get("observed_micros", value.get("elapsed_micros", 0))),
                "keysym": int(data.get("keysym", 0)) if data.get("keysym") != null else 0,
                "edge": str(data.get("edge", "")),
                "source_code": int(data.get("code", 0)),
                "repeat": bool(data.get("repeat", false)),
            })
    file.close()
    return true

func current_state() -> Dictionary:
    var scene := current_scene
    if scene == null:
        return {"phase": "startup", "level": "", "physics_frames": Engine.get_physics_frames(), "render_frames": Engine.get_frames_drawn()}
    if scene is Control:
        return {"phase": "menu", "level": "", "scene": str(scene.scene_file_path), "physics_frames": Engine.get_physics_frames(), "render_frames": Engine.get_frames_drawn()}
    var result := {
        "phase": "loading",
        "level": "manoir",
        "scene": str(scene.scene_file_path),
        "physics_frames": Engine.get_physics_frames(),
        "render_frames": Engine.get_frames_drawn(),
    }
    if not scene is Node3D: return result
    var actor = scene.get("actor") as CharacterBody3D
    var brain = scene.get("brain") as Node
    var ready := bool(scene.get("ready_to_play"))
    result["phase"] = "gameplay" if ready else "loading"
    if actor != null:
        var position: Vector3 = actor.global_position
        result["actor"] = {
            "translation": [position.x, -position.z, position.y],
            "current_state": int(actor.get("current_state")),
            "velocity": [actor.velocity.x, actor.velocity.y, actor.velocity.z],
            "on_floor": actor.is_on_floor(),
        }
    if brain != null:
        result["brain"] = {
            "running": bool(brain.get("running")),
            "normal_behavior": int(brain.get("current_normal")),
            "reflex_behavior": int(brain.get("current_reflex")),
            "failure": str(brain.get("unsupported_failure")),
        }
    return result

func dispatch_input(input: Dictionary) -> void:
    var code := x_keysym_to_godot(int(input["keysym"]))
    if code == 0:
        append_event("input_gap", {"reason": "unsupported X11 keysym", "source": input})
        return
    var key := InputEventKey.new()
    key.physical_keycode = code
    key.keycode = code
    key.pressed = str(input["edge"]) == "press"
    key.echo = bool(input["repeat"])
    Input.parse_input_event(key)
    Input.flush_buffered_events()
    append_event("input_dispatch", {
        "source_elapsed_micros": int(input["at_micros"]),
        "late_micros": max(0, elapsed_us() - int(input["at_micros"])),
        "source_code": int(input["source_code"]),
        "source_keysym": int(input["keysym"]),
        "physical_keycode": code,
        "edge": str(input["edge"]),
        "repeat": bool(input["repeat"]),
        "consumption_verified": false,
    })

func capture_frame() -> void:
    var texture := root.get_texture()
    if texture == null:
        append_event("capture_gap", {"stream": "video", "reason": "viewport texture unavailable"})
        return
    var requested := elapsed_us()
    RenderingServer.force_draw()
    var image := texture.get_image()
    var acquired := elapsed_us()
    var filename := "frames/%06d.png" % frames
    var error := image.save_png(output_dir.path_join(filename))
    if error != OK:
        append_event("capture_gap", {"stream": "video", "reason": "PNG write failed", "code": error})
        return
    append_event("frame", {
        "path": filename,
        "width": image.get_width(), "height": image.get_height(),
        "request_start_micros": requested,
        "reply_end_micros": acquired,
        "render_frames": Engine.get_frames_drawn(),
        "association": "forced viewport draw after process_frame; producer update unverified",
    })
    frames += 1

func run() -> void:
    var args := OS.get_cmdline_user_args()
    if args.size() != 3:
        push_error("Godot session replay requires source, output, and launch time")
        quit(1)
        return
    source_dir = args[0]
    output_dir = args[1]
    script_ticks_us = Time.get_ticks_usec()
    var launch_wall_us := int(args[2])
    startup_offset_us = max(0, int(Time.get_unix_time_from_system() * 1000000.0) - launch_wall_us)
    if not load_source():
        quit(1)
        return
    stream = FileAccess.open(output_dir.path_join("events.jsonl"), FileAccess.WRITE)
    if stream == null:
        push_error("Cannot open Godot session stream")
        quit(1)
        return
    append_event("launch", {"armed_before_process_launch": true}, 0)
    append_event("harness_attached", {
        "startup_clock_offset_micros": startup_offset_us,
        "clock_mapping": "parent wall launch time to Godot monotonic ticks at harness attach",
        "clock_uncertainty": "wall-clock mapping and pre-attach process startup are unverified",
    })
    var opened := change_scene_to_file("res://game/bootstrap.tscn")
    if opened != OK:
        append_event("session_end", {"ending": "main_scene_failed", "code": opened})
        stream.close()
        write_json(output_dir.path_join("summary.json"), {"ending":"main_scene_failed","elapsed_micros":elapsed_us()})
        quit(1)
        return
    append_event("scene_change", {"scene": "res://game/bootstrap.tscn"})
    while elapsed_us() < duration_us:
        await process_frame
        var now := elapsed_us()
        if previous_iteration_us >= 0 and now - previous_iteration_us > 500000:
            append_event("capture_gap", {
                "stream": "all", "reason": "harness main thread stalled",
                "start_micros": previous_iteration_us, "end_micros": now,
            })
        previous_iteration_us = now
        while next_input < source_inputs.size() and int(source_inputs[next_input]["at_micros"]) <= now:
            dispatch_input(source_inputs[next_input])
            next_input += 1
        if now >= next_state_us:
            append_event("state", {"state": current_state(), "consistency": "process_frame_sample", "completed_update": false})
            states += 1
            next_state_us = now + 500000
        if now >= next_frame_us:
            capture_frame()
            if now >= next_frame_us + frame_interval_us:
                append_event("frame_gap", {"reason":"capture exceeded selected interval", "skipped_intervals":(now - next_frame_us) / frame_interval_us})
            next_frame_us = now + frame_interval_us
    var ending := "deadline_overrun" if elapsed_us() > duration_us + 500000 else "completed"
    if next_input != source_inputs.size(): ending = "inputs_not_dispatched"
    append_event("session_end", {"ending": ending, "undispatched_inputs": source_inputs.size() - next_input})
    var total := elapsed_us()
    stream.close()
    write_json(output_dir.path_join("summary.json"), {
        "ending": ending, "elapsed_micros": total,
        "frames": frames, "states": states,
        "source_inputs": source_inputs.size(), "dispatched_inputs": next_input,
    })
    # Rust owns X11 capture finalization and appends its indexed video metadata
    # after this script has closed events.jsonl. Keep the window alive until
    # the parent publishes that it has sealed the segment.
    var marker_path := output_dir.path_join("video-finalized")
    var marker_deadline := Time.get_ticks_msec() + 30000
    while not FileAccess.file_exists(marker_path) and Time.get_ticks_msec() < marker_deadline:
        await create_timer(0.05).timeout
    if not FileAccess.file_exists(marker_path):
        push_error("Parent did not finalize continuous video within 30 seconds")
        quit(1)
        return
    quit(0)
