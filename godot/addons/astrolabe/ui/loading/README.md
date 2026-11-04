# Loading presentation

`screen.tscn` is a persistent, editable native Godot presentation component.
Instance it once beneath `/root/LevelTransition` (or another node that lives
across level replacement). It observes the transition service and never starts,
cancels, instantiates, replaces, or places a level itself.

On `transition_started` it loads `res://generated/hype/levels/<level>/loading.tres`,
a `vignette.gd` resource the exporter converts from the level's `<level>pgb.bin`.
When present, `Viewport/Canvas` shows the source era backdrop and the bitmap
bar on a uniformly fitted 640x480 canvas; the fill strip is clipped from the
track's left edge to the observed `ResourceLoader.load_threaded_get_status()`
fraction. The source draws no text, fade or animation on this screen, and
neither does the presenter. See `docs/astrolabe/ui.md` for the evidence.

Without a vignette, the `Viewport/Center/Content` status group shows the level
name, phase and the same fraction. A failure replaces the progress state with
the supplied message and a Dismiss button in either layout; the presenter does
not change the active scene or recover session state.

Callers read `phase_text`, `progress_shown` and `progress_value`, and set the
post-load phase with `show_phase(message, show_progress)`, rather than reaching
into the node tree.
