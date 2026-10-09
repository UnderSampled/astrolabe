extends RefCounted
## Hype's plain-wave software distance law, over native world positions.
## Disc snd_cpa.ini SATUR=2; engine reads BACK (absent), never its FADE key.
const SATURATION_WORD := 2 * 65536
const BACK_WORD := 0x7ffffffe

static func signed_word(value: int) -> int:
    var word := value & 0xffffffff
    return word - 0x100000000 if word >= 0x80000000 else word

static func magnitude_word(value: int) -> int:
    var word := value & 0xffffffff
    return (-word) & 0xffffffff if signed_word(word) < 0 else word

## 004fc2a0 signed magnitude comparison, low32 addition and SAR1.
static func pseudo_pair(left: int, right: int) -> int:
    var a := magnitude_word(left)
    var b := magnitude_word(right)
    if signed_word(a) < signed_word(b):
        return (b + (signed_word(a) >> 1)) & 0xffffffff
    return (a + (signed_word(b) >> 1)) & 0xffffffff

static func fixed_position(position: Vector3) -> PackedInt64Array:
    # 00432540..a0/00432020..a0 publish each world component before subtracting.
    # The adapter maps source XYZ to native X,-Z,Y.
    return PackedInt64Array([int(position.x * 65536.0) & 0xffffffff, int(-position.z * 65536.0) & 0xffffffff, int(position.y * 65536.0) & 0xffffffff])

## 004fc2d0 groups source X/Y first, then Z; no Euclidean square root.
static func distance_word(object_position: Vector3, microphone_position: Vector3) -> int:
    var object := fixed_position(object_position)
    var microphone := fixed_position(microphone_position)
    return pseudo_pair(pseudo_pair(object[0] - microphone[0], object[1] - microphone[1]), object[2] - microphone[2])

## 004fbf00/004fbf50. Float division mirrors finite fixed16.16 conversion;
## x87 fault/rounding behavior for invalid pairs is outside this native API.
static func distance_volume(distance: int, saturation: int = SATURATION_WORD, back: int = BACK_WORD) -> int:
    var value := signed_word(distance)
    var near := signed_word(saturation)
    var far := signed_word(saturation * 8)
    var tail := signed_word(back)
    if tail < near: tail = 0x7fffffff
    if value < near: return 127
    if value > tail:
        if tail >= far: return 0
        value = tail
    if value > far: return 0
    assert(far != near, "Source sound rolloff has a zero divisor")
    var ratio := int(float(far - value) / float(far - near) * 65536.0)
    return ((127 * ratio) >> 16) & 0xff

## Both Sxd converters multiply only when mixer flag4 is set. Gameplay sources
## have a nonnull preference and therefore publish it, including at level127.
static func converted_volume(parameter_byte: int, attenuation_byte: int) -> int:
    return (((parameter_byte & 0xff) * (attenuation_byte & 0xff)) >> 7) & 0xff
