extends Resource
## Native shared inventory state consumed by compiled source AI functions.
## Ordered member Resources own quantity/selection; category slots and replacement
## pairs reference catalogue indices. Public ID dictionaries are first-match views.
const STOCK_SCRIPT = preload("res://addons/astrolabe/character/inventory_stock.gd")
const DEFINITION_SCRIPT = preload("res://addons/astrolabe/character/inventory_definition.gd")
const SNAPSHOT = preload("res://addons/astrolabe/character/snapshot_values.gd")

@export var selected_weapon: int = -1
@export var quantities: Dictionary = {}
@export var definitions: Array[Resource] = []
@export var categories: Dictionary = {}
@export var gold_definition: Resource
@export var gold_category: String = ""
@export var gold: int = 0
@export var selected_object: int = -1
@export var selected_magic: int = -1
@export var source_initialized: bool = false
@export var admission_known: bool = true
## Ownership retained only for sparse legacy saves whose admission is unknown.
@export var legacy_owned_ids: Array[int] = []
var last_failure: String = ""
var admission_failure: String = ""

func clone_for_scene() -> Resource:
    var cloned := get_script().new() as Resource
    cloned.resource_local_to_scene = true
    for field in ["gold_definition", "gold_category", "gold", "selected_weapon", "selected_object", "selected_magic", "source_initialized", "admission_known", "admission_failure"]:
        cloned.set(field, get(field))
    var owned: Array[Resource] = []
    for value in definitions:
        var copy := value.duplicate(false)
        copy.resource_local_to_scene = true
        owned.append(copy)
    cloned.set("definitions", owned)
    if gold_definition != null:
        var special_copy := gold_definition.duplicate(false)
        special_copy.resource_local_to_scene = true
        cloned.set("gold_definition", special_copy)
    cloned.set("categories", categories.duplicate(true))
    cloned.set("quantities", quantities.duplicate(true))
    cloned.set("legacy_owned_ids", legacy_owned_ids.duplicate())
    return cloned

func _definition(item: int) -> DEFINITION_SCRIPT:
    for resource in definitions:
        var value := resource as DEFINITION_SCRIPT
        if value != null and value.item_id == item: return value
    return null

func _member(index: int) -> DEFINITION_SCRIPT:
    if index < 0 or index >= definitions.size(): return null
    return definitions[index] as DEFINITION_SCRIPT

func _admitted_definition(item: int) -> DEFINITION_SCRIPT:
    # The source walks category/slot order and stops each category at its first
    # vacant ordinary slot. The exporter omits the category's header child.
    for slots: Array in categories.values():
        for index in slots:
            if int(index) == -1: break
            var value := _member(int(index))
            if value != null and value.item_id == item: return value
    return null

func _refresh_views() -> void:
    quantities.clear()
    selected_weapon = -1
    selected_object = -1
    selected_magic = -1
    for value: DEFINITION_SCRIPT in definitions:
        if not quantities.has(value.item_id): quantities[value.item_id] = value.quantity
        if not value.selected: continue
        if value.kind == "Weapon" and selected_weapon == -1: selected_weapon = value.item_id
        if value.kind == "Object" and selected_object == -1: selected_object = value.item_id
        if value.kind.begins_with("Magic") and selected_magic == -1: selected_magic = value.item_id

func presentation_quantities() -> Dictionary:
    # Catalogue query quantities include unowned definitions and -1 sentinels.
    # Presentation ownership is admission, never dictionary-key presence.
    if not source_initialized: return quantities.duplicate()
    var owned: Dictionary = {}
    if not admission_known:
        for item in legacy_owned_ids:
            var value := _definition(item)
            if value != null: owned[item] = value.quantity
        return owned
    for slots: Array in categories.values():
        for index in slots:
            var value := _member(int(index))
            if value != null and not owned.has(value.item_id): owned[value.item_id] = value.quantity
    return owned

func _changed() -> void:
    _refresh_views()
    emit_changed()

func _select_if_none(value: DEFINITION_SCRIPT) -> void:
    if value.kind != "Weapon" and not value.kind.begins_with("Magic"): return
    for candidate: DEFINITION_SCRIPT in definitions:
        if candidate.selected and candidate.kind == value.kind: return
    value.selected = true

func _state_error() -> String:
    var sources: Dictionary = {}
    for resource in definitions:
        var value := resource as DEFINITION_SCRIPT
        if value == null or value.source_key.is_empty() or sources.has(value.source_key):
            return "Inventory member identity is malformed."
        sources[value.source_key] = true
        if not SNAPSHOT.integer(value.quantity, -32768, 32767) or not SNAPSHOT.integer(value.maximum, -32768, 32767):
            return "Inventory member quantity or capacity is malformed."
        for pair: Vector2i in value.replacements:
            if pair.x < -1 or pair.x >= definitions.size() or pair.y < -1 or pair.y >= definitions.size():
                return "Inventory replacement leaves its catalogue."
    for category in categories:
        var category_slots: Variant = categories[category]
        if not category is String or category.is_empty() or not category_slots is Array:
            return "Inventory category is malformed."
        for index in category_slots:
            if not SNAPSHOT.integer(index, -1, definitions.size() - 1):
                return "Inventory admission leaves its catalogue."
    return ""

func _failure(message: String) -> int:
    last_failure = message
    return 0

func maximum_capacity(item: int) -> int:
    last_failure = ""
    if not source_initialized:
        _failure("Inventory catalogue is unavailable for capacity queries.")
        return -1
    var identity := _signed32(item)
    var value: DEFINITION_SCRIPT = (gold_definition as DEFINITION_SCRIPT) if identity == -1 else _definition(identity)
    return value.maximum if value != null else -1

func change_weapon(item: int) -> int:
    return _select(item, "Weapon")

func select_object(item: int) -> int:
    return _select(item, "Object")

func _select(item: int, kind: String) -> int:
    last_failure = ""
    if not source_initialized: return _failure("Inventory admission is unavailable for selection.")
    if not admission_known: return _failure(admission_failure)
    var error := _state_error()
    if not error.is_empty(): return _failure(error)
    var value := _admitted_definition(_signed32(item))
    if value == null or value.kind != kind: return 0
    for candidate: DEFINITION_SCRIPT in definitions:
        if candidate.kind == kind: candidate.selected = false
    value.selected = true
    _changed()
    return 1

func add_object(stock: Resource) -> int:
    last_failure = ""
    if stock != null and stock.get_script() != STOCK_SCRIPT: return _failure("Pickup has incompatible native stock values.")
    var pickup := stock as STOCK_SCRIPT
    if pickup == null or pickup.category_key.is_empty() or pickup.item_key.is_empty(): return 0
    if not source_initialized: return _failure("Inventory catalogue is unavailable for this pickup.")
    if not admission_known: return _failure(admission_failure)
    if not SNAPSHOT.integer(pickup.count, -32768, 32767): return _failure("Pickup count is outside its signed source range.")
    if pickup.category_key == gold_category:
        var special := gold_definition as DEFINITION_SCRIPT
        if special == null or pickup.item_key != special.source_key: return 0
        if not _gold_ready(): return 0
        gold = mini(_signed16(gold + pickup.count), special.maximum)
        emit_changed()
        return -1
    if not categories.has(pickup.category_key): return _failure("Pickup category is outside the imported inventory.")
    var error := _state_error()
    if not error.is_empty(): return _failure(error)
    var incoming := -1
    for index in definitions.size():
        if _member(index).source_key == pickup.item_key:
            incoming = index
            break
    if incoming < 0: return _failure("Pickup item is outside the imported inventory.")
    var value := _member(incoming)
    var slots: Array = categories[pickup.category_key]
    # Availability 1 admits to the first free slot, with no quantity/selection
    # effects and no replacement-list traversal (41c7ea..41c84c).
    if value.availability == 1:
        var free := slots.find(-1)
        if free < 0: return 0
        slots[free] = incoming
        _changed()
        return value.item_id
    for slot in slots.size():
        var occupied: int = int(slots[slot])
        if occupied < 0:
            if value.availability != 0 and value.quantity != -1 and pickup.count == 0: return 0
            slots[slot] = incoming
            if slot == 0 and (value.kind == "Weapon" or value.kind.begins_with("Magic")):
                value.selected = true
            break
        # Rules precede the same-object quantity comparison for each occupied
        # slot. Null endpoints are ignored; duplicate numeric IDs remain distinct.
        for pair: Vector2i in value.replacements:
            if pair.x < 0 or pair.y < 0 or occupied != pair.x: continue
            var previous := _member(occupied)
            var replacement := _member(pair.y)
            var was_selected := previous.selected
            previous.selected = false
            if was_selected: replacement.selected = true
            slots[slot] = pair.y
            _select_if_none(replacement)
            _changed()
            return replacement.item_id
        if occupied == incoming: break
    # Source still changes quantity if no free/same/replacement slot was found.
    if value.quantity != -1:
        if value.quantity == value.maximum:
            _changed()
            return 0
        value.quantity = mini(_signed16(value.quantity + pickup.count), value.maximum)
    _changed()
    return value.item_id

func add_actor_object(target: Node) -> int:
    last_failure = ""
    if target == null: return 0
    if target.has_meta("source_stock"):
        var stock: Variant = target.get_meta("source_stock")
        if stock != null and not stock is Resource: return _failure("Pickup actor has incompatible native stock values.")
        return add_object(stock as Resource)
    for property in target.get_property_list():
        if property.name == "source_stock":
            var property_stock: Variant = target.get("source_stock")
            if property_stock != null and not property_stock is Resource: return _failure("Pickup actor has incompatible native stock values.")
            return add_object(property_stock as Resource)
    # A generated plain Global Node can have verified null stock without a
    # property. Godot removes null metadata, so the exporter retains this fact.
    var stock_bound: Variant = target.get_meta("source_stock_bound", false)
    if stock_bound is bool and stock_bound: return 0
    return _failure("Pickup actor has no native stock binding.")

func restore_legacy(weapon: int, amounts: Dictionary) -> String:
    if not source_initialized:
        selected_weapon = weapon
        quantities = amounts.duplicate()
        return ""
    # Sparse ID saves cannot identify distinct members with repeated IDs.
    var seen: Dictionary = {}
    for value: DEFINITION_SCRIPT in definitions:
        if amounts.has(value.item_id) and seen.has(value.item_id):
            return "Legacy inventory cannot resolve repeated source item IDs."
        seen[value.item_id] = true
    var restored: Dictionary = {}
    for key in categories:
        restored[key] = []
        restored[key].resize(categories[key].size())
        restored[key].fill(-1)
    var unresolved := ""
    for index in definitions.size():
        var value := _member(index)
        if not amounts.has(value.item_id) or int(amounts[value.item_id]) == 0: continue
        if value.pickup_category.is_empty() or not restored.has(value.pickup_category):
            unresolved = "Saved inventory admission is unknown: item " + str(value.item_id) + " has no typed pickup category."
            break
        var slots: Array = restored[value.pickup_category]
        var free := slots.find(-1)
        if free < 0:
            unresolved = "Saved inventory admission is unknown: source category slots are insufficient."
            break
        slots[free] = index
    for item in amounts:
        if _definition(int(item)) == null: return "Saved inventory item is outside the source catalogue."
    if weapon >= 0:
        var value := _definition(weapon)
        if value == null or value.kind != "Weapon" or not amounts.has(weapon):
            return "Saved weapon is outside the owned source inventory."
    for value: DEFINITION_SCRIPT in definitions:
        value.quantity = int(amounts.get(value.item_id, value.quantity))
        if value.kind == "Weapon": value.selected = value.item_id == weapon
    admission_known = unresolved.is_empty()
    legacy_owned_ids.clear()
    if not admission_known:
        for item in amounts: legacy_owned_ids.append(int(item))
    admission_failure = unresolved
    if admission_known: categories = restored
    _changed()
    return ""

func capture_snapshot() -> Dictionary:
    if not _state_error().is_empty(): return {}
    if source_initialized: _refresh_views()
    var catalogue: Array = []
    var members: Array = []
    var capacities: Dictionary = {}
    var gold_source := ""
    for value: DEFINITION_SCRIPT in definitions:
        catalogue.append({"id": value.item_id, "source": value.source_key})
        members.append({"source": value.source_key, "quantity": value.quantity, "selected": value.selected, "maximum": value.maximum})
        if not capacities.has(str(value.item_id)): capacities[str(value.item_id)] = value.maximum
    if gold_definition != null:
        var special := gold_definition as DEFINITION_SCRIPT
        if special == null: return {}
        capacities["-1"] = special.maximum
        gold_source = special.source_key
    var amounts: Dictionary = {}
    for item in quantities:
        if not item is int: return {}
        amounts[str(item)] = quantities[item]
    return {"version": 2, "catalog": "fix:/inventory/gameplay", "catalogue": catalogue,
        "member_states": members, "capacities": capacities, "quantities": amounts, "categories": categories.duplicate(true),
        "gold": gold, "gold_source": gold_source, "gold_category": gold_category, "selected_weapon": selected_weapon, "selected_object": selected_object,
        "selected_magic": selected_magic, "source_initialized": source_initialized,
        "admission_known": admission_known, "admission_failure": admission_failure, "legacy_owned_ids": legacy_owned_ids.duplicate()}

static func validate_snapshot_shape(value: Variant) -> String:
    if not SNAPSHOT.keys(value, ["version", "catalog", "catalogue", "member_states", "capacities", "quantities", "categories", "gold", "gold_source", "gold_category", "selected_weapon", "selected_object", "selected_magic", "source_initialized", "admission_known", "admission_failure", "legacy_owned_ids"]):
        return "Inventory snapshot has unsupported fields."
    if not SNAPSHOT.integer(value["version"], 2, 2) or value["catalog"] != "fix:/inventory/gameplay":
        return "Inventory snapshot version or catalogue is unsupported."
    if not value["source_initialized"] is bool or not value["admission_known"] is bool or not value["admission_failure"] is String or value["admission_failure"].length() > 4096:
        return "Inventory snapshot admission state is malformed."
    if not value["admission_known"] and value["admission_failure"].is_empty(): return "Unknown inventory admission needs a diagnostic."
    for field in ["gold_source", "gold_category"]:
        if not value[field] is String or value[field].length() > 256 or (value["source_initialized"] and value[field].is_empty()):
            return "Inventory snapshot special-resource binding is malformed."
    if not SNAPSHOT.integer(value["gold"], -32768, 32767): return "Inventory gold is outside signed source range."
    for field in ["selected_weapon", "selected_object", "selected_magic"]:
        if not SNAPSHOT.integer(value[field], -1, 32767): return "Inventory snapshot selection is malformed."
    var catalogue: Variant = value["catalogue"]
    var members: Variant = value["member_states"]
    if not catalogue is Array or catalogue.size() > 256 or not members is Array or members.size() != catalogue.size():
        return "Inventory snapshot catalogue is unbounded or incomplete."
    var sources: Dictionary = {}
    var first_ids: Dictionary = {}
    var repeated_ids: Dictionary = {}
    for index in catalogue.size():
        var item: Variant = catalogue[index]
        var member: Variant = members[index]
        if not SNAPSHOT.keys(item, ["id", "source"]) or not SNAPSHOT.integer(item["id"], 0, 32767) or not item["source"] is String or item["source"].is_empty() or item["source"].length() > 256:
            return "Inventory snapshot item identity is malformed."
        if sources.has(item["source"]): return "Inventory snapshot repeats a source member."
        sources[item["source"]] = true
        if not SNAPSHOT.keys(member, ["source", "quantity", "selected", "maximum"]) or member["source"] != item["source"] or not member["selected"] is bool or not SNAPSHOT.integer(member["quantity"], -32768, 32767) or not SNAPSHOT.integer(member["maximum"], -32768, 32767):
            return "Inventory snapshot member state is malformed."
        var key := str(int(item["id"]))
        if first_ids.has(key): repeated_ids[key] = true
        else: first_ids[key] = index
    var legacy: Variant = value["legacy_owned_ids"]
    if not legacy is Array or legacy.size() > 256: return "Inventory snapshot legacy ownership is unbounded."
    var legacy_seen: Dictionary = {}
    for item in legacy:
        if not SNAPSHOT.integer(item, 0, 32767) or legacy_seen.has(int(item)) or (value["source_initialized"] and not first_ids.has(str(int(item)))):
            return "Inventory snapshot legacy ownership leaves its catalogue."
        if value["source_initialized"] and repeated_ids.has(str(int(item))):
            return "Inventory snapshot legacy ownership has an ambiguous item ID."
        legacy_seen[int(item)] = true
    if value["admission_known"] and not legacy.is_empty(): return "Known inventory admission cannot have legacy ownership."
    for field in ["capacities", "quantities"]:
        var amounts: Variant = value[field]
        if not amounts is Dictionary or amounts.size() > 257: return "Inventory snapshot amounts are unbounded."
        for item in amounts:
            if not SNAPSHOT.item_key(item) or (field == "quantities" and int(item) < 0) or not SNAPSHOT.integer(amounts[item], -32768, 32767):
                return "Inventory snapshot amount is malformed."
    if value["source_initialized"]:
        if value["quantities"].size() != first_ids.size() or value["capacities"].size() != first_ids.size() + 1 or not value["capacities"].has("-1"):
            return "Inventory snapshot ID views are incomplete."
        for key in first_ids:
            var member: Dictionary = members[first_ids[key]]
            if not value["quantities"].has(key) or int(value["quantities"][key]) != int(member["quantity"]) or not value["capacities"].has(key) or int(value["capacities"][key]) != int(member["maximum"]):
                return "Inventory snapshot ID view differs from its first member."
    var categories: Variant = value["categories"]
    if not categories is Dictionary or categories.size() > 64: return "Inventory snapshot categories are unbounded."
    for category in categories:
        if not category is String or category.is_empty() or category.length() > 256 or not categories[category] is Array or categories[category].size() > 256:
            return "Inventory snapshot category is malformed."
        for item in categories[category]:
            if not SNAPSHOT.integer(item, -1, catalogue.size() - 1): return "Inventory snapshot slot leaves its catalogue."
    return ""

func validate_snapshot(value: Variant) -> String:
    var error := validate_snapshot_shape(value)
    if not error.is_empty(): return error
    if value["source_initialized"] != source_initialized: return "Inventory snapshot source binding differs."
    var expected := capture_snapshot()
    if not validate_snapshot_shape(expected).is_empty() or value["catalogue"].size() != expected["catalogue"].size() or value["capacities"].size() != expected["capacities"].size():
        return "Inventory snapshot item binding differs."
    if value["gold_source"] != expected["gold_source"] or value["gold_category"] != gold_category:
        return "Inventory snapshot special-resource binding differs."
    for index in definitions.size():
        if int(value["catalogue"][index]["id"]) != int(expected["catalogue"][index]["id"]) or value["catalogue"][index]["source"] != expected["catalogue"][index]["source"]:
            return "Inventory snapshot ordered item binding differs."
    for item in expected["capacities"]:
        if not value["capacities"].has(item): return "Inventory snapshot capacity binding differs."
    if value["categories"].size() != categories.size(): return "Inventory snapshot category binding differs."
    for category in categories:
        if not value["categories"].has(category) or value["categories"][category].size() != categories[category].size():
            return "Inventory snapshot slot layout differs."
    if source_initialized:
        var selections := {"selected_weapon": -1, "selected_object": -1, "selected_magic": -1}
        for index in definitions.size():
            if not value["member_states"][index]["selected"]: continue
            var definition := _member(index)
            var field := "selected_weapon" if definition.kind == "Weapon" else ("selected_object" if definition.kind == "Object" else ("selected_magic" if definition.kind.begins_with("Magic") else ""))
            if not field.is_empty() and selections[field] == -1: selections[field] = definition.item_id
        for field in selections:
            if int(value[field]) != selections[field]: return "Inventory snapshot selection differs from its members."
    return ""

func apply_snapshot(value: Dictionary) -> String:
    var error := validate_snapshot(value)
    if not error.is_empty(): return error
    for index in definitions.size():
        var definition := _member(index)
        var member: Dictionary = value["member_states"][index]
        definition.maximum = int(member["maximum"])
        definition.quantity = int(member["quantity"])
        definition.selected = bool(member["selected"])
    if gold_definition != null: gold_definition.set("maximum", int(value["capacities"]["-1"]))
    var amounts: Dictionary = {}
    for item in value["quantities"]: amounts[int(item)] = int(value["quantities"][item])
    quantities = amounts
    var restored_categories: Dictionary = {}
    for category in categories:
        var slots: Array = []
        for item in value["categories"][category]: slots.append(int(item))
        restored_categories[category] = slots
    categories = restored_categories
    for field in ["gold", "selected_weapon", "selected_object", "selected_magic"]: set(field, int(value[field]))
    admission_known = bool(value["admission_known"])
    legacy_owned_ids.clear()
    for item in value["legacy_owned_ids"]: legacy_owned_ids.append(int(item))
    admission_failure = str(value["admission_failure"])
    last_failure = ""
    if source_initialized: _changed()
    else: emit_changed()
    return ""

static func _signed16(value: int) -> int:
    var narrowed := value & 65535
    return narrowed - 65536 if narrowed >= 32768 else narrowed

static func _signed32(value: int) -> int:
    var narrowed := value & 4294967295
    return narrowed - 4294967296 if narrowed >= 2147483648 else narrowed

func _gold_ready() -> bool:
    var definition := gold_definition as DEFINITION_SCRIPT
    if not source_initialized or definition == null or not SNAPSHOT.integer(gold, -32768, 32767) or not SNAPSHOT.integer(definition.maximum, -32768, 32767):
        _failure("Special inventory state is not configured for source gold operations.")
        return false
    return true

func gold_value() -> int:
    last_failure = ""
    return gold if _gold_ready() else 0

func add_gold(value: int) -> int:
    last_failure = ""
    if not _gold_ready(): return 0
    var definition := gold_definition as DEFINITION_SCRIPT
    var amount := _signed16(gold + _signed32(value))
    var clamped := amount > definition.maximum
    gold = definition.maximum if clamped else amount
    return 0 if clamped else 1

func subtract_gold(value: int) -> int:
    last_failure = ""
    if not _gold_ready(): return 0
    var amount := _signed32(value)
    if gold < amount: return 0
    gold = _signed16(gold - amount)
    return 1

func selected_weapon_value() -> int:
    last_failure = ""
    if not source_initialized:
        _failure("Inventory catalogue is unavailable for weapon queries.")
        return -1
    _refresh_views()
    return selected_weapon

func selected_object_value() -> int:
    last_failure = ""
    if not source_initialized:
        _failure("Inventory catalogue is unavailable for Object queries.")
        return -1
    _refresh_views()
    return selected_object

func quantity_source(item: int) -> int:
    last_failure = ""
    if not source_initialized:
        _failure("Inventory catalogue is unavailable for quantity queries.")
        return -1
    var value := _definition(_signed32(item))
    return value.quantity if value != null else -1

func remove_source(item: int, count: int) -> int:
    last_failure = ""
    if not source_initialized: return _failure("Inventory catalogue is unavailable for quantity removal.")
    var requested := _signed32(count)
    var value := _definition(_signed32(item))
    if value == null: return _signed32(-requested)
    var remaining := _signed32(value.quantity - requested)
    if remaining >= 0:
        value.quantity = _signed16(remaining)
        _changed()
    return remaining

func quantity(item: int) -> int:
    if source_initialized: return quantity_source(item)
    if not quantities.has(item): return -1
    return int(quantities[item])
