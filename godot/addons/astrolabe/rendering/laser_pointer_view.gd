extends Sprite3D
## The engine laser pointer (procedures 337 and 338) drawn where the aim ray
## meets a surface. The source draw (0x4218e0, from the render pass 0x4df406)
## returns at once while the pointer is hidden, queries its ray (miss -50000),
## and draws the pointer texture at the hit in the world, so it is depth tested
## and shrinks with distance. Stephen's take (frames 352, 353, 382) shows it on
## the tree, hive and far cliff at about 25, 10 and 6 px. Its world size here is
## derived from those frames: the source's scale 2.5 has no verified unit, and
## its orientation (camera-facing or along the surface) is not established.
## See notes/research/crossbow-reticle-chain.md.

const WORLD_SIZE := 0.25

## The actor whose laser pointer state (source_laser_pointer) is drawn; read
## each frame, since the actor creates that state in its own _ready.
var actor: Node

func _init() -> void:
	name = "LaserPointer"
	billboard = BaseMaterial3D.BILLBOARD_ENABLED
	shaded = false
	double_sided = true
	alpha_cut = SpriteBase3D.ALPHA_CUT_DISABLED
	texture_filter = BaseMaterial3D.TEXTURE_FILTER_LINEAR
	visible = false

func bind(owner_actor: Node, image: Texture2D) -> void:
	actor = owner_actor
	texture = image
	if image != null:
		pixel_size = WORLD_SIZE / float(maxi(image.get_width(), 1))

func _process(_delta: float) -> void:
	var pointer: Resource = actor.get("source_laser_pointer") if is_instance_valid(actor) else null
	var shown := pointer != null and texture != null and bool(pointer.get("active")) and bool(pointer.get("target_hit"))
	visible = shown
	if not shown: return
	var target: Vector3 = pointer.get("target_position")
	var camera := get_viewport().get_camera_3d()
	# Lift it off the hit surface toward the eye so the surface does not hide it.
	global_position = target + (camera.global_position - target).normalized() * 0.03 if camera != null else target
