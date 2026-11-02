extends Control
## Selected Object and Weapon slots of the source HUD (FUN_0040cb50 / FUN_0040ca40).
## Each draws the inventory image of the selected item at its Fix.sda rectangle in
## 640x480 space. State stays in the actor's inventory; the compiled AI only
## requests visibility through Stats_DisplayMainActorSelectedObject / ...Weapon.
const ITEM_DIRECTORY := "res://generated/hype/shared/inventory/en/"
## Source rows 22 (Object) and 10 (Weapon): left, top, width, height.
const OBJECT_RECT := Rect2(269, 404, 48, 60)
const WEAPON_RECT := Rect2(10, 360, 48, 48)
@export var actor: Node
var suppressed: bool = false
var _requested := {"selected_object": false, "weapon": false}
var _items: Dictionary = {}
var _object_icon := TextureRect.new()
var _weapon_icon := TextureRect.new()
var _weapon_count := Label.new()
## Fix HUD panel descriptor 3 (4 stone-frame layers), activated by FUN_0040c150(3)
## immediately before the Object draw in FUN_0040cb50; exported as Source/Panel3.
var _object_panel: Control

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
    # Stand-in: the source count text offsets live in Fix.gpt and are not yet
    # extracted, so the count sits below the weapon image. Font: the HUD counter.
    _weapon_count.position = WEAPON_RECT.position + Vector2(0, WEAPON_RECT.size.y)
    _weapon_count.size = Vector2(WEAPON_RECT.size.x + 24, 28)
    _weapon_count.mouse_filter = Control.MOUSE_FILTER_IGNORE
    _weapon_count.visible = false
    var counter := get_parent().get_node_or_null("Source/CurrencyCount") as Label
    if counter != null:
        _weapon_count.add_theme_font_override("font", counter.get_theme_font("font"))
    add_child(_weapon_count)
    _object_panel = get_parent().get_node_or_null("Source/Panel3") as Control
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

func refresh() -> void:
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
