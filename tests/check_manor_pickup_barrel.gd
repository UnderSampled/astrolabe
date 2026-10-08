extends SceneTree
const CONTACT = preload("res://addons/astrolabe/character/contact_pickup.gd")
const BARREL = preload("res://addons/astrolabe/character/explosive_barrel.gd")
const CHARACTER = preload("res://addons/astrolabe/character/character.gd")
const INVENTORY = preload("res://addons/astrolabe/character/inventory.gd")
const ITEM = preload("res://addons/astrolabe/character/inventory_definition.gd")
const STOCK = preload("res://addons/astrolabe/character/inventory_stock.gd")
const BRAIN = preload("res://grant_lifecycle.gd")

func require(ok: bool, message: String) -> bool:
    if not ok:
        push_error(message)
        quit(1)
    return ok

func _initialize() -> void:
    call_deferred("run")

func run() -> void:
    var level := Node3D.new()
    root.add_child(level)
    var player := Node3D.new()
    var pickup := Node3D.new()
    level.add_child(player)
    level.add_child(pickup)
    player.position = Vector3(3, 4, 5)
    player.rotate_z(PI / 2.0)
    pickup.global_position = player.global_position + player.global_basis * Vector3(0.9, 2.2, -0.1)
    if not require(is_equal_approx(CONTACT.distance_z(pickup, player), 2.2), "pickup Z distance ignored target orientation"): return
    if not require(is_equal_approx(CONTACT.distance_xy(pickup, player), sqrt(0.82)), "pickup XY distance used world axes"): return
    pickup.global_position = player.global_position + player.global_basis * Vector3(0, -2.2, 0)
    if not require(is_equal_approx(CONTACT.distance_z(pickup, player), 2.2), "pickup Z distance retained a negative sign"): return
    pickup.rotation = Vector3(0, 0, PI / 2.0)
    var center := pickup.global_position + pickup.global_basis * Vector3(0, 0.4, 0)
    var point := center + pickup.global_basis * Vector3(0, 0.5, 0)
    if not require(CONTACT.is_above(pickup, point, 0.5, center), "above threshold lost inclusive boundary or picking center"): return
    if not require(not CONTACT.is_above(pickup, center, 0, center), "above accepted zero signed distance"): return
    if not require(not CONTACT.is_above(pickup, point, -0.5, center), "negative threshold lost strict upper boundary"): return
    if not require(CONTACT.is_above(pickup, point, -0.6, center), "negative threshold rejected interior positive distance"): return

    var target := Area3D.new()
    target.collision_layer = 4
    target.collision_mask = 0
    target.position = Vector3(10, 0, 0)
    var shape := CollisionShape3D.new()
    var sphere := SphereShape3D.new()
    sphere.radius = 1.0
    shape.shape = sphere
    shape.set_meta("source_collision_identifier", 0x1002)
    target.add_child(shape)
    level.add_child(target)
    await physics_frame
    await physics_frame
    var query := PhysicsShapeQueryParameters3D.new()
    query.shape = sphere
    query.transform = target.global_transform
    query.collision_mask = 4
    query.collide_with_areas = true
    query.collide_with_bodies = false
    var hits := level.get_world_3d().direct_space_state.intersect_shape(query, 8)
    if not require(hits.size() == 1, "native ZDE contact fixture did not intersect exactly one target shape"): return
    var hit: Dictionary = hits[0]
    for mask in [2, 4096, 4098, 0x10001002]:
        for repeat in 3:
            if not require(BARREL.contact_material_matches(hit["collider"], int(hit["shape"]), mask), "barrel mask rejected actual target material"): return
    if not require(not BARREL.material_matches(2, 4098) and not BARREL.material_matches(4096, 4098), "barrel mask accepted only some required bits"): return
    if not require(not BARREL.contact_material_matches(target, int(hit["shape"]), 1), "barrel query retained hard-coded movement mask one"): return
    shape.remove_meta("source_collision_identifier")
    if not require(not BARREL.contact_material_matches(target, int(hit["shape"]), 0), "barrel accepted an absent material for mask zero"): return
    shape.set_meta("source_collision_identifier", 0)
    if not require(BARREL.contact_material_matches(target, int(hit["shape"]), 0), "present material with zero mask must pass"): return
    var other_shape := CollisionShape3D.new()
    other_shape.shape = sphere
    other_shape.position = Vector3(5, 0, 0)
    other_shape.set_meta("source_collision_identifier", 2)
    target.add_child(other_shape)
    if not require(not BARREL.contact_material_matches(target, int(hit["shape"]), 2), "barrel substituted another target shape's material"): return
    if not require(not BARREL.contact_material_matches(target, -1, 2) and not BARREL.contact_material_matches(target, 9999, 2), "barrel accepted an invalid native target shape index"): return

    var inventory := make_inventory()
    var actor := make_drop(level, inventory, "potion")
    var brain: Node = actor.get_node("Brain")
    if not require(brain.start() and inventory.quantity(12) == 1, "compiled grant did not use actor-owned typed stock"): return
    if not require(actor.visible and not actor.is_queued_for_deletion(), "compiled suspension did not retain pickup before KillSelf"): return
    brain.advance(0.01)
    if not require(inventory.quantity(12) == 1, "suspended schedule granted twice"): return
    brain.advance(0.01)
    for repeat in 8:
        brain.advance(0.01)
    if not require(inventory.quantity(12) == 1 and actor.is_queued_for_deletion() and not actor.visible and actor.collision_layer == 0 and not brain.running, "KillSelf failed to stop repeated grant and remove contact"): return
    var wrong := make_drop(level, inventory, "")
    var wrong_brain: Node = wrong.get_node("Brain")
    if not require(wrong_brain.start() and inventory.quantity(12) == 1 and wrong.visible and not wrong.is_queued_for_deletion(), "rejected stock was granted or destroyed"): return
    wrong_brain.stop()

    var snapshot: Dictionary = inventory.capture_snapshot()
    var save := FileAccess.open("res://inventory-snapshot.bin", FileAccess.WRITE)
    if not require(save != null and inventory.validate_snapshot(snapshot).is_empty(), "inventory snapshot validation/write failed"): return
    save.store_var(snapshot)
    save.close()
    var load_save := FileAccess.open("res://inventory-snapshot.bin", FileAccess.READ)
    var reopened: Variant = load_save.get_var()
    load_save.close()
    var restored := make_inventory()
    if not require(restored.apply_snapshot(reopened).is_empty() and restored.quantity(12) == 1, "native inventory snapshot reload lost granted quantity"): return
    restored.definitions[0].quantity = 4
    if not require(inventory.quantity(12) == 1, "inventory restoration leaked state between resource owners"): return

    # Frame five is a strict coordination boundary in the source barrel AI,
    # not an invented fuse or damage timer.
    var animated := CHARACTER.new()
    animated.autoplay = false
    var animation_player := AnimationPlayer.new()
    animation_player.name = "AnimationPlayer"
    var animation := Animation.new()
    animation.length = 1.0
    animation.set_meta("source_fps", 30.0)
    var library := AnimationLibrary.new()
    library.add_animation("source", animation)
    animation_player.add_animation_library("", library)
    animated.add_child(animation_player)
    level.add_child(animated)
    animation_player.play("source")
    animation_player.seek(5.0 / 30.0, true)
    if not require(animated.frame_index == 5 and not animated.frame_index > 5, "native frame five crossed barrel coordination boundary"): return
    animation_player.seek(6.0 / 30.0, true)
    if not require(animated.frame_index == 6 and animated.frame_index > 5, "native frame six did not cross barrel coordination boundary"): return
    animation_player.pause()
    await process_frame
    if not require(not is_instance_valid(actor), "KillSelf did not release the dropped actor"): return
    level.free()
    print("MANOR_PICKUP_BARREL_PRIMITIVES_OK")
    quit(0)

func make_inventory() -> Resource:
    var inventory := INVENTORY.new()
    var item := ITEM.new()
    item.source_key = "potion"
    item.item_id = 12
    item.kind = "Object"
    item.maximum = 5
    item.pickup_category = "potions"
    inventory.source_initialized = true
    inventory.definitions.append(item)
    inventory.categories = {"potions": [-1]}
    var gold := ITEM.new()
    gold.source_key = "gold"
    gold.maximum = 200
    inventory.gold_definition = gold
    inventory.gold_category = "money"
    return inventory

func make_drop(level: Node3D, inventory: Resource, item_key: String) -> CharacterBody3D:
    var actor := CHARACTER.new()
    actor.autoplay = false
    actor.source_inventory = inventory
    var stock := STOCK.new()
    stock.item_key = item_key
    stock.category_key = "potions"
    stock.count = 1
    actor.set_meta("source_stock", stock)
    var animation_player := AnimationPlayer.new()
    animation_player.name = "AnimationPlayer"
    actor.add_child(animation_player)
    var brain := BRAIN.new()
    brain.name = "Brain"
    brain.automatic = false
    brain.current_normal = 0
    brain.main_actor = actor
    var emitter := Node3D.new()
    emitter.name = "SoundEmitter"
    actor.add_child(emitter)
    actor.add_child(brain)
    level.add_child(actor)
    brain.sound_emitter = emitter
    return actor
