use super::*;
pub(super) fn model() -> Model {
    Model {
        path: AssetPath::new("level:/models/triangle").unwrap(),
        name: "triangle".into(),
        positions: vec![[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]],
        normals: vec![[0., 0., 1.]; 3],
        sprite_elements: Vec::new(),
        submeshes: vec![Submesh {
            source_index: None,
            material: AssetRef::new("level:/materials/red").unwrap(),
            triangles: vec![[0, 1, 2]],
            texcoords: vec![[0., 0.], [1., 0.], [0., 1.]],
            uv_indices: vec![[2, 1, 0]],
            face_normals: vec![[0., 0., 1.]],
            vertex_indices: vec![0, 1, 2],
        }],
        vertex_materials: Vec::new(),
    }
}
#[test]
fn controlled_layout_derives_pointers_and_preserves_distinct_uv_indices() -> Result<()> {
    let mut m = model();
    let targets = BTreeMap::from([(
        m.submeshes[0].material.path.clone(),
        ReferenceTarget {
            address: 0x2000,
            block: (6, 1),
        },
    )]);
    let a = write_geometry(&m, &targets, 0x1000)?;
    let geom: GeometryWire = wire(&a.bytes)?;
    assert_eq!(geom.point_count, 3);
    assert_eq!(geom.points, 0x1030);
    assert_eq!(geom.spatial_data, 0);
    let groups = reloc(&a.rtb)?;
    let rows = &groups[&BlockKey(6, 2)];
    assert_eq!(rows.len(), a.pointer_count);
    for row in rows {
        let at = (row.value - 0x1000) as usize;
        let target = u32_at(&a.bytes, at)?;
        if row.id == 1 {
            assert_eq!(target, 0x2000);
        } else {
            assert!(target >= 0x1000 && target < 0x1000 + a.bytes.len() as u32);
        }
    }
    let uv = a
        .ranges
        .iter()
        .find(|r| r.name == "submeshes/0/uv_indices")
        .unwrap();
    assert_eq!(
        &a.bytes[uv.offset..uv.offset + uv.length],
        &[2, 0, 1, 0, 0, 0]
    );
    m.positions[1][0] = 2.;
    m.positions.push([3., 3., 3.]);
    m.normals.push([0., 0., 1.]);
    let b = write_geometry(&m, &targets, 0x1000)?;
    let changed: GeometryWire = wire(&b.bytes)?;
    assert_eq!(changed.point_count, 4);
    assert_ne!(changed.normals, geom.normals);
    assert_eq!(f32::from_bits(u32_at(&b.bytes, 0x30 + 12)?), 2.);
    Ok(())
}
#[test]
fn malformed_framing_and_indices_fail() {
    assert!(sna(&[5, 1, 0]).is_err());
    assert!(reloc(&[1, 6, 2, 0xff, 0xff, 0xff, 0x7f]).is_err());
    let mut m = model();
    m.submeshes[0].uv_indices[0][0] = 42;
    assert!(m.validate().is_err());
}
#[test]
fn verified_hype_records_do_not_include_adjacent_allocations() -> Result<()> {
    assert_eq!(
        encoded(&InstantiatedObjectWire {
            physical_object: 0,
            radiosity: 0
        })?
        .len(),
        8
    );
    assert_eq!(
        encoded(&RadiosityWire {
            lod_count: 1,
            reserved: 0,
            lods: 0
        })?
        .len(),
        8
    );
    assert_eq!(
        encoded(&RadiosityLodWire {
            vertex_count: 1,
            reserved: 0,
            colors: 0
        })?
        .len(),
        8
    );
    assert_eq!(
        encoded(&RadiosityColorWire {
            components: [-32768, 32767, -57, 0]
        })?
        .len(),
        8
    );
    assert_eq!(
        encoded(&GeometryWire {
            point_count: 1,
            point_reserved: 0,
            points: 0,
            normals: 0,
            point_materials: 0,
            unknown_10: 0,
            element_count: 0,
            element_reserved: 0,
            element_types: 0,
            elements: 0,
            spatial_data: 0,
            unknown_tail: [0; 3]
        })?
        .len(),
        0x30
    );
    assert_eq!(
        encoded(&TrianglesWire {
            material: 0,
            triangle_count: 0,
            uv_count: 0,
            triangles: 0,
            uv_indices: 0,
            normals: 0,
            uvs: 0,
            point_count: 0,
            point_reserved: 0,
            points: 0
        })?
        .len(),
        0x20
    );
    Ok(())
}
