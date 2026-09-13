# Scene and animation implementation decisions

Work for milestones 6 and 7, started 2026-09-12. Coverage is recorded separately
from these provisional design decisions.

The existing Model/ModelInstance split remains: geometry is shared by source
identity, while placement and radiosity belong to instances. A Scene owns its
ordered hierarchy and references those instances. Scene-local instances have no
ancestor transform chain; their owning nodes provide placement. Standalone
model exports retain the existing root-to-instance transform chain.

Rooted child order supplies provisional node IDs. Geometry and materials receive
the first referring content path, with transient address maps preserving sharing;
equal values at different source allocations remain distinct. No VM address is
persisted. A Scene is one MessagePack asset, independently shared models,
materials, collision geometry, families and animations are separate assets.
All three adapters consume these same values. Godot coordinate and lighting
adaptations stay in Godot; source records and generated pointers stay in OpenSpace.

Raymap is the initial layout reference: Object/SuperObject.cs, Object/Sector.cs,
Collide/GeometricObjectCollide.cs, Object/Properties/{Family,State,ObjectList,
Perso3dData}.cs and Animation/{AnimationMontreal,ComponentMontreal/*}.cs.
Its Montreal animation has sampled frames, per-frame channel records and
hierarchies. Dynamic parenting and object selection must survive the useful
model instead of being flattened into one static skeleton. Source interpretation
will be checked against the Hype decompilation and local corpus before claiming
support. Unresolved content must have explicit diagnostics.

Matrix pool identity and original order belong to the owning animation content;
compact form, byte alignment and pointer placement are derived when writing.
Fix assets use role references, resolved for the active level, rather than holding
a global binding to one level. These decisions remain provisional until actual
scene and animated-character routes exercise them.

## Source corrections and first environment route

- Hype `HIE_fn_hCreateSuperObject` allocates 0x40 bytes. The draw mask is
  +0x2c (`HIE_fn_lGetSuperObjectDrawMask`); +0x30 is the light pointer, not
  Raymap's flags field (`HIE_fn_hGetSuperObjectLight`). Bounding geometry is
  +0x34, the initial global matrix +0x38. Do not persist light addresses as flags.
- `GMT_fn_hCreateCollideMaterial` allocates four bytes and
  `GMT_fn_vCopyCollideMaterial` copies two shorts. Raymap's extra `typeForAI`
  word is outside this Hype structure. The game-material collision field can be
  null, the -1 default sentinel, or a property reference; preserve those distinctions.
  `FUN_00463df0` checks identifier bit 0x8000 when ignoring a physical collision
  response. Further gameplay responses still require the behavior domain.
- Static PO collision uses the fourth collision-set link (ZDR). Its GEO triangle
  records can retain UVs and vertex-material tables even though physics does not
  need UVs. Nullable vertex-material slots occur in the real environment; schema 3
  preserves them instead of inventing material references.
- `GEO_vCreateElementSprite` allocates an eight-byte header and 0x2c bytes per
  indexed sprite. `GEO_vSetCenterPointOfIndexedSprite` writes the first short,
  `GEO_vSetSizeOfIndexedSprite` writes +8/+0xc. This differs from Raymap's
  Montreal reader. The initial route diagnosed two Astrolabe IPOs (`world/1/8/2`, `world/1/11/2`);
  the later sprite slice below implements their basic geometry.

The initial route imports all 150 rooted nodes, 20 triangle IPOs, and their ZDR
collision dependencies. Nodes own local placement; reusable models and collision
assets are referenced. `--content scene` selects this route. The default `model`
selection remains the first-model regression fixture during development.
Collision fragments regenerate topology, UVs, normals, point subsets, collision
properties and sound-material IDs. Visual/mechanical material components, source
spatial acceleration and full package reconstruction remain outside that fragment.
This is ongoing milestone 6 work, not completed milestones 6 or 7.

## Original-engine observations during this work

Harness run `/home/stephen/Projects/hype-harness/local/runs/20260912-161045` used
the pinned English executable SHA-256
`048ff4f4f0447a1f54a5a73ff588df78b484015c78df99a8f2d7acd3469df7da`
and supplied save archive
`877576b5867b5ad44b41dc4b72f6b70bc89dbe5e202c86817f2858948a1d634c`.
The initial three-key load recipe stopped at the save list (phase 38); an extra
100ms Return loaded `manoir`, phase 9. A logged `navigate_level("astrolabe")`
memory transition reached the entrance corridor, phase 9. It changed the pending
level/transition state only, not exported asset data or saved files.

`screen-1789243966419264341.png` captures the entrance. Up for 700ms and Left for
450ms produced the subsequent movement/facing observations. Drawing the weapon
with x (100ms, 900ms gap) and Return (250ms, 100ms gap) produced the action images
`screen-1789244247552078066.png` and `screen-1789244248664457111.png`, with an
additional 800ms wait between them. Mana fell from 30 to 20. These are source
idle/movement/action baselines, not frame-exact animation comparisons or measured
collision trajectories. The game was stopped after capture.

`output/scene-preview.png` is the first actual Godot scene rendering. It shows
an exterior overview of the environment, unlike the source corridor viewpoint;
pixel/viewpoint agreement is not claimed. Godot focus/fly and explicit camera
arguments support the next comparisons. Native physics and direct/reopened-Rete
checks are in `tests/scene.rs`; record passing measured results before claiming
their acceptance.

## Shared geometry/materials and sampled character route

The source ZDR GEO equals the visual GEO for 18 of the 20 supported IPOs.
`CollisionGeometry::Model` now references the canonical Model in those cases;
only two IPOs own independent collision meshes. Material is one game-material
asset with optional visual data, sound-material ID, and collision properties.
There is no independent collision-material copy. A model/material edit therefore
changes both rendering and collision. The source-free collision fragment still
omits the visual/mechanical material components; the model fragment includes
visual and collision material properties.

The current real import includes 108 persos, 21 families, 17 configurations,
492 unique animation clips and two transform pools. The initial 366 triangle
models expanded to 436 total models with sprites (see below).
Hype's Fix-owned family has 389 states; the scene selects state 47 initially.
Names come from GPT object-type lists. Those lists need backwards traversal from
the tail: Raymap fills their missing forward links before reading names.
Family/template references use their actual Fix/level role; each perso retains
its initial/current/secondary state and initial/current configuration.

The engine's `FUN_004aeb10` establishes the allocator header: a u32 word count
including the header, with four-byte-aligned allocation payload. Walking the used
prefix assigns each referenced matrix its original allocation ordinal. All
204,927 channel matrix references in the imported unique pools match allocation
starts. The final free-block header is truncated in the saved used prefix. Pool
ordinals and distinct equal entries survive Rete; addresses, padding and encoding
choices do not. Matrix records regenerate in pool order, independent of the
order in which clips traverse them.

`POS_fn_hCreateMatrix` (004b8e50), `POS_fn_vDecompressPosition` (004bed70),
`POS_fn_vQuatToMat` (004bf250), `POS_fn_vSetScaleMatrix` (004baf20), and
`POS_fn_vFillCompressedPosition` (004c0990) corroborate compact forms. Quaternion
values remain unnormalized. Symmetric stretch retains all six components, unlike
Raymap's diagonal-only approximation. The decompiled zoom/axial expansion reads
unsigned words, while symmetric stretch reads signed words; the codec tests
include a zoom word above 32767. The matrix conversion keeps source orientation
and Godot applies its own coordinate change.

The current canonical encoder reproduces **204,474 of 204,927** imported compact
matrix records byte-for-byte. The other 453 have equivalent smaller forms:
24 identity sentinels, 294 translation-only, 112 rotation-only, and 23 unscaled
translation/rotation records. These are explicit pre-compression differences,
not a claim of exact source-package reconstruction. Allocation identities remain
separate even when their values become equal or identity. Controlled animation
fragments regenerate sampled channels, frame tables, hierarchy pairs, frame
rotation/translation data, speed matrices and pointers to rebuilt pools.

The temporary Godot character player loads generated native model scenes, changes
channel parents per sample, switches/hides individual parts without hiding their
child channels, wraps sampled playback, and uses the selected state's speed.
The viewer supports character/state selection, pause, and `--character Hype
--state 47 --frame 0` for repeatable pose captures. Native synthetic checks exercise
hierarchy reversal, removed parents, part scale/switch/hide, timing and stable
state selection. `output/hype-idle.png` is specifically **source state 47/frame 0**;
the filename does not establish that it is the original runtime's idle state.
`output/hype-state0.png` captures state 0/frame 0. They are diagnostic visual
captures, not accepted character/viewpoint parity. A temporary two-sided-material
comparison did not fix the remaining character differences; the generated
materials were restored afterwards.

Still unsupported in this route: advanced sprite projection; animation auxiliary events and
related-data links; movement application to the perso root; animation interpolation;
mechanical/AI behavior; full family/perso reconstruction; sectors/lights/fog and
scene sound routing. Frame motion values and speed matrices are retained, but are
not applied as unverified gameplay motion. The established original-game capture
above is the runtime reference, not acceptance of these generated fragments.

## Matched original-engine comparison and sprite/channel corrections

The second MCP run, `/home/stephen/Projects/hype-harness/local/runs/20260912-170705`,
uses the same executable and English-save hashes recorded above. It loaded the
supplied save into `manoir` (an extra Return at the save list was again needed),
then used the logged level-navigation intervention to enter `astrolabe`. No
asset bytes, saved games, or reconstructed content were patched in memory.

Read-only engine observations resolved the active module/block bases and checked
live objects against the imported source allocations. These VM observations stay
in the external harness logs and ignored research outputs, not in Rete. Timed
input is best effort, not frame-exact replay:

| Observation | Input and measured result | Source screenshot |
| --- | --- | --- |
| Idle | Current family state 47; clip has 50 frames, state speed 30, next state 47 | `screen-1789247289689717627.png` |
| Movement | Up 700ms; current state 32, 29 frames at speed 30; root moved from approximately (-24.355,-12.795,-5.061) to (-24.248,-11.390,-5.071). An 800ms wait returned to 47 | `screen-1789247560654262308.png` |
| Draw/action | x 100ms plus 900ms gap observed state 45; Return 250ms observed 290 (9 frames at speed 20), then after 800ms state 271 (20 frames at speed 20). Mana subsequently fell 30 to 20 | `screen-1789247602087586168.png`, `screen-1789247603198694918.png` |
| Wall contact | x 100ms/700ms gap, Left 600ms, then three Up holds of 1600ms each; forward input produced a wall slide | `screen-1789248392379007209.png` |

Four measured source root positions during the wall test were
(-24.341669,-12.617098,-5.061822), (-25.394295,-13.038147,-5.061503),
(-25.349222,-13.349561,-5.060037), and (-25.303896,-13.662743,-5.058564).
An independent native Godot ray probe at these coordinates finds the floor
0.0400 units below the root in each case. Forward rays at root height +0.75 find
the same wall with normal approximately (0.989950,0.000012,-0.141418) in Godot
axes, at distances 1.676410, 0.572244, 0.572825 and 0.573406. This supports
collision placement and the observed tangent direction, not equivalence of the
original character controller and Godot physics. See local
`output/original-collision-comparison.log` and the disposable probe in
`output/sprite-compare/collision_probe.gd`.

The active viewport camera was resolved through `FUN_00475040`,
`GLI_xGetViewportCamera` and `GLI_xGetCameraMatrix`, rather than guessing a view.
The inverse view matrix gives source camera position approximately
(-22.95,-4.04,-1.5), forward (-0.235956,-0.956087,-0.173844), with horizontal
FOV 1.5 radians and vertical FOV about 1.2197 radians. Godot's coordinate mapping
is (x,z,-y). The matched source capture is
`screen-1789249435283885513.png`; the Godot counterpart is
`output/original-camera-runtime-compare.png`. The framing, floor, walls, beams
and character placement agree visually. The Godot character is substantially
brighter, materials/filtering differ, and idle phase is not synchronized.

A reproducible diagnostic capture for this recorded state is:

```sh
godot --path output/channel-compare -- \
  --character Hype --state 47 --frame 0 \
  --actor-position -25.303896,-5.058564,13.662743 --actor-yaw -1.360568 \
  --hide-channels 28,71,80,136 \
  --camera -22.95,-1.5,4.04 --look-at -23.185956,-1.673844,4.996087 \
  --fov 69.8798 --near 0.25 --no-overlay \
  --capture /absolute/path/to/comparison.png
```

The hidden channels are **measured runtime overrides**, not converter defaults.
The original's P3d visibility list holds logical IDs 28, 71, 80 and 136 with the
hide bit; its saved source list is null. `FUN_00462330` applies this list to each
sample's default visibility and removes obsolete overrides. The development
capture uses the observed state explicitly. Without overrides,
`output/channel-idle-front.png` and `output/original-camera-compare.png` expose
extra effect props. Gameplay-driven visibility and lip-sync substitutions are
not yet implemented, and exported diagnostics state this. No claim is made that
source-default animation alone reproduces the running game's complete pose.

`FUN_004330f0` corrected the 16-byte Montreal channel declaration: the object
slot at +4 is **u16**, default visibility is byte +6, logical channel ID is +10,
opacity is +11, and kind is +12 (0 object, 2 hidden, 4 light, other retained).
Frame columns are not logical IDs. Some real frames repeat an ID; ordered
updates are retained rather than rejecting this source content. Source unknown
bytes remain explicit. Rete and regenerated records retain these fields; the
native test selects slot 300, changes column order and parents, switches/hides
parts, and tests state timing. Light/reserved payload rendering remains diagnosed.
The width correction is necessary for the corpus, but did **not** remove the
extra idle props: that separate issue is the runtime visibility list above.

The sprite slice now imports both previously unsupported IPOs and the character
sprite GEOs: **22 IPOs, 436 total models**, 22 ZDR collision bindings (20 share their render GEO; two are independent).
Each reusable sprite definition owns its parallel view values and material
references; a Model owns indexed placements and their center/size/constraint/UV
rectangle. The explicit element ordinal preserves mixed triangle/sprite order.
Definitions shared by placements remain shared through Rete and reconstruction.
The source UV rectangle is origin plus extent, not two opposite corners.

Engine declarations `GEO_vCreateElementSprite` (00467580),
`GEO_vCreateRotativSprite` (005be090), the setters at 00467960/004679e0/0046b490/
0046b4d0, and the renderer at 0047b200 corroborate the 8-byte group, 44-byte
placement and 24-byte shared definition. Godot projects basic camera-facing
views and selects by source thresholds, including the hidden-view flag.
Rotation/semi-look modes, constraints, UV overrides, blend/chrome and exact
sprite projection remain provisional and diagnosed. Source fragments regenerate
placements, shared definitions, view arrays, materials and relocations from the
canonical model. Synthetic Rete/fragment and native size/view tests cover these
contracts. Earlier same-camera captures `before-sprites-front.png` and
`sprite-idle-front.png` show added face detail; the initial missing-torso
hypothesis was not supported by the controlled comparison.

This is original-game observation plus a useful Godot comparison. It does not
establish acceptance of any Astrolabe-generated SNA by the original engine.

## Lighting/sector ownership review before implementation

The next slice keeps sector membership and neighbor relationships on scene-owned
sector components. Lights are independently shared assets: the same source light
appears in several sector lists, so copying its color/transform into each sector
would break editing and sharing. Node-attached lights reference the same assets.
Sector boundary polygons have sector targets in their material-shaped pointer
slot; they must not be decoded as game materials. Their spatial geometry and
links need their own useful component.

Raymap's Sector and LightInfo readers are the initial references, corroborated
against Hype's 160-byte sector and 200-byte light constructors/setters. The local
probe found 17 sectors and 54 unique lights in rooted-node/static/dynamic lists.
Signed color channels occur, including subtractive lights; negative values must
remain in Rete even where a target projection approximates them. The source
light on/off field and send-light switch are distinct. Static RLI must not receive
an additional copy of dynamic light contribution. A provisional Godot projection
must expose these limits rather than globally changing every material's shading.

A second wall-side action attempt confirmed that Return alone in state 47 did
not change state. After x (100ms plus 900ms gap), Return (250ms) again entered
290 then 271 after 800ms; captures `screen-1789249575845138508.png` and
`screen-1789249576938095063.png` show the weapon action and resulting effect.
The prior movement capture `screen-1789249547725623153.png` again observed 32.
Godot's `output/channel-movement.png` and `output/channel-action.png` inspect
these clips at frames 10 and 4, respectively; they are pose comparisons, without
AI/projectile simulation or synchronized animation phase.


The initial light slice is implemented: 54 Light assets and 17 scene-owned sector
components retain ordered static/dynamic light references, virtual-sector and
activation values. The fixed 200-byte binrw codec reproduces all source bytes
outside the derived matrix flags; a reread retains the exact useful values and
unknown fields. A dimmer reference is explicitly diagnosed and reconstruction
rejects it. Sector boundary/neighbor/audio/activity codecs remain outstanding.
The constructor/setters establish that light intensity updates the stored RGB;
the Godot adapter does not multiply it a second time by intensity_min.

Godot uses native vertex-lit materials for meshes without RLI, leaves baked RLI
variants unshaded, and restricts generated lights to the dynamic mesh layer.
Mixed-sign colors become separate positive/negative native light contributions.
Ambient light establishes the development environment's ambient value. Exact
attenuation, sector filtering, spotlight orientation, fog, animation of lights,
shadows and complete material response remain explicit projection gaps.
`output/original-camera-light-compare.png` shows a markedly closer character
brightness than the earlier unshaded capture, while preserving the environment
colors. It is still approximate visual agreement, not pixel parity.
The native test checks signed light nodes, transformed placement, cull masks and
one shared light referenced by a node and both sector lists. Pure tests check
source-free Rete edits, switch independence, signed colors and unknown fields.
An MCP read at the previously resolved ambient-light allocation returned enabled/send
switches and color (0.148,0.128,0.128,0.8). The next state check found the shared
game session in `monast1`; that read is therefore not an independently verified
active-level lighting baseline. No further Astrolabe-address reads or game
mutations were performed after detecting the state change.

Further sector getter corroboration corrects Raymap's list labels after +76.
Hype has **12** list headers: environment +4, surface +16, perso +28, static
lights +40, dynamic lights +52, streams +64, graphics +76, optional graphics
+88, collision +100, activity +112, sound interaction +124 and sound events
+136. Raymap omits optional graphics and compensates with three unknown words
at +136. Hype's `SECT_fn_hGetFirst*` getters at 004e0850 through 004e08f0 and
list insertion routines establish the actual ownership. Existing light offsets
are unaffected. Graphics entries have level/unknown shorts and a sector SO
pointer at +4, with next +8 (20 bytes). Collision/activity entries use target
+0 and next +4 (16 bytes); sound-interaction entries use target +0, an additional
word +4, and next +8 (20 bytes). Sound-event entries are 24 bytes: game-state byte +0, sector +4, event
handle +8, and next/previous/header +12/+16/+20. The sound getter at 004e0720
and game-state getter at 004e0730 establish these fields; resolve the event
through SND/RTS before traversing its domain. The 13 nonvirtual sector borders
contain triangle surfaces whose material-shaped references point to sector SOs.

The complete scene/light checkpoint passes **65 tests in 195.277 seconds**,
including release compilation, direct/reopened-Rete equivalence, native material
sharing, sprites, animation, lighting, collision, and all earlier media checks.
Report: `TestResults/20260912T220040.714895Z-851647.log.json`. A prior full run
failed because lighting material variants were duplicated per surface; the
accepted implementation caches one variant per source material and shading mode.

Scene audio research located the actual `.snd`/`.rts` files in
`disc/LangData/English/world/levels/astrolabe/`, rather than the main Gamedata
level directory. `astrolabe.snd` is 560 bytes (70 eight-byte entries), and RTS
is 299 bytes. `FUN_004e9e90` reads packed event ID plus pointer-site address,
relocates the site through RTS (`FUN_004eab90`), resolves the event through
`SND_fn_pGetBinEvent`, then writes its runtime handle into that site. These are
semantic event bindings to owning content fields; Rete must not retain the VM
sites. The editor library `disc/Sound/playmo_b.lcb` provides named event/resource
bindings, including Astrolabe music bank 103. The level startup function
`FUN_00402780` plays the event returned by `FUN_0040b490` when it is not -1.
SDA is a separate saved-data stream; do not infer its layout from the extension
or Raymap's reader, which reads beyond the actual 12-byte Astrolabe SDA value.

Sector expansion decision: keep sector boundaries and ordered local-node links
inside their owning Scene component. Borders use triangle groups with optional
neighbor-sector IDs, not materials or render models; the engine uses their
crossed faces to move between sectors (`SECT_fn_vTestChangeSector`, 004e04f0).
Graphics LOD links, optional graphics, collision, activity and sound propagation
are distinct lists. Resolve them against stable Scene IDs after rooted traversal;
never retain heap addresses in Rete. Reuse the existing GEO/triangle wire records
for topology, and emit a controlled sector fragment with caller-resolved external
SO/light targets. Environment/surface/stream and sound-event domains remain
explicitly outside that fragment until their own codecs exist. Godot should
expose boundary geometry and segment crossings for inspection, with no automatic
visibility/activation policy until camera and runtime routing are corroborated.

Sector implementation and original-engine comparison (2026-09-12):
- Source import retains 17 sectors, 13 border GEOs, 125 graphics links, zero
  optional-graphics links, 40 collision links, 203 activity links, zero sound
  propagation links, and 55 initial perso memberships. The supported links and
  all boundary positions, normals, triangles, UVs and subsets survive a generated
  layout and relocation-aware re-read. Synthetic tests cover sound propagation,
  unknown shorts/bytes, broken references after renaming, changed neighbor links
  and reordered activity lists. The writer is a controlled API with external
  SO/light layout inputs, not yet composed into the CLI's scene export.
- `SECT_fn_hResearchInWhatSectorIAm` (004e0020) uses downward intersection and
  activation bit 4; it is not general closed-polyhedron containment. Normal
  movement crosses border faces with `SECT_fn_vTestChangeSector` (004e04f0),
  then 004e0630 assigns the face's sector target. Godot exposes a two-sided
  `intersect_segment` inspection query plus hidden-by-default wireframes;
  it does not claim the source's one-sided/history/endpoint policy or automatic
  culling/activity behavior. The boundary never creates a physical collision wall.
- Hype MCP started fresh run `20260912-181801` (PID 881263, process starttime
  1372929), same executable SHA-256 `048ff4f4f0447a1f54a5a73ff588df78b484015c78df99a8f2d7acd3469df7da`.
  Loaded the supplied save with Down/Return/Return, 55ms holds and 800ms gaps,
  waited for phase 9 `manoir` plus four seconds, then used the logged
  `navigate_level("astrolabe")` transition. No asset data was patched.
- Resolved the live hero SO from global 007543d8, its Perso and +40 sector-info
  record, then enumerated all 17 live sector SOs under the father-sector node.
  Read-only traversal corroborated the twelve-list layout. Runtime memberships
  differ from saved initial memberships, so their active counts are not persisted
  back into imported assets.
- Atomic MCP sequence: verify phase 9/Astrolabe, read current sector/root,
  hold Up 1600ms (no release gap), read sector/root, capture screenshot, verify
  the level. The hero changed from source sector 12 to 13. Source coordinates
  changed from (-24.3552455902,-12.7951078415,-5.0605120659) to
  (-24.0554523468,-8.8576374054,-5.0755581856). Screenshot:
  `/home/stephen/Projects/hype-harness/local/runs/20260912-181801/screen-1789251697831850858.png`.
- The same endpoints in generated Godot find `world/1/13` on surface 0 of
  `world/1/12`, at fraction 0.5134422372 and Godot position
  (-24.20132,-5.068237,10.77344). Evidence:
  `output/original-sector-comparison.log` and disposable
  `output/sector-compare/original_sector_probe.gd`. This matches portal geometry
  and destination; it does not establish matching movement physics or acceptance
  of rebuilt SNA files by the original executable.

Audio follow-up research: the LCB's bank IDs combine events from multiple CSBs.
For example packed ID 0x00670009 is `S_DummyIA.csb^SndEventM:712`, while
bank 103 slots 0–2 are Astrolabe music. `_SND_fn_pGetBinEvent_4` at 004f21b0
uses the upper 16 bits as bank ID and the lower 16 as a 32-byte event index;
0xfffe in the upper half is a special editor/script lookup. The 70 Astrolabe
SND sites divide into module/block 17:1 (38 AI script fields), 5:1 (27 twelve-byte
event descriptors, whose first field is patched), and 20:1 (five localized
52-byte dialogue records at field +12). The latter block comes from `.lng`,
which shares SNA framing; main `.sna` alone does not contain it. The temporary
independent decoder probe is `output/sound-sidecar-probe.py` with JSON evidence.
These are source observations, not yet a scene-audio implementation.

A useful next step connects animation event arrays to those shared descriptors.
`FUN_00433e90` and `FUN_00434530` establish eight-byte event entries, with a
descriptor pointer at +0 and triggering frame byte +6. The animation header's
+16 count is **u8**, not the current provisional u32; +18 is a separate short
used by vertex morphing (`FUN_004330f0`). Descriptor kind byte +4 values 2/3
force skipped-frame playback to stop at the event. Do not interpret the whole
header +8/+12/+16 region as a linked-list header. Event descriptor identity is
shared with ObjectList parts; reverse references show twelve-byte sound records
used both by configuration slots and multiple eight-byte animation event arrays.
Promote this ownership once the event dispatch and remaining descriptor fields
are corroborated; source kind/unknown information must survive Rete.

Event dispatch is now located at `FUN_0043b910` / `FUN_0043bcd0` (0043b910,
0043bcd0). It walks the eight-byte array in playback order, reads frame +6 and
channel/frame-column +7, and checks the channel's runtime suppression state.
Descriptor kind +4 == 0 sends the first-word sound handle through
`FUN_004ec880(actorSO, DAT_0075ec70, event, 0, 0)`; a stop invocation calls
004ecc00 for that actor/bus. Kind 1 affects mechanics; kinds 2/3 operate held
objects/attachments; kind 4 controls a camera effect. Descriptor byte +7 is
used to mark the event's processed state. The preserved other descriptor bytes
need semantic naming before behavior is expanded.

The independent probe `output/animation-event-probe.py` inspected the 492
imported clip allocations: 304 contain events, totaling **593** event entries
(518 sound, 75 kind-2 attachment events). Header bytes +17/+18/+19 are zero
for all 492 fixtures, so the existing u32 count interpretation has not altered
these imports, but the declaration must still be corrected before adding event
support. Sound descriptors include two nonzero suppression-related bytes; these
must not be replaced by defaults. The probe retains only transient source evidence
under ignored output; it is not a second production parser.

The final sector checkpoint passes **67 tests in 225.55 seconds**, report
`TestResults/20260912T222959.258375Z-970703.log.json`, with no leaked children
and completed cleanup. Strict Clippy, formatting and diff checks pass. Visual QA
caught a nonpersistent ImmediateMesh wireframe after the first green run; using
ArrayMesh fixed it, and the native test now verifies saved line vertices.
`output/sector-wireframe-compare.png` shows the corrected cyan portals. The
original source movement segment is verified by the separate query comparison;
the wireframe image is a diagnostic view, not a synchronized character-pose
comparison.

### Implemented animation actions and cues

The ownership decision is now explicit: shared twelve-byte descriptors are
`AnimationAction` assets; `ObjectList` action slots and ordered `AnimationCue`
entries reference them. The real closure contains **147** actions (142 sound,
four attachment, one mechanics), **145** configured action slots, and **593**
cues in 304 clips. Of the sound actions, 140 have SND bindings and two are null.
The 101 actions referenced by cues are a subset of the complete configuration
closure. Slot kind -1 is absent, 0 is a model and 5 is an action. Model rendering
uses the engine's -1/0/4 discrimination in 004330f0; kind 5 is corroborated by
all of the action-slot descriptors and their animation-array reverse references.
Other configuration kinds fail explicitly until their content has a codec.

`SoundEventLink` identifies the shared library asset and its bank/member, with
an owner-specific lookup instead of generic direct-member hydration. The
OpenSpace adapter resolves SND receiving sites through RTS within the selected
Fix/level session. AI and localized-dialogue sites are not traversed by the
action codec. Original handles and site addresses are transient. Rete schema 4
stores the shared actions, cue order/frame/column/configuration-slot fields,
unknown descriptor bytes and corrected header fields (+16 u8 count, +17 unknown,
+18 u16 morph count). Source morphology payload reconstruction still rejects a
nonzero morph count.

The controlled writer emits shared action fragments per package owner and derives
SND/RTS from the chosen new layout. Animation arrays reference those generated
action targets. The CLI emits these fragments and the library alongside the
existing matrix/animation fragments. Tests compare all real descriptor
kind/unknown bytes, non-sound scalar payloads, and regenerated event IDs/RTS
sites. A separate synthetic test edits the sound member and cue frame after
Rete reload, preserving descriptor sharing and checking derived bytes.

Godot's temporary character player emits `cue_triggered(cue, action)` in source
array order, including state entry, skipped frames and complete loops. Visual
`apply_frame` inspection does not dispatch cues. Cue listeners see the triggering
pose, and a listener selecting another state cancels the old clip's remaining
tick. The action and resolved event name are available to runtime consumers;
automatic mixing, per-channel source suppression/reverse playback, attachments,
mechanics and camera effects remain explicit gaps. This is a timed-event slice,
not yet completed scene audio.

### Original walking-cue comparison

The Hype MCP was used again with the same verified executable hash
`048ff4f4f0447a1f54a5a73ff588df78b484015c78df99a8f2d7acd3469df7da` and
English SaveGame0 fixture. Fresh run
`/home/stephen/Projects/hype-harness/local/runs/20260912-185159` used PID 1108252
(starttime 1576714). Initial menu input stopped on the existing-save selection;
a screenshot confirmed the highlighted Gogoud's Manoir I slot, which was then
loaded with Return. After verified phase 9/manoir and a four-second wait,
`navigate_level("astrolabe")` was the only memory intervention. No content data
was patched.

The live hero and current-state pointers were resolved afresh. A 700ms Up action
selected Hype state 32. Its animation header reports **29 frames, 30 fps,
23 columns and two cues**. Read-only cue traversal found:

| Frame | Column | Configuration slot | Library event |
| --- | --- | --- | --- |
| 0 | 22 | 480 | bank 0/member 74, `PAS_Hype.csb^SndEventM:1` |
| 14 | 22 | 479 | bank 0/member 73, `PAS_Hype.csb^SndEventM:0` |

These agree with imported clip `fix:/animations/941`. Runtime descriptors point
to loaded 32-byte sound events. Their editor identities, operation and remaining
scalar fields match Bnk_0 entries 74/73; the resource field is relocated by the
engine and is not persisted as a pointer. Both operations are play (kind 1),
and the targets resolve to source resources 191/142 respectively. During a
subsequent sequence, runtime frames 7 and 9 report the next-cue cursor 1,
consistent with the frame-0 cue having been processed and frame 14 pending.
The final sample switched to another state. A subsequent uninterrupted 1100ms
Up hold captured state 32 at frame 26 with next-cue cursor 2, corroborating that
both scheduled cues had been processed. Timed-key durations are not frame-exact.

Local evidence: `output/original-animation-cue-evidence.json` and
`output/original-walk-cue-cursor.json`, and `output/original-walk-late-cue.json`.
This comparison verifies cue structure, identity binding and processed-cue
cursors, not waveform/mixer parity.

Further audio evidence for the next playback slice: `FUN_004ec350` computes
`1 + uniform(-parameter2, parameter2)` in signed 16.16 units for play events;
`FUN_004ec090` puts it in the first word of the sound parameter struct and uses
the same slot for pitch events (kind 6). Thus SetParam2 on EVT_SON_PLAY is a
pitch-variation range, not a delay. Footstep events store 0xB00 (0.04296875).
`FUN_004f3fd0` dispatches resource kinds 1/2/3/4/6/8/10/11 to separate codecs;
compound/random/selector resources must be represented before their pointers
are followed. The two footstep targets are composite material-dependent
resources, so choosing a fixed sample would not establish footstep parity.

The regenerated Godot project `output/action-compare` was probed with
`original_walk_cues.gd`. Selecting state 32 then advancing to frame 26 emitted
exactly the original frame-0 bank-0/member-74 and frame-14 bank-0/member-73
cues, with columns and slots intact. Advancing the remaining three frames
crossed the loop and dispatched frame 0 once. Native Godot evidence is
`output/original-godot-walk-cues.log`; this exercises the actual exported Hype
character and matching library identities, while audio mixing remains pending.

### Audio ownership decision for the playback slice

Extend the existing `SoundDefinitions` event/resource values with typed operations
and member links. A CSB owns its named sound program; an existing `SoundBank`
owns decoded waveforms. Event/resource/selector references identify whole owner
assets plus a named or numeric member, and the owning model exposes lookup.
The OpenSpace adapter canonicalizes case-insensitive source filenames and binds
sample definitions to the bank member or standalone APM which supplies their
waveform. Godot consumes that same useful program rather than reparsing CSB/LCB
syntax. Keep shared library bank/member identities for SND reconstruction.

Begin with play/stop, samples, weighted random choices and material switches,
preserving loop/control values and diagnosing unsupported program variants.
The real walking graph probes a shared Fix character, two library events,
material switches, reusable random groups and bank-owned samples; standalone
media remains usable through its existing API. This arrangement is provisional
until scene audio and source reconstruction are demonstrated together.

### Sound program playback and stop semantics

The useful audio model now represents play/stop operations, explicit event and
resource member references, sample controls, weighted random choices and SIF
switches. Rete schema 5 retains these values. OpenSpace resolves case-insensitive
source names, checks the LCB group/member against the BNM resource identity,
and binds whole bank assets to decoded waveform members. It rejects ambiguous
sample bindings. The local library has no duplicate resource bindings.

The corpus corrected an initial stop-target assumption: `EVT_SON_STOP` names a
`SndEventM`, not a `SndResourceM`. `FUN_004ebbf0` compares the voice's event
pointer at +8, populated by `FUN_004eba10`, while `FUN_004ebd60` either stops an
abortable voice or detaches a nonabortable one. The canonical stop target is an
event member. SCR 552 fixtures also require quoted `[ar]("reference",n)`
arguments, quoted decimal numbers and mixed-case booleans. These spellings stay
inside the source text codec; the Godot adapter consumes typed values.

The provisional Godot sound runtime projects one program file per definitions
asset and shared native waveform resources. It executes animation-triggered
play/stop, sample loops, weighted choices with no immediate repeat where
specified, selector defaults and explicit material selection, random pitch,
and spatial/nonspatial voices. Each actor owns random/selector state and voices;
source bank/member IDs remain inspectable. It emits a voice trace for development
checks. Source volume is provisionally mapped from the 0–127 range to linear
Godot gain; exact source loudness curves, nested-resource gain, attenuation,
reverb, automatic collision-material selection and live selector updates need
further comparison. Group loop flags do not cause one footstep invocation to
repeat forever. Unsupported operations/resource types are diagnosed.

`output/sound-compare` initially exported 2,706 generated files in 16.14 seconds,
including 178 shared waveform resources from six banks and six program files.
Its walking probe selected Stone explicitly and matched the original frame-0
member 74 and frame-14 member 73 cues in state 32, then the next loop's frame 0.
The selected paths went through right resource 29/random 31/sample 1 and left
resource 28/random 30/sample 0, with BNM identities 3735553 and 3735552. A seeded
third cue exercised no-repeat selection. Native AudioEffectCapture returned
5,120 frames with peak 0.90969 in that overlapping-cue probe. Evidence is
`output/original-godot-sound-probe.log` and
`output/sound-compare/original_sound_probe.gd`. This proves emitted waveform
playback and routing under the selected material; it does not establish matching
original mixer output or the original RNG sequence. The two imported looped
sample definitions end at 22,064 and 176,033 frames, agreeing with their decoded
BNM streams and native loop metadata.

Scene ambient emitters, AI/dialogue audio triggers, binary BNM event/program
compilation, full CSB behavior reconstruction and whole-level source assembly
remain incomplete. The animation sound slice is not full scene sound coverage.

A fresh original-game run at `local/runs/20260912-193116` used the same pinned
executable and English save hashes recorded above. The normal save recipe reached
`manoir`, followed by one logged level-navigation intervention to `astrolabe`.
After resolving hero/Perso/Perso3d pointers afresh, an uninterrupted 1,100ms Up
input reached state 32, frame 25, next-cue cursor 2. The header and descriptors
again gave 29 frames, 30 FPS, 23 columns, cues at 0/14 and event identities
0x390001/0x390000. Both play events retain 0xB00 pitch variation and true
stoppable/dynamic flags. This agrees with the Godot cue and pitch-range checks;
wall-clock inputs are not asserted to produce an exact animation frame. Guarded
MCP evidence is `output/original-sound-program-evidence.json`; no asset data was
patched and generated-package engine acceptance remains untested.

The synthetic two-waveform regression exposed a native Godot 4.7.2 crash while
importing WAV files concurrently (`propagate_notification` from a worker thread),
before the playback script ran. A standalone two-file reproduction failed with
threaded importing and succeeded with serial importing in 5.54 seconds total.
Logs are `output/wave-import-parallel.log` and `output/wave-import-serial.log`.
New generated projects and staging projects now set
[`editor/import/use_multiple_threads=false`](https://docs.godotengine.org/en/4.4/classes/class_projectsettings.html#class-projectsettings-property-editor-import-use-multiple-threads).
Existing authored project settings are preserved; projects retaining threaded
import may need the same setting with this engine build. This workaround does
not remove or change waveform fixtures or reduce test coverage.

### SDA classification correction from source and MCP

The `.sda` suffix does not identify an ambient-sound emitter table in this build.
Raymap initially suggested extra string globals, but its 20-byte expectation
cannot fit Astrolabe's 12-byte file. Engine `FUN_004e9360` opens the general raw
static-data stream; `FUN_004e94c0` reads it directly. The level load at 00402b60
reads its first two words into globals 00610c84 and 00610c90. Those globals are
used by rotating-sector and moving-surface support respectively. `FUN_00446790`
then calls 00446850, which reads the final word into 00745700, the `GPI_STRING`
text handle used by 00446890 for on-screen text. This is separate from SND/RTS.

The fresh MCP run above read 0, 0x13d4f008 and 7 from those three globals,
matching all 12 source bytes. The second value looks implausible as a count;
its retention does not authorize traversing a supposed array or treating it as
a pointer. Investigate its runtime relevance before promoting that field.
Evidence is `output/original-sda-globals.json`. No SDA codec is claimed here,
but an SDA-derived audio-emitter requirement would be based on a misclassified
file. Remaining ambient sound work must follow actual event/scene consumers.

The scene reconstruction route also checks localized library owners. After
source normalization, a reference can point to `sound:/Sound/English/voice.csb`;
LCB emission derives `voice.csb^SndEventM:…`, because the source language context
selects its directory. It must not copy the canonical `English/` directory into
a filename-only source argument. The synthetic library check and full scene
source-export route cover this distinction.

The accepted 70-test checkpoint is recorded in [testing.md](testing.md). A fresh
project at `output/sound-program-compare` then generated the same 2,706 files in
16.45 seconds (16.50 seconds including the bounded supervisor). The original
walking cue/Stone-footstep mixer probe passed again against that final project;
its log is `output/original-godot-sound-final.log`. Strict Clippy, formatting and
diff whitespace checks passed. This checkpoint does not complete milestones
6–7: ambient routing, remaining scene/family/Perso writers, fog/lighting coverage
and the other listed character/rendering gaps still require work.

## Family size correction and configuration reset snapshots

The next reconstruction audit found that Raymap's generic pre-R3 family tail is
too long for this Hype build. `FUN_00437270` allocates exactly 0x38 bytes. All 21
referenced families have an MMG size word of 15 (60 bytes including the four-byte
allocator header). Hype's live family at 0x2115f5c has the same allocation size;
the word at +56 is 17, the following state's allocation header. The former
`Family.properties` field incorrectly exposed that word's first byte. The paired
`FamilyWire` now reads 56 bytes, retains the three unresolved bytes after the
animation-bank byte, and removes the spurious property. Family bounds, extra
data and full state/Perso dependencies still need their own reconstruction work.

Raymap leaves ObjectList +16 unexplained. Hype's `FUN_004617c0` allocates a
24-byte header. `FUN_00461a60` allocates `count * 20` bytes at +16 and copies the
active table (+12) into it. `FUN_00461ac0` copies it back; `FUN_00442040` invokes
that restore during actor reinitialization. This is a saved reset configuration,
not a second unrelated asset or an array of opaque pointer integers. Two of the
17 imported configurations have reset snapshots, containing 507 and 497 entries.
All 1,376 active and 1,004 reset entries now use the same typed ObjectPart values.
Unknown header/entry bytes remain explicit. Shared model and action references
remain shared; mutable and reset tables can deliberately differ.

Fresh guarded MCP reads in run `20260912-193116`, PID 1339344, verify Hype's
507 active entries and its reset table are equal in the observed Astrolabe state.
The English executable and supplied save hashes are the same as the preceding
sound comparison. The family/configuration records, both table reads, hashes,
and zero changed slots are retained in ignored
`output/original-character-record-evidence.json`. Only the earlier logged level
navigation changed memory; these observations used read-only MCP calls. They
establish a source baseline, not acceptance of rebuilt SNA data or an observed
full actor-reset sequence.

Rete schema 6 preserves the optional reset snapshot and correct family tail.
The configuration writer regenerates both tables, scales, model/action links,
and RTB rows. Its root is detached from the intrusive family list. CLI scene
reconstruction now emits configuration fragments and assigns nonoverlapping
addresses to model fragments, so different PO dependencies do not collapse onto
one placeholder address. SNA fragment headers retain the derived base; neither
those addresses nor source VM observations enter Rete. Original scale allocation
sharing, enclosing family lists and full package assembly remain unsupported.

The temporary Godot player exports both configurations and exposes
`restore_configuration()`. It restores the object values and refreshes the current
pose without advancing time or firing cues. Each actor copies mutable values;
an edit cannot mutate the shared saved snapshot. The native animation test uses
a deliberately changed reset scale and checks the displayed part, unchanged
frame/cue count, and a second restore after another edit. This tests the object
table operation only; it does not claim complete AI/dynamics actor reset.

The focused source check passed:
all 17 configurations, 2,380 entries, scalar/unknown bytes, scale values,
reference targets and receiving-site relocation sets. Full-suite acceptance is
recorded in [testing.md](testing.md): 71 tests passed in 224.780 seconds.

A fresh `output/configuration-compare` project generated 2,706 files in 16.59
seconds including its bounded supervisor, with no leaked children or timeout.
The native `original_configuration_probe.gd` confirmed 507 active and 507 reset
entries and equal initial tables, matching the original MCP baseline. It edited
displayed slot 2 to scale (2,2,2), then restored (1,1,1), retaining idle state 47,
frame 0 and zero cues. This additional check passed in 0.65 seconds; its output
is `output/original-godot-configuration.log`. This is an actual generated Godot
node check, separate from the unobserved full original actor-reset behavior.

### Next family/state reconstruction decision

The dependency audit finds 18 distinct family bounding GEOs, each with one point
and a kind-7 indexed sphere. `GEO_fn_hCreateAndInitBoundingVolume` (00466440)
constructs precisely that representation; `FUN_00437dc0` assigns the family's
+44 bounding GEO to its perso SO. Reuse the existing canonical Collision geometry
and sphere codec as a shared shape asset referenced by `Family.bounding_geometry`.
Its role is determined by that reference; a bound does not become a physical
collider merely because the geometry type can also describe collision shapes.
This avoids a second sphere/GEO implementation. Godot should expose the bound for
inspection without changing physics. Family object-type IDs are explicit source
identity, independent of asset filenames. All audited family +48 extra-data
references are null; non-null values remain unsupported.

The state dependency audit corrects Raymap's apparent mechanics field at +44.
Hype's state has **64 bytes**, confirmed by the size/constructor at 0044a720 and
0044a730 and all 585 imported MMG allocations. The actual mechanics-card pointer
is **+48**, exposed by 0044a910/0044a920 and used by 00443690, 00403b90, 0043bcd0
and 0044c4a0. Offset +44 is an unclassified reference, null in this selection.
Offset +52 points to environment-sample lists (0044ae90/0044ae30), also null in
the selection. Both non-null cases are explicitly rejected. Speed is byte 59
(0044a8f0/0044a900); seven unresolved bytes at 56–58 and 60–63 are retained.

There are ten shared mechanics cards. The size dispatch in
`DNM_fn_xIdentityCardCreate` (00591464) gives 92 bytes for kinds 0/4/6/18, 16 for
3, 52 for 7, and 8 for 9/19. Named getters/setters at 005940b7–00595a9f identify
the character parameters for kinds 0/4/18, including mass, speed limits and
contact thresholds. Four character cards and one mass card are typed here;
five cards of kinds 3/6/7/19 retain explicit unpromoted bodies. Their allocations
contain no source relocation entries. These preserved bodies do not constitute
implemented physics. Godot exposes the selected state's profile for inspection.

Fresh guarded Hype MCP reads in run `20260912-200757` confirm Hype's family
bound is the same GEO referenced by its SO: source center (0,0,1.3), radius 1.7.
The current state +48 points to a kind-0 mechanics card with mass 120. The raw
results, executable hash, process identity, save fixture hash and navigation
recipe are in local `output/original-family-state-evidence.json`. Navigation to
Astrolabe is the recorded state intervention; asset data was not patched.


The fresh `output/family-compare` Godot project contains 2,734 generated files;
its supervised conversion completed in 17.30 seconds. Native
`original_family_probe.gd` then checked idle state 47 and family object type 1,
mechanics kind 0, mass 120, stairs step height 0.41 and maximum linear speed 50
against that MCP baseline. Its actual SphereMesh has center (0,1.3,0) after axis
conversion and radius 1.7. Showing/hiding it adds no CollisionObject3D and retains
state/frame. The probe passed in 0.71 seconds with clean process teardown:
`output/original-godot-family.log` and `output/original-godot-family-status.json`.
This verifies the selected profile and inspection geometry, not a character
physics simulation or acceptance of an exported original-game package.


### Scene/perso reconstruction boundary after the family writer

A source-allocation and relocation audit now prevents extending the old prefix
codecs as though they described complete Hype records:

| Record | Verified size | Astrolabe selection |
| --- | ---: | ---: |
| SuperObject | 64 bytes | 150 |
| Perso | 76 bytes | 108 |
| Perso graphics data | 444 bytes | 108 |
| StandardGame | 56 bytes | 108 |

The graphics size is returned by 00441da0 and used by 00441e50; StandardGame's
size is returned by 00438800. `HIE_fn_hCreateSuperObject` (004fc660) allocates and
initializes all 64 bytes. The old C# graphics codec covered only 32 bytes, and
Raymap's generic StandardGame layout extends beyond Hype's allocation. Neither
is a complete reconstruction recipe for this build.

HIE getters identify the local matrix at +32, computed global matrix at +36,
last-compute frame at +40, draw mask at +44, light at +48, bound at +52, initial
global matrix at +56 and float transparency level at +60. The last-compute
values in this source snapshot are 39/40; all transparency levels are 255.
Computed transform caches and frame stamps must be regenerated, not persisted
as layout/replay metadata. Initial pose semantics and matrix ownership/sharing
need an explicit model decision before the complete scene writer.

The graphics data contains an embedded transform at +24 (004425d0) and a global
matrix reference at +112 (00442590). Other receiving sites outside its supported
state/configuration prefix include +124, +324, +332/+336/+340, +380 and +384.
Several refer to engine dynamic allocations (relocation block 255/255). The
76-byte Perso also links dynamics, brain, collision, sector and other components.
These cannot be emitted as zero-filled padding or disguised as understood
content. Keep supported actor values/components in the canonical Perso and
independently shared content in their existing assets; do not create one public
asset per engine allocation just to copy those records. Scene assembly can use
typed dependency targets while each remaining component is promoted.

The bounded source audit is `scene_record_probe.rs`, called by the existing
character corpus test. Its raw receiving-site inventory is transient local
research output (`output/scene-record-audit.log`), not Rete metadata. This audit
was included in the 74-test GPT-binding checkpoint; it does not claim
implementation of the remaining dependencies.

### Active-level GPT placements for Fix-owned persos

The loader reads a count and Fix perso table at Hype GPT offset `0x2af0`, then
53 entries of an 88-byte transform followed by a level SuperObject reference.
It copies each transform to graphics+24 and calls `FUN_00438e10`, which replaces
StandardGame+12 and graphics+112 with this level's SO and global matrix. The
saved Fix backlinks can be stale, including out-of-range level addresses; this
is a missing loader binding step, not a relocation-base correction. Raymap's
later-engine Fix placement loop is useful context but skips Montreal.

`fix_bindings.rs` now applies those placements and pointer aliases inside each
`SourceSession`, without modifying shared Fix source blocks. Canonical scene
transforms therefore carry the active level placement through Rete and Godot.
A synthetic A→B→A test checks rebinding, clearing and unchanged shared bytes;
the corpus checks all 108 actors' SO backlinks, embedded local matrices and
global matrix aliases, including the 53 GPT overrides.

Fresh read-only Hype MCP observations in run `20260912-205121` corroborate the
count (53), table membership and Hype's three matrix/backlink aliases. The
executable and supplied English save hashes are unchanged from the established
baseline. The save loaded into `manoir`; a logged level-navigation intervention
selected `astrolabe`. No asset repair was made in memory. Raw process addresses
and results are transient in `output/original-fix-binding-evidence.json` and the
external harness log. Whole-scene reconstruction is still incomplete.
