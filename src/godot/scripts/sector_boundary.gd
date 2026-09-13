# Disposable portal-inspection support, independent of collision bodies and gameplay routing.
extends MeshInstance3D
@export var vertices: PackedVector3Array
@export var neighbors: PackedStringArray
@export var surfaces: PackedInt32Array

func build_wireframe():
    var lines = ArrayMesh.new()
    if vertices.is_empty():
        mesh = lines
        return
    var material = StandardMaterial3D.new()
    material.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
    material.albedo_color = Color(0.1, 0.9, 1.0)
    var line_vertices = PackedVector3Array()
    for i in range(0, vertices.size(), 3):
        for edge in [[0, 1], [1, 2], [2, 0]]:
            line_vertices.append(vertices[i + edge[0]])
            line_vertices.append(vertices[i + edge[1]])
    var arrays = []
    arrays.resize(Mesh.ARRAY_MAX)
    arrays[Mesh.ARRAY_VERTEX] = line_vertices
    lines.add_surface_from_arrays(Mesh.PRIMITIVE_LINES, arrays)
    lines.surface_set_material(0, material)
    mesh = lines

# Closest two-sided face crossing, in world coordinates. No scene state is mutated.
# Source one-sided traversal/history and its endpoint tolerances require runtime parity work.
func intersect_segment(start: Vector3, end: Vector3) -> Dictionary:
    var a = to_local(start)
    var b = to_local(end)
    var length = a.distance_to(b)
    if length < 0.000001: return {}
    var best = {}
    var nearest = 1.0
    for i in range(0, vertices.size(), 3):
        var hit = Geometry3D.segment_intersects_triangle(a, b, vertices[i], vertices[i + 1], vertices[i + 2])
        if hit == null: continue
        var fraction = a.distance_to(hit) / length
        if fraction > 0.000001 and fraction < nearest:
            nearest = fraction
            var face = int(i / 3)
            best = {"sector": neighbors[face], "surface": surfaces[face], "fraction": fraction, "position": to_global(hit)}
    return best
