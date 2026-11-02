extends RefCounted
## One save format for original and native saves: a ConfigFile of named keys.
## [restore/...] holds what Load Game applies. Converted originals also keep
## every source record under player/ and level_*/, with unknown/ for records
## not yet understood. See docs/openspace/save-games.md.
const FORMAT := "astrolabe-save"
const HYPE := "restore/hype"
const INVENTORY := "restore/inventory"
const BRAIN := "restore/brain/"
const WORLD := "restore/world/"
const MAGICK := {"magick_current_points": "current_points", "magick_maximum_points": "maximum_points", "magick_selected_spell_code": "selected_spell_code"}
const ACTOR_FIELDS := ["asset", "life", "armor"]


static func _vector(value: Vector3) -> Array:
	return [value.x, value.y, value.z]


static func _rows(basis: Basis) -> Array:
	return [_vector(basis.x), _vector(basis.y), _vector(basis.z)]


## The same dictionary Load validates and applies, from named keys.
static func to_state(config: ConfigFile, actor: String, global: String) -> Dictionary:
	var hype := {"magick": {}, "inventory": {"quantities": {}}}
	var transform := {"position": _vector(config.get_value(HYPE, "position", Vector3.ZERO))}
	if config.has_section_key(HYPE, "basis"):
		transform["basis"] = _rows(config.get_value(HYPE, "basis"))
	if config.has_section_key(HYPE, "position"): hype["transform"] = transform
	for key in MAGICK: hype["magick"][MAGICK[key]] = config.get_value(HYPE, key, 0)
	for key in ACTOR_FIELDS:
		if config.has_section_key(HYPE, key): hype[key] = config.get_value(HYPE, key)
	for key in config.get_section_keys(INVENTORY) if config.has_section(INVENTORY) else []:
		if key.begins_with("item_"):
			hype["inventory"]["quantities"][key.trim_prefix("item_")] = config.get_value(INVENTORY, key)
		else:
			hype["inventory"][key] = config.get_value(INVENTORY, key)
	var actors := {actor: hype}
	var world := {}
	for section in config.get_sections():
		var target: Dictionary = {}
		if section.begins_with(BRAIN):
			target = {}
			actors[section.trim_prefix(BRAIN)] = {"brain": target}
		elif section.begins_with(WORLD):
			world[section.trim_prefix(WORLD)] = target
		else:
			continue
		for key in config.get_section_keys(section): target[key] = config.get_value(section, key)
	var state := {"schema": config.get_value("meta", "state_schema", 1), "level": config.get_value("meta", "level", ""), "actors": actors, "world": world}
	for key in ["checkpoint", "training"]:
		if config.has_section_key("restore", key): state[key] = config.get_value("restore", key)
	return state


static func from_state(state: Dictionary, actor: String) -> ConfigFile:
	var config := ConfigFile.new()
	config.set_value("meta", "format", FORMAT)
	config.set_value("meta", "schema", 1)
	config.set_value("meta", "source", "native")
	config.set_value("meta", "state_schema", state["schema"])
	config.set_value("meta", "level", state["level"])
	var hype: Dictionary = state["actors"][actor]
	var transform: Dictionary = hype["transform"]
	var position: Array = transform["position"]
	config.set_value(HYPE, "position", Vector3(position[0], position[1], position[2]))
	var rows: Array = transform["basis"]
	config.set_value(HYPE, "basis", Basis(Vector3(rows[0][0], rows[0][1], rows[0][2]), Vector3(rows[1][0], rows[1][1], rows[1][2]), Vector3(rows[2][0], rows[2][1], rows[2][2])))
	for key in MAGICK: config.set_value(HYPE, key, hype["magick"][MAGICK[key]])
	for key in ACTOR_FIELDS:
		if hype.has(key): config.set_value(HYPE, key, hype[key])
	var inventory: Dictionary = hype["inventory"]
	for key in inventory:
		if key != "quantities": config.set_value(INVENTORY, key, inventory[key])
	for item in inventory["quantities"]:
		config.set_value(INVENTORY, "item_" + str(item), inventory["quantities"][item])
	for identity in state["actors"]:
		if state["actors"][identity].has("brain"):
			var brain: Dictionary = state["actors"][identity]["brain"]
			for key in brain: config.set_value(BRAIN + identity, key, brain[key])
	for identity in state["world"]:
		var fields: Dictionary = state["world"][identity]
		for key in fields: config.set_value(WORLD + identity, key, fields[key])
	for key in ["checkpoint", "training"]:
		if state.has(key): config.set_value("restore", key, state[key])
	return config


## Returns {status, config} or {status: "missing"/"error", message}.
static func open(path: String) -> Dictionary:
	if not FileAccess.file_exists(path):
		return {"status": "missing"}
	var config := ConfigFile.new()
	if config.load(path) != OK:
		return {"status": "error", "message": "Save cannot be read."}
	if config.get_value("meta", "format", "") != FORMAT:
		return {"status": "error", "message": "Save format is unsupported."}
	return {"status": "loaded", "config": config}
