# Astrolabe Intermediate Package Format

Astrolabe's intermediate package is the on-disk format emitted by `extract-intermediate` and consumed by `compile-intermediate`. It represents an OpenSpace level directory as native filesystem entries plus JSON metadata, while preserving enough encoded source data for byte-identical compilation when the package is not edited.

## Package Layout

```text
manifest.json
scene/
  <root-name>/
    <node-id>/
      node.json
      matrix.json
      static_matrix.json
      <child-node-id>/
sna/
  <sna-file-name-without-extension>/
    blocks/
      <block-key>/
        content.json
        elements/
          <order>_<kind>.bin
      <block-key>.encoded.bin
types/
  <kind>/
    <block-key>_<order>.<json|bin>
relocations/
  <rtb-file-name>.json
  <rtb-file-name>/
    blocks/
      <block-key>.encoded.bin
files/
  <loose-level-file>
semantic/
  scene-tree.json
  coverage.json
```

All paths stored in JSON documents are relative to the package root and use `/` separators.

## Manifest

`manifest.json` uses schema `astrolabe.level-intermediate.v1`.

The manifest is the package inventory. It lists:

- `levelName`: logical level name.
- `sourceDirectoryName`: source directory basename.
- `snaFiles`: SNA containers and their ordered blocks.
- `relocationTables`: RTB/fix relocation documents.
- `looseFiles`: non-SNA level files copied under `files/`.
- `semantic`: optional inspection outputs.

Each SNA block record stores its block identity (`module`, `id`, `key`, `order`), virtual base address, OpenSpace header fields, payload paths, payload hashes, and original storage metadata. Blocks with payloads normally have both:

- `contentPath`: path to a structured SNA block content document.
- `originalStorage.encodedPath`: path to the original encoded payload bytes.

The encoded payload leaf preserves the exact compressed or stored payload from the source SNA file.

## SNA Block Content

Each payload-bearing block has a `content.json` using schema `astrolabe.sna-block-content.v1`.

The document identifies the source SNA file, block order, block key, module, block id, virtual base address, original decompressed data hash, and an ordered `elements` array.

Each element contains:

- `order`: concatenation order within the decompressed block.
- `kind`: element serializer kind.
- `dataPath`: JSON or binary leaf containing the element data.
- `sha256`: hash of the serialized element bytes at extraction time.
- `labels`: parser coverage labels that produced the element.

The compiler rebuilds the decompressed block by serializing elements in `order` and concatenating the results. The content document does not store per-element byte positions or byte lengths.

## Element Leaves

Element leaves are either structured JSON documents or binary files.

Structured JSON element schemas currently include:

- `astrolabe.scene-node.v1`
- `astrolabe.super-object.v1`
- `astrolabe.matrix.v1`
- `astrolabe.geometric-object.v1`
- `astrolabe.physical-object.v1`
- `astrolabe.ipo.v1`
- `astrolabe.game-material.v1`
- `astrolabe.uint32-record.v1`
- `astrolabe.float3-array.v1`

`scene/.../node.json` is also a source element when an SNA content element points to it. Matrix files under `scene/` are source elements when referenced by SNA content.

Binary leaves are opaque byte sequences. They are valid content elements and are serialized directly into the rebuilt decompressed block.

`raw` and `padding` elements live under the owning SNA block directory. Parser-labeled binary elements live under `types/<kind>/`.

## Scene Tree

`scene/` stores a filesystem hierarchy for scene nodes that can be associated with the parsed OpenSpace scene graph.

Each node folder contains `node.json` using schema `astrolabe.scene-node.v1`. The node file contains SuperObject fields plus scene-facing metadata:

- stable node id and package path.
- scene root name.
- optional display name.
- optional matrix and static matrix paths.
- child node paths.
- parsed SuperObject fields such as type, data pointer, child list, sibling links, parent pointer, matrix pointers, draw flags, flags, and bounding volume pointer.

When a scene node or matrix file is referenced by a SNA content element, that scene file is authoritative for compilation.

## Relocation Tables

Relocation documents use schema `astrolabe.relocation-table.v1`.

Each relocation document represents one RTB-style table and contains ordered pointer blocks. A pointer block records:

- block identity (`module`, `id`, `key`, `order`).
- pointer entry size.
- hash of the decoded pointer block data.
- original encoded storage metadata.
- pointer records.
- optional trailing bytes as Base64.

Each pointer record stores the in-memory pointer location, target module, target block id, and the two preserved tail bytes used by Hype pointer entries.

The paired `.encoded.bin` leaf preserves the exact encoded pointer-block payload from the source relocation table.

## Loose Files

Files that are part of the level directory but are not SNA containers or relocation tables are copied under `files/` and listed in the manifest with size and SHA-256 hash. Typical examples include GPT, PTX, SDA, and small binary sidecar files.

## Semantic Inspection Files

`semantic/scene-tree.json` and `semantic/coverage.json` are inspection outputs. They are not source inputs for compilation.

The compiler uses the manifest, SNA block content documents, referenced element leaves, relocation JSON documents, relocation encoded leaves, SNA encoded leaves, and loose files.

## Compilation Rules

For an unchanged SNA block, the compiler reuses the original encoded payload when the serialized content hash still matches the manifest hash. This preserves byte-for-byte output for unedited packages.

If a SNA block's serialized content changes, the compiler writes the block payload uncompressed and updates the OpenSpace checksum fields.

SNA payload size fields are computed from emitted content. Header fields that are known derivations of emitted size are recomputed from emitted size.

For an unchanged relocation pointer block, the compiler reuses the original encoded pointer-block payload when the serialized pointer data hash still matches the relocation document. If pointer data changes, the compiler writes that pointer block uncompressed and updates its checksum fields.

Loose files are copied from their package leaves during compilation.
