extends Resource
## Shared state read by the original magick AI functions. The selected spell
## uses the AI function's source code. Orrery observes a menu selection index;
## its mapping to this code is still unverified.

@export var current_points: int = 0
@export var maximum_points: int = 0
@export var selected_spell_code: int = -1

func ready() -> bool:
    return maximum_points > 0 and current_points >= 0 and current_points <= maximum_points and selected_spell_code >= 0

## Function 230's argument is decoded, but its return semantics still need a
## source observation. Returning null makes compiled AI fail at its call site.
func remove_points(_amount: int) -> Variant:
    return null
