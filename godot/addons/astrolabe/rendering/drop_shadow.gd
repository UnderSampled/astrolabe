extends MeshInstance3D
## An actor's drop shadow, chosen by its Perso3dData kind as SHW_fn_vDrawShadow
## (004e4e30) does: 1 fake blob (FUN_004e4840), 2 realistic, 3 and 4 placed from
## the lights flagged for shadows. Kinds 2 to 4 use the blob as a stand-in until
## they are built; add a case to _place for each. The texture is a stand-in for
## Fix fixtex/character shadows/fake_sh%d.

const REACH := 1.0
const START_HEIGHT := 0.2
const FLOOR_NORMAL_MINIMUM := 0.3
const HALF_LENGTH := 0.8
const HALF_WIDTH := 0.5
const REALISTIC_RADIUS := 1.2
const ALPHA := 0.55
const LIFT := 0.02

var actor: CollisionObject3D
var kind := 1
var source_scale := Vector2.ONE

static func attach(owner_actor: CollisionObject3D, quality: int, scale_pair: Vector2) -> void:
	var shadow: MeshInstance3D = load("res://addons/astrolabe/rendering/drop_shadow.gd").new()
	shadow.name = "DropShadow"
	shadow.actor = owner_actor
	shadow.kind = quality
	shadow.source_scale = scale_pair
	owner_actor.add_child(shadow)

func _ready() -> void:
	top_level = true
	cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	layers = 2
	var plane := PlaneMesh.new()
	plane.size = Vector2.ONE
	mesh = plane
	var gradient := Gradient.new()
	gradient.set_color(0, Color(0, 0, 0, ALPHA))
	gradient.set_color(1, Color(0, 0, 0, 0))
	var texture := GradientTexture2D.new()
	texture.gradient = gradient
	texture.fill = GradientTexture2D.FILL_RADIAL
	texture.fill_from = Vector2(0.5, 0.5)
	texture.fill_to = Vector2(1.0, 0.5)
	var surface := StandardMaterial3D.new()
	surface.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	surface.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	surface.albedo_texture = texture
	surface.render_priority = 1
	material_override = surface

func _physics_process(_delta: float) -> void:
	if not is_instance_valid(actor) or not actor.is_visible_in_tree():
		visible = false
		return
	match kind:
		_: _place_blob()

## The down ray, the height fade and the yaw of FUN_004e4840.
func _place_blob() -> void:
	var from := actor.global_position + Vector3.UP * START_HEIGHT
	var query := PhysicsRayQueryParameters3D.create(from, from + Vector3.DOWN * (REACH + START_HEIGHT), actor.collision_mask, [actor.get_rid()])
	var hit := get_world_3d().direct_space_state.intersect_ray(query)
	if hit.is_empty() or hit["normal"].y <= FLOOR_NORMAL_MINIMUM:
		visible = false
		return
	var height: float = actor.global_position.y - hit["position"].y
	var fade := 1.0 / (maxf(height, 0.0) * 0.3 + 1.0)
	var up: Vector3 = hit["normal"]
	var ahead := actor.global_basis.z.slide(up)
	if ahead.is_zero_approx(): ahead = Vector3.FORWARD.slide(up)
	ahead = ahead.normalized()
	var across := up.cross(ahead).normalized()
	var size := Vector2(HALF_WIDTH, HALF_LENGTH) * 2.0 * source_scale * fade
	# FUN_004e4430 sizes a realistic shadow from the larger scale.
	if kind >= 2: size = Vector2.ONE * 2.0 * REALISTIC_RADIUS * maxf(source_scale.x, source_scale.y)
	var pose := Basis(across * size.x, up, ahead * size.y)
	global_transform = Transform3D(pose, hit["position"] + up * LIFT)
	visible = true
