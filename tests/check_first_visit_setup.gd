extends SceneTree
const CHARACTER = preload("res://addons/astrolabe/character/character.gd")
const INVENTORY = preload("res://addons/astrolabe/character/inventory.gd")
const ITEM = preload("res://addons/astrolabe/character/inventory_definition.gd")
const STOCK = preload("res://addons/astrolabe/character/inventory_stock.gd")
const NAMES = preload("res://addons/astrolabe/ui/map_names.gd")
const LABEL = preload("res://addons/astrolabe/ui/map_name.gd")
const LIFE = preload("res://addons/astrolabe/character/hit_points.gd")
const MAGIC = preload("res://addons/astrolabe/character/magick.gd")
const BRAIN = preload("res://setup.gd")

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
    var names := NAMES.new()
    names.name = "SourceMapNames"
    for name_id in [7, 8, 7]:
        var entry := LABEL.new()
        entry.name_id = name_id
        names.entries.append(entry)
    level.add_child(names)
    var item := ITEM.new()
    item.source_key = "weapon"
    item.item_id = 10
    item.kind = "Weapon"
    item.maximum = 20
    item.quantity = 2
    item.pickup_category = "weapons"
    var gold := ITEM.new()
    gold.source_key = "gold"
    gold.maximum = 200
    var inventory := INVENTORY.new()
    inventory.source_initialized = true
    inventory.definitions.append(item)
    inventory.gold_definition = gold
    inventory.gold_category = "money"
    inventory.categories = {"weapons": [-1, -1]}
    var stock := STOCK.new()
    stock.category_key = "weapons"
    stock.item_key = "weapon"
    stock.count = 3
    var actor := CHARACTER.new()
    actor.autoplay = false
    actor.source_inventory = inventory
    actor.source_hit_points = LIFE.new()
    actor.source_hit_points.current = 3
    actor.source_hit_points.limit = 4
    actor.source_magick = MAGIC.new()
    actor.set_meta("source_stock", stock)
    var brain := BRAIN.new()
    brain.automatic = false
    brain.current_normal = 0
    brain.main_actor = actor
    var emitter := Node3D.new()
    actor.add_child(emitter)
    brain.sound_emitter = emitter
    actor.add_child(brain)
    level.add_child(actor)
    brain.sound_emitter = emitter
    if not require(brain.start(), "compiled setup did not start"): return
    if not require(brain.unsupported_failure.is_empty() and brain.dsg_0, "compiled setup did not finish"): return
    if not require(actor.source_hit_points.maximum == 10 and actor.source_hit_points.current == 5 and actor.source_hit_points.limit == 4, "byte-valued life setup differs"): return
    if not require(actor.source_magick.maximum_points == 50 and actor.source_magick.current_points == 50 and actor.source_magick.selected_spell_code == -1, "magic setup incorrectly requires a selected spell"): return
    if not require(item.maximum == 8 and item.quantity == 5 and inventory.selected_weapon == 10, "native pickup/capacity/selection differs"): return
    if not require(names.entries[0].active and not names.entries[1].active and names.entries[2].active and actor.visible, "name activation changed the wrong state"): return
    names.activate_name(7, 256)
    if not require(not names.entries[0].active and not names.entries[2].active, "name activation did not narrow the low byte"): return
    inventory.change_maximum_capacity(999, 50)
    if not require(item.maximum == 8, "absent item capacity mutated catalogue"): return
    inventory.change_maximum_capacity(-1, 65535)
    if not require(gold.maximum == -1 and item.quantity == 5, "special capacity narrowing changed quantity"): return
    var clone: Resource = inventory.clone_for_scene()
    if not require(clone.restore_legacy(10, {10: 6}).is_empty() and item.quantity == 5 and clone.quantity(10) == 6, "legacy restoration leaked between scenes"): return
    var snapshot: Dictionary = inventory.capture_snapshot()
    if not require(INVENTORY.validate_snapshot_shape(snapshot).is_empty(), "native inventory snapshot rejected its own special capacity"): return
    actor.source_magick.current_points = 65535
    actor.source_magick.change_maximum(20)
    if not require(actor.source_magick.current_points == 65535 and actor.source_magick.add_points(2) == 65537 and actor.source_magick.current_points == 1, "magic addition lost full sum or word clamp"): return
    brain.stop()
    level.free()
    print("FIRST_VISIT_SETUP_OK")
    quit(0)
