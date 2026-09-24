extends Resource
## Shared state read by the original magick AI functions. The selected spell
## uses the AI function's source code. Orrery can read the selected magic-name
## row and match it to the active inventory; the live result is still unverified.

@export var current_points: int = 0
@export var maximum_points: int = 0
@export var selected_spell_code: int = -1

func ready() -> bool:
    return maximum_points > 0 and current_points >= 0 and current_points <= maximum_points and selected_spell_code >= 0

## Function 230 subtracts when enough mana remains, then returns UInt success.
## The verified original handler clamps the written total to the maximum.
func remove_points(amount: int) -> int:
    assert(amount >= 0 and current_points >= 0 and maximum_points >= 0)
    if amount > current_points:
        return 0
    current_points = mini(current_points - amount, maximum_points)
    return 1
