extends Node3D
## One source light. Its Positive and Negative children are native Light3D nodes
## for native lighting; the parity layer reads these fields. Signed colours and
## the separate on and send switches stay as the source holds them.

@export var source_path := ""
@export var source_name := ""
## 1 parallel, 2 omni, 3 spot, 4 ambient, 6 fog; others are not lit by the parity layer.
@export var kind := 0
@export var lit := false
@export var sends := false
@export var cast_shadows := false
@export var creates_shadows := 0
@export var rotating := false
@export var pulsing := false
@export var far := 0.0
@export var near := 0.0
## Spot angles, or fog parameters for kind 6.
@export var angles := Vector2.ZERO
## The spot cone terms read by 004922d0.
@export var rotation_step := 0.0
@export var pulse_step := 0.0
@export var source_color := Color(0, 0, 0, 0)
@export var painting := 0
@export var intensity_min := 0.0
@export var unresolved_dimmer := false
