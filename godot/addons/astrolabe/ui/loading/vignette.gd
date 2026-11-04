extends Resource
## A level's converted source loading vignette (`<level>pgb.bin`, 0050ef30).
## Geometry is in the source 640x480 screen basis.

## Full-screen era image drawn while the level loads.
@export var backdrop: Texture2D
## Bitmap bar art: the frame state at `frame_y` and the fill state at `fill_y`,
## each `bar_rect.size.y` rows tall. Colour-keyed pixels are transparent.
@export var bar: Texture2D
## Screen rectangle of the bar art.
@export var bar_rect := Rect2i(60, 410, 512, 60)
@export var frame_y := 0
@export var fill_y := 60
## Fill track within the bar art; the fill grows from min to max.
@export var fill_min_x := 50
@export var fill_max_x := 461
## Source step count that fills the track (MaxValueBar).
@export var max_value := 1


## Bar-local fill extent in source pixels for a completed fraction.
func fill_width(fraction: float) -> int:
	var track := fill_max_x - fill_min_x
	return clampi(int(floor(track * clampf(fraction, 0.0, 1.0))), 0, track)
