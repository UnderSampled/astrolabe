# Semantic dual-layer framework

Shared contract for non-animation domain aggregation. **Animation** (`animation/families.json` + `transforms.json`) is the reference implementation; all other domains follow the same shape.

## Dual layer

| Layer | Where | Role |
|-------|--------|------|
| **Authoring** | Domain document (`scene/tree.json`, `geometry/meshes.json`, …) | Nested ownership, human-readable graphs, URI edges |
| **Stream** | `sna/.../content.json` **`astrolabe.sna-block-content.v2` segments only** | Ordered leaves + `kind: expand` of runs; array order is stream order |

**Forbidden:** VM pointer integers as reconstruction plan; file-per-struct forests as primary authoring view; JSON float arrays as dense payload; **v1 `elements[]` content.json** (no read/write fallback — re-import).

## Documents

| Domain | Path | Schema |
|--------|------|--------|
| Scene | `scene/tree.json` | `astrolabe.scene-tree.v2` |
| Geometry | `geometry/meshes.json` + `geometry/buffers/*.bin` | `astrolabe.geometry-pool.v1` |
| AI | `ai/models.json` + `ai/scripts/*.sexpr` | `astrolabe.ai-pool.v1` |
| Characters | `characters/persos.json` | `astrolabe.character-pool.v1` |
| Sectors | `sectors/sectors.json` | `astrolabe.sector-pool.v1` |
| Sidecars | `sidecars/level.json` | `astrolabe.level-sidecars.v1` |
| Animation (done) | `animation/families.json`, `transforms.json` | existing |

Each pool doc has:

- `byId` — streamable leaves (`id`, `kind`, `record` / `bufferPath` / `sexprPath`, optional `children`)
- `runs` — ordered id lists for `content.json` expand
- Optional authoring roots (`roots` / `namedRoots` / nested trees)

URI forms:

- `{doc}#/byId/{id}`
- `{doc}#/byId/{id}/matrix` (scene field)
- `{doc}#/runs/{runId}`

## Code map

| Piece | File | Responsibility |
|-------|------|----------------|
| Kind sets | `Rete/SemanticDomainKinds.cs` | Which codec kinds belong to which domain |
| URI helpers | `Rete/SemanticPoolPaths.cs` | URI builders + byId/run parse |
| Models | `FileFormats/Semantic/SemanticPoolDocuments.cs` | Document DTOs |
| Orchestrator | `Rete/SemanticDomainAggregator.AggregateAll` | Import post-pass order |
| Scene | `Rete/SceneTreeAggregator.cs` | Collapse `scene/**` forest |
| Geometry/AI/Character/Sector | `Rete/SemanticDomainAggregator` (+ domain hooks) | Pool + segment rewrite |
| Sidecars | `Rete/SidecarAggregator.cs` | GPT/PTX/SDA/SND → semantic + wire |
| Linearize | `Rete/SnaBlockContentLinearizer` | expand runs for all pool docs |
| Export wire | `Rete/SemanticPoolExport` + `ReferenceJson` | byId → codec/bin bytes |
| Hub scene | `Hub/ReteSceneLoader` | Prefer `scene/tree.json` |

Import hook (after animation):

```text
ImportPackage → RewritePointerReferences
  → AnimationTreeImporter.AggregateLevelPackage
  → SemanticDomainAggregator.AggregateAll
  → AnnotateOpaquePointers…
```

## Domain implementer checklist

1. Ensure kinds are in `SemanticDomainKinds` (or domain-specific set).
2. Authoring shape: nested docs + URI children; dense data only as descriptor + `.bin`.
3. Rewrite `content.json` to v2 segments; contiguous domain leaves → one expand run.
4. Delete legacy `types/<kind>/` micro-files for moved leaves.
5. Export path: `SemanticPoolExport` / domain export returns **byte-identical** wire for unedited import.
6. Unit tests on linearize + export only — **do not** run full disc parity (orchestrator does once at end).
7. List residual opaques (undocumented leaves) in comments or residual notes.

## Parity policy

- **No** full `astrolabe` import/export parity while implementing a single domain.
- One end-of-goal disc run; categorize failures; batch fixes/optimizations (SNA-only export, caching, etc.).
