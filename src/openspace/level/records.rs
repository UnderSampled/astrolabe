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
