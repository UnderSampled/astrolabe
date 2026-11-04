extends Node3D
## Game session using the normally exported, already saved level.
## Hype's generated Brain owns gameplay choices. The measured English state
## seeds the manor; a native slot restores supported mutable player fields.

const LEVEL := "res://generated/hype/levels/manoir/level.tscn"
var level_path := LEVEL
var initial_level_name := "manoir"
const GATE_ACTOR := "level:/persos/world/0/15"
const MANOIR_START = preload("res://game/manoir_start.gd")
const SAVE_STATE = preload("res://game/save_state.gd")
const SAVE_STARTUP = preload("res://game/save_startup.gd")
const NATIVE_AUDIO_EMITTER = preload("res://addons/astrolabe/audio/emitter.gd")
const SOURCE_CAMERA = preload("res://addons/astrolabe/camera/source_camera.gd")
const SCRIPT_WARMUP = preload("res://game/script_warmup.gd")
const StandInReport = preload("res://addons/astrolabe/behavior/stand_in_report.gd")
const LOADING_SCREEN_PATH := NodePath("/root/LevelTransition/LoadingScreen")
const INITIAL_ROUTE_ERROR_META := "astrolabe_initial_route_error"
const LEVEL_END_META := "astrolabe_level_end"
@export var new_game: bool = false

signal loading_stage_changed(stage: String, elapsed_ms: float)
signal initial_level_transition_finished(result: Dictionary)

@onready var demo_camera: Camera3D = $DemoCamera
@onready var status_label: Label = $Hud/Panel/Margin/Rows/Status
@onready var mana_label: Label = $Hud/Panel/Margin/Rows/Mana
@onready var content_cache: Node = get_node_or_null("/root/GameContent")
@onready var inventory_host: Node = $InventoryHost
@onready var pause_host: Node = $PauseHost
@onready var vignette_host: Node = $VignetteHost

var level: Node3D
var actor: CharacterBody3D
var brain: Node
var world_brain: Node
## Entering play is one of the source's settings write points (mode-3 entry
## calls FUN_00402780, which saves Current.cfg through FUN_00428910).
var ready_to_play := false:
    set(value):
        if value and not ready_to_play:
            var preferences := get_node_or_null("/root/UserSettings")
            if preferences != null and preferences.has_method("commit_control_bindings"):
                preferences.call("commit_control_bindings")
        ready_to_play = value
var loading := false
var loading_stage := "idle"
var load_started_usec := 0
var load_timings_ms: Dictionary = {}
var saved_max_fps := -1
var crossbow_aim_mode := false
var crossbow_aim_pitch := 0.0
var opening_brains: Array[Node] = []
var opening_failures: Dictionary = {}
var opening_camera: Camera3D
var opening_finished := false
var _transition_snapshot: Dictionary = {}
var _transition_brain_states: Array[Dictionary] = []
var _transition_physics_states: Array[Dictionary] = []
var _transition_camera: Camera3D
var _transition_source_camera_owner: Dictionary = {}
var _initial_transition_pending := false
var _initial_failure_recovery_started := false
var _initial_failure_recovery_cancelled := false
var source_camera_controller_state: Resource
## The selected file remains explicit through loading and an R reload.
var selected_save: Dictionary = {}
## Hype's source game-state selector is one shared byte, not actor Brain state.
var source_game_state_mask: int = 0
## A source death request is separate from Proc_SetGameState's selector mask.
## This native terminal state is transient and never belongs to a checkpoint.
var source_death_requested := false

## Proc_PlayerIsDead requests the original engine's death/reset phase. Until
## its level-reset lifecycle is supported, end native gameplay here. Reload
## remains an explicit user action governed by the existing native save policy.
func source_player_is_dead() -> bool:
    if source_death_requested: return true
    if loading or _initial_transition_pending or not is_instance_valid(level):
        return false
    source_death_requested = true
    ready_to_play = false
    pause_host.call("close_pause")
    inventory_host.call("close_inventory")
    if crossbow_aim_mode: set_crossbow_aim_mode(false)
    # Native actor emitters contain effects and speech. The separate shared
    # music channel is not one of the two source channels reset by death.
    for emitter in level.find_children("*", "Node3D", true, false):
        if emitter is NATIVE_AUDIO_EMITTER and not emitter.is_music:
            emitter.kill_all()
            # Forced-stop speech completion belongs to the emitter's later
            # polling frame; do not synchronously clear the speech holder.
    for source_brain in level.find_children("Brain", "Node", true, false):
        source_brain.set("automatic", false)
        source_brain.call("stop")
    for body in level.find_children("*", "CharacterBody3D", true, false):
        body.velocity = Vector3.ZERO
        body.set_physics_process(false)
    level.process_mode = Node.PROCESS_MODE_DISABLED
    demo_camera.set_process(false)
    demo_camera.set_process_unhandled_input(false)
    for action in InputMap.get_actions():
        if str(action).begins_with("BT_") or str(action).begins_with("Button_"):
            Input.action_release(action)
    $Hud/Panel/Margin/Rows/Save.disabled = true
    $Hud/Panel/Margin/Rows/Controls.text = "R  reload session     Esc  main menu"
    stage("dead", "Hype has died. Press R to reload the session.")
    return true

## Presentation/event request is separate from the source game-state selector.
var source_vignette_pending := false
var _source_vignette_presenting := false
var _source_vignette_image: Texture2D
var _source_vignette_milliseconds := 0

func source_vignette_return_to_menu(image: Texture2D, milliseconds: int) -> bool:
    # Both source operands have already been captured by the compiled caller.
    # Signed32 nonpositive values queue neither artwork nor menu transition.
    var duration := milliseconds & 0xffffffff
    if duration >= 0x80000000: duration -= 0x100000000
    if duration <= 0: return true
    if image == null or loading or _initial_transition_pending or not is_instance_valid(level) or _source_vignette_presenting:
        return false
    _source_vignette_image = image
    _source_vignette_milliseconds = duration
    if not source_vignette_pending:
        source_vignette_pending = true
        # Queue before requesting the menu. Consume after the current AI call,
        # allowing the rest of its frame to publish any final request values.
        call_deferred("_present_source_vignette")
    return true

func _present_source_vignette() -> void:
    if not source_vignette_pending or _source_vignette_presenting: return
    _source_vignette_presenting = true
    ready_to_play = false
    pause_host.call("close_pause")
    inventory_host.call("close_inventory")
    if crossbow_aim_mode: set_crossbow_aim_mode(false)
    for source_brain in level.find_children("Brain", "Node", true, false):
        source_brain.set("automatic", false)
        source_brain.call("stop")
    for body in level.find_children("*", "CharacterBody3D", true, false):
        body.set_physics_process(false)
    level.process_mode = Node.PROCESS_MODE_DISABLED
    demo_camera.set_process(false)
    demo_camera.set_process_unhandled_input(false)
    $Hud.hide()
    for action in InputMap.get_actions():
        if str(action).begins_with("BT_") or str(action).begins_with("Button_"):
            Input.action_release(action)
    vignette_host.connect("completed", _source_vignette_completed, CONNECT_ONE_SHOT)
    if not bool(vignette_host.call("present", _source_vignette_image, _source_vignette_milliseconds)):
        push_error("The queued source vignette could not be presented")

func _source_vignette_completed() -> void:
    _source_vignette_image = null
    source_vignette_pending = false
    # The original pending presenter tears down before event4 leaves gameplay.
    # Reuse the game's established session/menu teardown and destination.
    leave_session("res://game/bootstrap.tscn")

func source_set_game_state(state_id: int) -> void:
    var state_byte := state_id & 0xff
    if state_byte == 0:
        source_game_state_mask = 0
        return
    var shift := (state_byte - 1) & 0x1f
    source_game_state_mask = (1 << shift) & 0xff

## Func195 recognizes zero and individual bits. Combined masks preserve a
## stale evaluator result in Hype; native compiled callers diagnose null.
func source_get_game_state() -> Variant:
    match source_game_state_mask & 0xff:
        0: return 0
        1: return 1
        2: return 2
        4: return 3
        8: return 4
        16: return 5
        32: return 6
        64: return 7
        128: return 8
    return null

## The session panel is a developer aid; F1 shows it over the game.
func _toggle_session_panel() -> void:
    $Hud/Panel.visible = not $Hud/Panel.visible


func _ready() -> void:
    $Hud/Panel.visible = OS.has_environment("ASTROLABE_SESSION_PANEL")
    # Lock the incoming session before its deferred initial request starts.
    # The resource loader is active before open_level begins restoration.
    _initial_transition_pending = true
    var camera_state_script := "res://addons/astrolabe/camera/source_controller_state.gd"
    if ResourceLoader.exists(camera_state_script):
        source_camera_controller_state = load(camera_state_script).new()
    inventory_host.connect("unavailable", func(message: String): status_label.text = message)
    pause_host.connect("unavailable", func(message: String): status_label.text = message)
    configure_crossbow_inputs()
    load_started_usec = Time.get_ticks_usec()
    stage("session", "Loading New Game..." if new_game else "Loading saved game...")
    mana_label.text = ""
    $Hud/Panel/Margin/Rows/Save.pressed.connect(save_game)
    if new_game:
        $Hud/Panel/Margin/Rows/Save.disabled = true
        $Hud/Panel/Margin/Rows/Controls.text = "Tab  inventory     R  restart opening     Esc  pause"
    var transitions := get_node_or_null("/root/LevelTransition")
    if transitions != null:
        if transitions.has_signal("transition_failed"):
            transitions.connect("transition_failed", _on_initial_transition_failed)
        if transitions.has_signal("transition_cancelled"):
            transitions.connect("transition_cancelled", _on_initial_transition_cancelled)
    call_deferred("request_initial_level")

func request_initial_level() -> void:
    _initial_transition_pending = true
    var transitions := get_node_or_null("/root/LevelTransition")
    if transitions != null:
        if not new_game:
            var travel: Dictionary = SAVE_STATE.read_for_load(str(get_tree().root.get_meta(SAVE_STATE.LOAD_SOURCE_META, "")))
            if str(travel.get("source", "")) == SAVE_STATE.TRAVEL_SOURCE and travel["status"] == "loaded":
                if StandInReport.reached(self, "travel-save", "a travel save loads its level fresh, without per-level world records"): return
                initial_level_name = str(travel["level"])
                level_path = "res://generated/hype/levels/%s/level.tscn" % initial_level_name
        if bool(transitions.call("request_initial_level", initial_level_name)):
            return
        fail("The manor loading service could not start the initial load.")
        return
    open_level()

func _on_initial_transition_failed(level_name: String, message: String) -> void:
    if _initial_transition_pending and level_name == initial_level_name:
        fail(message)

func _on_initial_transition_cancelled(level_name: String) -> void:
    if _initial_transition_pending and level_name == initial_level_name:
        _initial_transition_pending = false

func open_level_from_transition(packed: PackedScene, path: String) -> Dictionary:
    if path != level_path or packed == null or level != null or loading:
        return {"ok": false, "message": "The initial manor request does not match this session."}
    if content_cache != null:
        content_cache.call("retain_level", path, packed)
    _initial_transition_pending = true
    call_deferred("open_level")
    var result: Dictionary = await initial_level_transition_finished
    return result

func bind_source_camera_controller_state() -> void:
    _bind_source_camera_controller_state(level)

func _bind_source_camera_controller_state(target_level: Node3D) -> void:
    if target_level == null:
        return
    if source_camera_controller_state == null:
        var camera_state_script := "res://addons/astrolabe/camera/source_controller_state.gd"
        if ResourceLoader.exists(camera_state_script):
            source_camera_controller_state = load(camera_state_script).new()
    if source_camera_controller_state == null:
        return
    target_level.set_meta("source_camera_controller_state", source_camera_controller_state)
    for source_camera in target_level.find_children("SourceCamera", "Camera3D", true, false):
        if _has_property(source_camera, "controller_state"):
            source_camera.set("controller_state", source_camera_controller_state)

func open_level() -> void:
    if loading or level != null:
        return
    loading = true
    var restore_message := "Initializing new game..." if new_game else "Restoring saved game..."
    _show_initial_restore_phase(restore_message)
    stage("state_restoration", restore_message)
    if not ResourceLoader.exists(level_path):
        fail("The manor has not been generated. See godot/README.md.")
        return
    if not new_game:
        var source := str(get_tree().root.get_meta(SAVE_STATE.LOAD_SOURCE_META, ""))
        selected_save = SAVE_STATE.read_for_load(source)
        if selected_save["status"] != "loaded":
            fail(str(selected_save.get("message", "The selected saved game is unavailable.")))
            return
    var packed: PackedScene = content_cache.call("get_level", level_path) as PackedScene if content_cache != null else null
    if packed == null:
        stage("resource_loading", "Loading the manor...")
        limit_loading_frames()
        # The manor reaches the same meshes through level and instance scenes.
        # Keep dependency loading in one worker to avoid competing subresource
        # cache registrations, while the session remains responsive.
        var warmup = SCRIPT_WARMUP.new()
        warmup.start(level_path)
        while not warmup.finished():
            await get_tree().process_frame
        warmup.complete()
        if not is_inside_tree():
            return
        var request := ResourceLoader.load_threaded_request(
            level_path, "PackedScene", false, ResourceLoader.CACHE_MODE_REUSE)
        if request != OK:
            fail("The manor scene could not begin loading.")
            return
        while is_inside_tree() and ResourceLoader.load_threaded_get_status(level_path) == ResourceLoader.THREAD_LOAD_IN_PROGRESS:
            await get_tree().process_frame
        if not is_inside_tree():
            return
        if ResourceLoader.load_threaded_get_status(level_path) == ResourceLoader.THREAD_LOAD_FAILED:
            # As in level_transition: a threaded script preload occasionally
            # fails to parse and loads on a second try with fresh cache entries.
            push_warning("Retrying the level load after a threaded load failure.")
            if ResourceLoader.load_threaded_request(level_path, "PackedScene", false, ResourceLoader.CACHE_MODE_REPLACE_DEEP) == OK:
                while is_inside_tree() and ResourceLoader.load_threaded_get_status(level_path) == ResourceLoader.THREAD_LOAD_IN_PROGRESS:
                    await get_tree().process_frame
                if not is_inside_tree():
                    return
        if ResourceLoader.load_threaded_get_status(level_path) != ResourceLoader.THREAD_LOAD_LOADED:
            fail("The manor scene failed to load.")
            return
        packed = ResourceLoader.load_threaded_get(level_path) as PackedScene
        warmup.release()
        restore_frame_limit()
    if packed == null:
        fail("The generated manor scene could not be opened.")
        return
    if content_cache != null:
        content_cache.call("retain_level", level_path, packed)
    stage("resource_ready", "Preparing the manor...")
    level = packed.instantiate() as Node3D
    if level == null:
        fail("The generated manor scene is invalid.")
        return
    bind_source_camera_controller_state()
    stage("instantiated", "Starting the opening..." if new_game else "Restoring saved game...")
    actor = SAVE_STATE.resolve_player(level)
    brain = actor.get_node_or_null("Brain") if actor != null else null
    if str(selected_save.get("source", "")) == SAVE_STATE.TRAVEL_SOURCE:
        await _open_travel_save()
        return
    var source_brains: Variant = level.get_meta("source_actor_brains", {})
    if source_brains is Dictionary and source_brains.has(SAVE_STATE.DUMMY):
        world_brain = level.get_node_or_null(source_brains[SAVE_STATE.DUMMY]) as Node
    if actor == null or brain == null:
        fail("The generated manor is missing Hype or his Brain.")
        return
    if new_game:
        await open_new_game()
        return
    if world_brain == null or not world_brain.get_property_list().any(func(property): return str(property["name"]) == "dsg_8"):
        fail("The generated manor is missing the saved dummy world flag.")
        return
    var gate_brain := SAVE_STATE._source_brain(level, GATE_ACTOR)
    var gate := gate_brain.get_parent() as CharacterBody3D if gate_brain != null else null
    if gate == null or str(gate.get_meta("source_perso", "")) != GATE_ACTOR:
        fail("The generated manor is missing iMGrille_Manoir_Pont or its Brain.")
        return
    var fixture := MANOIR_START.load_fixture()
    if fixture.is_empty():
        fail("The English-save starting state is missing.")
        return
    var startup := SAVE_STARTUP.new()
    var startup_error := startup.hold_before_tree(level)
    if not startup_error.is_empty():
        fail(startup_error)
        return
    add_child(level)
    stage("tree_ready", "Restoring saved game...")
    startup_error = startup.hold_after_tree()
    if not startup_error.is_empty():
        fail(startup_error)
        return
    var fixture_error: String = MANOIR_START.apply(actor, brain, fixture)
    if not fixture_error.is_empty():
        fail(fixture_error)
        return
    var source_error: String = SAVE_STATE.apply(actor, level, selected_save["state"])
    if not source_error.is_empty():
        fail(source_error)
        return
    if str(selected_save["source"]).begins_with(SAVE_STATE.ORIGINAL_SOURCE):
        if StandInReport.reached(actor, "load-placement", "Hype placed at the live-measured manor spot after an original load"): return
        MANOIR_START.place(actor, fixture)
        var candidate_path: String = SAVE_STATE.ORIGINAL_SAVES.path_join(str(selected_save["source"]).get_slice(":", 1))
        selected_save["records"] = SAVE_STATE.apply_source_records(level, candidate_path)
        SAVE_STATE.apply_spell_handles(actor, candidate_path)
        print("Save records applied: ", selected_save["records"])
    stage("restored", "Preparing to play...")
    # The first physics tick enters the saved world before movement starts.
    # A second tick lets CharacterBody3D update native contact before Brain runs.
    await get_tree().physics_frame
    if not is_inside_tree():
        return
    startup.release_physics()
    await get_tree().physics_frame
    if not is_inside_tree():
        return
    stage("physics_ready", "Starting Hype...")
    _locate_player_sector()
    if not restore_source_camera_presentation(false): return
    startup_error = startup.start_brains()
    if not startup_error.is_empty():
        fail(startup_error)
        return
    await get_tree().process_frame
    if not is_inside_tree():
        return
    if not restore_source_camera_presentation(): return
    if not is_instance_valid(brain) or not bool(brain.get("running")) or not str(brain.get("unsupported_failure")).is_empty():
        fail("The restored player behavior stopped before controls became ready.")
        return
    if not actor.is_physics_processing() or not bool(actor.get("source_object_active")):
        fail("The restored player is inactive before controls became ready.")
        return
    if source_get_game_state() != 1:
        fail("The restored world has not entered the source gameplay state.")
        return
    ready_to_play = true
    loading = false
    var loaded_message := _loaded_message()
    stage("playable", loaded_message)
    _finish_initial_transition(true, "")

## Says what was restored and what was not, rather than implying a whole save.
func _loaded_message() -> String:
    if not str(selected_save["source"]).begins_with(SAVE_STATE.ORIGINAL_SOURCE):
        return "Game loaded"
    var coverage: Dictionary = selected_save.get("coverage", {})
    return "Original save: %d records, %d understood, %d unknown, %d applied" % [coverage.get("records", 0), int(coverage.get("verified", 0)) + int(coverage.get("source", 0)), int(coverage.get("unknown", 0)) + int(coverage.get("unmapped", 0)), selected_save.get("records", {}).get("applied", 0)]


## The engine searches for an actor's sector, and Hype's saved sector is empty.
## Without one, fight and pickup contacts against him are rejected. Boundary
## crossings keep it current after this.
func _locate_player_sector() -> void:
    var index := level.get_node_or_null("SourceSectorIndex")
    if index == null or actor.source_current_sector() != null: return
    var found: Dictionary = index.call("resolve_position", actor.global_position)
    var sector := index.call("sector_node", str(found.get("sector_id", ""))) as Node3D
    if sector != null: actor.source_refresh_contact_sector(sector)


## The outgoing Micro died with its level; a map change selects the main actor.
func _bind_main_microphone(main: Node3D, owner_brain: Node) -> void:
    var emitter: GDScript = load("res://addons/astrolabe/audio/emitter.gd")
    # Stand-in: a main actor without an imported Micro listens along its own axes.
    var basis: Variant = emitter.call("microphone_basis_for", main)
    if basis == null and StandInReport.reached(main, "micro-basis", "main actor without an imported Micro listens along its own axes"): return
    emitter.call("switch_micro_perso", owner_brain, null, main, Basis.IDENTITY if basis == null else basis)


## A level outside the manor starts as a fresh arrival, then takes the saved
## player, Fix Brains and pose.
func _open_travel_save() -> void:
    if actor == null or brain == null:
        fail("The saved level has no main actor.")
        return
    add_child(level)
    _transition_snapshot = selected_save["snapshot"]
    var error := _apply_transition_state(actor, level)
    _apply_world_state(level, _transition_snapshot.get("world", {}))
    var main_normal := int(_transition_snapshot.get("main_normal", -1))
    _transition_snapshot = {}
    if not error.is_empty():
        fail(error)
        return
    if main_normal >= 0 and brain != null: brain.call("change_normal", main_normal)
    actor.global_transform = selected_save["pose"]
    _locate_player_sector()
    await get_tree().physics_frame
    if not is_inside_tree(): return
    _bind_main_microphone(actor, brain)
    if not restore_source_camera_presentation(false): return
    await get_tree().process_frame
    if not is_inside_tree(): return
    if not restore_source_camera_presentation(): return
    ready_to_play = true
    loading = false
    stage("playable", "Game loaded")
    _finish_initial_transition(true, "")


func open_new_game() -> void:
    # Use the complete saved scene and its original initial AI. No save fixture,
    # gate lift, AI excerpt, held Brain or authored trigger advances this opening.
    for source_brain in level.find_children("Brain","Node",true,false):
        opening_brains.append(source_brain)
    var paths: Dictionary = level.get_meta("source_actor_nodes",{})
    var camera_actor: Node = level.get_node_or_null(paths["level:/persos/world/0/2"]) if paths.has("level:/persos/world/0/2") else null
    opening_camera = camera_actor.get_node_or_null("SourceCamera") as Camera3D if camera_actor != null else null
    add_child(level)
    _locate_player_sector()
    # The running original starts a new game with spell slot 0, Fire, selected.
    var magick: Resource = actor.get("source_magick")
    if magick != null and int(magick.get("selected_spell_code")) < 0:
        magick.set("selected_spell_code", MANOIR_START.FIRST_SPELL_CODE)
    # The imported viewport registration owns the opening camera. Its native
    # component and compiled AI own every subsequent pose and target change.
    if not is_instance_valid(opening_camera):
        fail("The generated manor is missing its named opening camera.")
        return
    if not restore_source_camera_presentation(): return
    await get_tree().process_frame
    if not is_inside_tree(): return
    if not restore_source_camera_presentation(): return
    ready_to_play = true
    loading = false
    stage("opening","New Game — opening. R restarts; Esc pauses.")
    update_opening_status()
    _finish_initial_transition(true, "")

## The selected source owner and Godot's rendered Camera3D are separate.
## Reclaim presentation after restoring saves and starting compiled Brains;
## target, motion mode and actor pose remain owned by the source camera AI.
## The first pass may precede Brain registration, so readiness checks again
## after the process frame and requires the concrete selected source owner.
func restore_source_camera_presentation(required: bool = true) -> bool:
    var viewport := get_viewport()
    var selected: Camera3D = SOURCE_CAMERA.source_primary_camera(viewport)
    if selected == null:
        if not required: return true
        fail("The manor did not register its primary source camera.")
        return false
    if not is_instance_valid(level) or not level.is_ancestor_of(selected) or not bool(selected.get("source_viewport_registered")) or int(selected.get("source_viewport_context")) != 1:
        fail("The selected source camera does not belong to the active manor viewport.")
        return false
    selected.make_current()
    if viewport.get_camera_3d() != selected:
        fail("The primary source camera could not become the rendered view.")
        return false
    return true

func _finish_initial_transition(ok: bool, message: String) -> void:
    if not _initial_transition_pending:
        return
    _initial_transition_pending = false
    initial_level_transition_finished.emit({"ok": ok, "message": message})

func begin_level_transition() -> bool:
    if not ready_to_play or loading or not is_instance_valid(level) or not is_instance_valid(actor) or not is_instance_valid(brain):
        return false
    if pause_host.has_method("close_pause"):
        pause_host.call("close_pause")
    if inventory_host.has_method("close_inventory"):
        inventory_host.call("close_inventory")
    _transition_snapshot = _capture_transition_state()
    if not str(_transition_snapshot.get("error", "")).is_empty():
        _transition_snapshot.clear()
        return false
    var viewport := get_viewport()
    _transition_camera = viewport.get_camera_3d()
    # Destination _ready publishes its semantic owner before installation can
    # fail. Keep this binding independently of the rendered presentation camera.
    _transition_source_camera_owner = {
        "present": viewport.has_meta(SOURCE_CAMERA.SOURCE_PRIMARY_CAMERA_OWNER),
        "binding": viewport.get_meta(SOURCE_CAMERA.SOURCE_PRIMARY_CAMERA_OWNER, null),
    }
    _transition_brain_states.clear()
    _transition_physics_states.clear()
    for source_brain in level.find_children("Brain", "Node", true, false):
        var was_running := bool(source_brain.get("running"))
        var automatic := bool(source_brain.get("automatic"))
        _transition_brain_states.append({"brain": source_brain, "running": was_running, "automatic": automatic})
        source_brain.set("automatic", false)
        if was_running:
            source_brain.call("stop")
    for body in level.find_children("*", "CharacterBody3D", true, false):
        _transition_physics_states.append({"body": body, "enabled": body.is_physics_processing()})
        body.set_physics_process(false)
    ready_to_play = false
    loading = true
    stage("level_transition", "Loading destination level...")
    return true

func install_level_transition(destination: Node3D, entry: String, camera_entry: String, packed: PackedScene) -> Dictionary:
    if not loading or not is_instance_valid(level) or not is_instance_valid(destination):
        return {"ok": false, "message": "The active game session is no longer available."}
    var new_actor := _destination_actor(destination)
    if new_actor == null:
        return {"ok": false, "message": "The destination level has no authored main actor."}
    var entry_actor: Node3D = new_actor
    if not entry.is_empty():
        entry_actor = _destination_entry(destination, entry)
        if entry_actor == null:
            return {"ok": false, "message": "The destination entry actor %s is missing or ambiguous." % entry}
    var camera_actor: Node3D
    if not camera_entry.is_empty():
        camera_actor = _destination_entry(destination, camera_entry)
        if camera_actor == null:
            return {"ok": false, "message": "The destination camera entry actor %s is missing or ambiguous." % camera_entry}
    var destination_camera := _destination_camera(destination)
    if camera_actor != null and destination_camera == null:
        return {"ok": false, "message": "The destination has no unambiguous primary source camera for its named entry."}
    # Source level initialization copies both named marker matrices before
    # actor startup. Compose native local transforms outside the scene tree so
    # contact and camera _ready callbacks observe the intended entry poses.
    _place_destination_actor(destination, new_actor, entry_actor)
    if camera_actor != null:
        _place_destination_actor(destination, destination_camera.get_parent() as Node3D, camera_actor)
    var new_brain := new_actor.get_node_or_null("Brain")
    if new_brain == null:
        return {"ok": false, "message": "The destination main actor has no Brain."}
    var restore_error := _restore_transition_resources(new_actor, destination, true)
    if not restore_error.is_empty():
        return {"ok": false, "message": restore_error}

    # All destination actors are fresh instances. Suppress each automatic
    # deferred Brain start until Fix state, entry placement and physics are ready.
    var destination_brains: Array[Dictionary] = []
    for source_brain in destination.find_children("Brain", "Node", true, false):
        destination_brains.append({"brain": source_brain, "automatic": bool(source_brain.get("automatic"))})
        source_brain.set("automatic", false)
    var destination_bodies: Array[Dictionary] = []
    for body in destination.find_children("*", "CharacterBody3D", true, false):
        destination_bodies.append({"body": body})

    var previous_level := level
    destination.name = "Level_" + str(destination.get_meta("source_level", "destination"))
    if new_actor is CharacterBody3D:
        new_actor.velocity = Vector3.ZERO
        new_actor.set_physics_process(false)
    new_brain.set("automatic", false)
    _bind_source_camera_controller_state(destination)
    add_child(destination)
    for state in destination_bodies:
        state["enabled"] = state["body"].is_physics_processing()
        state["body"].set_physics_process(false)
    restore_error = _apply_transition_state(new_actor, destination)
    if not restore_error.is_empty():
        remove_child(destination)
        destination.queue_free()
        return {"ok": false, "message": restore_error}
    if content_cache != null:
        content_cache.call("retain_level", "res://generated/hype/levels/%s/level.tscn" % str(destination.get_meta("source_level", "")).to_lower(), packed)

    demo_camera.set("target", new_actor)
    if destination_camera != null:
        demo_camera.call("release_transition_view")
        destination_camera.make_current()
    else:
        demo_camera.call("release_transition_view")
        demo_camera.make_current()
    if new_actor is CharacterBody3D:
        new_actor.set_physics_process(true)
    await get_tree().physics_frame
    if not is_inside_tree() or not is_instance_valid(destination):
        return {"ok": false, "message": "The game session ended while preparing the destination."}
    await get_tree().physics_frame
    if not is_inside_tree() or not is_instance_valid(destination):
        return {"ok": false, "message": "The game session ended while preparing the destination."}
    _bind_main_microphone(new_actor, new_brain)
    new_brain.set("automatic", true)
    if not bool(new_brain.call("start")):
        remove_child(destination)
        destination.queue_free()
        return {"ok": false, "message": "The destination main actor's behavior could not start."}

    level = destination
    actor = new_actor as CharacterBody3D
    brain = new_brain
    world_brain = null
    for state in destination_bodies:
        if state["body"] != new_actor:
            state["body"].set_physics_process(bool(state["enabled"]))
    for state in destination_brains:
        var source_brain := state["brain"] as Node
        if source_brain == new_brain: continue
        source_brain.set("automatic", bool(state["automatic"]))
        # Arm inactive automatic actors too. Brain.advance keeps first entry
        # pending until activation; enabling actor processing alone cannot start it.
        if bool(state["automatic"]):
            source_brain.call("start")
    if new_game:
        opening_brains.clear()
        for state in destination_brains: opening_brains.append(state["brain"])
        opening_camera = destination_camera
    ready_to_play = true
    loading = false
    _transition_snapshot.clear()
    _transition_brain_states.clear()
    _transition_physics_states.clear()
    _transition_camera = null
    _transition_source_camera_owner.clear()
    if is_instance_valid(previous_level):
        remove_child(previous_level)
        previous_level.queue_free()
    stage("playable", "Explore %s" % str(level.get_meta("source_level", "the new level")))
    return {"ok": true}

func abort_level_transition() -> void:
    if loading:
        _resume_previous_level()

func _resume_previous_level() -> void:
    # Restore source selection before a restarted outgoing Brain can call the
    # camera API. An absent/null old binding must remove destination ownership.
    var viewport := get_viewport()
    var binding: Variant = _transition_source_camera_owner.get("binding", null)
    if not _transition_source_camera_owner.is_empty():
        if bool(_transition_source_camera_owner.get("present", false)) and binding != null:
            viewport.set_meta(SOURCE_CAMERA.SOURCE_PRIMARY_CAMERA_OWNER, binding)
        elif viewport.has_meta(SOURCE_CAMERA.SOURCE_PRIMARY_CAMERA_OWNER):
            viewport.remove_meta(SOURCE_CAMERA.SOURCE_PRIMARY_CAMERA_OWNER)
    for state in _transition_physics_states:
        var body := state.get("body") as Node
        if is_instance_valid(body):
            body.set_physics_process(bool(state.get("enabled", false)))
    for state in _transition_brain_states:
        var source_brain := state.get("brain") as Node
        if not is_instance_valid(source_brain):
            continue
        source_brain.set("automatic", bool(state.get("automatic", false)))
        if bool(state.get("running", false)):
            source_brain.call("start")
    if is_instance_valid(actor):
        demo_camera.set("target", actor)
        demo_camera.call("release_transition_view")
    if is_instance_valid(_transition_camera): _transition_camera.make_current()
    ready_to_play = true
    loading = false
    _transition_snapshot.clear()
    _transition_brain_states.clear()
    _transition_physics_states.clear()
    _transition_camera = null
    _transition_source_camera_owner.clear()
    stage("playable", "Explore the manor" if str(level.get_meta("source_level", "manoir")) == "manoir" else "Explore %s" % str(level.get_meta("source_level", "the level")))

func _destination_actor(destination: Node3D) -> CharacterBody3D:
    var identity := str(destination.get_meta("source_main_actor", ""))
    var actors: Variant = destination.get_meta("source_actor_nodes", {})
    if identity.is_empty() or not actors is Dictionary or not actors.has(identity):
        return null
    return destination.get_node_or_null(actors[identity]) as CharacterBody3D

func _destination_entry(destination: Node3D, name: String) -> Node3D:
    var entries: Variant = destination.get_meta("source_actor_entries", {})
    if not entries is Dictionary:
        return null
    var candidates: Variant = entries.get(name.to_lower(), [])
    if not candidates is Array or candidates.size() != 1:
        return null
    return destination.get_node_or_null(candidates[0]) as Node3D

func _destination_camera(destination: Node3D) -> Camera3D:
    var selected: Camera3D
    for camera in destination.find_children("SourceCamera", "Camera3D", true, false):
        if not bool(camera.get("initially_current")): continue
        if not camera.get_parent() is Node3D: return null
        if selected != null: return null
        selected = camera as Camera3D
    return selected

func _destination_pose(destination: Node3D, node: Node) -> Transform3D:
    var pose := Transform3D.IDENTITY
    while node != null and node != destination:
        if node is Node3D: pose = node.transform * pose
        node = node.get_parent()
    return pose

func _place_destination_actor(destination: Node3D, target: Node3D, marker: Node3D) -> void:
    var parent_pose := _destination_pose(destination, target.get_parent())
    target.transform = parent_pose.affine_inverse() * _destination_pose(destination, marker)
    # A named marker's imported physical-sector binding supplies the new
    # starting context; entry placement is not a walk through old-sector portals.
    var sector_path: Variant = marker.get("source_contact_sector_path")
    if sector_path is NodePath and not sector_path.is_empty():
        var sector := marker.get_node_or_null(sector_path)
        if sector != null: target.set("source_contact_sector_path", target.get_path_to(sector))

func _capture_transition_state() -> Dictionary:
    var state: Dictionary = {"player": {}, "main_brain": {}, "fix_brains": {}, "error": ""}
    if not is_instance_valid(actor):
        return state
    var inventory: Resource = actor.get("source_inventory")
    var magick: Resource = actor.get("source_magick")
    var hit_points: Resource = actor.get("source_hit_points")
    if inventory != null:
        state["player"]["inventory"] = inventory.call("capture_snapshot")
        if state["player"]["inventory"].is_empty(): state["error"] = "The outgoing inventory state is unsupported."
    if magick != null:
        state["player"]["magick"] = {"current_points": magick.get("current_points"), "maximum_points": magick.get("maximum_points"), "selected_spell_code": magick.get("selected_spell_code")}
    if hit_points != null:
        state["player"]["hit_points"] = hit_points.call("capture_snapshot")
    var armor_catalog: Resource = level.get_meta("source_armor_catalog", null) as Resource
    if armor_catalog != null:
        state["player"]["armor"] = armor_catalog.call("capture_snapshot")
        if state["player"]["armor"].is_empty(): state["error"] = "The outgoing armor state is unsupported."
    state["main_brain"] = _capture_brain_state(brain)
    state["main_normal"] = int(brain.get("current_normal")) if brain != null else -1
    var source_actor_nodes: Variant = level.get_meta("source_actor_nodes", {})
    if source_actor_nodes is Dictionary:
        for identity in source_actor_nodes:
            if not str(identity).begins_with("fix:/persos/"):
                continue
            var source_actor := level.get_node_or_null(source_actor_nodes[identity])
            var source_brain := source_actor.get_node_or_null("Brain") if source_actor != null else null
            if source_brain != null:
                state["fix_brains"][str(identity)] = _capture_brain_state(source_brain)
    return state

## Saves outside the manor have no source record table, so each level actor is
## kept by identity: its designer fields, behavior and pose.
func _capture_world_state() -> Dictionary:
    var world: Dictionary = {}
    var source_actor_nodes: Variant = level.get_meta("source_actor_nodes", {})
    if not source_actor_nodes is Dictionary: return world
    for identity in source_actor_nodes:
        if not str(identity).begins_with("level:/persos/"): continue
        var node := level.get_node_or_null(source_actor_nodes[identity]) as Node3D
        var node_brain := node.get_node_or_null("Brain") if node != null else null
        if node_brain == null or node == actor: continue
        world[identity] = {"brain": _capture_brain_state(node_brain), "normal": int(node_brain.get("current_normal")), "transform": node.global_transform, "references": _capture_references(node_brain, source_actor_nodes)}
    return world

## An init rule's waypoints and actor links are not plain values, and a resumed
## actor never reruns that rule, so they are kept by what they name.
func _capture_references(source_brain: Node, source_actor_nodes: Dictionary) -> Dictionary:
    var references: Dictionary = {}
    for property in source_brain.get_property_list():
        var property_name := str(property.get("name", ""))
        if not property_name.begins_with("dsg_"): continue
        var value: Variant = source_brain.get(property_name)
        if value is Node:
            for identity in source_actor_nodes:
                if level.get_node_or_null(source_actor_nodes[identity]) == value:
                    references[property_name] = {"actor": str(identity)}
                    break
        elif value is Resource and value.get("source_position") != null:
            references[property_name] = {"position": value.get("source_position"), "radius": value.get("radius"), "identity": value.get("source_identity")}
    return references

func _apply_references(target_brain: Node, destination: Node3D, references: Dictionary, source_actor_nodes: Dictionary) -> void:
    for property_name in references:
        var saved: Dictionary = references[property_name]
        if saved.has("actor"):
            if source_actor_nodes.has(saved["actor"]):
                target_brain.set(property_name, destination.get_node_or_null(source_actor_nodes[saved["actor"]]))
            continue
        var waypoint: Resource = load("res://addons/astrolabe/character/waypoint.gd").new()
        waypoint.source_position = saved["position"]
        waypoint.radius = saved["radius"]
        waypoint.source_identity = saved["identity"]
        waypoint.supported = true
        target_brain.set(property_name, waypoint)

func _apply_world_state(destination: Node3D, world: Dictionary) -> void:
    var source_actor_nodes: Variant = destination.get_meta("source_actor_nodes", {})
    if not source_actor_nodes is Dictionary: return
    for identity in world:
        if not source_actor_nodes.has(identity): continue
        var node := destination.get_node_or_null(source_actor_nodes[identity]) as Node3D
        var node_brain := node.get_node_or_null("Brain") if node != null else null
        if node_brain == null: continue
        var saved: Dictionary = world[identity]
        _apply_brain_state(node_brain, saved["brain"])
        _apply_references(node_brain, destination, saved.get("references", {}), source_actor_nodes)
        node.global_transform = saved["transform"]
        if int(saved["normal"]) >= 0: node_brain.call("change_normal", int(saved["normal"]))

func _capture_brain_state(source_brain: Node) -> Dictionary:
    var values: Dictionary = {}
    if source_brain == null:
        return {}
    for property in source_brain.get_property_list():
        var property_name := str(property.get("name", ""))
        if not property_name.begins_with("dsg_"):
            continue
        var value: Variant = source_brain.get(property_name)
        if _is_transferable_value(value):
            values[property_name] = value.duplicate(true) if value is Array or value is Dictionary else value
    return {"model": str(source_brain.get("source_model_path")), "values": values}

func _restore_transition_resources(new_actor: CharacterBody3D, destination: Node3D, validate_only: bool) -> String:
    if not _transition_snapshot.has("player"):
        return ""
    var player: Dictionary = _transition_snapshot["player"]
    for component in [{"key": "inventory", "property": "source_inventory"}, {"key": "hit_points", "property": "source_hit_points"}]:
        var key: String = component["key"]
        if not player.has(key): continue
        # Inventory is level-owned and is bound to the actor in _enter_tree.
        # Preflight runs before that binding; use the same saved native owner.
        var target: Resource = destination.get_meta("source_inventory", null) as Resource if key == "inventory" else new_actor.get(component["property"])
        if target == null or not target.has_method("apply_snapshot"):
            return "The destination actor has no native %s state." % key
        if validate_only:
            # Inventory owns nested definitions explicitly; generic Resource
            # duplication is not its ownership contract. Life has scalar fields.
            target = target.call("clone_for_scene") if target.has_method("clone_for_scene") else target.duplicate(false)
            if target == null: return "The destination %s state cannot be cloned." % key
        var error: String = target.call("apply_snapshot", player[key])
        if not error.is_empty(): return error
    if player.has("armor"):
        var armor: Resource = destination.get_meta("source_armor_state", null) as Resource
        if armor == null or not armor.has_method("restore_snapshot"):
            return "The destination level has no shared native armor state."
        if validate_only:
            var catalog: Resource = armor.get("catalog")
            if catalog == null or not catalog.has_method("clone_for_scene"):
                return "The destination armor catalogue cannot be cloned."
            var owned_catalog: Resource = catalog.call("clone_for_scene")
            if owned_catalog == null: return "The destination armor catalogue cannot be cloned."
            armor = armor.duplicate(false)
            armor.set("catalog", owned_catalog)
        var armor_error: String = armor.call("restore_snapshot", player["armor"])
        if not armor_error.is_empty(): return armor_error
    var magick: Resource = destination.get_meta("source_magick", null) as Resource
    if player.has("magick"):
        if magick == null: return "The destination actor has no native magick state."
        if validate_only: magick = magick.duplicate(false)
        for key in player["magick"]:
            magick.set(key, player["magick"][key])
    return ""

func _apply_transition_state(new_actor: CharacterBody3D, destination: Node3D) -> String:
    var error := _restore_transition_resources(new_actor, destination, false)
    if not error.is_empty(): return error
    var new_brain := new_actor.get_node_or_null("Brain")
    _apply_brain_state(new_brain, _transition_snapshot.get("main_brain", {}))
    var fix_states: Dictionary = _transition_snapshot.get("fix_brains", {})
    var source_actor_nodes: Variant = destination.get_meta("source_actor_nodes", {})
    if source_actor_nodes is Dictionary:
        for identity in fix_states:
            if not source_actor_nodes.has(identity):
                continue
            var target_actor := destination.get_node_or_null(source_actor_nodes[identity])
            var target_brain := target_actor.get_node_or_null("Brain") if target_actor != null else null
            _apply_brain_state(target_brain, fix_states[identity])
    return ""

func _apply_brain_state(target_brain: Node, saved: Dictionary) -> void:
    if target_brain == null or saved.is_empty() or str(target_brain.get("source_model_path")) != str(saved.get("model", "")):
        return
    for property_name in saved.get("values", {}):
        if _has_property(target_brain, str(property_name)):
            var value: Variant = saved["values"][property_name]
            target_brain.set(property_name, value.duplicate(true) if value is Array or value is Dictionary else value)

func _has_property(object: Object, property_name: String) -> bool:
    return object.get_property_list().any(func(property): return str(property.get("name")) == property_name)

func _is_transferable_value(value: Variant) -> bool:
    var value_type := typeof(value)
    if value_type in [TYPE_NIL, TYPE_BOOL, TYPE_INT, TYPE_FLOAT, TYPE_STRING, TYPE_STRING_NAME,
        TYPE_VECTOR2, TYPE_VECTOR2I, TYPE_RECT2, TYPE_RECT2I, TYPE_VECTOR3, TYPE_VECTOR3I,
        TYPE_TRANSFORM2D, TYPE_VECTOR4, TYPE_VECTOR4I, TYPE_PLANE, TYPE_QUATERNION,
        TYPE_AABB, TYPE_BASIS, TYPE_TRANSFORM3D, TYPE_PROJECTION, TYPE_COLOR,
        TYPE_PACKED_BYTE_ARRAY, TYPE_PACKED_INT32_ARRAY, TYPE_PACKED_INT64_ARRAY,
        TYPE_PACKED_FLOAT32_ARRAY, TYPE_PACKED_FLOAT64_ARRAY, TYPE_PACKED_STRING_ARRAY,
        TYPE_PACKED_VECTOR2_ARRAY, TYPE_PACKED_VECTOR3_ARRAY, TYPE_PACKED_COLOR_ARRAY]:
        return true
    if value is Array:
        for item in value:
            if not _is_transferable_value(item): return false
        return true
    if value is Dictionary:
        for key in value:
            if not _is_transferable_value(key) or not _is_transferable_value(value[key]): return false
        return true
    return false

func update_opening_status() -> void:
    for source_brain in opening_brains:
        if not is_instance_valid(source_brain): continue
        var failure := str(source_brain.get("unsupported_failure"))
        if not failure.is_empty(): opening_failures[str(source_brain.get_path())] = failure
    if not opening_failures.is_empty():
        status_label.text = ("Opening complete; %d actor behaviors stopped." if opening_finished else "Opening incomplete: %d actor behaviors stopped. R restarts; Esc pauses.") % opening_failures.size()
    if opening_finished or not is_instance_valid(opening_camera): return
    var paths: Dictionary = level.get_meta("source_actor_nodes", {})
    var global_actor := level.get_node_or_null(paths.get("fix:/persos/15", NodePath()))
    var global_brain := global_actor.get_node_or_null("Brain") if global_actor != null else null
    var ui := level.get_node_or_null("AstrolabeUI")
    if global_brain == null or ui == null: return
    var adopted: Node = ui.get("adopted_screen")
    if is_instance_valid(adopted): ui = adopted
    # Source camera selector -1 restores the imported preset, normally mode0.
    if bool(global_brain.get("dsg_147")) and bool(brain.get("running")) and int(brain.get("current_normal")) == 4 and int(brain.get("current_reflex")) == 3 and not bool(ui.get("widescreen")) and int(opening_camera.get("motion_mode")) == int(opening_camera.get("initial_motion_mode")) and opening_camera.get("targeted_actor") == actor:
        opening_finished = true
        stage("playable", "Opening complete — explore the manor" if opening_failures.is_empty() else "Opening complete; %d actor behaviors stopped." % opening_failures.size())

func save_game() -> void:
    if source_vignette_pending or (new_game and not bool(get("opening_finished"))) or not ready_to_play or not is_instance_valid(actor):
        return
    var error: String
    if str(level.get_meta("source_level_name", "")) == "manoir":
        error = SAVE_STATE.write(actor, level)
    else:
        var snapshot := _capture_transition_state()
        snapshot["world"] = _capture_world_state()
        error = SAVE_STATE.write_travel(str(level.get_meta("source_level_name")), snapshot, actor.global_transform)
    status_label.text = "Game saved" if error.is_empty() else error

func stage(name: String, message: String) -> void:
    loading_stage = name
    status_label.text = message
    var elapsed_ms := (Time.get_ticks_usec() - load_started_usec) / 1000.0
    load_timings_ms[name] = elapsed_ms
    loading_stage_changed.emit(name, elapsed_ms)
    if OS.get_environment("ASTROLABE_PROFILE_LOADING") == "1":
        print("GAME_LOAD ", JSON.stringify({"stage": name, "elapsed_ms": elapsed_ms, "static_memory_bytes": OS.get_static_memory_usage()}))

func limit_loading_frames() -> void:
    if Engine.max_fps == 0 or Engine.max_fps > 30:
        saved_max_fps = Engine.max_fps
        Engine.max_fps = 30

func restore_frame_limit() -> void:
    if saved_max_fps >= 0:
        Engine.max_fps = saved_max_fps
        saved_max_fps = -1

func _process(delta: float) -> void:
    if not ready_to_play or not is_instance_valid(actor) or not is_instance_valid(brain):
        return
    if new_game:
        update_opening_status()
        if not opening_finished: return
    var magick: Resource = actor.get("source_magick")
    if magick != null:
        mana_label.text = "Fire  %d / %d mana" % [int(magick.get("current_points")), int(magick.get("maximum_points"))]
    var inventory: Resource = actor.get("source_inventory")
    if inventory != null and int(inventory.get("selected_weapon")) == 80:
        mana_label.text = "Crossbow  %d bolts" % int(inventory.call("quantity", 80))
    var aiming := inventory != null and int(inventory.get("selected_weapon")) == 80 and Input.is_action_pressed("BT_Camera_Look")
    if aiming != crossbow_aim_mode:
        set_crossbow_aim_mode(aiming)
    if crossbow_aim_mode:
        if Input.is_action_pressed("BT_Camera_LookUp"): crossbow_aim_pitch = minf(crossbow_aim_pitch + 45.0 * delta, 35.0)
        if Input.is_action_pressed("BT_Camera_LookDown"): crossbow_aim_pitch = maxf(crossbow_aim_pitch - 45.0 * delta, -35.0)
        demo_camera.crossbow_aim_pitch_degrees = crossbow_aim_pitch
        update_crossbow_aim_target()
    elif inventory != null and int(inventory.get("selected_weapon")) == 80:
        # Not aiming: the pointer still needs a target for the bolt's guard; point it
        # straight ahead of Hype at his own height so the bolt keeps his sighting.
        actor.call("source_set_laser_pointer_target", actor.global_position + actor.global_basis.z * 30.0)
    var failure := str(brain.get("unsupported_failure"))
    if not failure.is_empty():
        status_label.text = "This behavior is not supported yet. Press R to reload."

func configure_crossbow_inputs() -> void:
    for action in ["BT_Camera_Look", "BT_Camera_LookLeft", "BT_Camera_LookRight", "BT_Camera_LookUp", "BT_Camera_LookDown", "BT_Camera_TireArbalete"]:
        if not InputMap.has_action(action): InputMap.add_action(action)
    var preferences := get_node_or_null("/root/UserSettings")
    # The Controls page's look binding (source default Numpad0) owns freelook.
    if preferences == null or preferences.call("get_control_bindings") == null:
        bind_physical_key("BT_Camera_Look", KEY_KP_0)
        bind_physical_key("BT_Camera_Look", KEY_INSERT)
    if preferences != null:
        preferences.call("configure_gameplay_inputs", false)

func bind_physical_key(action: StringName, key: Key) -> void:
    if not InputMap.has_action(action): InputMap.add_action(action)
    var event := InputEventKey.new()
    event.physical_keycode = key
    if not InputMap.action_has_event(action, event): InputMap.action_add_event(action, event)

func set_crossbow_aim_mode(enabled: bool) -> void:
    crossbow_aim_mode = enabled
    var preferences := get_node_or_null("/root/UserSettings")
    if preferences != null:
        preferences.call("configure_gameplay_inputs", enabled)
    if not enabled: crossbow_aim_pitch = 0.0

func update_crossbow_aim_target() -> void:
    if not is_instance_valid(actor): return
    var viewport := get_viewport()
    var presentation := viewport.get_camera_3d()
    if not is_instance_valid(presentation): return
    var center := viewport.get_visible_rect().size * 0.5
    var origin := presentation.project_ray_origin(center)
    var direction := presentation.project_ray_normal(center)
    var query := PhysicsRayQueryParameters3D.create(origin, origin + direction * 2000.0, 1)
    query.exclude = [actor.get_rid()]
    var hit := actor.get_world_3d().direct_space_state.intersect_ray(query)
    var target := hit["position"] as Vector3 if not hit.is_empty() else origin + direction * 2000.0
    var surface_hit := not hit.is_empty() and origin.distance_to(target) <= 100.0 # source pointer range, 0x420b90
    actor.call("source_set_laser_pointer_target", target, surface_hit)

func _unhandled_key_input(event: InputEvent) -> void:
    if source_vignette_pending:
        get_viewport().set_input_as_handled()
        return
    var key := event as InputEventKey
    if key == null or not key.pressed or key.echo:
        return
    if not ready_to_play:
        get_viewport().set_input_as_handled()
        if key.physical_keycode == KEY_ESCAPE:
            leave_session("res://game/bootstrap.tscn")
        return
    # Port session shortcuts have no source counterpart; a key the player bound
    # on the Controls page belongs to that binding (the original lets F1, F5
    # and R be bound like any named key).
    var preferences := get_node_or_null("/root/UserSettings")
    if preferences != null and preferences.has_method("key_is_bound") and bool(preferences.call("key_is_bound", key)):
        return
    if key.physical_keycode == KEY_F1:
        get_viewport().set_input_as_handled()
        _toggle_session_panel()
    elif key.physical_keycode == KEY_F5:
        get_viewport().set_input_as_handled()
        save_game()
    elif key.physical_keycode == KEY_R:
        get_viewport().set_input_as_handled()
        leave_session("res://game/new_game.tscn" if new_game else "res://game/game_session.tscn")
    elif key.physical_keycode == KEY_ESCAPE and not pause_available():
        # Keep the existing exit route while loading or after a failed entry.
        get_viewport().set_input_as_handled()
        leave_session("res://game/bootstrap.tscn")

func _input(event: InputEvent) -> void:
    if source_vignette_pending:
        get_viewport().set_input_as_handled()
        return
    if source_death_requested:
        var death_key := event as InputEventKey
        if death_key != null and death_key.pressed and not death_key.echo:
            if death_key.physical_keycode == KEY_R:
                leave_session(scene_file_path)
            elif death_key.physical_keycode == KEY_ESCAPE:
                leave_session("res://game/bootstrap.tscn")
        if event is InputEventKey or event is InputEventMouseButton or event is InputEventMouseMotion or event is InputEventJoypadButton or event is InputEventJoypadMotion:
            get_viewport().set_input_as_handled()
        return
    if ready_to_play or not (_initial_transition_pending or loading):
        return
    var key := event as InputEventKey
    if key != null and key.pressed and not key.echo and key.physical_keycode == KEY_ESCAPE:
        get_viewport().set_input_as_handled()
        leave_session("res://game/bootstrap.tscn")
        return
    if event is InputEventKey or event is InputEventMouseButton or event is InputEventMouseMotion or event is InputEventJoypadButton or event is InputEventJoypadMotion:
        get_viewport().set_input_as_handled()

## A level exit that leads somewhere not yet converted ends the game gracefully.
func source_level_not_converted(level_name: String) -> bool:
    get_tree().root.set_meta(LEVEL_END_META, "You reached the end of the converted game: \"%s\" is not converted yet." % level_name)
    leave_session.call_deferred("res://game/bootstrap.tscn")
    return true


func leave_session(destination: String) -> void:
    if destination == "res://game/game_session.tscn" and not selected_save.is_empty():
        get_tree().root.set_meta(SAVE_STATE.LOAD_SOURCE_META, selected_save["source"])
    var transitions := get_node_or_null("/root/LevelTransition")
    if transitions != null and bool(transitions.call("is_transitioning")):
        if not bool(transitions.call("cancel_current_transition")):
            return
    _initial_failure_recovery_cancelled = true
    pause_host.call("close_pause")
    inventory_host.call("close_inventory")
    ready_to_play = false
    if is_instance_valid(brain):
        brain.call("stop")
    if is_instance_valid(actor):
        actor.set_physics_process(false)
    if crossbow_aim_mode: set_crossbow_aim_mode(false)
    for action in InputMap.get_actions():
        if str(action).begins_with("Button_"):
            Input.action_release(action)
    get_tree().change_scene_to_file(destination)

func pause_available() -> bool:
    return not source_vignette_pending and ready_to_play and is_instance_valid(actor) and is_instance_valid(level)

func inventory_available() -> bool:
    if source_vignette_pending or not ready_to_play or not is_instance_valid(actor) or not is_instance_valid(level): return false
    var ui := level.get_node_or_null("AstrolabeUI")
    return ui == null or not ui.get("widescreen")

func _exit_tree() -> void:
    _initial_failure_recovery_cancelled = true
    restore_frame_limit()
    if is_instance_valid(brain):
        brain.call("stop")

func fail(message: String) -> void:
    var initial_failure := _initial_transition_pending
    push_error(message)
    restore_frame_limit()
    if is_instance_valid(brain):
        brain.call("stop")
    if is_instance_valid(actor):
        actor.set_physics_process(false)
    if is_instance_valid(level):
        if level.get_parent() == self:
            remove_child(level)
        level.queue_free()
    level = null
    actor = null
    brain = null
    ready_to_play = false
    loading = false
    stage("failed", message)
    _finish_initial_transition(false, message)
    if initial_failure:
        _initial_failure_recovery_cancelled = false
        get_tree().root.set_meta(INITIAL_ROUTE_ERROR_META, message)
        if not _initial_failure_recovery_started:
            _initial_failure_recovery_started = true
            call_deferred("_return_to_menu_after_initial_failure")

func _show_initial_restore_phase(message: String) -> void:
    var presenter := get_node_or_null(LOADING_SCREEN_PATH)
    if presenter == null or not bool(presenter.get("visible")):
        return
    # ResourceLoader's fraction describes scene dependencies only. Once state
    # restoration begins, stop presenting that fraction as total route progress.
    if presenter.has_method("show_phase"):
        presenter.call("show_phase", message, false)

func _return_to_menu_after_initial_failure() -> void:
    # Capture the persistent service/presenter before yielding. This coroutine
    # belongs to the session, which can leave the tree while it waits for the
    # user to dismiss an initial-load error.
    if not is_inside_tree():
        return
    var scene_tree := get_tree()
    if scene_tree.current_scene != self:
        return
    var transitions := scene_tree.root.get_node_or_null("LevelTransition")
    var presenter := transitions.get_node_or_null("LoadingScreen") if transitions != null else null

    # Let LevelTransition publish the terminal failure before checking the
    # persistent presenter. The captured reference remains safe if this
    # session leaves the active scene tree during the wait.
    await scene_tree.process_frame
    if not _initial_failure_recovery_is_current(scene_tree):
        return
    while is_instance_valid(presenter) and bool(presenter.get("visible")):
        await scene_tree.process_frame
        if not _initial_failure_recovery_is_current(scene_tree):
            return

    var error := scene_tree.change_scene_to_file("res://game/bootstrap.tscn")
    if error != OK and _initial_failure_recovery_is_current(scene_tree):
        _initial_failure_recovery_started = false
        var message := "The game could not be opened. Press Esc to return to the menu."
        stage("failed", message)
        status_label.text = message
        status_label.show()
        push_error("Could not return to the menu after initial load failed: %s" % error_string(error))

func _initial_failure_recovery_is_current(scene_tree: SceneTree) -> bool:
    return is_inside_tree() and not _initial_failure_recovery_cancelled and scene_tree.current_scene == self
