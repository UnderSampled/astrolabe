extends Resource
## Shared state read by the original magick AI functions. The selected spell
## uses the AI function's source code. Orrery can read the selected magic-name
## row and match it to the active inventory; the live result is still unverified.

@export var current_points: int = 0
@export var maximum_points: int = 0
@export var selected_spell_code: int = -1

func ready() -> bool:
    return maximum_points > 0 and current_points >= 0 and current_points <= maximum_points and selected_spell_code >= 0

## Pool APIs do not require a selected spell. Maximum changes can leave current
## above maximum; the original stores both independent unsigned 16-bit values.
func points_ready() -> bool:
    return current_points >= 0 and current_points <= 65535 and maximum_points >= 0 and maximum_points <= 65535

## Proc434 at 0053debf: one low-word store, no current-points side effect.
func change_maximum(value: int) -> void:
    maximum_points = value & 65535
    emit_changed()

## Function229 at 005365e0 returns the full 32-bit sum, while the low word is
## compared unsigned against maximum and the stored current value is clamped.
func add_points(amount: int) -> int:
    assert(points_ready())
    var total := (current_points + amount) & 4294967295
    current_points = mini(total & 65535, maximum_points)
    emit_changed()
    return total

## Function 230 subtracts when enough mana remains, then returns UInt success.
## The verified original handler clamps the written total to the maximum.
func remove_points(amount: int) -> int:
    assert(amount >= 0 and current_points >= 0 and maximum_points >= 0)
    if amount > current_points:
        return 0
    current_points = mini(current_points - amount, maximum_points)
    return 1
