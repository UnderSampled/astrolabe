# Loading presentation

`screen.tscn` is a persistent, editable native Godot presentation component.
Instance it once beneath `/root/LevelTransition` (or another node that lives
across level replacement). It observes the transition service and never starts,
cancels, instantiates, replaces, or places a level itself.

The screen displays the actual `ResourceLoader.load_threaded_get_status()`
fraction emitted as `transition_progress` for the `resource_loading` phase. It
does not smooth the value, add a timer, or present it as total transition
completion. The heading is the English equivalent of the source caption
`Chargement de la map`, followed by the requested level name. The Control tree
is intentionally free of a background image and
animation: source inspection found window-caption updates, but no dedicated
in-frame loading artwork, progress bar, animation, or minimum display duration.
The resource uses the maintained UI theme so it remains editable in Godot.

Completion and cancellation close the presentation immediately. A failure
replaces the progress state with the supplied message and a Dismiss button; the
presenter does not change the active scene or recover session state. Input
policy beyond mouse blocking remains with the transition/session owner.
