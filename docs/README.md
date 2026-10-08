# Documentation map

`docs/` explains the current system and source knowledge: what the formats mean, what the Rust model represents, and how the target adapters are designed. Date-specific experiments, progress reports, implementation checkpoints, handoffs, and open plans live in [`../notes/`](../notes/README.md).

## OpenSpace knowledge

[`openspace/`](openspace/README.md) indexes Hype's OpenSpace Montreal formats and observed semantics, with provenance and confidence stated in each topic. The [[docs/openspace/sna-format|SNA framing]] and [[docs/openspace/relocation-tables|relocation tables]] pages explain how records and pointer maps fit together; the [cheat-code catalog](openspace/cheat-codes.md) records the observed input bindings. Raymap helps navigate the format; engine/decompilation evidence has higher authority when interpretations disagree.

## Astrolabe design

[`astrolabe/rust-architecture.md`](astrolabe/rust-architecture.md) explains the shared model and modular OpenSpace, Rete, and Godot adapters. [`astrolabe/contributing-rust.md`](astrolabe/contributing-rust.md) maps the code and extension workflow. [`astrolabe/godot-project.md`](astrolabe/godot-project.md) covers project ownership and regeneration. [`astrolabe/ui.md`](astrolabe/ui.md) defines maintained screen-space UI, including the loading-presentation boundary. The [[docs/astrolabe/testing|testing guide]] describes validation practice. Current coverage and changing priorities are in notes and [`../plan.md`](../plan.md).

[`astrolabe/session-recordings.md`](astrolabe/session-recordings.md) defines complete-session evidence, initial save-file fixtures, independent OpenSpace/Godot harnesses and extensible recording comparison.

[`astrolabe/probe-workflow.md`](astrolabe/probe-workflow.md) gives the preflight and ownership rules for temporary Godot and original-game probes.

## Rete design

[`rete/rete-design.md`](rete/rete-design.md) describes Rete as persistence for the useful model. [`rete/cross-package-uris.md`](rete/cross-package-uris.md) describes the current package-role reference contract. The model distinguishes shared Fix, active Level, texture, and sound references. Earlier address-named layouts are preserved in [[notes/historical/cross-package-uris-csharp|the C# URI proposal]]; other historical implementation notes are under [`../notes/historical/`](../notes/historical/).

## Orrery design

[`orrery/controller.md`](orrery/controller.md) describes the OpenSpace runtime analysis tool, its inspection/control interfaces and its ownership boundary. Recording/replay extends that tool; the behavior suite remains an external consumer. [`orrery/comparison-model.md`](orrery/comparison-model.md) explains the distinct data, behavior, parity, and package-acceptance claims. The [[notes/orrery-live-session-2026-09-23|live session record]] captures the tested controller operations; scenario integration work is in the [[notes/realtime-comparison-plan|comparison plan]].
