extends SceneTree

const SNAPSHOT = preload("res://addons/astrolabe/character/snapshot_values.gd")
const CATALOG = preload("res://addons/astrolabe/character/armor_catalog.gd")
const DEFINITION = preload("res://addons/astrolabe/character/armor_definition.gd")
const PIECE = preload("res://addons/astrolabe/character/armor_piece.gd")

func _initialize() -> void:
	if not SNAPSHOT.keys({"version": 1, "catalog": "fix:/gameplay/armor"}, ["version", "catalog"]):
		push_error("exact snapshot keys were rejected")
		quit(1)
		return
	if SNAPSHOT.keys({"version": 1, "catalog": "fix:/gameplay/armor", "extra": 0}, ["version", "catalog"]):
		push_error("extra snapshot key was accepted")
		quit(1)
		return
	if SNAPSHOT.integer(1, -1, 1) != true or SNAPSHOT.integer(2, -1, 1):
		push_error("integer range validation differs")
		quit(1)
		return
	# JSON fixtures (english_save_manoir.json) parse integral values as floats, so
	# integral floats are accepted; fractions, non-finite numbers and bools are not.
	if not SNAPSHOT.integer(1.0, 0, 2) or SNAPSHOT.integer(1.5, 0, 2) or SNAPSHOT.integer(NAN, 0, 2) or SNAPSHOT.integer(INF, 0, 2) or SNAPSHOT.integer(true, 0, 1):
		push_error("non-integer snapshot value was accepted")
		quit(1)
		return

	var piece: Resource = PIECE.new()
	piece.set("source_key", "fix:/armor/pieces/0")
	piece.set("broken", 1)
	piece.set("strikes", 2)
	var pieces: Array[Resource] = []
	pieces.append(piece)
	var definition: Resource = DEFINITION.new()
	definition.set("source_key", "fix:/armor/definitions/0")
	definition.set("pieces", pieces)
	var definitions: Array[Resource] = []
	definitions.append(definition)
	var catalog: Resource = CATALOG.new()
	catalog.set("definitions", definitions)
	var copy: Resource = catalog.call("clone_for_scene")
	var copy_definitions: Array[Resource] = copy.get("definitions")
	var copy_definition: Resource = copy_definitions[0]
	var copy_pieces: Array[Resource] = copy_definition.get("pieces")
	var copy_piece: Resource = copy_pieces[0]
	if copy == catalog or copy_definition == definition or copy_piece == piece:
		push_error("catalog clone shared mutable Resources")
		quit(1)
		return
	copy_piece.set("broken", 0)
	if piece.get("broken") != 1 or copy_piece.get("broken") != 0:
		push_error("catalog clone mutation crossed scene ownership")
		quit(1)
		return
	print("SNAPSHOT_VALUES_OK")
	quit(0)
