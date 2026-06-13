# Astrolabe Plan

Astrolabe imports data from *Hype: The Time Quest* into inspection formats and Godot-oriented project assets. The project should stay focused on a clean C# core library, a practical CLI, and output that can eventually power a Godot recreation or compatible engine.

The game uses Ubisoft's OpenSpace Montreal engine, shared with *Rayman 2* and *Tonic Trouble*. Raymap, BinarySerializer.OpenSpace, OpenSpaceToolbox, and the other checked-in reference projects are the main research sources. Reference code is useful, but `Astrolabe.Core` should remain independent of Unity.

## Game Files

Users must provide their own legally obtained copy of the game. Astrolabe should continue to work from either a mounted disc directory or pre-extracted raw files.

Local testing currently uses the raw disc copy under `disc/`, especially `disc/Gamedata/World/Levels/astrolabe`.

## Current Direction

The main development focus is the reversible native-filesystem intermediate package for OpenSpace level data.

The intermediate package should make level data increasingly meaningful as normal files and folders. JSON should describe structure, relationships, pointers, names, and buffer formats. Binary files should remain available for dense arrays, preserved payloads, and genuinely unknown spans. Compilation must remain byte-perfect for unedited packages while being driven by the intermediate content rather than by original byte positions.

Implementation details and promotion checklists live under `notes/`. Stable package and file-format documentation lives under `docs/`.

## Near-Term Work

Start by reading `README.md`, `docs/intermediate-format.md`, and `notes/intermediate-type-checklist.md` to recover the current command surface, package format, and implementation checklist.

Finish promoting documented intermediate leaves into structured JSON, JSON-described binary buffers, and S-expression ASTs for AI scripts and behavior trees. Start with geometry/material data, then Perso/family/object-list data, animation/state machines, AI/script/DSG data, and sectors/collision.

Each promotion should preserve reversible compilation, expose useful names and pointers where possible, and keep unknown fields explicit instead of hiding documented structures in opaque blobs. Avoid exploding the package into thousands of tiny JSON files when a larger hierarchical JSON document can describe the same structure clearly.

## Later Work

Once the intermediate package has enough semantic coverage, use it as the source layer for Godot export. The Godot side should generate scene trees, mesh resources, materials, animations, scripts/interactions where practical, and enough metadata to keep improving the conversion without losing reversibility.
