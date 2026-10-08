extends RefCounted
## Synthetic state with maintained native Resources and actual floor contact.
const MAGICK = preload("res://addons/astrolabe/character/magick.gd")
const INVENTORY = preload("res://addons/astrolabe/character/inventory.gd")
const ITEM = preload("res://addons/astrolabe/character/inventory_definition.gd")
const LIFE = preload("res://addons/astrolabe/character/hit_points.gd")
const ARMOR = preload("res://addons/astrolabe/character/armor_catalog.gd")
const ARMOR_STATE = preload("res://addons/astrolabe/character/armor_state.gd")
const DEFINITION = preload("res://addons/astrolabe/character/armor_definition.gd")
const PIECE = preload("res://addons/astrolabe/character/armor_piece.gd")
const SAVE = preload("res://game/save_state.gd")

class Player extends CharacterBody3D:
    var source_magick: Resource = MAGICK.new()
    var source_inventory: Resource = INVENTORY.new()
    var source_hit_points: Resource = LIFE.new()
    # Exported Hype does not bind this property; armor belongs to the level.
    var source_armor: Resource
    var current_state: int = 47
    var pending_state: int = -1
    var source_airborne: bool = false
    var external_velocity: Vector3 = Vector3.ZERO
    var generated_object: Node

class PlayerBrain extends Node:
    var dsg_11: bool = true
    var current_normal: int = 4
    var current_reflex: int = 3
    var running: bool = true
    var unsupported_failure: String = ""

class GlobalBrain extends Node:
    var dsg_149: bool = false
    var dsg_147: bool = true
    var dsg_220: bool = false
    var dsg_221: bool = false
    var dsg_222: bool = false
    var dsg_223: bool = false
    var dsg_224: bool = false
    var dsg_225: bool = false
    var dsg_226: bool = false
    var dsg_227: bool = false
    var dsg_228: bool = false

class DummyBrain extends Node:
    var dsg_8: bool = false

class BarrelActor extends Node3D:
    func source_set_object_active_state(active: bool): visible = active
    func source_kill(): queue_free()

class BarrelBrain extends Node:
    var source_model_path: String = "level:/ai/models/488"
    var dsg_4: int = 0
    var dsg_8: bool = false

class ArmorObserver extends Node:
    var source_armor: Resource

static func create(root: Node) -> Dictionary:
    var level := Node3D.new()
    level.name = "SyntheticManor"
    level.set_meta("source_level", "manoir")
    root.add_child(level)
    var actor := Player.new()
    # Resolve by semantic identity even when authored names/hierarchy change.
    actor.name = "RenamedPlayer"
    actor.set_meta("source_perso", SAVE.PLAYER)
    level.add_child(actor)
    var player_brain := PlayerBrain.new()
    player_brain.name = "Brain"
    actor.add_child(player_brain)
    var global_actor := Node3D.new()
    global_actor.name = "Global"
    level.add_child(global_actor)
    var global_brain := GlobalBrain.new()
    global_brain.name = "Brain"
    global_actor.add_child(global_brain)
    var dummy_actor := Node3D.new()
    dummy_actor.name = "Dummy"
    level.add_child(dummy_actor)
    var dummy_brain := DummyBrain.new()
    dummy_brain.name = "Brain"
    dummy_actor.add_child(dummy_brain)
    level.set_meta("source_actor_nodes", {SAVE.PLAYER: NodePath("RenamedPlayer")})
    level.set_meta("source_actor_brains", {SAVE.GLOBAL: NodePath("Global/Brain"), SAVE.DUMMY: NodePath("Dummy/Brain")})
    actor.source_magick.current_points = 12
    actor.source_magick.maximum_points = 50
    actor.source_magick.selected_spell_code = 61
    var inventory: Resource = actor.source_inventory
    for index in 2:
        var item: Resource = ITEM.new()
        item.source_key = "fix:/inventory/synthetic/" + str(index)
        item.item_id = 21 + index
        item.kind = "Weapon" if index == 0 else "Object"
        item.pickup_category = "weapons" if index == 0 else "objects"
        item.maximum = 20 + index
        item.quantity = 5 + index
        item.selected = index == 0
        inventory.definitions.append(item)
    inventory.source_initialized = true
    inventory.categories = {"weapons": [0, -1], "objects": [-1]}
    var gold: Resource = ITEM.new()
    gold.source_key = "fix:/inventory/synthetic/gold"
    gold.maximum = 200
    inventory.gold_definition = gold
    inventory.gold_category = "money"
    inventory.gold = 37
    inventory.capture_snapshot()
    var bindings: Dictionary = level.get_meta("source_actor_brains")
    for index in SAVE.BARRELS.size():
        var barrel := BarrelActor.new()
        barrel.name = "Barrel" + str(index)
        barrel.set_meta("source_perso", SAVE.BARRELS[index])
        level.add_child(barrel)
        var brain := BarrelBrain.new()
        brain.name = "Brain"
        barrel.add_child(brain)
        bindings[SAVE.BARRELS[index]] = level.get_path_to(brain)
    actor.source_hit_points.current = 7
    actor.source_hit_points.maximum = 10
    actor.source_hit_points.limit = 12
    var armor: Resource = ARMOR.new()
    for definition_index in 2:
        var definition: Resource = DEFINITION.new()
        definition.source_key = "fix:/armor/synthetic/definition/" + str(definition_index)
        for piece_index in 2:
            var piece: Resource = PIECE.new()
            piece.source_key = definition.source_key + "/piece/" + str(piece_index)
            definition.pieces.append(piece)
        definition.damage_factors.resize(15)
        definition.damage_factors.fill(1.0)
        definition.break_thresholds = PackedByteArray([1, 2, 3, 4, 5])
        armor.definitions.append(definition)
    armor.selected = 0
    var armor_state: Resource = ARMOR_STATE.new()
    armor_state.catalog = armor
    armor_state.selected_index = 0
    level.set_meta("source_armor_catalog", armor)
    level.set_meta("source_armor_state", armor_state)
    var shared_consumer := ArmorObserver.new()
    shared_consumer.name = "ArmorObserver"
    shared_consumer.source_armor = armor
    level.add_child(shared_consumer)
    var floor := StaticBody3D.new()
    level.add_child(floor)
    var floor_shape := CollisionShape3D.new()
    var box := BoxShape3D.new()
    box.size = Vector3(20, 0.2, 20)
    floor_shape.shape = box
    floor_shape.position.y = -0.1
    floor.add_child(floor_shape)
    var actor_shape := CollisionShape3D.new()
    var capsule := CapsuleShape3D.new()
    capsule.radius = 0.25
    capsule.height = 1.0
    actor_shape.shape = capsule
    actor_shape.position.y = 0.5
    actor.add_child(actor_shape)
    return {"actor": actor, "level": level, "global_brain": global_brain, "dummy_brain": dummy_brain}

static func settle(actor: CharacterBody3D, tree: SceneTree) -> void:
    for tick in 3:
        await tree.physics_frame
        actor.velocity = Vector3(0, -1, 0)
        actor.move_and_slide()
    actor.velocity = Vector3.ZERO
