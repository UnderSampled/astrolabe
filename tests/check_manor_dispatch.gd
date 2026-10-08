extends SceneTree
## Bounded source-program execution with synthetic media/rigs, plus the actual
## authored inventory host and save adapter. This is not a whole-level export.
const CHARACTER = preload("res://addons/astrolabe/character/character.gd")
const INVENTORY = preload("res://addons/astrolabe/character/inventory.gd")
const ITEM = preload("res://addons/astrolabe/character/inventory_definition.gd")
const STOCK = preload("res://addons/astrolabe/character/inventory_stock.gd")
const LIFE = preload("res://addons/astrolabe/character/hit_points.gd")
const ARMOR_CATALOG = preload("res://addons/astrolabe/character/armor_catalog.gd")
const ARMOR = preload("res://addons/astrolabe/character/armor_state.gd")
const EMITTER = preload("res://addons/astrolabe/audio/emitter.gd")
const HOST = preload("res://game/inventory_host.gd")
const SAVE = preload("res://game/save_state.gd")
const ARTWORK = preload("res://addons/astrolabe/ui/pickup_artwork.gd")

class FixtureBrain extends Node:
    var source_model_path := "fix:/ai/models/28"
    var dsg_4 := 0
    var dsg_8 := false
    var dsg_19 := false
    var dsg_20 := 0
    var dsg_21 := 0
    var dsg_22 := 0
    var dsg_23 := 0
    var dsg_28 := 0
    var dsg_32 := 0
    var dsg_56 := 0
    var dsg_97 := 0
    var dsg_119 := 0
    var dsg_149 := false
    var dsg_147 := false
    var dsg_220 := true
    var dsg_227 := true
    var dsg_228 := true
    var current_normal := 4
    var main_actor: Node3D
    func stop(): pass
    func source_is_in_behavior(_group, target): return current_normal == target
    func change_normal(target, _defer = false): current_normal = target; return true

class IncompatibleBarrelBrain extends Node:
    var source_model_path := "level:/ai/models/488"
    var dsg_4 := 0
    var dsg_8 := 1

func check(ok: bool, message: String) -> bool:
    if not ok:
        push_error(message)
        quit(1)
    return ok

func _initialize(): call_deferred("run")

func body(parent: Node3D, label: String) -> CharacterBody3D:
    var actor := CHARACTER.new()
    actor.name = label
    actor.autoplay = false
    var player := AnimationPlayer.new()
    player.name = "AnimationPlayer"
    actor.add_child(player)
    var emitter := EMITTER.new()
    emitter.name = "SoundEmitter"
    actor.add_child(emitter)
    parent.add_child(actor)
    actor.set_process(false)
    actor.set_physics_process(false)
    return actor

func brain(actor: Node, model: String = "fix:/ai/models/28") -> FixtureBrain:
    var result := FixtureBrain.new()
    result.name = "Brain"
    result.source_model_path = model
    actor.add_child(result)
    return result

func zone(actor: Node3D, label: String, layer: int, identifier: int) -> Area3D:
    var area := Area3D.new()
    area.name = label
    area.collision_layer = layer
    area.collision_mask = 0
    area.monitoring = false
    var shape := CollisionShape3D.new()
    shape.name = "Shape"
    shape.shape = SphereShape3D.new()
    shape.shape.radius = 1.0
    shape.set_meta("source_zone", 0)
    shape.set_meta("source_collision_identifier", identifier)
    shape.set_meta("source_contact_material_present", true)
    area.add_child(shape)
    actor.add_child(area)
    return area

func run():
    var session := Node3D.new()
    root.add_child(session)
    current_scene = session
    var level := Node3D.new()
    session.add_child(level)
    var host := HOST.new()
    host.name = "InventoryHost"
    level.add_child(host)
    var image := Image.create(2, 2, false, Image.FORMAT_RGBA8)
    image.fill(Color.WHITE)
    var artwork := ARTWORK.new()
    artwork.images = {12: [ImageTexture.create_from_image(image)]}
    host.pickup_artwork = artwork
    var player := body(level, "Hype")
    var player_brain := brain(player)
    var inventory := INVENTORY.new()
    var item := ITEM.new()
    item.source_key = "potion"
    item.item_id = 12
    item.maximum = 10
    item.kind = "Object"
    item.pickup_category = "potions"
    inventory.definitions.append(item)
    inventory.source_initialized = true
    inventory.categories = {"potions": [-1]}
    var gold := ITEM.new()
    gold.source_key = "gold"
    gold.maximum = 200
    inventory.gold_definition = gold
    inventory.gold_category = "money"
    inventory.gold = 37
    player.source_inventory = inventory
    player.source_magick.current_points = 20
    player.source_magick.maximum_points = 50
    player.source_magick.selected_spell_code = 0
    player.source_hit_points = LIFE.new()
    player.source_hit_points.current = 50
    player.source_hit_points.maximum = 50
    var global_actor := body(level, "Global")
    var global_brain := brain(global_actor)
    var lever := body(level, "Lever")
    var lever_brain := brain(lever)
    var armor := ARMOR.new()
    armor.catalog = ARMOR_CATALOG.new()
    level.set_meta("source_armor_state", armor)
    var barrel := body(level, "Barrel")
    var barrel_brain := brain(barrel, "level:/ai/models/488")
    level.set_meta("source_actor_brains", {SAVE.GLOBAL: level.get_path_to(global_brain), SAVE.DUMMY: level.get_path_to(lever_brain), SAVE.BARRELS[0]: level.get_path_to(barrel_brain)})
    var sector := Node3D.new()
    sector.name = "Sector"
    sector.set_meta("contact_sector_active", true)

    # Instantiate a native template under an authored game session. MainActor,
    # external Brain slots, stock ownership and picking marker must survive.
    var prototype := body(level, "Prototype")
    var template_brain = load("res://generated-binding.gd").new()
    template_brain.name = "Brain"
    template_brain.automatic = false
    prototype.add_child(template_brain)
    var marker := Marker3D.new()
    marker.name = "PickingCenter"
    marker.position = Vector3(0, 0.4, 0)
    prototype.add_child(marker)
    prototype.source_picking_center_path = NodePath("PickingCenter")
    prototype.set_meta("source_stock_bound", true)
    var prototype_stock := STOCK.new()
    prototype_stock.item_key = "potion"
    prototype_stock.category_key = "potions"
    prototype_stock.count = 1
    prototype.set_meta("source_stock", prototype_stock)
    for child in prototype.get_children(): child.owner = prototype
    var packed := PackedScene.new()
    if not check(packed.pack(prototype) == OK and ResourceSaver.save(packed, "res://native-template.tscn") == OK, "native template save failed"): return
    prototype.free()
    var action := Animation.new()
    action.length = 1.0
    var library := AnimationLibrary.new()
    library.add_animation("state_0", action)
    barrel.get_node("AnimationPlayer").add_animation_library("", library)
    barrel_brain.main_actor = player
    if not check(barrel.source_execute_action_and_generate_object("res://native-template.tscn", 0, Vector3.ZERO) == 1, "native generation rejected converted template"): return
    var first := level.get_child(level.get_child_count() - 1) as CharacterBody3D
    if not check(first.get_parent() == level and first.get_node("Brain").main_actor == player and first.get_node("Brain").actor_brain_0 == global_brain, "generated drop bound to barrel/session instead of MainActor/level"): return
    if not check(is_equal_approx(first.source_picking_center().y - first.global_position.y, 0.4), "generated picking-center binding was lost"): return
    if not check(barrel.source_execute_action_and_generate_object("res://native-template.tscn", 0, Vector3.ZERO) == 1, "second native generation failed"): return
    var second := level.get_child(level.get_child_count() - 1) as CharacterBody3D
    if not check(barrel.source_set_stock_count(first, 65535) and first.get_meta("source_stock").count == -1, "stock setter lost actor ownership or signed16 narrowing"): return
    if not check(barrel.source_set_stock_count(first, 9), "stock setter rejected spawned binding"): return
    player.set_meta("source_stock_bound", true)
    if not check(barrel.source_set_stock_count(player, 99), "absent stock should be a valid no-op"): return
    if not check(second.get_meta("source_stock").count == 1, "generated actors share mutable stock"): return
    first.free()
    second.free()
    sector.set_meta("contact_sector_neighbors", [])
    level.add_child(sector)
    barrel.source_contact_sector_path = barrel.get_path_to(sector)
    player.source_contact_sector_path = player.get_path_to(sector)
    zone(barrel, "ZDE", 4, 0)
    var player_zone := zone(player, "ZDE", 4, 4098)
    await physics_frame
    await physics_frame
    if not check(barrel.source_collide_perso_zde_no_with_type_zde(1, 4098), "actual Cond28 dispatcher lost all-bit target material"): return
    if not check(not barrel.source_collide_perso_zde_no_with_type_zde(2, 4098), "actual dispatcher ignored one-based ZDE slot"): return
    player_zone.get_node("Shape").set_meta("source_collision_identifier", 2)
    if not check(not barrel.source_collide_perso_zde_no_with_type_zde(1, 4098), "actual dispatcher accepted partial mask"): return
    player_zone.get_node("Shape").set_meta("source_collision_identifier", 4098)
    player.source_contact_eligible = false
    if not check(not barrel.source_collide_perso_zde_no_with_type_zde(1, 4098), "actual dispatcher ignored StandardGame eligibility"): return
    player.source_contact_eligible = true
    sector.set_meta("contact_sector_active", false)
    if not check(not barrel.source_collide_perso_zde_no_with_type_zde(1, 4098), "actual dispatcher ignored inactive sector"): return
    sector.set_meta("contact_sector_active", true)

    # Run the imported pickup's unmodified Reflex0, from its settled Normal1.
    var pickup := body(level, "Drop")
    pickup.source_inventory = inventory
    var stock := STOCK.new()
    stock.item_key = "potion"
    stock.category_key = "potions"
    stock.count = 1
    pickup.set_meta("source_stock_bound", true)
    pickup.set_meta("source_stock", stock)
    var compiled = load("res://fix__ai_models_1080.gd").new()
    compiled.name = "Brain"
    compiled.automatic = false
    compiled.current_normal = 1
    compiled.current_reflex = 0
    compiled.main_actor = player
    compiled.actor_brain_0 = global_brain
    pickup.add_child(compiled)
    compiled.sound_emitter = pickup.get_node("SoundEmitter")
    if not check(compiled.start() and compiled.unsupported_failure.is_empty(), "imported pickup did not reach grant"): return
    if not check(inventory.quantity(12) == 1 and player_brain.dsg_97 == 12 and pickup.is_queued_for_deletion(), "imported pickup failed grant/kill ownership"): return
    if not check(global_brain.dsg_19 and global_brain.dsg_20 == 1 and host.get_child_count() == 1, "source bookkeeping or actual pickup HUD disconnected"): return
    for tick in 5: compiled.advance(0.016)
    if not check(inventory.quantity(12) == 1, "imported pickup granted repeatedly"): return
    await process_frame

    # Run the imported dummy Normal1. WaitFrames2 precedes source damage10.
    var dummy := body(level, "ExplosionDummy")
    var dummy_brain = load("res://fix__ai_models_671.gd").new()
    dummy_brain.name = "Brain"
    dummy_brain.automatic = false
    dummy_brain.current_normal = 1
    dummy_brain.main_actor = player
    dummy_brain.dsg_7 = barrel
    dummy.add_child(dummy_brain)
    dummy_brain.sound_emitter = dummy.get_node("SoundEmitter")
    if not check(dummy_brain.start(), "imported explosion dummy failed to start"): return
    for tick in 6: dummy_brain.advance(0.016)
    if not check(dummy_brain.unsupported_failure.is_empty() and player.source_hit_points.current == 40 and player_brain.dsg_119 == 10 and player_brain.current_normal == 15 and dummy_brain.dsg_5 == 1, "source dummy damage10/queue completion disconnected"): return

    # Save ownership retains inventory/gold and one placed consumed flag.
    barrel_brain.dsg_8 = true
    var saved := SAVE.capture(player, level)
    if not check(SAVE.validate(saved).is_empty(), "actual game save rejected pickup/barrel state"): return
    inventory.gold = 0
    item.quantity = 0
    inventory.categories = {"potions": [-1]}
    barrel_brain.dsg_8 = false
    global_brain.dsg_220 = false
    global_brain.dsg_227 = false
    global_brain.dsg_228 = false
    if not check(SAVE.apply(player, level, saved).is_empty(), "actual game save restore failed"): return
    if not check(inventory.quantity(12) == 1 and inventory.gold == 37 and barrel_brain.dsg_8 and not barrel.visible, "save lost pickup or per-barrel consumed state"): return
    if not check(global_brain.dsg_220 and global_brain.dsg_227 and global_brain.dsg_228, "save lost per-item tutorial flags"): return
    var invalid := saved.duplicate(true)
    invalid["actors"][SAVE.GLOBAL]["brain"]["dsg_227"] = 1
    if not check(not SAVE.validate(invalid).is_empty(), "save accepted non-Boolean tutorial flag"): return
    if not check(not saved["world"].has("Drop"), "transient drop leaked into native save ownership"): return
    # Newly mapped actor flags must use the merged slot's live Bool preflight,
    # before any magick/inventory publication can partially restore the slot.
    var invalid_brain := IncompatibleBarrelBrain.new()
    barrel.add_child(invalid_brain)
    var bindings: Dictionary = level.get_meta("source_actor_brains")
    var valid_barrel_path: NodePath = bindings[SAVE.BARRELS[0]]
    bindings[SAVE.BARRELS[0]] = level.get_path_to(invalid_brain)
    player.source_magick.current_points = 3
    inventory.gold = 7
    if not check(SAVE.capture(player, level).is_empty(), "capture accepted a non-Boolean placed barrel flag"): return
    if not check(not SAVE.apply(player, level, saved).is_empty() and player.source_magick.current_points == 3 and inventory.gold == 7, "incompatible barrel Brain partially changed save components"): return
    bindings[SAVE.BARRELS[0]] = valid_barrel_path
    invalid_brain.free()
    # Feedback pauses with gameplay while the authored InventoryHost continues
    # to own menu input. Its native tween resumes and frees the transient view.
    if not check(host.source_display_pickup(12, 60, 1.0, 0.0, 320, 240, 32, 32), "timed pickup feedback failed"): return
    var popup: Control = host.get_child(host.get_child_count() - 1)
    paused = true
    await create_timer(0.08).timeout
    if not check(is_instance_valid(popup) and is_equal_approx(popup.modulate.a, 1.0), "pickup feedback ran during inventory/game pause"): return
    paused = false
    await create_timer(0.12).timeout
    if not check(not is_instance_valid(popup), "pickup feedback did not finish after resume"): return
    session.free()
    print("MANOR_ACTUAL_DISPATCH_OK")
    quit(0)
