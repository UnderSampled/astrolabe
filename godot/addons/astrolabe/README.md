# Astrolabe support, interface 1

This directory is the authoritative source distributed with the Rust exporter.
It depends only on Godot 4.7.2. Generated assets use explicit resource paths.
No autoload, disc, Rete store, Rust process or checkout path is needed at runtime.

`rendering/sprite.gd` selects between persisted native billboard views using a
camera-facing dot product. Its thresholds, normal and child meshes are native
scene properties. Advanced OpenSpace sprite orientation/UV modes remain partial.

`scene/sector_boundary.gd` offers segment intersection with persisted triangle
boundaries. Its saved ArrayMesh is only a hidden inspection overlay; it does not
implement sector activation or sound propagation.

Source checkout support is maintained through version control. Adopted projects
update it explicitly with `astrolabe update-godot-support PROJECT`. Content
regeneration checks the interface version and validates the installed support;
it never replaces local support edits.
