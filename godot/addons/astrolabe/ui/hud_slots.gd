extends Control
## Selected Object and Weapon slots of the source HUD (FUN_0040cb50 / FUN_0040ca40).
## Each draws the inventory image of the selected item at its Fix.sda rectangle in
## 640x480 space. State stays in the actor's inventory; the compiled AI only
## requests visibility through Stats_DisplayMainActorSelectedObject / ...Weapon.
const ITEM_DIRECTORY := "res://generated/hype/shared/inventory/en/"
## Source rows 22 (Object) and 10 (Weapon): left, top, width, height.
const OBJECT_RECT := Rect2(269, 404, 48, 60)
const WEAPON_RECT := Rect2(10, 360, 48, 48)
const COUNT_GLYPH_OFFSET := Vector2(5.6, -12.6)
@export var actor: Node
var suppressed: bool = false
var _requested := {"selected_object": false, "weapon": false, "life": false, "armor": false, "magic": false, "cash": false}
var _items: Dictionary = {}
var _object_icon := TextureRect.new()
var _weapon_icon := TextureRect.new()
var _weapon_count := Label.new()
## Fix HUD panel descriptor 3 (4 stone-frame layers), activated by FUN_0040c150(3)
## immediately before the Object draw in FUN_0040cb50; exported as Source/Panel3.
var _object_panel: Control
## Fix HUD panel descriptors 0 (stone frames) and 2, drawn first by FUN_0040c270.
var _frame_panel: Control
var _cash_panel: Control
## FUN_0043ab40 draws each piece of the selected armor at Fix.sda rectangle 2.
const ARMOR_RECT := Rect2(10, 420, 48, 48)
const StandInReport = preload("res://addons/astrolabe/behavior/stand_in_report.gd")
var _armor := Control.new()
## Spell slots and magic-points arc (0043a450, 0043a6a0/00439e50), drawn from
## the HUD Source node's metadata and the actor's magick and inventory.
var _magic := Control.new()
var _source: Control
var _armor_state := ""

func _ready() -> void:
    mouse_filter = Control.MOUSE_FILTER_IGNORE
    for pair in [[_object_icon, OBJECT_RECT], [_weapon_icon, WEAPON_RECT]]:
        var icon: TextureRect = pair[0]
        icon.position = pair[1].position
        icon.size = pair[1].size
        icon.mouse_filter = Control.MOUSE_FILTER_IGNORE
        icon.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
        icon.stretch_mode = TextureRect.STRETCH_SCALE
        icon.texture_filter = CanvasItem.TEXTURE_FILTER_LINEAR
        icon.visible = false
        add_child(icon)
    # FUN_0040ca40 anchors the count at the weapon row's corner plus the Fix.sda
    # text offset, then FON projects it in 3D (not ported). Stand-in: the label
    # sits at a measured offset from that anchor, matched to the linear take's
    # frame 469 (glyph box within 1 px). Font: the HUD counter.
    StandInReport.reached(self, "hud-count-offset", "weapon count at a measured offset, FON 3D projection not ported")
    var anchor := get_parent().get_node_or_null("Source/WeaponCountAnchor") as Control
    _weapon_count.position = (anchor.position if anchor != null else WEAPON_RECT.position) + COUNT_GLYPH_OFFSET
    _weapon_count.size = Vector2(128, 28)
    _weapon_count.mouse_filter = Control.MOUSE_FILTER_IGNORE
    _weapon_count.visible = false
    var counter := get_parent().get_node_or_null("Source/CurrencyCount") as Label
    if counter != null:
        _weapon_count.add_theme_font_override("font", counter.get_theme_font("font"))
    add_child(_weapon_count)
    _object_panel = get_parent().get_node_or_null("Source/Panel3") as Control
    _frame_panel = get_parent().get_node_or_null("Source/Panel0") as Control
    _cash_panel = get_parent().get_node_or_null("Source/Panel2") as Control
    _armor.mouse_filter = Control.MOUSE_FILTER_IGNORE
    add_child(_armor)
    _source = get_parent().get_node_or_null("Source") as Control
    _magic.mouse_filter = Control.MOUSE_FILTER_IGNORE
    _magic.draw.connect(_draw_magic)
    add_child(_magic)
    refresh()

func bind_actor(value: Node) -> void:
    if is_instance_valid(actor) and actor.hud_visibility_requested.is_connected(set_indicator_visible):
        actor.hud_visibility_requested.disconnect(set_indicator_visible)
    actor = value
    for key in _requested: _requested[key] = false
    if is_instance_valid(actor):
        actor.hud_visibility_requested.connect(set_indicator_visible)
        for key in _requested: _requested[key] = bool(actor.source_hud_display.get(key, false))
    refresh()

func set_indicator_visible(indicator: String, enabled: bool) -> void:
    if not _requested.has(indicator): return
    _requested[indicator] = enabled
    refresh()

func _item(item_id: int) -> Resource:
    if item_id < 0: return null
    if not _items.has(item_id):
        var path := ITEM_DIRECTORY + "item_%d.tres" % item_id
        _items[item_id] = load(path) if ResourceLoader.exists(path) else null
    return _items[item_id]

func _process(_delta: float) -> void:
    refresh()

func _armor_catalog() -> Resource:
    var node: Node = actor
    while is_instance_valid(node):
        if node.has_meta("source_armor_catalog"): return node.get_meta("source_armor_catalog") as Resource
        node = node.get_parent()
    return null

func _refresh_armor(show: bool) -> void:
    var catalog := _armor_catalog() if show else null
    var definition: Resource = catalog.call("active") if catalog != null else null
    var state := ""
    if definition != null:
        for piece in definition.pieces: state += str(piece.source_key, ":", piece.broken, ",")
    if state == _armor_state: return
    _armor_state = state
    for child in _armor.get_children(): child.queue_free()
    if definition == null: return
    var artwork: Array = [definition.intact_artwork, definition.broken_artwork, definition.background_artwork]
    for piece in definition.pieces:
        var image: Texture2D = artwork[clampi(piece.broken, 0, 2)]
        if image == null: continue
        # Corners are u0, u1, v_top, v_bottom with v up; the image row is 1 - v.
        var r: Vector4 = piece.rectangle
        var atlas := AtlasTexture.new()
        atlas.atlas = image
        atlas.region = Rect2(r.x * image.get_width(), (1.0 - r.z) * image.get_height(), (r.y - r.x) * image.get_width(), (r.z - r.w) * image.get_height())
        var rect := TextureRect.new()
        rect.texture = atlas
        rect.mouse_filter = Control.MOUSE_FILTER_IGNORE
        rect.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
        rect.stretch_mode = TextureRect.STRETCH_SCALE
        rect.position = ARMOR_RECT.position + Vector2(r.x, 1.0 - r.z) * ARMOR_RECT.size
        rect.size = Vector2(r.y - r.x, r.z - r.w) * ARMOR_RECT.size
        _armor.add_child(rect)

func refresh() -> void:
    _magic.visible = not suppressed and bool(_requested["magic"])
    _magic.queue_redraw()
    var frame_on: bool = not suppressed and (_requested["life"] or _requested["armor"] or _requested["magic"] or _requested["weapon"])
    if _frame_panel != null: _frame_panel.visible = frame_on
    if _cash_panel != null: _cash_panel.visible = not suppressed and not frame_on and bool(_requested["cash"])
    _refresh_armor(not suppressed and bool(_requested["armor"]))
    var inventory: Resource = actor.get("source_inventory") as Resource if is_instance_valid(actor) else null
    var show := not suppressed and inventory != null
    var object_item: Resource = _item(int(inventory.get("selected_object"))) if show else null
    var weapon_item: Resource = _item(int(inventory.get("selected_weapon"))) if show else null
    _object_icon.visible = bool(_requested["selected_object"]) and object_item != null and not object_item.textures.is_empty()
    if _object_icon.visible: _object_icon.texture = object_item.textures[0]
    _weapon_icon.visible = bool(_requested["weapon"]) and weapon_item != null and not weapon_item.textures.is_empty()
    if _weapon_icon.visible:
        _weapon_icon.texture = weapon_item.textures[0]
        var count := int(inventory.call("quantity", weapon_item.item_id))
        _weapon_count.text = str(count)
    _weapon_count.visible = _weapon_icon.visible
    if _object_panel != null:
        # The original shows no frame while nothing is selected (New Game handoff).
        _object_panel.visible = _object_icon.visible

func _row(index: int) -> Rect2:
    # Active Fix.sda rows: x0, y0, x1, y1, width, height.
    var table: PackedFloat32Array = _source.get_meta("source_rectangles")
    var at := index * 6
    return Rect2(table[at], table[at + 1], table[at + 4], table[at + 5])

func _spell_item(kind: String) -> Resource:
    # 0041d660: the selected inventory handle whose kind is "Magic" + suffix.
    var inventory: Resource = actor.get("source_inventory") if is_instance_valid(actor) else null
    if inventory == null: return null
    for definition in inventory.get("definitions"):
        if definition != null and str(definition.kind) == kind and bool(definition.selected):
            return _item(int(definition.item_id))
    return null

func _draw_magic() -> void:
    if _source == null or not _source.has_meta("magic_slot_kinds") or not is_instance_valid(actor): return
    var magick: Resource = actor.get("source_magick")
    if magick == null: return
    var kinds: PackedStringArray = _source.get_meta("magic_slot_kinds")
    var rows: PackedInt32Array = _source.get_meta("magic_slot_rows")
    var selected_rect := Rect2()
    var selected_found := false
    for slot in kinds.size():
        var item := _spell_item(kinds[slot])
        if item == null or item.textures.size() < 2: continue
        var row := _row(rows[slot])
        if int(item.item_id) == int(magick.get("selected_spell_code")):
            var big := _row(4)
            # 0043a450 draws to x + w - 1, y + h - 1.
            selected_rect = Rect2(row.position + big.position, big.size - Vector2.ONE)
            selected_found = true
            _magic.draw_texture_rect(item.textures[0], selected_rect, false)
            var small := _row(9)
            _magic.draw_texture_rect(item.textures[1], Rect2(row.position + big.position + small.position, small.size), false)
        else:
            _magic.draw_texture_rect(item.textures[0], row, false)
            var small := _row(8)
            _magic.draw_texture_rect(item.textures[1], Rect2(row.position + small.position, small.size), false)
            _magic.draw_texture_rect(_source.get_meta("magic_slot_overlay"), row, false)
    if selected_found: _draw_points_arc(selected_rect, magick)

## 00439e50: a fan from the slot centre that starts at the top-left corner and
## runs down the left edge, along the bottom, up the right and back along the
## top; the end vertex moves linearly along its edge. Texture v runs up.
func _draw_points_arc(rect: Rect2, magick: Resource) -> void:
    var maximum := int(magick.get("maximum_points"))
    var fraction := float(int(magick.get("current_points"))) / float(maximum) if maximum > 0 else 0.0
    if fraction <= 0.0: return
    fraction = minf(fraction, 1.0)
    var l := rect.position.x
    var t := rect.position.y
    var r := rect.end.x
    var b := rect.end.y
    var uv := func(p: Vector2) -> Vector2: return Vector2((p.x - l) / (r - l), (p.y - t) / (b - t))
    # Corners in sweep order, with the fraction at which the sweep reaches each.
    var path := [[Vector2(l, t), 0.0], [Vector2(l, b), 0.25], [Vector2(r, b), 0.5], [Vector2(r, t), 0.75], [Vector2(l, t), 1.0]]
    var points := PackedVector2Array([rect.get_center(), Vector2(l, t)])
    for i in range(1, path.size()):
        var start: Vector2 = path[i - 1][0]
        var finish: Vector2 = path[i][0]
        var from: float = path[i - 1][1]
        var to: float = path[i][1]
        if fraction >= to:
            points.append(finish)
        else:
            points.append(start.lerp(finish, (fraction - from) / (to - from)))
            break
    var uvs := PackedVector2Array()
    for p in points: uvs.append(uv.call(p))
    _magic.draw_colored_polygon(points, Color.WHITE, uvs, _source.get_meta("magic_points_arc"))
