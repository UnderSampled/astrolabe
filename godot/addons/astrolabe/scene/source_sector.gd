extends Node3D
## One source sector and its routing lists, as the converter read them.

@export var sector_name := ""
@export var virtual_sector := 0
@export var activation := 0
@export var has_boundary := false
@export var environments := PackedStringArray()
## Light asset paths; SectorLighting resolves them to SourceLight nodes.
@export var static_lights := PackedStringArray()
@export var dynamic_lights := PackedStringArray()
@export var persos := PackedStringArray()
## Each entry is {"sector", "level", "unknown"}.
@export var graphics: Array[Dictionary] = []
@export var optional_graphics: Array[Dictionary] = []
@export var collision := PackedStringArray()
@export var activity := PackedStringArray()
## Each entry is {"sector", "volume"}.
@export var sound: Array[Dictionary] = []
## Music on entry, 004480e0. Each entry is {"state", "from", "event"}.
@export var sound_events: Array[Dictionary] = []
@export var unknown := PackedByteArray([0, 0, 0])
