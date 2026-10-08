# Disposable portal-inspection support, independent of collision bodies and gameplay routing.
extends MeshInstance3D
@export var vertices: PackedVector3Array
@export var neighbors: PackedStringArray
@export var surfaces: PackedInt32Array
@export var face_normals: PackedVector3Array

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
# Sector history uses the separate source-directed crossing method below; endpoint
# tolerance parity remains unverified.
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

# Source 00551dd0/00559b90 accepts only a front-to-back crossing: the starting
# point must be on the positive side and the segment direction must point
# through the triangle plane. Sector history uses the first such face.
func first_source_sector_crossing(start: Vector3, end: Vector3) -> Dictionary:
    var a := to_local(start)
    var b := to_local(end)
    var direction := b - a
    if direction.length_squared() < 0.0000000001:
        return {}
    var nearest := 1.0
    var best := {}
    for i in range(0, vertices.size(), 3):
        var face := int(i / 3)
        var v0 := vertices[i]
        var v1 := vertices[i + 1]
        var v2 := vertices[i + 2]
        var normal := face_normals[face] if face_normals.size() == vertices.size() / 3 else (v1 - v0).cross(v2 - v0)
        if normal.length_squared() < 0.0000000001:
            continue
        normal = normal.normalized()
        var start_side := normal.dot(a - v0)
        var direction_side := normal.dot(direction)
        if start_side <= 0.0 or direction_side >= 0.0:
            continue
        var fraction := -start_side / direction_side
        if fraction <= 0.0 or fraction >= nearest or fraction >= 1.0:
            continue
        var point := a + direction * fraction
        if not _point_in_triangle(point, v0, v1, v2):
            continue
        nearest = fraction
        best = {"sector": neighbors[face], "position": to_global(point), "fraction": fraction}
    return best

func _point_in_triangle(point: Vector3, a: Vector3, b: Vector3, c: Vector3) -> bool:
    var ab := b - a
    var ac := c - a
    var ap := point - a
    var d00 := ab.dot(ab)
    var d01 := ab.dot(ac)
    var d11 := ac.dot(ac)
    var d20 := ap.dot(ab)
    var d21 := ap.dot(ac)
    var denominator := d00 * d11 - d01 * d01
    if absf(denominator) < 0.0000000001:
        return false
    var v := (d11 * d20 - d01 * d21) / denominator
    var w := (d00 * d21 - d01 * d20) / denominator
    return v >= 0.0 and w >= 0.0 and v + w <= 1.0
