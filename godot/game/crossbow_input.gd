extends Node
## Authored physical-key and camera presentation routing. Generated Hype AI
## still decides aiming, firing, ammunition, motion and projectile behavior.
const ACTIONS := {
    "Button_Left": KEY_LEFT,
    "Button_Right": KEY_RIGHT,
    "Button_Up": KEY_UP,
    "Button_Down": KEY_DOWN,
    "Button_Hit": KEY_SPACE,
    "BT_Camera_LookLeft": KEY_LEFT,
    "BT_Camera_LookRight": KEY_RIGHT,
    "BT_Camera_LookUp": KEY_UP,
    "BT_Camera_LookDown": KEY_DOWN,
    "BT_Camera_TireArbalete": KEY_SPACE,
}
var aim_mode := false
var aim_pitch := 0.0
var actor: CharacterBody3D
var camera: Camera3D
var original_events: Dictionary = {}
var configured := false

func configure() -> void:
    if configured: return
    configured = true
    var actions := ACTIONS.keys()
    actions.append("BT_Camera_Look")
    for action in actions:
        original_events[action] = InputMap.action_get_events(action) if InputMap.has_action(action) else null
        if not InputMap.has_action(action): InputMap.add_action(action)
    bind_key("BT_Camera_Look", KEY_KP_0)
    bind_key("BT_Camera_Look", KEY_INSERT)
    for action in ["Button_Left", "Button_Right", "Button_Up", "Button_Down", "Button_Hit"]:
        bind_key(action, ACTIONS[action])

func bind_key(action: StringName, key: Key) -> void:
    if not InputMap.has_action(action): InputMap.add_action(action)
    var event := InputEventKey.new()
    event.physical_keycode = key
    if not InputMap.action_has_event(action, event): InputMap.action_add_event(action, event)

func route_key(action: StringName, key: Key, enabled: bool) -> void:
    var event := InputEventKey.new()
    event.physical_keycode = key
    if enabled:
        if not InputMap.action_has_event(action, event): InputMap.action_add_event(action, event)
    elif InputMap.action_has_event(action, event):
        InputMap.action_erase_event(action, event)

func set_mode(enabled: bool) -> void:
    aim_mode = enabled
    var movement := ["Button_Left", "Button_Right", "Button_Up", "Button_Down"]
    var aiming := ["BT_Camera_LookLeft", "BT_Camera_LookRight", "BT_Camera_LookUp", "BT_Camera_LookDown"]
    for index in movement.size():
        route_key(movement[index], ACTIONS[movement[index]], not enabled)
        route_key(aiming[index], ACTIONS[aiming[index]], enabled)
    route_key("Button_Hit", KEY_SPACE, not enabled)
    route_key("BT_Camera_TireArbalete", KEY_SPACE, enabled)
    if not enabled: aim_pitch = 0.0

func update(delta: float, player: CharacterBody3D, view: Camera3D) -> void:
    actor = player
    camera = view
    var inventory: Resource = actor.get("source_inventory")
    var aiming := inventory != null and int(inventory.get("selected_weapon")) == 80 and Input.is_action_pressed("BT_Camera_Look")
    if aiming != aim_mode: set_mode(aiming)
    if not aim_mode: return
    if Input.is_action_pressed("BT_Camera_LookUp"): aim_pitch = minf(aim_pitch + 45.0 * delta, 35.0)
    if Input.is_action_pressed("BT_Camera_LookDown"): aim_pitch = maxf(aim_pitch - 45.0 * delta, -35.0)
    camera.set("crossbow_aim_pitch_degrees", aim_pitch)
    update_target()

func update_target() -> void:
    if not is_instance_valid(actor) or not is_instance_valid(camera): return
    var center := camera.get_viewport().get_visible_rect().size * 0.5
    var origin := camera.project_ray_origin(center)
    var direction := camera.project_ray_normal(center)
    var query := PhysicsRayQueryParameters3D.create(origin, origin + direction * 2000.0, 1)
    query.exclude = [actor.get_rid()]
    var hit := actor.get_world_3d().direct_space_state.intersect_ray(query)
    var target := hit["position"] as Vector3 if not hit.is_empty() else origin + direction * 2000.0
    var surface_hit := not hit.is_empty() and origin.distance_to(target) <= 100.0 # source pointer range, 0x420b90
    actor.call("source_set_laser_pointer_target", target, surface_hit)

func teardown() -> void:
    if not configured: return
    for action in original_events:
        Input.action_release(action)
        if original_events[action] == null:
            InputMap.erase_action(action)
        else:
            InputMap.action_erase_events(action)
            for event in original_events[action]: InputMap.action_add_event(action, event)
    original_events.clear()
    configured = false
    aim_mode = false
    aim_pitch = 0.0
    if is_instance_valid(camera): camera.set("crossbow_aim_pitch_degrees", 0.0)
    actor = null
    camera = null

func _exit_tree() -> void:
    teardown()
