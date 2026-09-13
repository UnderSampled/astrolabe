//! Packed source records and their named relocation sites. Both directions use
//! these declarations; offsets describe wire fields, never Rust memory layout.
use binrw::{BinRead, BinWrite};
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Owner {
    Fix,
    Level,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct BlockKey(pub(super) u8, pub(super) u8);
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct Address {
    pub(super) owner: Owner,
    pub(super) key: BlockKey,
    pub(super) offset: usize,
}
#[derive(Clone)]
pub(super) struct Block {
    pub(super) base: u32,
    pub(super) data: Vec<u8>,
}
#[derive(Debug, Clone, BinRead, BinWrite)]
#[brw(little)]
pub(super) struct SnaStart {
    pub(super) module: u8,
    pub(super) id: u8,
    pub(super) base: i32,
}
#[derive(Debug, Clone, BinRead, BinWrite)]
#[brw(little)]
pub(super) struct SnaInfo {
    /// Mmg block information: capacity end is inclusive; the next two are allocator state.
    pub(super) capacity_end: u32,
    pub(super) free_list: u32,
    pub(super) high_water: u32,
    pub(super) size: u32,
}
#[derive(Debug, Clone, BinRead, BinWrite)]
#[brw(little)]
pub(super) struct RelocGroup {
    pub(super) module: u8,
    pub(super) id: u8,
    pub(super) count: u32,
}
#[derive(Debug, Clone, BinRead, BinWrite)]
#[brw(little)]
pub(super) struct RelocRow {
    pub(super) value: u32,
    pub(super) module: u8,
    pub(super) id: u8,
    pub(super) unknown: [u8; 2],
}
#[derive(Debug, Clone, BinRead, BinWrite)]
#[brw(little)]
pub(super) struct SuperObjectWire {
    pub(super) kind: u32,
    pub(super) data: u32,
    pub(super) first_child: u32,
    pub(super) last_child: u32,
    pub(super) child_count: u32,
    pub(super) next: u32,
    pub(super) previous: u32,
    pub(super) parent: u32,
    pub(super) local_transform: u32,
    pub(super) global_transform: u32,
    pub(super) last_compute_frame: u32,
    pub(super) draw_mask: u32,
    pub(super) light: u32,
    pub(super) bounding_volume: u32,
    pub(super) initial_global_transform: u32,
    pub(super) opacity: f32,
}
/// FUN_00437270 allocates 0x38 bytes. Raymap's later-engine properties byte at
/// +56 is outside Hype's record and would read the following MMG allocation header.
#[derive(BinRead, BinWrite)]
#[brw(little)]
pub(super) struct FamilyWire {
    pub(super) next: u32,
    pub(super) previous: u32,
    pub(super) header: u32,
    pub(super) index: u32,
    pub(super) states: ListWire,
    pub(super) default_configuration: u32,
    pub(super) configurations: ListWire,
    pub(super) bounding_volume: u32,
    #[br(assert(extra_data == 0, "family extra-data dependency is unsupported"))]
    pub(super) extra_data: u32,
    pub(super) animation_bank: u8,
    pub(super) unknown: [u8; 3],
}
/// FUN_004617c0 allocates 24 bytes. FUN_00461a60 copies count*20 bytes into
/// reset_parts; FUN_00461ac0 restores that table during actor reinitialization.
#[derive(BinRead, BinWrite)]
#[brw(little)]
pub(super) struct ObjectListWire {
    pub(super) next: u32,
    pub(super) previous: u32,
    pub(super) header: u32,
    pub(super) parts: u32,
    pub(super) reset_parts: u32,
    pub(super) count: u16,
    pub(super) unknown: [u8; 2],
}
#[derive(BinRead, BinWrite)]
#[brw(little)]
/// Hype allocates 64 bytes (0044a720/0044a730). Mechanics is +48
/// (0044a910/0044a920), not Raymap's later-engine +44; speed is byte 59.
pub(super) struct CharacterStateWire {
    pub(super) next: u32,
    pub(super) previous: u32,
    pub(super) header: u32,
    pub(super) animation: u32,
    pub(super) transitions: ListWire,
    pub(super) prohibited: ListWire,
    pub(super) next_state: u32,
    #[br(assert(unknown_reference == 0, "state dependency at +44 is unsupported"))]
    pub(super) unknown_reference: u32,
    pub(super) mechanics: u32,
    #[br(assert(environment_samples == 0, "state environment samples are unsupported"))]
    pub(super) environment_samples: u32,
    pub(super) unknown: [u8; 3],
    pub(super) speed: u8,
    pub(super) unknown_tail: [u8; 4],
}
#[derive(BinRead, BinWrite)]
#[brw(little)]
pub(super) struct StateTransitionWire {
    pub(super) next: u32,
    pub(super) previous: u32,
    pub(super) header: u32,
    pub(super) target: u32,
    pub(super) via: u32,
    pub(super) linking_type: u8,
    pub(super) unknown: [u8; 3],
}
#[derive(BinRead, BinWrite)]
#[brw(little)]
pub(super) struct ProhibitedStateWire {
    pub(super) next: u32,
    pub(super) previous: u32,
    pub(super) header: u32,
    pub(super) state: u32,
}
/// Hype engine allocates 0x30, not Raymap/C#'s 0x40 with spurious sphere fields.
#[derive(Debug, Clone, BinRead, BinWrite)]
#[brw(little)]
pub(super) struct GeometryWire {
    pub(super) point_count: u16,
    pub(super) point_reserved: u16,
    pub(super) points: u32,
    pub(super) normals: u32,
    pub(super) point_materials: u32,
    pub(super) unknown_10: u32,
    pub(super) element_count: u16,
    pub(super) element_reserved: u16,
    pub(super) element_types: u32,
    pub(super) elements: u32,
    pub(super) spatial_data: u32,
    pub(super) unknown_tail: [u32; 3],
}
/// Engine GEO_vCreateElementIndexedTriangles allocates exactly 0x20 bytes.
#[derive(Debug, Clone, BinRead, BinWrite)]
#[brw(little)]
pub(super) struct TrianglesWire {
    pub(super) material: u32,
    pub(super) triangle_count: u16,
    pub(super) uv_count: u16,
    pub(super) triangles: u32,
    pub(super) uv_indices: u32,
    pub(super) normals: u32,
    pub(super) uvs: u32,
    pub(super) point_count: u16,
    pub(super) point_reserved: u16,
    pub(super) points: u32,
}
/// IPO_fn_hAllocInstanciatedPhysicalObject allocates exactly two pointers (8 bytes).
#[derive(Debug, Clone, BinRead, BinWrite)]
#[brw(little)]
pub(super) struct InstantiatedObjectWire {
    pub(super) physical_object: u32,
    pub(super) radiosity: u32,
}
/// ISI_fn_hCreateISI / hCreateISILOD allocate 8-byte records. Their counts are signed
/// shorts, not the u32 used by Raymap; the engine initializes the second short to zero.
#[derive(Debug, Clone, BinRead, BinWrite)]
#[brw(little)]
pub(super) struct RadiosityWire {
    #[br(assert(lod_count > 0))]
    pub(super) lod_count: i16,
    #[br(assert(reserved == 0, "unsupported ISI reserved field"))]
    pub(super) reserved: u16,
    pub(super) lods: u32,
}
#[derive(Debug, Clone, BinRead, BinWrite)]
#[brw(little)]
pub(super) struct RadiosityLodWire {
    #[br(assert(vertex_count > 0))]
    pub(super) vertex_count: i16,
    #[br(assert(reserved == 0, "unsupported ISI LOD reserved field"))]
    pub(super) reserved: u16,
    pub(super) colors: u32,
}
#[derive(Debug, Clone, BinRead, BinWrite)]
#[brw(little)]
pub(super) struct RadiosityColorWire {
    /// ISI_ColISIToGEO converts all four signed components with scale 1/256, unclamped.
    pub(super) components: [i16; 4],
}
/// Level PTX, corroborated by engine FUN_00483860/FUN_00483930. The initial u32 is
/// texture table capacity, not a pointer; the channel table covers all 1024 slots.
#[derive(Debug, Clone, BinRead, BinWrite)]
#[brw(little)]
pub(super) struct LevelTexturesWire {
    #[br(assert(capacity == 1024, "unsupported Hype texture table capacity"))]
    pub(super) capacity: u32,
    #[br(assert(level_slots <= capacity))]
    pub(super) level_slots: u32,
    #[br(count = level_slots)]
    pub(super) pointers: Vec<u32>,
    #[br(assert(loaded_textures <= capacity))]
    pub(super) loaded_textures: u32,
    #[br(count = capacity)]
    pub(super) memory_channels: Vec<u32>,
    pub(super) current_channel: u32,
}
/// Uncompressed SO matrix. See POS_fn_vMulMatrixVertex and the level research notes.
#[derive(BinRead, BinWrite)]
#[brw(little)]
pub(super) struct TransformWire {
    #[br(assert(flags & 0x80 != 0, "compressed SO transform is unsupported"))]
    pub(super) flags: u32,
    pub(super) translation: [f32; 3],
    pub(super) rotation: [[f32; 3]; 3],
    pub(super) basis: [[f32; 3]; 3],
}

#[derive(BinRead, BinWrite)]
#[brw(little)]
pub(super) struct PhysicalObjectWire {
    pub(super) visual_set: u32,
    pub(super) collision_set: u32,
    pub(super) bounding_volume: u32,
}
#[derive(BinRead, BinWrite)]
#[brw(little)]
pub(super) struct VisualSetWire {
    pub(super) flags: u32,
    pub(super) lods: u16,
    pub(super) kind: u16,
    pub(super) distances: u32,
    pub(super) objects: u32,
}
#[derive(BinRead, BinWrite)]
#[brw(little)]
pub(super) struct GameMaterialWire {
    pub(super) visual: u32,
    pub(super) mechanics: u32,
    pub(super) sound: u32,
    pub(super) collision: u32,
}
#[derive(BinRead, BinWrite)]
#[brw(little)]
pub(super) struct VisualMaterialWire {
    pub(super) flags: u32,
    pub(super) ambient: [f32; 4],
    pub(super) diffuse: [f32; 4],
    pub(super) specular: [f32; 4],
    pub(super) color: [f32; 4],
    pub(super) unknown_44: u32,
    pub(super) texture: u32,
    pub(super) scroll_position: [f32; 2],
    pub(super) scroll_speed: [f32; 2],
    pub(super) scroll_mode: u32,
    pub(super) refresh: u32,
    pub(super) animation_first: u32,
    pub(super) animation_current: u32,
    pub(super) animation_count: u16,
    pub(super) reserved: u16,
    pub(super) animation_time: u32,
    pub(super) properties: [u8; 4],
    pub(super) runtime_tail: [u32; 3],
}
#[derive(BinRead, BinWrite)]
#[brw(little)]
pub(super) struct TextureInfoWire {
    pub(super) runtime_head: [u32; 2],
    pub(super) flags: u32,
    pub(super) stored_height: u32,
    pub(super) stored_width: u32,
    pub(super) unknown_14: u32,
    pub(super) runtime_state: [u32; 6],
    pub(super) logical_height: u32,
    pub(super) logical_width: u32,
    pub(super) remaining_state: [u32; 11],
    pub(super) name: [u8; 80],
    pub(super) sample_flags: u8,
    pub(super) addressing: u8,
    pub(super) reserved: u16,
    pub(super) runtime_tail: u32,
}

impl SuperObjectWire {
    pub(super) const DATA: usize = 4;
    pub(super) const FIRST_CHILD: usize = 8;
    pub(super) const NEXT: usize = 20;
    pub(super) const LOCAL_TRANSFORM: usize = 32;
}

impl GeometryWire {
    pub(super) const POINTS: usize = 4;
    pub(super) const NORMALS: usize = 8;
    pub(super) const POINT_MATERIALS: usize = 12;
    pub(super) const ELEMENT_TYPES: usize = 24;
    pub(super) const ELEMENTS: usize = 28;
}

impl TrianglesWire {
    pub(super) const MATERIAL: usize = 0;
    pub(super) const TRIANGLES: usize = 8;
    pub(super) const UV_INDICES: usize = 12;
    pub(super) const NORMALS: usize = 16;
    pub(super) const UVS: usize = 20;
    pub(super) const POINTS: usize = 28;
}

impl VisualMaterialWire {
    pub(super) const TEXTURE: usize = 72;
}

impl VisualSetWire {
    pub(super) const DISTANCES: usize = 8;
    pub(super) const OBJECTS: usize = 12;
}

impl InstantiatedObjectWire {
    pub(super) const PHYSICAL_OBJECT: usize = 0;
    pub(super) const RADIOSITY: usize = 4;
}

impl RadiosityWire {
    pub(super) const LODS: usize = 4;
}

impl RadiosityLodWire {
    pub(super) const COLORS: usize = 4;
}

impl PhysicalObjectWire {
    pub(super) const VISUAL_SET: usize = 0;
    pub(super) const COLLISION_SET: usize = 4;
}

impl GameMaterialWire {
    pub(super) const VISUAL: usize = 0;
}

/// GEO_vCreateElement{Spheres,AlignedBoxes}: 8-byte header, signed short count.
#[derive(BinRead, BinWrite)]
#[brw(little)]
pub(super) struct CollisionArrayWire {
    #[br(assert(count >= 0))]
    pub(super) count: i16,
    #[br(assert(reserved == 0))]
    pub(super) reserved: u16,
    pub(super) data: u32,
}
/// GEO_xGetCenterPointOfIndexedSphere / GEO_xGetRadiusOfIndexedSphere.
#[derive(BinRead, BinWrite)]
#[brw(little)]
pub(super) struct CollisionSphereWire {
    pub(super) center: u16,
    #[br(assert(reserved == 0))]
    pub(super) reserved: u16,
    pub(super) radius: f32,
    pub(super) material: u32,
}
#[derive(BinRead, BinWrite)]
#[brw(little)]
pub(super) struct CollisionBoxWire {
    pub(super) min: u16,
    pub(super) max: u16,
    pub(super) material: u32,
}
/// GMT_fn_vCopyCollideMaterial copies two shorts; Raymap's extra u32 is not Hype data.
#[derive(BinRead, BinWrite)]
#[brw(little)]
pub(super) struct CollisionMaterialWire {
    pub(super) kind: u16,
    pub(super) identifier: u16,
}

/// Montreal sampled animation header (Raymap AnimationMontreal, observed Hype allocation 116).
#[derive(BinRead, BinWrite)]
#[brw(little)]
pub(super) struct AnimationWire {
    pub frames: u32,
    pub frame_count: u8,
    pub speed: u8,
    pub channel_count: u8,
    pub unknown: u8,
    pub events: u32,
    pub unknown_12: u32,
    pub event_count: u8,
    pub unknown_17: u8,
    pub morph_count: u16,
    pub speed_matrix: TransformWire,
    pub unknown_108: u32,
    pub related: u32,
}
#[derive(BinRead, BinWrite)]
#[brw(little)]
pub(super) struct AnimationFrameWire {
    pub channels: u32,
    pub rotation: u32,
    pub translation: u32,
    pub hierarchy: u32,
}
#[derive(BinRead, BinWrite)]
#[brw(little)]
pub(super) struct AnimationChannelWire {
    pub transform: u32,
    pub object: u16,
    pub visibility: u8,
    pub unknown: [u8; 3],
    pub channel: u8,
    pub opacity: u8,
    pub kind: u8,
    pub tail: [u8; 3],
}
#[derive(BinRead, BinWrite)]
#[brw(little)]
pub(super) struct ListWire {
    pub first: u32,
    pub last: u32,
    pub count: u32,
}
/// GEO_vCreateElementSprite (00467580): eight-byte group, 44-byte indexed entries.
#[binrw::binrw]
#[brw(little)]
#[derive(Debug, Clone)]
pub(super) struct SpriteElementWire {
    pub count: u16,
    pub unknown: u16,
    pub entries: u32,
}
#[binrw::binrw]
#[brw(little)]
#[derive(Debug, Clone)]
pub(super) struct IndexedSpriteWire {
    pub center: u16,
    pub unknown: u16,
    pub sprite: u32,
    pub size: [f32; 2],
    pub constraint: [f32; 3],
    pub uv_origin_extent: [[f32; 2]; 2],
}
/// GEO_vCreateRotativSprite (005be090): parallel view-property arrays, shared by placements.
#[binrw::binrw]
#[brw(little)]
#[derive(Debug, Clone)]
pub(super) struct SpriteWire {
    pub count: u16,
    pub unknown: u16,
    pub modes: u32,
    pub thresholds: u32,
    pub sizes: u32,
    pub displacements: u32,
    pub materials: u32,
}

/// Hype SECT_fn_vSetLevelElementLSTGraphicInt / SetSectorPointed, 20-byte entry.
#[derive(BinRead, BinWrite)]
#[brw(little)]
pub(super) struct GraphicLinkWire {
    pub level: u16,
    pub unknown: u16,
    pub sector: u32,
    pub next: u32,
    pub previous: u32,
    pub header: u32,
}
/// Hype SCT_fn_lGetVolumeInSoundList, 20-byte propagation entry.
#[derive(BinRead, BinWrite)]
#[brw(little)]
pub(super) struct SoundLinkWire {
    pub sector: u32,
    pub volume: i32,
    pub next: u32,
    pub previous: u32,
    pub header: u32,
}
/// Hype collision/activity/perso entries are target plus three list links (16 bytes).
#[derive(BinRead, BinWrite)]
#[brw(little)]
pub(super) struct NodeLinkWire {
    pub target: u32,
    pub next: u32,
    pub previous: u32,
    pub header: u32,
}

/// SECT_fn_hCreateSector allocates 160 bytes with twelve list headers.
#[derive(BinRead, BinWrite)]
#[brw(little)]
pub(super) struct SectorWire {
    pub border: u32,
    pub lists: [ListWire; 12],
    pub virtual_sector: u8,
    pub unknown: [u8; 3],
    pub activation: u32,
    pub name: u32,
}

/// FUN_0043b910 / FUN_0043bcd0: ordered cue array and independently shared action.
#[derive(BinRead, BinWrite)]
#[brw(little)]
pub(super) struct AnimationCueWire {
    pub action: u32,
    pub configuration_slot: u16,
    pub frame: u8,
    pub channel: u8,
}
#[derive(BinRead, BinWrite)]
#[brw(little)]
pub(super) struct AnimationActionWire {
    pub payload: u32,
    pub kind: u8,
    pub unknown: [u8; 7],
}
#[derive(BinRead, BinWrite)]
#[brw(little)]
pub(super) struct ObjectPartWire {
    pub scale: u32,
    pub content: u32,
    pub unknown: [u8; 4],
    pub kind: i16,
    pub tail: [u8; 6],
}
/// SND relocates the receiving site through RTS, then resolves a packed event identity.
#[derive(BinRead, BinWrite)]
#[brw(little)]
pub(super) struct SoundSiteWire {
    pub event: u32,
    pub site: u32,
}
