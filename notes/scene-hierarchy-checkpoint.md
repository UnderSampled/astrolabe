# Scene hierarchy implementation checkpoint

This note records a dated implementation snapshot. The stable SO field meanings and ownership rules are in [the scene hierarchy guide](../docs/openspace/scene-hierarchy.md).

At this checkpoint, Rete schema 9 persisted the complete supported SO record and typed bound references. The focused run passed all 20 selected library/scene-graph tests in 46.45 seconds including compilation and cleanup, recorded in `output/scene-graph-focused-tests.log`.

The API was not yet composed into CLI scene assembly; complete Perso, sector/environment and package layouts remained dependent work. The CLI emitted supported leaf fragments including node picking bounds. Per-node fading was retained and diagnosed but not projected by the Godot compatibility renderer. Original-engine acceptance of a rebuilt whole scene remained unverified. No reference submodule or original game asset was modified.
