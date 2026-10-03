extends Resource
## Converted dialogue meaning. Original text/media belongs under generated/.
@export var identity: String
@export var locale: String = "en"
@export var translation: Translation
@export var box: int = 1
@export var lines: Array[Resource] = []
@export var presentation: Resource
@export var voice: Resource
@export var owner_identity: String
@export var completion_model: String
@export var completion_normal: int = -1
@export var allow_skip: bool = true

func lines_per_page() -> int:
    return 2 if box == 3 else 6

func message(key: StringName) -> String:
    # A selected locale owns a whole bundle, including voice and timing. Native
    # TranslationServer handles a supported locale; a missing locale falls back
    # to this bundle's English Translation without changing the game's locale.
    var translated: String = TranslationServer.translate(key)
    return str(translation.get_message(key)) if translated == str(key) and translation != null else translated
