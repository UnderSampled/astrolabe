# Rooted Hype model import: evidence and supported reconstruction

Research/implementation note, 2026-09-12. Consulted Raymap first, then checked discrepancies against the patched Hype executable decompilation and local source bytes. The Rust importer follows real GPT roots and relocations; it does not discover models by scanning for plausible vertex buffers. Original-engine rendering was not run for this work.

## Verified source route

`astrolabe.gpt + 0x10` → actual-world SuperObject → ordered child lists → IPO SuperObject (`0x0d` or `0x15`) → IPO's PhysicalObject → visual set → selected LOD → geometry → triangle elements → GameMaterial → VisualMaterial → TextureInfo → named GF content in CNT.

Raymap references, relative to `reference/raymap/Assets/Scripts/OpenSpace/`:

- `General/Settings.cs`, `PlaymobilHypePC`: Montreal engine, PC little endian, double linked lists, compressed SNA/RT envelopes, no pointer masking.
- `Loader/R2Loader.cs`, `ReadLVL`: Montreal GPT prefix and actual/dynamic/father roots.
- `FileFormat/SNA.cs`, `ReadSNA`, `CreatePointers`, `RelocatePointerFile`: block bases, local/global block ownership, and the difference between RTB and RTP/RTT.
- `Object/SuperObject.cs`, `Read`/`GetSOType`: hierarchy and IPO tags.
- `Object/IPO.cs`, `Object/PhysicalObject.cs`: physical object and LOD references.
- `Visual/GeometricObject.cs`, `Visual/GeometricObjectElementTriangles.cs`: separate source point and UV indices; triangle material is a **GameMaterial** for Hype, not directly a VisualMaterial.
- `General/GameMaterial.cs`, `Visual/VisualMaterial.cs`, `Visual/TextureInfo.cs`: useful material/texture relationship.

### Corrections established against the engine

These are actual discrepancies with historical notes and/or Raymap's general Montreal reader. Do not use their larger record lengths for Hype reconstruction: doing so reads neighboring allocation metadata as fields.

| Record/rule | Hype evidence | Consequence |
| --- | --- | --- |
| SNA header | Raymap `ReadSNA` reads the extra byte only **after** Montreal; real files parse at 6/22-byte headers | Module/id are bytes, base `i32` starts at +2. `docs/openspace/sna-format.md`'s +3 base is not correct for this fixture. |
| Geometry | [GEO_vCreateGeometricObject](../reference/decompilation/functions/0046c9c0_GEO_vCreateGeometricObject.c) allocates and zeros `0x30` bytes | The claimed sphere at +0x30/+0x34 in Raymap/C# is outside the record. Derive bounds from points. |
| Triangle element | [GEO_vCreateElementIndexedTriangles](../reference/decompilation/functions/00467aa0_GEO_vCreateElementIndexedTriangles.c) allocates `0x20`; count at +0x18, point subset pointer +0x1c | The C# / general-reader tail after +0x20 is not part of the Hype record. |
| IPO | [IPO_fn_hAllocInstanciatedPhysicalObject](../reference/decompilation/functions/004a4580_IPO_fn_hAllocInstanciatedPhysicalObject.c) allocates `0x08` | PhysicalObject pointer +0, instance radiosity pointer +4. |
| Radiosity ISI / LOD | [ISI_fn_hCreateISI](../reference/decompilation/functions/004a9c20_ISI_fn_hCreateISI.c), [ISI_fn_hCreateISILOD](../reference/decompilation/functions/004a9a00_ISI_fn_hCreateISILOD.c) | Each header is 8 bytes: signed-short count, initialized zero short, array pointer +4. Each color is four signed shorts. Raymap reads the counts as u32. |
| PhysicalObject | [PO_fn_hAllocPhysicalObject](../reference/decompilation/functions/004b89c0_PO_fn_hAllocPhysicalObject.c) allocates `0x0c` | Three pointers: visual set, collision set, bounding volume. |
| Visual set | [GLI_p_stCreateVisualSet](../reference/decompilation/functions/004864e0_GLI_p_stCreateVisualSet.c) allocates `0x10`; [GLI_vSetVisualSetLOD](../reference/decompilation/functions/00486af0_GLI_vSetVisualSetLOD.c) uses +8/+0xc arrays | Header at +0, LOD count +4, kind +6, distance/data arrays +8/+0xc. |
| Material texture | [GLI_xGetMaterialTexture](../reference/decompilation/functions/0048fa90_GLI_xGetMaterialTexture.c) reads +0x48 | Comments saying “0x4c” describe the cursor after the pointer. |
| TextureInfo name | [GLI_vGetTextureName](../reference/decompilation/functions/00482cd0_GLI_vGetTextureName.c) uses decimal 100 (`0x64`) | Read name at +0x64, up to the 80-byte field capacity. |
| SO transform roles | [HIE_fn_hGetSuperObjectMatrix](../reference/decompilation/functions/004661b0_HIE_fn_hGetSuperObjectMatrix.c) returns +0x20; [global getter](../reference/decompilation/functions/00466190_HIE_fn_hGetSuperObjectGlobalMatrix.c) returns +0x24 | Raymap's `off_staticMatrix` at +0x24 is actually the global matrix; global-at-T0 is at +0x38. |
| Full transform bases | [POS_fn_vMulMatrixVertex](../reference/decompilation/functions/004b97f0_POS_fn_vMulMatrixVertex.c) directly multiplies the last 3×3 at +0x34; [without-scale version](../reference/decompilation/functions/004b98d0_POS_fn_vMulMatrixVertexWithoutScale.c) uses +0x10 | The last matrix is the complete affine basis, **not** a scale matrix to multiply by the first. |

Full source SO matrices are 88 bytes: flags `u32`, translation 3 floats, unscaled basis 9 floats, full basis 9 floats. Flag bit `0x80` marks uncompressed storage; animation-style compact transforms are outside this import slice. The selected actual-world tree has local flag values `0x80`, `0xb0`, `0x1b0`, `0x1f0`, all uncompressed. The importer explicitly rejects other compressed SO forms.

## Relocation and package context

SNA: read `(module:u8,id:u8,base:i32)`; base `-1` ends that empty entry. Otherwise read four `u32` values: inclusive capacity end, free-list head, allocator high-water address and decoded size. These are established by engine `FUN_004e86e0`, `Mmg_fn_vInitBlock`, `FUN_004aeb10` and `FUN_004e8ad0`; see [SNA framing](../docs/openspace/sna-format.md). Positive size is followed by a 20-byte compression envelope and payload. Zero-size entries carry source-local bases and may refer to a populated shared Fix block. Source addresses must be interpreted relative to that source file's base, then resolved to the populated owner. A flat address-range lookup is insufficient.

RT files: byte group count; each group is `(module:u8,id:u8,count:u32)`; nonempty groups contain one compression envelope. Hype entries are 8 bytes: `value:u32,target_module:u8,target_id:u8,unknown:[u8;2]`.

- RTB: `value` is the **address of the pointer field** inside the owning block. Read the pointer's target value at that field, then use the entry's target block and source-local base.
- RTP/RTT in Montreal: `value` is the **pointer value appearing in GPT/PTX**, not a GPT/PTX byte position. Root resolution must match that value to a relocation entry.
- Load shared Fix and one active level in a session. Merge `fixlvl.rtb` into Fix pointer context; do not mutate a persistent shared family binding.
- The local fixture has 111 formerly sentinel Fix sites resolved by `fixlvl` (107 to `06:02`, 4 to `05:01`). Four `fixlvl` sentinel rows overlap valid `06:01` Fix mappings. Raymap only inserts valid pointer mappings: a sentinel must not erase a previously valid Fix mapping. Synthetic local/shared-base, null and invalid-reference tests cover the small contract.
- VM addresses below are test observations and diagnostics, never public model names or Rete references.

## Real `astrolabe` fixture observations

The source SNA SHA-256 is `0a678d2d15b5850e8847bfbfa13e2d22bfae3b4e1df099a0309ae844305bf772` (611,889 bytes). A scratch independent Python parser using the system `liblzo2` decoded all envelopes with exact declared lengths before the Rust implementation was tested.

| Populated level block | Source base | Decoded bytes |
| --- | --- | --- |
| `05:01` | `0x09d00020` | 904,349 |
| `06:02` | `0x09370020` | 1,360,321 |
| `11:01` | `0x09000020` | 103,461 |

The other six entries are zero-size shared-block views. RTB row counts are 51,616, 15,926 and 2,380 for these blocks respectively. RTP contains 86 entries; RTT contains 125.

| Root | GPT offset | Pointer value |
| --- | --- | --- |
| Actual world | `0x10` | `0x093f19f4` |
| Dynamic world | `0x14` | `0x093f182c` |
| Father sector | `0x18` | `0x093f1b4c` |

The actual-world tree contains **150 SuperObjects and 22 IPO instances**. Its first IPO in child order is `world/1/0/0`:

| Object | Pointer value |
| --- | --- |
| SuperObject | `0x093f1dfc` |
| IPO | `0x093f6bf0` |
| PhysicalObject | `0x093f6bfc` |
| Visual set (one LOD, kind 0) | `0x093f6c0c` |
| Geometry | `0x093f6c30` |
| Position array | `0x093f6c84` |
| Point-normal array | `0x093f6dc0` |
| Instance radiosity ISI header | `0x093f8a58` |
| Radiosity LOD table (one entry) | `0x093f8a64` |
| Radiosity colors (26 entries) | `0x093f8a70` |

The model has **26 points, 4 triangle elements and 32 triangles** (13/5/2/12 per element), plus 26 per-point GameMaterial references. First triangle indices are `[3,0,14]`; its separate UV indices are `[6,7,8]`.

The four texture names are:

- `astrolabe\2ze01txy.tga`
- `astrolabe\etoiletxynz.tga`
- `astrolabe\2reliefs5txy.tga`
- `astrolabe\boisnattxy.tga`

These are logical texture names, resolved to CNT members/GF content by the texture adapter. Their `.tga` extension does not imply a standalone TGA input. Model references use normalized paths such as `texture:/astrolabe/2ze01txy.tga`.

First point in source axis order is `[-7.135419845581055,-8.166790008544922,2.3007700443267822]`. The unchanged source position-array bytes hash to `2547f1d553a966352bfad4e91ad1204375c5797fbbdc509689e231bc67753874`; point normals hash to `95f8bee09620dc162b7df1617cfc5bb31937842c8802ab24eb6c043f7d9ded57`. These provide independently recorded expectations rather than only self-roundtrip tests.

## Useful model and implementation boundary

`Model` owns source positions/normals, selected instance/LOD radiosity, triangle groups with independent UV indices/values and face normals, per-point material references, selected LOD distance, and the root-to-instance transform chain. `Material` owns useful coefficients, texture reference, sampling sizes/flags/addressing, scroll parameters and sound-material identifier. `TextureInfo.source_name` retains the exact authored texture name (including `.tga`, spelling and case) independently of the resolved `.gf` asset reference. This is a source naming fact, not a cached pointer or encoded record; aliases may resolve to shared pixels while preserving distinct source names. Newly authored materials may omit it to derive a name from their dependency. Source export prefers this name rather than silently writing the canonical GF filename into TextureInfo. Shared material identity comes from importer identity maps, then file references. The Rete model contains no VM address.

Source numeric arrays stay in original axis order. Godot alone converts coordinates, winding and UV image conventions, and expands/reindexes corners where position/UV sharing differs. Do not normalize source normals or multiply the full transform basis by its unscaled basis. Local geometry bounds are derived from actual points.

Public functions:

- `inspect_level(game_dir, selection)` returns root/block/coverage observations.
- `import_first_model(game_dir, selection)` returns useful model/material assets and explicit diagnostic gaps.
- `write_sna_block(data, block, base)` derives a bounded nonempty SNA entry/envelope for controlled current data; its compact allocator header is explicitly not full game-package assembly.
- `write_geometry(model, material_targets, base)` derives a controlled geometry fragment and RTB from externally supplied current material layout.
- `write_model_fragment(model, materials, base)` also derives an IPO/PhysicalObject/one-LOD visual set, optional instance radiosity ISI/LOD/colors, GameMaterial/VisualMaterial/TextureInfo records, PTX and RTT. It needs no source directory or source bytes.

The fragment writer reconstructs supported fields and arrays, with generated sizes/offsets/addresses/relocations. Unknown material/texture runtime state is initialized and reported; spatial acceleration, collision, unsupported geometry fields, other LODs, full scene lighting/world records, full matrix reconstruction and original allocator bookkeeping remain outside this fragment. It is **not** a complete loadable SNA or a full-byte-parity claim. Initialized unknown fields must not be counted as reconstructed source coverage. Completing those fields requires further engine-informed work; no encoded source replay is used.

The source geometry's +0x0c array is a real per-point GameMaterial table, retained as meaningful references. The +0x20 spatial structure is unresolved in this slice and is explicitly outside reconstruction. This prevents confusing successfully rendered geometry with full source understanding.

## Level PTX layout and generated references

The original flat-word PTX scan was wrong: values such as 1024 and channel tags are metadata, not pointers. Raymap `MapLoader.ReadTexturesLvl` identifies the prefix/table distinction; engine [FUN_00483860](../reference/decompilation/functions/00483860_FUN_00483860.c) and [FUN_00483930](../reference/decompilation/functions/00483930_FUN_00483930.c) establish the exact level layout:

1. Texture capacity (`u32`, 1024 for Hype).
2. Number of level slots (`u32`), followed by that many TextureInfo pointers. Fix owns the preceding capacity-minus-level-slots positions.
3. Loaded-texture count (`u32`).
4. One memory-channel word per full-capacity slot (1024 words).
5. Current memory channel (`u32`).

The local level has capacity 1024, 589 level slots (125 nonzero pointers), 560 loaded textures and current channel 2. Its PTX length is 6468 bytes. Every nonzero nonsentinel pointer in the declared pointer array must resolve through RTT; nulls are allowed. Metadata is never treated as pointers. Declared counts, truncation, trailing bytes, absent RTT entries and out-of-block targets are checked explicitly. The local Fix.ptx has a count-sized shape: capacity 1024, 435 pointer words, a following channel count of 435, 435 channel words, and a final current-channel word (3496 bytes total). A focused data audit resolved all 435 pointers through Fix.rtt to named TextureInfo records whose names match `Textures.cnt`. `FUN_00483bb0` writes count-sized arrays to `BINARY_TEX_GLOBALS.BIN`; it does not establish the Fix.ptx reader contract. The current parser intentionally handles level PTX only.

The controlled model fragment assigns its textures to a fresh no-Fix table: capacity/level slots both 1024, selected pointer slots first, remaining slots null, selected channels 1 and current channel 1. Four textures produce an 8208-byte PTX and four RTT entries whose values exactly match the generated TextureInfo addresses. Original global texture-slot/channel placement is not reconstructed or persisted. This table is a bounded fragment context, not a claim of complete level texture-runtime state.

## Per-instance radiosity

The first model's IPO +4 resolves through RTB to its ISI header; ISI +4 resolves to an 8-byte-per-LOD table; selected LOD +4 resolves to the exact color buffer. The importer follows these links and requires selected LOD/vertex counts to agree with geometry. It does not infer colors from a nearby allocation, supply white defaults, or read the allocator padding between records. Nonzero reserved header fields, invalid signed counts, missing relocations, and truncated buffers fail explicitly.

Raymap's `Visual/ISI/{Radiosity,RadiosityLOD,ColorISI}.cs` describes the hierarchy, and `GeometricObjectElementTriangles.cs` indexes colors by original geometry position before corner expansion. However, the IPO's radiosity loading and vertex-color assignment are commented out in that snapshot. Its `ColorISI` property also clamps values. The engine is authoritative: [ISI_ColISIToGEO](../reference/decompilation/functions/004aa3b0_ISI_ColISIToGEO.c) converts all four **signed** shorts to floats with scale **1/256 without clamping**. The create/get functions above establish signed 16-bit counts and exact allocation sizes.

`Model.radiosity: Vec<[i16; 4]>` retains those meaningful quantized lighting values, one per source position, or an empty vector when absent. `radiosity_rgba(index)` presents an unclamped float vector. The current selected-instance `Model` also owns its transform chain; if later scene import separates shared geometry from instances, these colors must stay with the instance/LOD that owns them.

An independent local Python/liblzo2 pointer traversal observed 26 colors (208 bytes), SHA-256 `46a6eb8f51f9a3607ec340d87cdcccea481aed6e7a99575b8c14a6c7c9019fa5`. The first value is `[133,41,0,0]`; entry 2 is `[143,52,-57,0]`; entry 11 is `[396,48,-238,0]`. Negative and above-one values are real source data. All fourth components happen to be zero for this fixture; they are retained rather than dropped or interpreted as zero opacity.

These values are lighting contributions. [ISI_fn_vAmbientLight](../reference/decompilation/functions/004aa5e0_ISI_fn_vAmbientLight.c) accumulates ambient RGB through the same signed representation. [FUN_00492c50](../reference/decompilation/functions/00492c50_FUN_00492c50.c) transfers RLI to the renderer's lighting buffer. [FUN_004948c0](../reference/decompilation/functions/004948c0_FUN_004948c0.c) derives material lighting coefficients, and [FUN_0049d150](../reference/decompilation/functions/0049d150_FUN_0049d150.c) combines per-vertex contributions with those coefficients and ambient terms before clamping output. Render flags and dynamic lighting affect this calculation. Multiplying texture pixels directly by raw RLI or treating its fourth component as material opacity does not reproduce the full engine. Godot owns the presentation/approximation; retaining exact RLI is not a claim of full scene illumination parity.

For the static material-lit branch, with dynamic terms omitted, a source-informed preview is `RGB = clamp(2 * (material.ambient.rgb + material.diffuse.rgb * RLI.rgb / 256), 0, 1)`, followed by texture modulation. The engine multiplies material coefficients by 510 and clamps to `DAT_005ed910`; reading that constant directly from the patched executable's PE section gives float 255.0. Effective material flags are the material flags intersected with renderer context flags: this branch requires `0x200` and `0x20000000`, while loading RLI also requires context `0x1000`. The selected materials have both material flags, but complete context flags and dynamic contributions are not modeled here, so applying the formula in Godot is an explicit static approximation. All selected diffuse RGB values are approximately 0.698039; ambient RGB is zero except the second material, which has approximately 0.254902. Keeping alpha 1 in this preview avoids confusing the zero fourth lighting component with opacity.

The source-free fragment writer emits an 8-byte IPO wrapper, an ISI header, one selected LOD record, and the current color values. It derives counts, alignment, offsets, pointers and RTB rows. `ModelFragment.instantiated_object_offset` identifies the emitted wrapper for the caller; no source address is retained in the model. The unchanged color payload is byte-exact; after changing entry 2 blue from -57 to -128, the output contains the edited signed short. Full original allocation order and other LODs remain outside this controlled fragment.

## Validation

`cargo test --lib openspace::level` covers declarative record sizes, generated-layout pointer updates after resizing/editing, separate UV mapping, SNA framing around semantic geometry, cross-block references after decoding emitted containers, explicit Fix-to-level and Level-to-Fix resolution, null/sentinel handling, conflicting/missing relocation sites, and actual PTX table bounds. Add `-- --include-ignored` to run the local independent-observation test: complete decoded level block hashes and RTB/RTP/RTT row hashes match independently recorded Python/liblzo2 results. Nine tests, including this opt-in source check, took 0.16 seconds after compilation.

`cargo test --test level -- --ignored` runs the local user-data test (set `ASTROLABE_GAME_DIR` to override `disc/`). It validates the source hash, rooted counts and dependency names, independent array-byte expectations, actual Rete serialization, material/model edits, deterministic source-free fragment export, generated texture-reference files, and 75 derived pointer sites for the selected model. These include the four generated IPO/ISI/LOD relationships. The test also checks exact source radiosity bytes, a meaningful negative-component edit, mismatched vertex/color count rejection, authored texture-name survival after resolving to a GF alias, independent point-normal bytes, actual source PTX reference count, and exact generated PTX/RTT-to-TextureInfo links. The real fixture run took about 1.2 seconds after compilation in the development environment. It is deliberately opt-in because game data is not distributed with the tests.

No original-engine load or screenshot comparison was performed for this slice. Generated-fragment success does not claim runtime acceptance of modified packages. Godot import/rendering validation belongs to the composed application test.
