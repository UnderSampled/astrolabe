# Hype inventory presentation

The source boundary imports meaningful inventory presentation into
`model::inventory::InventoryPresentation`. Rete stores that value and its original
texture/font closure. Godot exports native scenes and Resources ahead of runtime.
No virtual addresses, allocation lists, relocated pointer tables or source font
instructions are part of the portable presentation or runtime UI.

For the supplied PC Montreal profile, `Fix.gpt` stores pointer declarations and
`Fix.sda` stores scalar declarations separately. `0041bec0`/`0041c290` and
`004e94c0`/`004e97a0` corroborate these streams. The inventory scalar section
starts after 20 prior bytes; the typed reader includes initial category, colours,
category/item sizes and scales, description/detail positions and counter offset.
Pointer declaration slots 18152/18156/18160 identify the catalogue, categories and
six-slot row. These constants exist only at the OpenSpace boundary.

The SuperObject hierarchy (`Fix` block 6:1) uses type 19 for InvBox, type 18 for
instantiated inventory items and type 22 for SetOfSprites. `004a3ca0` and
`004a3dc0`–`004a3eb0` establish the 32-byte box, including neighbour references
and position. Root category positions are Cartesian centres. Child magic groups
store angle in degrees and radius; `00419750` resolves them relative to the
parent, including parent selection scale. The importer converts angle/radius to
relative pixels. Native controls retain that hierarchy and apply native transforms.

The original roots provide five categories, three magic groups and six horizontal
item slots. Their focus graph is persisted as native NodePaths. Selection scales
are 0.5/0.85 for categories and 0.75/0.75/1 for ordinary/equipped/current items.
Sprite list layers retain their order, original TextureInfo identity and explicit
size, with source defaults when the size is zero. The original background name
resolves through Vignette.cnt; category/item/font images resolve through Textures.cnt.

The Fix.sda presentation wire at file offset 272 stores category/item motion
distance and rate pairs. The inventory setup callback `00418380` names them
`BoxesDistance`, `BoxesScrollingSpeed`, `CasesDistance` and
`CasesScrollingSpeed`; `0041b500` applies the signed offset by elapsed
milliseconds and stops at the zero crossing. Timer type 0 is `timeGetTime()` at
1,000 ticks per second (`00502620`, `00502680`). The supplied profile decodes to
`[96, 0.3, 50, 0.7]`, so the native linear category transition lasts 320 ms and
the six-slot row transition lasts about 71.43 ms. Up/down move the category layer
by +96/−96 pixels; moving forward/back in the item list starts the row at
−50/+50 pixels. `AnimationPlayer` and `AnimationLibrary` resources keep these
motions editable in Godot. Directional input is held while either source
transition runs, and open/close resets its position. The source selection scale
and color changes remain immediate; the checked source routines do not establish
an inventory open/close tween.

The catalogue has 62 instantiated items. Stock pickup actions retain actual
category and item references at +8/+12 (`0045bf00`, `0045c350`, `0045c370`,
`0041c720`). The importer validates repeated bindings and publishes the 46
corroborated ordinary items. Twelve NotFound magic placeholders and four other
entries lack that binding and remain unsupported. Category membership is never
inferred from a name or fabricated native list. Runtime presentation ownership comes from admitted category slots in the
shared native inventory Resource. Sparse legacy saves retain their saved IDs in
`legacy_owned_ids`; a full catalogue quantity dictionary, including negative
sentinels, does not establish ownership. See the [gameplay contract](inventory-gameplay.md).

The inventory multilingual registration chain starts at pointer slot 18124.
Its 52 writable text sites identify item-label table slots. `004a2310` and
`004a2340` save/load values in this chain order. The first English block 10:1
region in Fix.dlg supplies those 52 values, followed by a different world-label
chain. CP1252 decoding preserves typographic apostrophes. IDs 80/81 are Crossbow
bolts/Modern Crossbow bolts, not swords. `00419ef0` suppresses negative counters
and availability value 1; other owned nonnegative quantities can draw a counter.

InventoryFont is decoded through the existing source bitmap-font geometry/UV
reader. A shared export-time helper writes a native FontFile cache. The fixed size
is set before its name to preserve all glyph caches. The inventory has 127 glyph
entries; it uses its own atlas and source projection. Dialogue keeps its existing
atlas, projection, outline and glyph values. No fallback font is enabled.

The custom `Inventory.ini` structure also declares HUD-adjacent status elements
(`JaugeArmor`, `JaugeMagic`, `JaugeSword`, `JaugePommeau`, era/time text and
background-fading settings). Those are not Fix.sda values: their actual layout,
asset bindings and drawing callback have not been recovered in the available
decompilation, and the installed data tree has no loose `Inventory.ini`. The
Fix.sda `text_position` and `stats_position` origins are now retained in the
useful model as `(480, 375)` and `(64, 64)`. They do not provide the missing
gauge/text layout or asset bindings. These remain explicit source-coverage gaps;
no stand-in bars, icons, time or era values are emitted.

The authored host owns session/pause and item actions. It calls the maintained
weapon-selection API for owned Weapon items and propagates unresolved admission
explicitly. Source timing becomes native animation tracks; consumption, magic
effects and unknown status elements remain separate gameplay domains. The
focused inventory check reloads Rete, finishes the independent native scene and
checks admitted ownership, motion timing, blocked input and interrupted-motion
reset. It does not establish complete original-game or actual-session parity.

The supplied manual (`disc/HypeManual.pdf`, printed pages 12–15 and 18) establishes
Tab as the default inventory key and configurable controls. Esc opens a separate
pause menu (Resume, Load another adventure, Configuration, Return to main menu).
The native inventory host consumes a non-echo Tab press before GUI focus navigation
and ignores releases. The actual game composes separate maintained inventory, pause and configuration
screens; inventory does not own those menu operations.
