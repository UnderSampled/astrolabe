extends RefCounted
## Lights for dynamic meshes as the source sends them: each actor takes the
## lights of its own sector (004dfc80 walks sector +0x28, keeping lights that are
## on), and lights outside that list do not touch it.
##
## The parity layer only: with it off, dynamic meshes stay on their native
## materials and the native Light3D nodes under each SourceLight.
##
## Written to one global float texture. Row 0 is the level ambient, for meshes
## with no sector; row N+1 is sector N. Texel 0 of a row is the summed ambient and
## the light count. Each light then takes four texels:
##   position, kind
##   color
##   near, far, inner cone, outer cone
##   axis: direction toward a parallel light, or the backward axis of a spot

var _rows: Dictionary = {}

func build(level: Node) -> void:
	var lights := {}
	for node in level.get_children():
		if node.get("source_path") != null and node.lit and node.sends:
			lights[node.source_path] = node
	var sectors: Array = []
	var lists: Array = []
	var widest := 0
	for sector in level.find_children("*", "Node3D", true, false):
		if sector.get("static_lights") == null: continue
		var list: Array = []
		for path in sector.static_lights + sector.dynamic_lights:
			if lights.has(path): list.append(lights[path])
		sectors.append(sector)
		lists.append(list)
		widest = maxi(widest, list.size())
	var image := Image.create_empty(1 + 4 * widest, sectors.size() + 1, false, Image.FORMAT_RGBAF)
	_write_row(image, 0, lights.values().filter(func(l: Node) -> bool: return l.kind == 4))
	for i in sectors.size():
		_rows[sectors[i]] = i + 1
		_write_row(image, i + 1, lists[i])
	RenderingServer.global_shader_parameter_set("source_sector_lights", ImageTexture.create_from_image(image))

func row(sector: Node3D) -> int:
	return _rows.get(sector, 0)

func _write_row(image: Image, y: int, lights: Array) -> void:
	var ambient := Color(0.0, 0.0, 0.0)
	var count := 0
	for light: Node3D in lights:
		var c: Color = light.source_color
		if light.kind == 4:
			ambient += Color(c.r, c.g, c.b, 0.0)
		elif light.kind in [1, 2, 3]:
			var x := 1 + 4 * count
			var origin := light.position
			var axis := light.basis.y.normalized()
			image.set_pixel(x, y, Color(origin.x, origin.y, origin.z, light.kind))
			image.set_pixel(x + 1, y, c)
			image.set_pixel(x + 2, y, Color(light.near, light.far, light.rotation_step, light.pulse_step))
			image.set_pixel(x + 3, y, Color(axis.x, axis.y, axis.z, 0.0))
			count += 1
	ambient.a = count
	image.set_pixel(0, y, ambient)
