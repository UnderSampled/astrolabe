# Astrolabe Godot project

Open `project.godot` with Godot **4.7.2**. The bootstrap and synthetic example
work without Hype data. `addons/astrolabe/` is maintained converter support;
`game/` is independently authored content. All extracted/generated content and
Godot caches are ignored by location.

From the repository root, with the supplied English Hype disc under `disc/`:

```sh
cargo build --release --locked
target/release/astrolabe port disc godot --level astrolabe --locale en
```

The selected level is `generated/hype/levels/astrolabe/level.tscn`. Its optional
`preview.tscn` supplies camera/inspection controls. Open level and perso scenes
directly to inspect their saved contents and AnimationPlayer libraries.

To adopt this structure elsewhere, run `astrolabe init-godot PROJECT` once.
`port` also initializes a missing project. Project settings and authored files
belong to the adopter after initialization. Use `update-godot-support` for an
explicit support update. Ordinary content regeneration preserves these files
and unselected generated levels, and rejects modified generated files.

Build variations by instancing generated scenes in `game/levels/` or
`game/characters/`, adding your own nodes/scripts/resources there. Commit only
independently authored work. Such compositions need their documented generated
dependencies rehydrated before they can open fully. Editing a generated file
directly causes a regeneration conflict; move a deliberate independent copy
under your ownership if it should stop receiving converter updates.
