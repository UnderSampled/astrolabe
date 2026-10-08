extends Resource
## A ready-to-play native waveform and its Godot playback properties.
@export var stream: AudioStream
@export var gain: float = 1.0
@export var spatial: bool = false
@export var pitchable: bool = true
@export var source_sample: String
## Native interactive music stream identity; empty for ordinary samples.
@export var theme_identity: String
var trace: PackedStringArray = []
