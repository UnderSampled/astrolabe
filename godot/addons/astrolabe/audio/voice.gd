extends Resource
## A ready-to-play native waveform and its Godot playback properties.
@export var stream: AudioStreamWAV
@export var gain: float = 1.0
@export var spatial: bool = false
@export var pitchable: bool = true
@export var source_sample: String
var trace: PackedStringArray = []
