# Cross-package references in the Rust model

OpenSpace assets have package context. A level refers to shared Fix assets, while Fix content can refer to records supplied by the active level. The useful model represents those relationships as typed asset references; it does not persist OpenSpace virtual addresses as Rete paths. The Rete adapter resolves references against package roles and asset identities.

The stable roles are `fix:/`, `level:/`, `texture:/`, and `sound:/`. `fix:/` and `level:/` distinguish the shared Fix root from the active Level root without naming one particular level directory in shared content. `texture:/` and `sound:/` identify shared media assets through typed model references and package context. These roles do not prescribe PNG/WAV sidecars or a particular Rete path layout.

OpenSpace virtual addresses, block identity, and relocation sites remain private to source import/export. Import resolves them into model references. Export derives pointers and relocation tables from the current model and verified format rules. Rete load and Godot export do not reconstruct SNA memory layout.

A level session binds shared Fix references to one active level. Batch conversion may select levels sequentially, but shared Fix assets must not cache a global level binding. The broader [[docs/astrolabe/rust-architecture|adapter architecture]] keeps these role references distinct from source-memory details. See [the Rete asset model](rete-design.md) and [OpenSpace relocation notes](../openspace/fixlvl-rtb.md).

This contract describes the current architecture. The older address-named slot files, JSON fragments, image/audio sidecars, and union-manifest plan are retained in [`../../notes/historical/cross-package-uris-csharp.md`](../../notes/historical/cross-package-uris-csharp.md) as C# research history, not as Rust requirements.
