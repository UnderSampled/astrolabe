extends Node
## Level-owned native bindings. Export resolves these paths before PackedScene
## publication; gameplay never discovers sectors or meshes by walking the level.
## Explicit order survives text-resource dictionary key serialization.
@export var sector_order: PackedStringArray
@export var sector_paths: Dictionary = {}
@export var visual_paths: Dictionary = {}
@export var mesh_paths: Dictionary = {}

var _metadata: Dictionary = {}
var _primitive_geometry: Dictionary = {}
const RAY_START_MARGIN := 0.0001

func sector_node(sector_id: String) -> Node3D:
    var path: Variant = sector_paths.get(sector_id)
    if not path is NodePath or path.is_empty(): return null
    var sector := _bound_node(path) as Node3D
    if sector == null or str(sector.get_meta("source_sector_id", "")) != sector_id:
        return null
    return sector

func sector_data(sector: Node) -> Variant:
    if not is_instance_valid(sector): return null
    var sector_id := str(sector.get_meta("source_sector_id", ""))
    if sector_node(sector_id) != sector or sector.get("activation") == null: return null
    return {"virtual_sector": sector.get("virtual_sector"), "activation": sector.get("activation"), "collision": sector.get("collision"), "unknown": sector.get("unknown"), "boundary": {} if sector.get("has_boundary") else null}

func resolve_position(position: Vector3) -> Dictionary:
    if not position.is_finite(): return {}
    var nearest_distance := INF
    var nearest_sector_id := ""
    var complete := true
    for sector_id in sector_order:
        var sector := sector_node(str(sector_id))
        if sector == null: continue
        var data: Variant = sector_data(sector)
        if not data is Dictionary:
            complete = false
            continue
        if int(data.get("virtual_sector", 0)) != 0 or (int(data.get("activation", 0)) & 4) == 0:
            continue
        if not data.has("boundary"):
            complete = false
            continue
        if data["boundary"] is Dictionary:
            var boundary := sector.get_node_or_null("SectorBoundary") as Node3D
            if boundary == null or not boundary.has_method("source_query_geometry"):
                complete = false
            else:
                var vertices: Variant = boundary.get("vertices")
                if not vertices is PackedVector3Array or vertices.size() % 3 != 0:
                    complete = false
                elif not vertices.is_empty():
                    var geometry := boundary.call("source_query_geometry") as TriangleMesh
                    if geometry == null:
                        complete = false
                    else:
                        var distance := _ray_distance(boundary, geometry, position)
                        if distance >= 0.0 and distance < nearest_distance:
                            nearest_distance = distance
                            nearest_sector_id = str(sector_id)
        var unknown: Variant = data.get("unknown")
        if not unknown is Array or unknown.is_empty():
            complete = false
            continue
        if int(unknown[0]) != 0: continue
        var visuals: Variant = visual_paths.get(sector_id)
        if not visuals is Array:
            complete = false
            continue
        for path in visuals:
            if not path is NodePath:
                complete = false
                continue
            var visual := _bound_node(path) as Node3D
            if visual == null: continue
            var kind := int(visual.get_meta("source_node_kind", -1))
            if kind not in [13, 21] or int(visual.get_meta("source_visual_lod_count", -1)) != 1:
                # Types 1/11 and multi-LOD visuals still lack equivalent GEO.
                complete = false
                continue
            var meshes: Variant = mesh_paths.get(path)
            if not meshes is Array or meshes.is_empty():
                complete = false
                continue
            for mesh_path in meshes:
                if not mesh_path is NodePath:
                    complete = false
                    continue
                var mesh_node := _bound_node(mesh_path) as MeshInstance3D
                if mesh_node == null or mesh_node.mesh == null:
                    complete = false
                    continue
                var mesh := mesh_node.mesh
                var geometry: TriangleMesh
                if mesh is ArrayMesh:
                    # Godot owns the shared ArrayMesh BVH and its invalidation.
                    geometry = (mesh as ArrayMesh).generate_triangle_mesh()
                elif mesh is PrimitiveMesh:
                    var cached := _primitive_query_geometry(mesh as PrimitiveMesh)
                    if not cached["complete"]:
                        complete = false
                        continue
                    geometry = cached["geometry"] as TriangleMesh
                else:
                    complete = false
                    continue
                if geometry == null:
                    # A valid mesh without triangle surfaces is an empty GEO.
                    continue
                var distance := _ray_distance(mesh_node, geometry, position)
                if distance >= 0.0 and distance < nearest_distance:
                    nearest_distance = distance
                    nearest_sector_id = str(sector_id)
    return {
        "sector_id": nearest_sector_id,
        "distance": nearest_distance,
        "fraction": nearest_distance,
        "complete": complete,
    }

func _primitive_query_geometry(mesh: PrimitiveMesh) -> Dictionary:
    var mesh_id := mesh.get_instance_id()
    if _primitive_geometry.has(mesh_id):
        return _primitive_geometry[mesh_id]
    # PrimitiveMesh has get_faces(), but no script-bound generate_triangle_mesh().
    # Extract faces only on a cache miss; all ray traversal remains native.
    var faces := mesh.get_faces()
    var geometry: TriangleMesh
    var complete := faces.size() % 3 == 0
    if complete and not faces.is_empty():
        geometry = TriangleMesh.new()
        if not geometry.create_from_faces(faces):
            geometry = null
            complete = false
    # Retain the resource as well as its nullable result: IDs cannot be reused,
    # and empty/failed geometry is not rebuilt for every camera query. Bind only
    # the ID so the resource's signal cannot retain itself through its callable.
    var invalidate := _invalidate_primitive_geometry.bind(mesh_id)
    var cached := {"mesh": mesh, "geometry": geometry, "complete": complete, "invalidate": invalidate}
    _primitive_geometry[mesh_id] = cached
    # Initial face extraction can generate the primitive and emit changed.
    # Subscribe afterwards so that first generation cannot evict this result.
    mesh.changed.connect(invalidate)
    return cached

func _invalidate_primitive_geometry(mesh_id: int) -> void:
    var cached: Dictionary = _primitive_geometry.get(mesh_id, {})
    if cached.is_empty(): return
    var mesh := cached["mesh"] as PrimitiveMesh
    var invalidate: Callable = cached["invalidate"]
    if is_instance_valid(mesh) and mesh.changed.is_connected(invalidate):
        mesh.changed.disconnect(invalidate)
    _primitive_geometry.erase(mesh_id)

func _bound_node(path: NodePath) -> Node:
    var level := get_parent()
    var node := get_node_or_null(path)
    # No global/live-node caches survive a level transition. Also reject paths
    # reparented outside this level, or nodes already queued for teardown.
    if not is_instance_valid(level) or level.is_queued_for_deletion() or not is_instance_valid(node) or node.is_queued_for_deletion():
        return null
    return node if level.is_ancestor_of(node) else null

func _ray_distance(node: Node3D, geometry: TriangleMesh, position: Vector3) -> float:
    if absf(node.global_basis.determinant()) < 0.0000000001: return -1.0
    var origin := node.to_local(position)
    var direction := node.to_local(position + Vector3.DOWN) - origin
    var length_squared := direction.length_squared()
    if length_squared < 0.0000000001: return -1.0
    # Native rays exclude hits at the start. Move back by a small ray parameter
    # to retain surface-origin classification, then restore the original unit
    # parameter (also valid for nonuniform/scaled/sheared node transforms).
    var hit := geometry.intersect_ray(origin - direction * RAY_START_MARGIN, direction)
    if hit.is_empty(): return -1.0
    var point: Vector3 = hit["position"]
    var distance := (point - origin).dot(direction) / length_squared
    if distance < -0.000001:
        # The start margin can first hit a face just behind the original ray.
        # That face must not hide a valid forward intersection.
        hit = geometry.intersect_ray(origin, direction)
        if hit.is_empty(): return -1.0
        point = hit["position"]
        distance = (point - origin).dot(direction) / length_squared
    return maxf(distance, 0.0) if distance >= -0.000001 else -1.0

func _exit_tree() -> void:
    for mesh_id in _primitive_geometry.keys():
        _invalidate_primitive_geometry(int(mesh_id))
    _metadata.clear()
