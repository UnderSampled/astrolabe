# OpenSpace AI Script Format

OpenSpace games (Rayman 2, Hype: The Time Quest, etc.) use a bytecode-like scripting system for game logic. Scripts control character behavior, dialog, animations, and game state.

## Storage Location

AI scripts are embedded in SNA level files, not stored as separate files. They're accessed through the scene hierarchy:

```
SNA (level data)
└── Perso (character/object)
    └── Brain
        └── Mind
            ├── AIModel (shared behavior template)
            │   ├── behaviors_normal[]  (Intelligence behaviors)
            │   ├── behaviors_reflex[]  (Reflex behaviors)
            │   ├── macros[]            (Reusable script fragments)
            │   └── DsgVar              (Designer variable definitions)
            ├── DsgMem                  (Per-instance variable values)
            └── Intelligence            (Current behavior state)
```

- **AIModel** is shared - multiple Persos can reference the same template
- **Behavior** contains one or more **Script**s
- **Script** contains an array of **ScriptNode**s

## ScriptNode Binary Format

Each ScriptNode is 8 bytes (on PC):

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| 0x00 | 4 | param | Parameter value (integer, float bits, or pointer) |
| 0x04 | 2 | padding | Unused |
| 0x06 | 1 | indent | Tree depth (0 = end of script) |
| 0x07 | 1 | type | Node type (indexes into type tables) |

The `indent` field encodes tree structure: a node at indent N+1 is a child of the previous node at indent N. This makes the format a **serialized S-expression tree**.

### Example

Binary nodes (simplified):
```
[param=0,  indent=1, type=KeyWord_If]
[param=0,  indent=2, type=Cond_Equal]
[param=0,  indent=3, type=DsgVarRef]
[param=5,  indent=3, type=Constant]
[param=0,  indent=2, type=KeyWord_Then]
[param=42, indent=3, type=Procedure_ShowText]
[param=42, indent=4, type=TextRef]
[param=0,  indent=0, type=End]
```

Equivalent tree:
```lisp
(if
  (cond-equal
    (dsgvar 0)
    (constant 5))
  (then
    (proc-show-text
      (text-ref 42))))
```

## Node Types

The `type` byte indexes into game-specific tables. Node types are defined per-game in `AITypes_*.cs` files.

### Type Categories

| Category | Description | Examples |
|----------|-------------|----------|
| **KeyWord** | Control flow | If, IfNot, Then, Else |
| **Condition** | Boolean tests | Cond_And, Cond_Equal, Cond_Lesser |
| **Operator** | Math/assignment | Plus, Minus, Mul, Affect (=) |
| **Function** | Returns a value | Func_GetPosition, Func_Random |
| **Procedure** | No return value | Proc_PlaySound, Proc_ShowText |
| **MetaAction** | Async/blocking | Meta_WaitFrames, Meta_Goto |
| **Field** | Object property access | Field_Speed, Field_Position |
| **References** | Pointers to objects | PersoRef, TextRef, WayPointRef |
| **Literals** | Constant values | Constant, Real, String, Vector |

### Reference Types

These node types have `param` as a pointer or index:

| Type | param meaning |
|------|---------------|
| `TextRef` | Index into localization table |
| `PersoRef` | Pointer to Perso in scene |
| `WayPointRef` | Pointer to waypoint |
| `SoundEventRef` | Sound event ID |
| `ComportRef` | Pointer to Behavior |
| `DsgVarRef` | DsgVar index |
| `SuperObjectRef` | Pointer to SuperObject |
| `ObjectTableRef` | Pointer to object list |

## Type Tables (Hype)

For Hype: The Time Quest, types are defined in:
- `reference/raymap/Assets/Scripts/OpenSpace/AI/AITypes/AITypes_Hype.cs`

Key tables:
- `keywordTable[]` - control flow keywords
- `conditionTable[]` - boolean conditions
- `operatorTable[]` - math/logic operators
- `functionTable[]` - functions (return value)
- `procedureTable[]` - procedures (no return)
- `metaActionTable[]` - async actions
- `fieldTable[]` - object fields

The `type` byte for each node category indexes into the corresponding table.

## DsgVar (Designer Variables)

Each AIModel can define typed variables accessible to scripts:

| DsgVarType | Description |
|------------|-------------|
| Boolean | true/false |
| Byte, Short, Int, UInt | Integers |
| Float | Floating point |
| Vector | 3D vector |
| Text | Localized text index |
| Perso | Reference to character |
| WayPoint | Reference to waypoint |
| *Array | Arrays of above types |

DsgVar values can be per-instance (DsgMem) or shared (AIModel.DsgVar).

## Localization References

Scripts reference localized text by numeric index:
```lisp
(proc-show-text (text-ref 42))
```

The index looks up into a separate localization table loaded from the SNA fix file. See `LocalizationStructure.cs` for the text table format.

## Raymap Reference Files

All paths relative to `reference/raymap/Assets/Scripts/OpenSpace/` in the astrolabe repo.

### Core Parsing
- `AI/ScriptNode.cs` - Node parsing and NodeType enum (line 398+)
- `AI/Script.cs` - Script container, reads node array
- `AI/Behavior.cs` - Behavior container with scripts
- `AI/AIModel.cs` - AI model with behaviors/macros
- `AI/Brain.cs` - Brain → Mind → AIModel link
- `AI/Mind.cs` - Per-perso AI state

### Type Definitions
- `AI/AITypes.cs` - Base type table structure
- `AI/AITypes/AITypes_Hype.cs` - Hype-specific tables
- `AI/AITypes/AITypes_R2.cs` - Rayman 2 tables

### Variables
- `AI/DsgVar.cs` - Variable definitions
- `AI/DsgVarValue.cs` - Variable values
- `AI/DsgMem.cs` - Per-instance memory

### Translation (C# output)
- `AI/TranslatedScript.cs` - Tree reconstruction and C# codegen
- `Exporter/AIModelExporter.cs` - Full AIModel to C# class

## Suggested S-Expression Format

For a Godot interpreter, export scripts as S-expressions:

```lisp
;; behavior definition
(behavior "patrol" :type normal
  (script 0
    (if (cond-equal (dsgvar 0) (const 5))
      (then
        (proc-show-text (text-ref 42))
        (meta-wait-frames (const 30))))
    (proc-change-comport (comport-ref "idle"))))
```

Key mappings:
- `KeyWord` → lowercase symbols: `if`, `then`, `else`
- `Condition` → `cond-*` prefix
- `Procedure` → `proc-*` prefix
- `Function` → `func-*` prefix
- `Operator` → infix or `op-*` prefix
- References → `*-ref` suffix with index/pointer

The interpreter walks the tree, dispatching on node type to Godot implementations.
