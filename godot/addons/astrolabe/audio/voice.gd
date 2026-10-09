extends Resource
## A ready-to-play native waveform and its Godot playback properties.
@export var stream: AudioStream
@export var gain: float = 1.0
@export var spatial: bool = false
@export var pitchable: bool = true
## Converted plain events publish object coordinates independently of the
## resource's SetSpacable capability. Authored native voices retain spatial.
@export var source_parameter_conversion: bool = false
@export var source_sample: String
## Native interactive music stream identity; empty for ordinary samples.
@export var theme_identity: String
var trace: PackedStringArray = []
