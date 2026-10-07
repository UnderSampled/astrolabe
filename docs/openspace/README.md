# OpenSpace knowledge index

These pages document Hype: The Time Quest's OpenSpace Montreal data and the source evidence used to interpret it. The original engine and its decompilation are authoritative; Raymap is usually the first practical reference. Historical implementation results and dated experiments live in [`../../notes/`](../../notes/README.md).

## Package and memory formats

- The [file format overview](file-formats.md) and [complete catalogue](file-format-catalogue.md) describe package membership. The [[docs/openspace/sna-format|SNA framing]] and [[docs/openspace/relocation-tables|relocation tables]] explain roots, blocks, and pointer maps; [Fix-to-level relocations](fixlvl-rtb.md) documents cross-package targets.

## Scene, geometry, and behavior

The [[docs/openspace/scene-hierarchy|scene hierarchy]] preserves ordered ownership and placement. [Perso meshes and animation](perso-mesh-animation.md) and the [[docs/openspace/geometry-format|geometry records]] describe reusable character and mesh data. The [[docs/openspace/ai-script-format|AI script format]] and [lighting reference](lighting.md) cover behavior and rendering inputs; [sector environments and fog](environment-fog.md) explains environmental selection.

The [Hype armor catalog](armor.md) documents its Fix ownership, relocation-rooted records, and supported native gameplay operations.

The [Hype cheat-code catalog](cheat-codes.md) records all 272 observed text bindings, their engine action names, and which have manor/Fix AI references. The [hidden-lever investigation](../../notes/hype-cheat-exploration-2026-09-12.md) distinguishes live-tested effects from bindings and records the 36 teleport destinations.

The [save-game format notes](save-games.md) document the verified PC save envelope, SAI record layout, and current limits on mapping saved values to gameplay state.

## Media and rendering

The [CNT archive](cnt-format.md) contains [GF images](gf-format.md); the [[docs/openspace/texture|texture model]] retains palette, index, and mip information as needed. [Standalone audio formats](audio.md) and their [[docs/openspace/audio|useful model values]] preserve more than a playback waveform where required. [Material transparency](material-transparency.md) records the renderer-side interpretation.

- [Inventory presentation](inventory.md): source layout, category/item bindings, localization and native export boundaries.
