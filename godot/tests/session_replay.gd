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
var frame_interval_us := int(OS.get_environment("ASTROLABE_REPLAY_FRAME_US")) if OS.has_environment("ASTROLABE_REPLAY_FRAME_US") else 250000
var next_frame_us := 0
var next_state_us := 0
var duration_us := 0
var previous_iteration_us := -1
## Loading takes different times in each engine, so inputs replay against two
## anchors: menu inputs from the menu being ready, the rest from manoir entry.
const MENU_SETTLE_US := 1000000
const ENTRY_DEADLINE_US := 120000000
var source_entry_us := -1
var source_first_input_us := -1
var menu_ready_us := -1
var entry_us := -1
## After manoir entry the engines drift: the intro's length differs and a level
## takes its own time to load. Each anchor restarts input timing at a moment both
## engines reach. Entries are {"source_us", "kind", "level", "port_us"}.
var anchors: Array = []
const ANCHOR_GRACE_US := 40000000
## ASTROLABE_REPLAY_RESYNC_US: open-loop input drifts within seconds. Each interval
## the port records how far Hype drifted, then is put back on the source pose.
var resyncs: Array = []
## Hype's scripted or locked normals in the manor and Brigand takes: dialogue
## (28), cutscene lock (48), scripted walks (49, 50), item and key sequences
## (33, 61, 74, 78, 81, 90, 91), the statue opening (16, 66 to 70) and 41.
const SCRIPTED_NORMALS := [16, 28, 33, 41, 48, 49, 50, 61, 66, 67, 68, 69, 70, 74, 78, 81, 90, 91]
## Source pause/options spans after manor entry, as [start, end] micros.
var menu_spans: Array = []
var next_resync := 0

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
        0x63, 0x43: return KEY_C
        # Stephen's take rebinds the aim key to v at 586 s; Insert is its default.
        0x76, 0x56: return KEY_INSERT
        0xffe2: return KEY_SHIFT
        0xffe5: return KEY_CAPSLOCK
        0xffff: return KEY_DELETE
        0xff57: return KEY_END
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
        if str(value.get("kind", "")) == "state" and source_entry_us < 0:
            var sampled: Dictionary = value.get("data", {}).get("state", {})
            if int(sampled.get("phase", -1)) == 9 and str(sampled.get("level", "")) == "manoir":
                source_entry_us = int(value.get("elapsed_micros", 0))
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
    if source_entry_us < 0:
        push_error("Source session has no manoir entry state; replay needs a full recording")
        return false
    source_inputs.sort_custom(func(a, b): return a["at_micros"] < b["at_micros"])
    var seen_level := ""
    var file2 := FileAccess.open(source_dir.path_join("events.jsonl"), FileAccess.READ)
    while not file2.eof_reached():
        var line := file2.get_line()
        # Control returns at the first normal 4 past the opening's own first minute.
        if anchors.is_empty() and line.contains('"kind":"actor"'):
            var sample: Dictionary = JSON.parse_string(line)
            if int(sample["elapsed_micros"]) > source_entry_us + 60000000 and int(sample["data"]["normal_behavior"]["current"]) == 4:
                anchors.append({"source_us": int(sample["elapsed_micros"]), "kind": "control", "level": "", "port_us": -1})
        if OS.has_environment("ASTROLABE_REPLAY_RESYNC_US") and line.contains('"kind":"actor"'):
            var sample: Dictionary = JSON.parse_string(line)
            var at := int(sample["elapsed_micros"])
            var interval := int(OS.get_environment("ASTROLABE_REPLAY_RESYNC_US"))
            if (OS.has_environment("ASTROLABE_REPLAY_FROM_US") or (not anchors.is_empty() and at > int(anchors[0]["source_us"]))) and (resyncs.is_empty() or at - int(resyncs.back()["at_micros"]) >= interval):
                var form: Dictionary = sample["data"]["actor"]["global_transform"]
                form = form.get("Affine", form)
                resyncs.append({"at_micros": at, "translation": form["translation"], "basis": form["basis"], "normal": sample["data"]["normal_behavior"]["current"], "action": sample["data"]["actor"]["current_action"]})
        if not line.contains('"kind":"epoch"'): continue
        var data: Dictionary = JSON.parse_string(line).get("data", {})
        # Pause (16) and options (38) after entry: the port's menus differ, so
        # keys sent there would land somewhere else. Their spans are skipped.
        var epoch_at := int(JSON.parse_string(line).get("elapsed_micros", 0))
        var epoch_phase := int(data.get("phase", 0))
        if epoch_at > source_entry_us:
            if epoch_phase in [16, 38] and (menu_spans.is_empty() or int(menu_spans.back()[1]) >= 0):
                menu_spans.append([epoch_at, -1])
            elif not epoch_phase in [16, 38] and not menu_spans.is_empty() and int(menu_spans.back()[1]) < 0:
                menu_spans.back()[1] = epoch_at
        if int(data.get("phase", 0)) != 9: continue
        var level := str(data.get("level", ""))
        if not seen_level.is_empty() and level != seen_level:
            anchors.append({"source_us": int(JSON.parse_string(line).get("elapsed_micros", 0)), "kind": "level", "level": level, "port_us": -1})
        seen_level = level
    file2.close()
    # The Esc that opens the pause menu lands a few tenths of a second before
    # the source enters phase 16; it belongs to the menu span, or the port's
    # own pause menu opens and later gameplay keys pick its entries.
    for span in menu_spans:
        for input in source_inputs:
            var at := int(input["at_micros"])
            if int(input["keysym"]) == 0xff1b and str(input["edge"]) == "press" and at < int(span[0]) and at >= int(span[0]) - 1000000:
                span[0] = at
    if OS.has_environment("ASTROLABE_REPLAY_FROM_US"):
        # Segment replay: start from a save written at this take time.
        # Earlier inputs and anchors are dropped; resyncs start here.
        source_entry_us = int(OS.get_environment("ASTROLABE_REPLAY_FROM_US"))
        source_inputs = source_inputs.filter(func(input): return int(input["at_micros"]) >= source_entry_us)
        anchors = anchors.filter(func(anchor): return anchor["kind"] == "level" and int(anchor["source_us"]) > source_entry_us)
        resyncs = resyncs.filter(func(mark): return int(mark["at_micros"]) > source_entry_us)
    if not source_inputs.is_empty(): source_first_input_us = int(source_inputs[0]["at_micros"])
    return true

## The port's time for one input, or -1 until its anchor has been reached.
func scheduled_us(input: Dictionary) -> int:
    var at := int(input["at_micros"])
    if at >= source_entry_us:
        if entry_us < 0: return -1
        var scheduled := entry_us + at - source_entry_us
        for anchor in anchors:
            if at < int(anchor["source_us"]): break
            if int(anchor["port_us"]) < 0: return -1
            scheduled = int(anchor["port_us"]) + at - int(anchor["source_us"])
        return scheduled
    # A menu input whose anchor has passed still goes out, in recorded order.
    return menu_ready_us + MENU_SETTLE_US + at - source_first_input_us if menu_ready_us >= 0 else -1

func track_anchors(now: int) -> void:
    var scene := current_scene
    if menu_ready_us < 0 and scene is Control and scene.get("menu") != null:
        menu_ready_us = now
        append_event("replay_anchor", {"anchor": "menu_ready", "port_micros": now, "source_first_input_micros": source_first_input_us})
        if OS.has_environment("ASTROLABE_REPLAY_SAVE"):
            # ASTROLABE_REPLAY_SAVE=original:SaveGame1.cfg (in game/saves) opens Load Game directly.
            root.set_meta(load("res://game/save_state.gd").LOAD_SOURCE_META, OS.get_environment("ASTROLABE_REPLAY_SAVE"))
            scene.call("open_game", "res://game/game_session.tscn", scene.get("load_game"))
    if entry_us < 0 and scene is Node3D and bool(scene.get("ready_to_play")):
        entry_us = now
        append_event("replay_anchor", {"anchor": "manoir_entry", "port_micros": now, "source_micros": source_entry_us})

func port_level() -> String:
    var scene := current_scene
    var level = scene.get("level") if scene is Node3D else null
    return str(level.get_meta("source_level_name", "")) if is_instance_valid(level) else ""

func track_replay_anchors(now: int) -> void:
    var scene := current_scene
    for i in anchors.size():
        var anchor: Dictionary = anchors[i]
        if int(anchor["port_us"]) >= 0: continue
        var reached := false
        if scene is Node3D and bool(scene.get("ready_to_play")):
            if anchor["kind"] == "control": reached = bool(scene.get("opening_finished")) or not bool(scene.get("new_game"))
            else: reached = port_level() == anchor["level"]
        # Past the grace the port is not coming: keep replaying and record that.
        var previous: int = entry_us if i == 0 else int(anchors[i - 1]["port_us"])
        var source_previous: int = source_entry_us if i == 0 else int(anchors[i - 1]["source_us"])
        var overdue: bool = entry_us >= 0 and now > previous + int(anchor["source_us"]) - source_previous + ANCHOR_GRACE_US
        if reached or overdue:
            anchor["port_us"] = now
            append_event("replay_anchor", {"anchor": anchor["kind"], "level": anchor["level"], "port_micros": now, "source_micros": anchor["source_us"], "reached": reached})
        break

func source_to_native(value: Array) -> Vector3:
    return Vector3(value[0], value[2], -value[1])

func apply_resync(now: int) -> void:
    while next_resync < resyncs.size():
        var mark: Dictionary = resyncs[next_resync]
        var due := scheduled_us(mark)
        if due < 0 or now < due: return
        next_resync += 1
        var scene := current_scene
        var actor = scene.get("actor") as CharacterBody3D if scene is Node3D else null
        if actor == null: continue
        var want := source_to_native(mark["translation"])
        var state := current_state()
        append_event("resync", {"source_micros": mark["at_micros"], "gap": actor.global_position.distance_to(want), "source": mark["translation"], "port": state.get("actor", {}).get("translation"), "port_level": state.get("level"), "source_normal": mark["normal"], "port_normal": state.get("brain", {}).get("normal_behavior"), "source_action": mark["action"], "port_action": state.get("actor", {}).get("current_state")})
        # A scripted walk or cutscene ends when Hype reaches its waypoint;
        # teleporting him mid-script strands it (exit lock, endless stairs
        # walk). Skip while either engine has Hype in a scripted behavior.
        # The source's level at this mark: the last level anchor at or before it.
        var source_level := "manoir"
        for anchor in anchors:
            if anchor["kind"] == "level" and int(anchor["source_us"]) <= int(mark["at_micros"]): source_level = str(anchor["level"])
        if str(state.get("level", "")) != source_level:
            append_event("resync_skipped", {"source_micros": mark["at_micros"], "reason": "different level", "source_level": source_level, "port_level": state.get("level")})
            continue
        if int(mark["normal"]) in SCRIPTED_NORMALS or int(state.get("brain", {}).get("normal_behavior", -1)) in SCRIPTED_NORMALS:
            append_event("resync_skipped", {"source_micros": mark["at_micros"], "source_normal": mark["normal"], "port_normal": state.get("brain", {}).get("normal_behavior")})
            continue
        var rows: Array = mark["basis"]
        var axes := Basis(Vector3(1, 0, 0), Vector3(0, 0, -1), Vector3(0, 1, 0))
        var pose := Basis(Vector3(rows[0][0], rows[0][1], rows[0][2]), Vector3(rows[1][0], rows[1][1], rows[1][2]), Vector3(rows[2][0], rows[2][1], rows[2][2]))
        actor.global_transform = Transform3D(axes * pose * axes.inverse(), want)
        actor.velocity = Vector3.ZERO
        actor.set("_contact_sector", null)
        actor.set("source_contact_sector_path", NodePath(""))
        actor.set("_contact_previous", want)
        scene.call("_locate_player_sector")

func current_state() -> Dictionary:
    var scene := current_scene
    if scene == null:
        return {"phase": "startup", "level": "", "physics_frames": Engine.get_physics_frames(), "render_frames": Engine.get_frames_drawn()}
    if scene is Control:
        return {"phase": "menu", "level": "", "scene": str(scene.scene_file_path), "physics_frames": Engine.get_physics_frames(), "render_frames": Engine.get_frames_drawn()}
    var result := {
        "phase": "loading",
        "level": port_level(),
        "scene": str(scene.scene_file_path),
        "physics_frames": Engine.get_physics_frames(),
        "render_frames": Engine.get_frames_drawn(),
    }
    if not scene is Node3D: return result
    var actor = scene.get("actor") as CharacterBody3D
    var brain = scene.get("brain") as Node
    var ready := bool(scene.get("ready_to_play"))
    result["phase"] = "gameplay" if ready else "loading"
    if ready and bool(scene.get("new_game")):
        result["phase"] = "opening_preview"
        result["opening_failures"] = scene.get("opening_failures").duplicate()
    if actor != null:
        var position: Vector3 = actor.global_position
        result["actor"] = {
            "translation": [position.x, -position.z, position.y],
            "forward": [actor.global_basis.z.x, -actor.global_basis.z.z, actor.global_basis.z.y],
            "current_state": int(actor.get("current_state")),
            "velocity": [actor.velocity.x, actor.velocity.y, actor.velocity.z],
            "on_floor": actor.is_on_floor(),
        }
        if actor.get("source_inventory") != null:
            result["actor"]["weapon"] = actor.source_inventory_weapon()
            result["actor"]["bolts"] = actor.source_inventory_quantity(80)
    if brain != null:
        result["brain"] = {
            "running": bool(brain.get("running")),
            "normal_behavior": int(brain.get("current_normal")),
            "reflex_behavior": int(brain.get("current_reflex")),
            "failure": str(brain.get("unsupported_failure")),
        }
    # Global designer fields: ASTROLABE_REPLAY_GLOBAL_FIELDS=dsg_147,dsg_229
    var level_node = scene.get("level")
    if OS.has_environment("ASTROLABE_REPLAY_GLOBAL_FIELDS") and level_node is Node:
        var paths: Dictionary = (level_node as Node).get_meta("source_actor_nodes", {})
        var global_actor: Node = (level_node as Node).get_node_or_null(paths.get("fix:/persos/15", NodePath()))
        var global_brain: Node = global_actor.get_node_or_null("Brain") if global_actor != null else null
        if global_brain != null:
            var fields := {}
            for field in OS.get_environment("ASTROLABE_REPLAY_GLOBAL_FIELDS").split(","): fields[field] = global_brain.get(field)
            result["global"] = fields
    # Watched actors' behavior, by source name: ASTROLABE_REPLAY_WATCH=a,b
    var level = scene.get("level")
    if OS.has_environment("ASTROLABE_REPLAY_WATCH") and level is Node:
        var watched := {}
        for body in (level as Node).find_children("*", "CharacterBody3D", true, false):
            var name := str(body.get_meta("source_perso_name", ""))
            if name in OS.get_environment("ASTROLABE_REPLAY_WATCH").split(","):
                var watched_brain: Node = body.get_node_or_null("Brain")
                watched[name] = [int(body.get("current_state")), int(watched_brain.get("current_normal")) if watched_brain else -1]
        result["watched"] = watched
    return result

func dispatch_input(input: Dictionary) -> void:
    var code := x_keysym_to_godot(int(input["keysym"]))
    if code == 0:
        append_event("input_gap", {"reason": "unsupported X11 keysym", "source": input})
        return
    var key := InputEventKey.new()
    key.physical_keycode = code
    key.keycode = code
    # X11 reports the side of Shift_L/R and Control_L/R; Godot carries it as location.
    match int(input["keysym"]):
        0xffe1, 0xffe3: key.location = KEY_LOCATION_LEFT
        0xffe2, 0xffe4: key.location = KEY_LOCATION_RIGHT
    key.pressed = str(input["edge"]) == "press"
    key.echo = bool(input["repeat"])
    Input.parse_input_event(key)
    Input.flush_buffered_events()
    append_event("input_dispatch", {
        "source_elapsed_micros": int(input["at_micros"]),
        "scheduled_micros": scheduled_us(input),
        "late_micros": max(0, elapsed_us() - scheduled_us(input)),
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
    DirAccess.make_dir_recursive_absolute(output_dir.path_join("frames"))
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
    while true:
        await process_frame
        track_anchors(elapsed_us())
        track_replay_anchors(elapsed_us())
        apply_resync(elapsed_us())
        if next_input >= source_inputs.size() and anchors.back()["port_us"] >= 0: break
        if OS.has_environment("ASTROLABE_REPLAY_UNTIL_US") and entry_us >= 0 and elapsed_us() > entry_us + int(OS.get_environment("ASTROLABE_REPLAY_UNTIL_US")) - source_entry_us: break
        if entry_us < 0 and elapsed_us() >= ENTRY_DEADLINE_US: break
        var now := elapsed_us()
        if previous_iteration_us >= 0 and now - previous_iteration_us > 500000:
            append_event("capture_gap", {
                "stream": "all", "reason": "harness main thread stalled",
                "start_micros": previous_iteration_us, "end_micros": now,
            })
        previous_iteration_us = now
        while next_input < source_inputs.size():
            var input: Dictionary = source_inputs[next_input]
            # The original's opening can outlast the port's. A key pressed in its
            # last dialogue would reach the port's gameplay, where it means
            # something else, so it is dropped once the port has handed over.
            if not anchors.is_empty() and anchors[0]["kind"] == "control" and int(anchors[0]["port_us"]) >= 0 and int(input["at_micros"]) < int(anchors[0]["source_us"]) and int(input["at_micros"]) > source_entry_us:
                append_event("input_gap", {"reason": "pre-handoff input after the port's handoff", "source": input})
                next_input += 1
                continue
            var in_menu := false
            for span in menu_spans:
                if int(input["at_micros"]) >= int(span[0]) and (int(span[1]) < 0 or int(input["at_micros"]) < int(span[1])): in_menu = true
            # Releases still go out, so no key stays held into gameplay.
            if in_menu and str(input["edge"]) == "press":
                append_event("input_gap", {"reason": "source pause or options menu", "source": input})
                next_input += 1
                continue
            var due := scheduled_us(input)
            if due < 0 or due > now: break
            dispatch_input(input)
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
    var ending := "completed" if entry_us >= 0 else "no_manoir_entry"
    if entry_us >= 0 and elapsed_us() > entry_us + duration_us - source_entry_us + 500000: ending = "deadline_overrun"
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
    var marker_deadline := Time.get_ticks_msec() + 180000
    while not FileAccess.file_exists(marker_path) and Time.get_ticks_msec() < marker_deadline:
        await create_timer(0.05).timeout
    if not FileAccess.file_exists(marker_path):
        push_error("Parent did not finalize continuous video within 180 seconds")
        quit(1)
        return
    quit(0)
