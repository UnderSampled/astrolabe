extends Resource
## One choice of a source question item (DLG answer record). Its rows follow
## the last body row; selecting it stores its owner behavior as the pending
## callback, plays its speech on MainActor, then continues with `next` or closes.
@export var lines: Array[Resource] = []
@export var completion_normal: int = -1
@export var voice: Resource
@export var one_shot: bool = false
## Gold added (>= 0) or spent (< 0) by the main actor's inventory (0056e0e0).
@export var gold: int = 0
## Item drawing continues with (AnswerNext), by name; empty closes.
@export var next: String = ""
## AnswerBad: item shown instead for a failed cost check code (0056e2c0).
@export var shortfall_codes: PackedInt32Array = PackedInt32Array()
@export var shortfall_items: PackedStringArray = PackedStringArray()

## 0056e3b0: one entry takes any code, otherwise match the code, else the first.
func shortfall_item(code: int) -> String:
    if shortfall_items.is_empty(): return ""
    if shortfall_items.size() == 1: return shortfall_items[0]
    for index in shortfall_codes.size():
        if shortfall_codes[index] == code: return shortfall_items[index]
    return shortfall_items[0]
