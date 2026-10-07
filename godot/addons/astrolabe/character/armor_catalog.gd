extends Resource
## Shared gameplay state: Fight_ChangeArmor changes the active definition globally.
const DEFINITION = preload("res://addons/astrolabe/character/armor_definition.gd")
const PIECE = preload("res://addons/astrolabe/character/armor_piece.gd")
const SNAPSHOT = preload("res://addons/astrolabe/character/snapshot_values.gd")
@export var definitions: Array[Resource] = []
@export var source_path: String = "fix:/gameplay/armor"
@export var selected: int = -1
## Native randomness belongs to this shared armor state, independently of AI
## callers. Source table/index machinery and its exact sequence are not ported.
var random_source := RandomNumberGenerator.new()

## One mutable graph per active scene; callers in that scene share this copy.
## Artwork remains shared Texture2D content. Explicit array traversal makes
## definition/piece ownership independent of generic deep-duplicate policy.
func clone_for_scene() -> Resource:
    var catalog: Resource = duplicate(false)
    var owned_definitions: Array[Resource] = []
    for source_definition in definitions:
        var definition: DEFINITION = source_definition as DEFINITION
        assert(definition != null, "Armor catalog requires native definitions")
        var owned_definition: DEFINITION = definition.duplicate(false) as DEFINITION
        var owned_pieces: Array[Resource] = []
        for piece in definition.pieces:
            assert(piece != null, "Armor definition requires native pieces")
            var owned_piece: Resource = piece.duplicate(false)
            owned_piece.resource_local_to_scene = true
            owned_pieces.append(owned_piece)
        owned_definition.pieces = owned_pieces
        owned_definition.artwork_sizes = definition.artwork_sizes.duplicate()
        owned_definition.resource_local_to_scene = true
        owned_definitions.append(owned_definition)
    catalog.set("definitions", owned_definitions)
    catalog.resource_local_to_scene = true
    return catalog

func active() -> DEFINITION:
    return definitions[selected] as DEFINITION if selected >= 0 and selected < definitions.size() else null

func select(value: int) -> void:
    var narrowed: int = value & 255
    selected = narrowed if narrowed < definitions.size() else -1
    emit_changed()

func set_resistance(value: int) -> void:
    var definition: DEFINITION = active()
    assert(definition != null, "Fight_SetArmorResistance needs selected source armor")
    definition.set_resistance(value)
    emit_changed()

func repair() -> void:
    var definition: DEFINITION = active()
    assert(definition != null, "Fight_RepairArmor needs selected source armor")
    definition.repair()
    emit_changed()

func resistance() -> int:
    var definition: DEFINITION = active()
    assert(definition != null, "Fight_GetArmorResistance needs selected source armor")
    return definition.resistance_rank

## 0043aa20/0043aa70: mutate one armor piece and return the effective byte damage.
## A null selection bypasses armor. -1 rejects an invalid authored Resource graph.
func damage(value: int) -> int:
    var incoming := value & 255
    if selected == -1: return incoming
    if selected < 0 or selected >= definitions.size(): return -1
    var definition: DEFINITION = active()
    if definition == null: return -1
    var count := definition.pieces.size()
    var rank := definition.resistance_rank
    if count > 255 or rank < 0 or rank >= 5 or definition.break_thresholds.size() != 5 or definition.damage_factors.size() != 5 * (count + 1): return -1
    if definition.minimum_damage < 0 or definition.minimum_damage > 255: return -1
    var unbroken: Array[int] = []
    var broken := 0
    for index in count:
        var piece: Resource = definition.pieces[index]
        if piece == null: return -1
        var state: Variant = piece.get("broken")
        var strikes: Variant = piece.get("strikes")
        if not state is int or state < 0 or state > 1 or not strikes is int or strikes < 0 or strikes > 255: return -1
        if state == 0: unbroken.append(index)
        else: broken += 1
    if broken != definition.broken_count or definition.current_damage_piece < -1 or definition.current_damage_piece >= count: return -1
    # _ftol truncates the float product to a signed 64-bit integer. Validate
    # before mutating pieces; source data uses ordinary finite coefficients.
    for coefficient in definition.damage_factors:
        var product := float(incoming) * coefficient
        if not is_finite(product) or product < -9223372036854775808.0 or product >= 9223372036854775808.0: return -1
    var current_piece := definition.current_damage_piece
    if not unbroken.is_empty():
        if not unbroken.has(current_piece):
            # Rejection sampling over all pieces in the source selects uniformly
            # among intact pieces. Native RNG preserves that choice distribution.
            current_piece = unbroken[random_source.randi_range(0, unbroken.size() - 1)]
        var piece: Resource = definition.pieces[current_piece]
        var strikes := (int(piece.get("strikes")) + 1) & 255
        piece.set("strikes", strikes)
        if strikes > int(definition.break_thresholds[rank]):
            piece.set("broken", 1)
            broken += 1
            current_piece = -1
        piece.emit_changed()
    definition.current_damage_piece = current_piece
    definition.broken_count = broken
    var scaled := float(incoming) * definition.damage_factors[rank * (count + 1) + broken]
    var effective := maxi(int(scaled) & 255, definition.minimum_damage)
    definition.emit_changed()
    emit_changed()
    return effective

## Native checkpoint state only; artwork/factors/thresholds are authored assets.
func capture_snapshot() -> Dictionary:
    var states: Array = []
    for resource in definitions:
        var definition := resource as DEFINITION
        if definition == null: return {}
        var pieces: Array = []
        for resource_piece in definition.pieces:
            var piece := resource_piece as PIECE
            if piece == null: return {}
            pieces.append({"source": piece.get("source_key"), "broken": piece.get("broken"), "strikes": piece.get("strikes")})
        states.append({"source": definition.get("source_key"), "rank": definition.resistance_rank, "broken_count": definition.broken_count,
            "current_piece": definition.current_damage_piece, "pieces": pieces})
    return {"version": 1, "catalog": source_path, "selected": selected, "definitions": states}

static func validate_snapshot_shape(value: Variant) -> String:
    if not SNAPSHOT.keys(value, ["version", "catalog", "selected", "definitions"]) or not SNAPSHOT.integer(value["version"], 1, 1) or value["catalog"] != "fix:/gameplay/armor":
        return "Armor snapshot version or fields are unsupported."
    var states: Variant = value["definitions"]
    if not states is Array or states.size() > 255 or not SNAPSHOT.integer(value["selected"], -1, states.size() - 1):
        return "Armor snapshot selection or definition count is invalid."
    for definition in states:
        if not SNAPSHOT.keys(definition, ["source", "rank", "broken_count", "current_piece", "pieces"]) or not SNAPSHOT.integer(definition["rank"], 0, 4) or not definition["source"] is String or definition["source"].is_empty() or definition["source"].length() > 256:
            return "Armor snapshot resistance is invalid."
        var pieces: Variant = definition["pieces"]
        if not pieces is Array or pieces.size() > 255 or not SNAPSHOT.integer(definition["current_piece"], -1, pieces.size() - 1) or not SNAPSHOT.integer(definition["broken_count"], 0, pieces.size()):
            return "Armor snapshot piece state is invalid."
        var broken := 0
        for piece in pieces:
            if not SNAPSHOT.keys(piece, ["source", "broken", "strikes"]) or not SNAPSHOT.integer(piece["broken"], 0, 1) or not SNAPSHOT.integer(piece["strikes"], 0, 255) or not piece["source"] is String or piece["source"].is_empty() or piece["source"].length() > 256:
                return "Armor snapshot piece damage is invalid."
            broken += int(piece["broken"])
        if broken != int(definition["broken_count"]): return "Armor snapshot broken count differs from its pieces."
    return ""

func validate_snapshot(value: Variant) -> String:
    var error := validate_snapshot_shape(value)
    if not error.is_empty(): return error
    if value["catalog"] != source_path: return "Armor snapshot catalogue binding differs."
    if value["definitions"].size() != definitions.size(): return "Armor snapshot definition binding differs."
    for index in definitions.size():
        var definition: DEFINITION = definitions[index] as DEFINITION
        if definition == null or value["definitions"][index]["source"] != definition.get("source_key") or value["definitions"][index]["pieces"].size() != definition.pieces.size():
            return "Armor snapshot piece binding differs."
        for piece_index in definition.pieces.size():
            var piece := definition.pieces[piece_index] as PIECE
            if piece == null: return "Armor resource has a missing native piece."
            if value["definitions"][index]["pieces"][piece_index]["source"] != piece.get("source_key"):
                return "Armor snapshot ordered piece binding differs."
    return ""

func apply_snapshot(value: Dictionary) -> String:
    var error := validate_snapshot(value)
    if not error.is_empty(): return error
    selected = int(value["selected"])
    for index in definitions.size():
        var definition: DEFINITION = definitions[index] as DEFINITION
        var state: Dictionary = value["definitions"][index]
        definition.resistance_rank = int(state["rank"])
        definition.broken_count = int(state["broken_count"])
        definition.current_damage_piece = int(state["current_piece"])
        for piece_index in definition.pieces.size():
            var piece: Resource = definition.pieces[piece_index]
            var piece_state: Dictionary = state["pieces"][piece_index]
            piece.set("broken", int(piece_state["broken"]))
            piece.set("strikes", int(piece_state["strikes"]))
            piece.emit_changed()
        definition.emit_changed()
    emit_changed()
    return ""
