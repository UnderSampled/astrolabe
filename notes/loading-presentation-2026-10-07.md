# Loading presentation — 2026-10-07

## Ownership and implementation

`godot/addons/astrolabe/ui/loading/screen.tscn` supplies a persistent native
`CanvasLayer` and editable Control tree. Its script only observes the
`LevelTransition` service. The level worker and this branch agreed on:

| Signal | Presentation response |
| --- | --- |
| `transition_started(from_level, to_level, entry)` | Show the destination and observed resource-loading state at 0.0. |
| `transition_progress(to_level, progress, phase)` | Display only the actual threaded `PackedScene` and dependency fraction for `resource_loading`. |
| `transition_completed(level, entry)` | Hide immediately. |
| `transition_failed(level, message)` | Replace progress with the supplied error and a dismiss action. |
| `transition_cancelled(level)` | Hide immediately. |

Progress is not a whole-transition percentage. There is no animation, delay,
percentage extrapolation, or scene/session logic in the presenter. The component
must be instantiated once beneath `/root/LevelTransition` or another persistent
host before the first transition. The application integrator owns that one-time
composition; this change does not edit `bootstrap.gd`, `demo.gd`, `new_game.tscn`,
the content exporter or the schema.
The heading translates the initial source caption `Chargement de la map` and
adds the actual requested level identifier.

## Original-source evidence

The local decompilation tree is `reference/decompilation/`, with project
`reference/decompilation/ghidra_project/Hype_nGlide_patch.gpr`.

- `functions/00401395_FUN_00401395.c`: `FUN_00401395` calls
  `CWnd::SetWindowTextA` with its input string.
- `functions/00402b60_FUN_00402b60.c`: the initial source loading caption is
  set to `Chargement de la map`.
- `functions/0043c8e0_FUN_0043c8e0.c`: level loading successively sets window
  captions for loading the level, always, waypoints, characters, dialogue
  initialization, paths, stencils, game interfaces, highlight data and map
  processing. This routine has no frame presentation, progress-bar or animation
  calls in its decompiled body.

The game data in `disc/Gamedata/World/Levels/` has no standalone loading image
or layout file. The generated main-menu backdrop comes from
`MENU/menu_princ.gf`; it belongs to the menu and is not treated as loading art.
No art was extracted for this feature because no dedicated loading art was
identified; the scene intentionally has no backdrop or substitute image.

The decompilation input
`reference/hype_patch/MaiDFXvr_bleu.exe` hashes to
`ed105a423f6e1bcf0460aa8f7357e2bae3783ca8aadfff8ef8a13b93d967d97d`, while the
private installed executable at
`/home/stephen/Games/hype-the-time-quest-win32/drive_c/Games/Hype/MaiDFXvr_bleu.exe`
hashes to `048ff4f4f0447a1f54a5a73ff588df78b484015c78df99a8f2d7acd3469df7da`.
Although the complete executable hashes differ, `objdump` disassembly matched
byte-for-byte at the relevant installed/decompiled virtual-address ranges
`0x401395–0x4013aa`, `0x402b60–0x4038af`, and `0x43c8e0–0x43cfff`. These include
the caption setter, initial map caption, and level-load caption stages; the
loading-status conclusion is therefore confirmed for the installed executable.
The rest of the binary is not claimed identical.

## Validation limits

The original game was not run. No broad captures, new harness or software
installation was performed. `git diff --check` passed. A direct Godot 4.7.2
headless launch of `screen.tscn` exited successfully and parsed the native scene
and script; it reported the expected missing `/root/LevelTransition` warning
because the signal owner is not in this isolated branch yet. A headless editor
filesystem/class scan completed, but Godot also reported sandbox socket and
editor-settings write errors, so it is not counted as a clean editor check.
Signal-by-signal runtime behavior still needs a focused check after integration
with the signal-owning worker. The feature has no dedicated source artwork or
in-frame layout evidence; the centered native status group is an editable Godot
presentation for actual threaded load progress, not a claim of original-screen
parity.
