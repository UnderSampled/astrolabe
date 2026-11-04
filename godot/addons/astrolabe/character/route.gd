extends Resource
## Native route geometry. The exporter resolves source geometry/sharing;
## gameplay consumes the saved curves, never source records or samples.
@export var curve: Curve3D
@export var radii: PackedFloat32Array
@export var closed: bool = false
@export var supported: bool = false
## Selected motion may reach a fixed point before later unsupported links.
@export var point_supported: PackedByteArray
@export var link_supported: PackedByteArray
## Per link: the Bezier or circle-arc link as a native curve from waypoint i
## to its end; empty (no points) for StaticLine.
@export var link_curves: Array[Curve3D] = []
## Per link: its last sample index (the source sampling rate; 0 for StaticLine).
@export var link_rates: PackedByteArray
@export var source_identity: String
