# AI and Behavior System

**Location:** Embedded within SNA data blocks
**Purpose:** Character behavior, state machines, and scripted interactions

## Overview

The OpenSpace AI system uses a hierarchical structure:

```
Brain
└── Mind
    ├── AI Model (shared definition)
    │   ├── Normal Behaviors
    │   ├── Reflex Behaviors
    │   ├── Macros
    │   └── DsgVar Definitions
    ├── Intelligence Normal
    │   ├── Current Behavior
    │   └── Action Tree
    ├── Intelligence Reflex
    │   ├── Current Behavior
    │   └── Action Tree
    └── DsgMem (runtime variables)
```

## Brain

Entry point for AI on a Perso (character).

### Structure

```
Offset  Size    Type        Description
------  ----    ----        -----------
0x00    4       Pointer     Offset
0x04    4       Pointer     Mind pointer
```

## Mind

Central AI container linking model, intelligences, and memory.

### Structure

```
Offset  Size    Type        Description
------  ----    ----        -----------
0x00    4       Pointer     Offset
0x04    4       Pointer     AI model pointer
0x08    4       Pointer     Intelligence normal pointer
0x0C    4       Pointer     Intelligence reflex pointer
0x10    4       Pointer     DsgMem pointer
0x14    ?       string      Name (optional)
```

## AI Model

Shared AI definition containing behaviors and variables.

### Structure

```
Offset  Size    Type        Description
------  ----    ----        -----------
0x00    4       Pointer     Offset
0x04    4       u32         Number of normal behaviors
0x08    4       Pointer     Normal behaviors array pointer
0x0C    4       u32         Number of reflex behaviors
0x10    4       Pointer     Reflex behaviors array pointer
0x14    4       u32         Number of macros
0x18    4       Pointer     Macros array pointer
0x1C    4       Pointer     DsgVar definitions pointer
```

## Intelligence

Dual-mode AI controller (Normal for regular behavior, Reflex for reactions).

### Structure

```
Offset  Size    Type        Description
------  ----    ----        -----------
0x00    4       Pointer     Offset
0x04    4       Pointer     Current behavior (comport) pointer
0x08    4       Pointer     Default behavior pointer
0x0C    4       Pointer     Action tree pointer
```

## Behavior (Comport)

Individual behavior state with associated scripts.

### Structure

```
Offset  Size    Type        Description
------  ----    ----        -----------
0x00    4       Pointer     Offset
0x04    4       u32         Number of scripts
0x08    4       Pointer     Scripts array pointer
0x0C    4       Pointer     Schedule script pointer
0x10    ?       string      Name
```

### Behavior Types

- **Normal**: Regular gameplay behaviors (idle, walk, attack, etc.)
- **Reflex**: Reactive behaviors triggered by events (hit, death, etc.)

## Macro

Reusable script subroutine called from behaviors.

### Structure

```
Offset  Size    Type        Description
------  ----    ----        -----------
0x00    4       Pointer     Offset
0x04    4       Pointer     Script pointer
0x08    ?       string      Name
```

## Script

Sequence of script nodes forming executable logic.

### Structure

```
Offset  Size    Type        Description
------  ----    ----        -----------
0x00    4       Pointer     Offset
0x04    4       u32         Number of nodes
0x08    4       Pointer     Nodes array pointer
```

## Script Node

Individual instruction, condition, or operation.

### Structure

```
Offset  Size    Type        Description
------  ----    ----        -----------
0x00    4       Pointer     Offset
0x04    1       u8          Indent level (nesting depth)
0x05    1       u8          Node type
0x06    4       Pointer     Parameter pointer
0x0A    ?       ...         Type-specific data
```

### Node Types

| Type | Category | Description |
|------|----------|-------------|
| 0 | Keyword | Control flow (if, else, while, etc.) |
| 1 | Condition | Boolean test |
| 2 | Operator | Arithmetic/logic operation |
| 3 | Function | Value-returning call |
| 4 | Procedure | Action call |
| 5 | MetaAction | Special action |
| 6 | Field | Data access |
| 7 | DsgVarRef | Designer variable reference |
| 8 | DsgVarId | Designer variable by ID |
| 9 | Constant | Literal value |
| 10 | Real | Float constant |
| 11 | Button | Input button |
| 12 | Vector | 3D vector constant |
| 13 | Mask | Bitmask constant |
| 14 | String | String constant |
| 15 | Subroutine | Macro call |
| 16 | Null | Null value |
| 17+ | ... | Engine-specific types |

## Designer Variables (DsgVar)

Parameterizable variables for customizing AI behavior per instance.

### DsgVar Types

| ID | Type | Description |
|----|------|-------------|
| 0 | Boolean | True/false |
| 1 | Byte | 8-bit unsigned |
| 2 | UByte | 8-bit unsigned |
| 3 | Short | 16-bit signed |
| 4 | UShort | 16-bit unsigned |
| 5 | Int | 32-bit signed |
| 6 | UInt | 32-bit unsigned |
| 7 | Float | 32-bit float |
| 8 | Vector | 3D vector |
| 9 | List | Generic list |
| 10 | Comport | Behavior reference |
| 11 | Action | Action reference |
| 12 | Caps | Capabilities |
| 13 | Input | Input reference |
| 14 | SoundEvent | Sound reference |
| 15 | Light | Light reference |
| 16 | GameMaterial | Material reference |
| 17 | VisualMaterial | Visual material |
| 18 | Perso | Character reference |
| 19 | WayPoint | Navigation point |
| 20 | Graph | Navigation graph |
| 21 | Text | Text/string |
| 22 | SuperObject | Scene object reference |
| 23 | SOLinks | SuperObject links |
| 24 | PersoArray | Array of characters |
| 25 | VectorArray | Array of vectors |
| 26 | FloatArray | Array of floats |
| 27 | IntArray | Array of integers |
| 28 | WayPointArray | Array of waypoints |
| 29 | TextArray | Array of strings |
| 30 | TextRefArray | Array of text refs |
| 31 | GraphArray | Array of graphs |
| 32 | SOLinksArray | Array of SO links |
| 33 | SoundEventArray | Array of sounds |
| 34 | Array6 | Generic array 6 |
| 35 | Array9 | Generic array 9 |
| 36 | Array10 | Generic array 10 |
| 37 | Array11 | Generic array 11 |
| 38 | Way | Waypoint system |
| 39 | ActionArray | Array of actions |
| 40 | SuperObjectArray | Array of objects |
| 41 | ObjectList | Object list |

### DsgVar Definition

```
Offset  Size    Type        Description
------  ----    ----        -----------
0x00    4       Pointer     Offset
0x04    1       u8          Type (DsgVarType)
0x05    4       u32         Initial value offset (in DsgMem)
0x09    4       u32         Memory offset (in DsgMem)
```

## DsgMem

Runtime memory for designer variables.

### Structure

```
Offset  Size    Type        Description
------  ----    ----        -----------
0x00    4       Pointer     Offset
0x04    4       u32         Memory size
0x08    4       Pointer     Initial values buffer
0x0C    4       Pointer     Current values buffer
```

## Script Translation

Script nodes can be translated to readable pseudocode:

```python
def translate_script(nodes: list[ScriptNode]) -> str:
    """Translate script nodes to pseudocode."""
    lines = []
    for node in nodes:
        indent = "    " * node.indent_level
        match node.type:
            case NodeType.KEYWORD:
                lines.append(f"{indent}{node.keyword_name}")
            case NodeType.CONDITION:
                lines.append(f"{indent}if {translate_condition(node)}")
            case NodeType.PROCEDURE:
                lines.append(f"{indent}{node.procedure_name}({translate_params(node)})")
            case NodeType.FUNCTION:
                lines.append(f"{indent}{node.function_name}({translate_params(node)})")
            # ... etc
    return "\n".join(lines)
```

## Hype-Specific AI Types

Hype uses custom AI types defined in `AITypes_Hype.cs`:
- Custom keywords, conditions, functions, procedures
- Game-specific field accessors
- Unique action types

## Python Data Classes

```python
from dataclasses import dataclass
from enum import IntEnum
from typing import Optional, Any

class DsgVarType(IntEnum):
    BOOLEAN = 0
    BYTE = 1
    UBYTE = 2
    SHORT = 3
    USHORT = 4
    INT = 5
    UINT = 6
    FLOAT = 7
    VECTOR = 8
    LIST = 9
    COMPORT = 10
    ACTION = 11
    # ... etc

class ScriptNodeType(IntEnum):
    KEYWORD = 0
    CONDITION = 1
    OPERATOR = 2
    FUNCTION = 3
    PROCEDURE = 4
    META_ACTION = 5
    FIELD = 6
    DSGVAR_REF = 7
    DSGVAR_ID = 8
    CONSTANT = 9
    REAL = 10
    BUTTON = 11
    VECTOR = 12
    # ... etc

@dataclass
class ScriptNode:
    indent_level: int
    node_type: ScriptNodeType
    param_ptr: int
    # Type-specific fields populated during parsing
    value: Any = None
    name: str = ""

@dataclass
class Script:
    nodes: list[ScriptNode]

@dataclass
class Macro:
    name: str
    script: Script

@dataclass
class Behavior:
    name: str
    scripts: list[Script]
    schedule_script: Optional[Script]

@dataclass
class DsgVarDef:
    var_type: DsgVarType
    initial_offset: int
    memory_offset: int

@dataclass
class DsgMem:
    size: int
    initial_buffer: bytes
    current_buffer: bytes

@dataclass
class AIModel:
    normal_behaviors: list[Behavior]
    reflex_behaviors: list[Behavior]
    macros: list[Macro]
    dsgvar_defs: list[DsgVarDef]

@dataclass
class Intelligence:
    current_behavior: Optional[Behavior]
    default_behavior: Optional[Behavior]
    action_tree_ptr: int

@dataclass
class Mind:
    name: str
    ai_model: AIModel
    intelligence_normal: Intelligence
    intelligence_reflex: Intelligence
    dsgmem: DsgMem

@dataclass
class Brain:
    mind: Mind
```

## Conversion to GDScript

When converting AI to GDScript:

1. **Behavior** → GDScript state in state machine
2. **Script** → GDScript function
3. **DsgVar** → Export variables
4. **Conditions** → if/match statements
5. **Procedures** → Function calls
6. **Macros** → Helper functions

Example output:

```gdscript
extends CharacterBody3D

# Designer Variables
@export var patrol_speed: float = 5.0
@export var detect_distance: float = 10.0
@export var target: NodePath

# States
enum State { IDLE, PATROL, CHASE, ATTACK }
var current_state: State = State.IDLE

func _process(delta):
    match current_state:
        State.IDLE:
            _state_idle(delta)
        State.PATROL:
            _state_patrol(delta)
        # ...

func _state_idle(delta):
    if _detect_player():
        current_state = State.CHASE
```

## References

- Raymap source: `reference/raymap/Assets/Scripts/OpenSpace/AI/Brain.cs`
- Raymap source: `reference/raymap/Assets/Scripts/OpenSpace/AI/Mind.cs`
- Raymap source: `reference/raymap/Assets/Scripts/OpenSpace/AI/AIModel.cs`
- Raymap source: `reference/raymap/Assets/Scripts/OpenSpace/AI/Behavior.cs`
- Raymap source: `reference/raymap/Assets/Scripts/OpenSpace/AI/Script.cs`
- Raymap source: `reference/raymap/Assets/Scripts/OpenSpace/AI/ScriptNode.cs`
- Raymap source: `reference/raymap/Assets/Scripts/OpenSpace/AI/DsgVar.cs`
- Raymap source: `reference/raymap/Assets/Scripts/OpenSpace/AI/AITypes/AITypes_Hype.cs`
