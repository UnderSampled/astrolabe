# Architecture implementation checkpoints

This file retains implementation details removed from the stable architecture pages because they describe a particular state of work. Current coverage and priorities are maintained in [the implementation report](implementation-progress.md).

## Rust application status observed in the original architecture draft

The primary CLI workflow was defined as `astrolabe port <game-directory> <godot-project>`, while project initialization, support updates, and content regeneration were assigned separate ownership. Supporting operations were named `import-openspace`, `export-godot`, `export-openspace`, `inspect`, and `formats`. The original draft described these operations as implemented only for an initial slice and noted a provisional preview arrangement; consult the current implementation report and CLI help for present coverage.

Read-only tool discovery on 2026-09-12 found Rust/Cargo 1.98.1 and Godot `4.7.2.stable.arch_linux.ed1daf0bf`, with Lutris and Wine also on PATH. PATH discovery alone did not establish a validated game installation or working harness.

## Earlier Rete and geometry checkpoint

An earlier Rete design snapshot used schema 2 to store reusable `Model` geometry separately from `ModelInstance` placement, selected LOD distance, and radiosity. Instances referenced shared models, and dependency validation checked lighting length against the model. At that checkpoint the importer selected one rooted instance; broader scene traversal was still planned. Synthetic shared instances exercised the boundary through Rete, source fragments, and Godot. The current geometry/instance boundary remains part of [Rete design](../docs/rete/rete-design.md); do not use the old schema number or coverage statement as current status.
