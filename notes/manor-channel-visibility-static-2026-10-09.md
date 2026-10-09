# Manor channel initialization: static correction

This isolated change starts from clean main
`681b53ea74fbc44bec107be5fe5176e367a92753`. It addresses the two channel2
initialization failures in the one authorized acceptance, without rerunning it.
No tests, builds, lints, formatter checks, exports, gameplay, original-game runs,
workers or pushes were performed. Main, generated assets, reference repositories,
game files, saves and user settings were not edited.

## Cause and asset evidence

Read-only acceptance evidence is
`output/integration-promotion-20261009/acceptance/engine.log:885` onward.
Level1238 Normal0 immediately calls `source_set_channel_active(2, false)` and
then selects Normal2. Fix524 Normal0 first requests action0, disables channel2,
and then selects Normal2. Both stopped because the previous API returned false
when no native channel node matched.

The saved generated Persos identify `level:/persos/world/0/160`
(`i_Event_Fleches`) and `level:/persos/world/0/162` (`iEventPourHype`), both in
`fix:/families/3`, named `event`. Both use the same family/configuration rig,
`characters/event__d3bd3244/e92559521091`, which declares only logical channel1.

The existing `output/research-manoir-rete/` schema35 snapshot has these useful
values. It was read as MessagePack data; no importer or converter was executed.

| Family action | Animation | Frame columns | Logical ID | Kind | Default visibility |
| --- | --- | --- | --- | --- | --- |
| 0 | `fix:/animations/23375` | one | absent, source sentinel255 | Object | 1 |
| 1 | `fix:/animations/23380` | one | 1 | Hidden | 1 |
| 2 | `fix:/animations/23386` | one | 1 | Hidden | 1 |

Each animation has one frame. Actions1/2 retain an attachment cue at frame0,
column0, configuration slot2. The family has all three action IDs; action0
continues to itself. The generated action code uses native AnimationPlayer
selection and `_transition_target` admits these actions directly.

The family snapshot SHA256 is
`65f28d00ea1f62d41162ea64fb062fc8f94989438e1c308234082b1f35386a7e`.
The two Perso snapshot hashes are respectively
`6f0fcc26b1a80355c4c574234e0f5259790970227a735830d3ab6a542eb6fd1c`
and `539aa24612c08ac0b5159ef0d2edece70ebd9af05704268a4ecc8d21934d78d9`.
This older useful-model snapshot agrees with the current generated rig; it is
supporting static evidence, not a fresh source import or regeneration result.

`src/openspace/level/records.rs::AnimationChannelWire` reads logical ID byte+10,
visibility byte+6, object slot u16+4 and kind byte+12. The importer maps255 to
absence and retains every other ID. `src/godot/animation.rs::bake` keeps
duplicate ID occurrences independently, and the rig unions channels from all
family actions. These declarations agree with engine `004330f0`. Raymap's
Montreal parser usefully establishes the frame/column structure but leaves
these field meanings partly unknown; its objectIndex byte declaration is not
the authority for the corrected Hype layout.

There is no demonstrated lost channel2 in this family and no ordinal/one-based
reinterpretation to apply. Requesting action0 cannot manufacture channel2: its
single source channel has the absent-ID sentinel. The API's presence requirement
was the failure; raw operands also needed the source's unsigned-byte narrowing.

## Original operation and later consumers

The paired source translator supplied reconstructed `00541880` in original-C
commit `93d7d9efcf859469338bd95e35d3eb0ae26e919c`, branch
`reconstruction/channel-condition-task20`, file
`src/actor/channel_activation_procedure.c`. Its complete span ends at return
`00541988`. The peer's task20 handoff/evidence records the inspected PE profiles
and input/span hashes; the handler and dependency spans match both profiles.
This Astrolabe change does not copy that source or claim new decompilation credit.

The handler captures the Ultra receiver's Data3D before evaluating the operand
with the original caller, then narrows the raw payload to unsigned8. The source
IDs255 and values beyond255 have no extra validation or special bypass.
Supported compiled operands here are already typed static channel literals,
so preserving their original u32 values in the model and narrowing in the shared
runtime API preserves source reconstruction and all supported receiver paths.

Proc105 calls `00462030` on Data3D+150's visibility override list, conditionally
clears the matched module's bit23, then always calls `0044c170` to insert the
byte ID in the ordered, duplicate-free Data3D+158 list. Missing module lookup
skips only the flag write. Proc104 removes an override via `00461fa0`,
conditionally sets module bit23 and always removes the disabled ID via
`0044c0e0`. Missing channels therefore still have owned state effects.

Existing read-only C provides the later semantics:

- `00462030` retains an existing disabled override, removes an enabled override
  and inserts a disabled override when the key is absent. `00462330` resolves
  a sample's default and removes an override when it agrees with that default.
  Its high bit denotes enabled visibility; older notes calling it a hide bit
  should not determine the implementation.
- `004330f0` uses byte+10 as logical identity, skips255 and invokes `00462330`
  for object-kind samples. Hidden/absent samples cannot consume an override
  intended for a later object sample. Frame-column hierarchy and cue addressing
  remain separate from logical IDs.
- `004499b0` is gated by StandardGame+10 versus the current frame stamp. Before
  animation update it calls `0044c220`, which reapplies every Data3D+158 disabled
  ID using the same override helper and conditional module flag write. This is
  an ordinary actor update, not a sector-change callback.

## Native implementation and scope

The actor owns `character/channel_visibility.gd`. Its ordinary Dictionary and
PackedInt32Array represent visibility overrides and persistent disabled IDs.
Every self, SelfActor, MainActor and named-actor activation/deactivation route
uses the same byte-narrowing implementation. Matching duplicate native channel
parents receive the immediate visibility change. An absent ID still updates
the component and succeeds. Missing `Channels` bindings, malformed matching
nodes and unavailable receivers remain errors; compiler diagnostics describe
that contract instead of requiring a particular channel node.

The exporter adds a current `sample_visibility` property to existing native
channel nodes through `character/channel.gd`. Native Animation value keys
supply0/1 for object samples and-1 for hidden/absent samples. Continuous native
evaluation with nearest interpolation consumes redundant overrides even when
the source default stays identical across frames. Part-selection keys are
independent of channel visibility, so source-default hiding does not block the
immediate activation of an already selected part. Opacity remains a separate
native transparency value.

Before native animation advance (including action selection) and explicit frame
seeking, the character reapplies disabled IDs. Godot's native nodes and tracks
own transform/part evaluation. No skeleton channels, module allocations,
linked lists, source frame tables or runtime asset interpreters were added.
The shared model and Rete schema remain unchanged; the model's channel statement
documentation now describes the real API contract. Support interface83 stages
both new scripts and their UID files.

This statically removes the two false missing-channel failures while retaining
the original state operations. It is not evidence that either Brain now runs
successfully, that generated resources parse, or that the whole acceptance
passes. Interface installation, regeneration and all verification remain with
the integration lead under separate authorization.

The native projection still inherits the existing parent-visibility treatment
of attached descendants. Related-animation interpolation, hidden/light payload
rendering, arbitrary source module caching and source channel-state save
conversion remain outside demonstrated coverage. The current-state property
uses the supported native animation path; exact original render-frame timing
and same-ID multi-occurrence behavior have not been exercised by this change.
