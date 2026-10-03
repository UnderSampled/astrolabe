extends Control
## Visibility is presentation state. Values remain in the actor's native resources.
@export var actor: Node
var requested: Dictionary = {}
var suppressed: bool = false
var indicators: Dictionary = {}

func _ready() -> void:
    indicators = {"life": $Life, "armor": $Armor, "magic": $Magic,
        "selected_object": $SelectedObject, "weapon": $Weapon}
    for key in indicators: requested[key] = false
    if is_instance_valid(actor): bind_actor(actor)
    refresh()

func bind_actor(value: Node) -> void:
    if is_instance_valid(actor) and actor.hud_visibility_requested.is_connected(set_indicator_visible):
        actor.hud_visibility_requested.disconnect(set_indicator_visible)
    actor = value
    for key in indicators: requested[key] = false
    if is_instance_valid(actor):
        actor.hud_visibility_requested.connect(set_indicator_visible)
        for key in actor.source_hud_display: requested[key] = actor.source_hud_display[key]
    refresh()

func set_indicator_visible(indicator: String, enabled: bool) -> void:
    if not indicators.has(indicator): return
    requested[indicator] = enabled
    refresh()

func refresh() -> void:
    for key in indicators: indicators[key].visible = bool(requested.get(key, false)) and not suppressed

func _process(_delta: float) -> void:
    if not is_instance_valid(actor): return
    var points: Resource = actor.get("source_hit_points")
    if points != null and points.maximum > 0:
        $Life.max_value = points.maximum
        $Life.value = points.current
    var magic: Resource = actor.get("source_magick")
    if magic != null and magic.maximum_points > 0:
        $Magic.max_value = magic.maximum_points
        $Magic.value = magic.current_points
    # Armor values must be supplied by promoted gameplay fields; never seed
    # invented totals. Texture slots accept generated inventory icon resources.

func set_meter(indicator: String, current: float, maximum: float) -> bool:
    if indicator not in ["life", "armor", "magic"] or maximum <= 0: return false
    var meter: ProgressBar = indicators[indicator]
    meter.max_value = maximum
    meter.value = current
    return true
