extends SceneTree
## Component regression using the actual exported manor and arrow resource.
## A high-altitude starting pose supplies clear space; no source collision,
## lifetime counter, velocity or cleanup behavior is patched.
const MANOIR_START = preload("res://game/manoir_start.gd")
var actor: CharacterBody3D
var owner_brain: Node
var arrow: CharacterBody3D
var released: bool = false
var release_frame: int
var release_clock: float
var exit_record: Dictionary = {}
var frames: Array = []
var last_zde_hit: bool = false

func require(ok: bool, reason: String) -> bool:
    if not ok:
        push_error(reason)
        quit(1)
    return ok

func _initialize() -> void:
    call_deferred("run")

func snapshot() -> Dictionary:
    var brain := arrow.get_node("Brain")
    return {
        "frame": Engine.get_physics_frames(),
        "clock_ms": brain.get("clock_milliseconds"),
        "counter": brain.get("dsg_0"),
        "normal": brain.get("current_normal"),
        "failure": brain.get("unsupported_failure"),
        "slide_count": arrow.get_slide_collision_count(),
        "zde_hit": last_zde_hit,
        "position": [arrow.global_position.x, -arrow.global_position.z, arrow.global_position.y],
        "velocity": [arrow.velocity.x, -arrow.velocity.z, arrow.velocity.y],
        "sighting": [arrow.source_sighting().x, arrow.source_sighting().y, arrow.source_sighting().z],
        "guards": [owner_brain.get("dsg_120"), owner_brain.get("dsg_58"), owner_brain.get("dsg_17")],
    }

func on_release(spawned: Node3D, _attachment: Node3D) -> void:
    arrow = spawned as CharacterBody3D
    released = true
    release_frame = Engine.get_physics_frames()
    release_clock = float(arrow.get_node("Brain").get("clock_milliseconds"))
    arrow.tree_exiting.connect(on_exit)

func on_exit() -> void:
    exit_record = snapshot()

func run() -> void:
    var args := OS.get_cmdline_user_args()
    if not require(args.size() == 3, "requires template path, evidence path and aim yaw"): return
    var aim_yaw := float(args[2])
    if not require(is_finite(aim_yaw) and absf(aim_yaw) <= 0.2, "bounded aim fixture"): return
    var scene := load("res://generated/hype/levels/manoir/level.tscn").instantiate() as Node3D
    for brain in scene.find_children("Brain", "Node", true, false):
        brain.set("automatic", false)
    root.add_child(scene)
    actor = scene.get_node("world/0/0/Hype") as CharacterBody3D
    owner_brain = actor.get_node("Brain")
    if not require(MANOIR_START.apply(actor, owner_brain, MANOIR_START.load_fixture()).is_empty(), "saved fixture"): return
    owner_brain.set("automatic", false)
    # Reaching a declared clear-space component fixture, not a normal-game save.
    actor.global_position.y = 200.0
    actor.motion_enabled = false
    owner_brain.set("dsg_29", 0)
    var target := actor.global_position + actor.global_basis.z.rotated(Vector3.UP, aim_yaw) * 1000.0
    actor.source_laser_pointer.target_position = target
    actor.source_laser_pointer.target_valid = true
    actor.generated_object_released.connect(on_release)
    if not require(actor.request_action(48), "source armed idle action"): return
    if not require(actor.source_execute_action_and_generate_object(args[0], 73, Vector3(0,-45,0)) == 1, "source generation request"): return
    # The maintained animation attachment releases the saved arrow template.
    for tick in range(120):
        actor.advance(1.0 / 60.0)
        await physics_frame
        if released: break
    if not require(released, "animation did not release the exported arrow"): return
    for tick in range(190):
        await physics_frame
        await process_frame
        if not exit_record.is_empty(): break
        if is_instance_valid(arrow):
            last_zde_hit = arrow.source_collide_perso_zde_no_with_type_zde(1, 1)
            frames.append(snapshot())
    var evidence := {
        "scope": "native_exported_crossbow_free_flight_component",
        "fixture": "supplied manor assets; shooter moved to source Z=200; owner AI held; original-game counterpart pending",
        "template": args[0], "aim_yaw_rad": aim_yaw, "release_frame": release_frame, "release_clock_ms": release_clock,
        "frames": frames, "exit": exit_record,
    }
    var file := FileAccess.open(args[1], FileAccess.WRITE)
    file.store_string(JSON.stringify(evidence) + "\n")
    file.close()
    if not require(not exit_record.is_empty(), "free-flying arrow did not clean up before 190 ticks"): return
    if not require(exit_record["counter"] == 151 and exit_record["failure"] == "", "cleanup did not follow the compiled lifetime counter"): return
    if not require(exit_record["guards"] == [0,false,false], "expiry did not clear all owner guards"): return
    if not require(exit_record["slide_count"] == 0 and not exit_record["zde_hit"], "flight ended through contact rather than expiry"): return
    if not require(frames.size() > 140, "missing sustained free-flight evidence"): return
    for frame in frames:
        if not require(frame["slide_count"] == 0 and not frame["zde_hit"] and frame["failure"] == "", "flight had contact or a Brain failure"): return
    var initial: Dictionary = frames[0]
    var velocity := Vector3(initial["velocity"][0], initial["velocity"][1], initial["velocity"][2])
    var sighting := Vector3(initial["sighting"][0], initial["sighting"][1], initial["sighting"][2])
    if not require(velocity.distance_to(sighting * 45.0) < 0.1, "representative launch direction differs from released sighting"): return
    var position := Vector3(initial["position"][0], initial["position"][2], -initial["position"][1])
    var source_target_direction := target - position
    var source_sighting := Vector3(sighting.x, sighting.z, -sighting.y)
    if not require(source_sighting.normalized().dot(source_target_direction.normalized()) > 0.999, "compiled arrow did not aim toward the controlled target"): return
    if not require(initial["guards"] == [1,true,true], "missing active-flight owner guards"): return
    var final_position: Array = exit_record["position"]
    var distance := Vector3(final_position[0], final_position[1], final_position[2]).distance_to(Vector3(initial["position"][0], initial["position"][1], initial["position"][2]))
    if not require(distance > 100.0, "missing distant free flight"): return
    print("CROSSBOW_FREE_FLIGHT_OK counter=151 guards=0 contact=0 frames=", frames.size())
    quit(0)
