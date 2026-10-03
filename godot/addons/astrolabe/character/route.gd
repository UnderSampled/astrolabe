extends Resource
## Native fixed route geometry. The exporter resolves source geometry/sharing;
## gameplay consumes the saved Curve3D, never source link records or samples.
@export var curve: Curve3D
@export var radii: PackedFloat32Array
@export var closed: bool = false
@export var supported: bool = false
@export var source_identity: String
