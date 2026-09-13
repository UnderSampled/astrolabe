//! A reusable model can be placed and lit independently, through each adapter.
use anyhow::Result;
use astrolabe::{
    godot,
    model::{
        AssetPath, AssetRef, Selection,
        content::Collection,
        geometry::{Material, Model, ModelInstance, SourceTransform, Submesh, VisualMaterial},
        io::{Content, ContentRoot, Load, Save},
    },
    openspace::{self, level::write_model_fragment},
    rete::{self, Package, Store},
};
use std::{fs, path::Path, sync::Arc, time::Duration};

fn fixture() -> Result<(Model, Material, Vec<ModelInstance>)> {
    let material = Material {
        path: AssetPath::new("level:/materials/plain")?,
        visual: Some(VisualMaterial {
            texture: None,
            texture_info: None,
            ambient: [0.; 4],
            diffuse: [1.; 4],
            specular: [0.; 4],
            color: [1.; 4],
            flags: 0,
            scroll_speed: [0.; 2],
            scroll_position: [0.; 2],
            scroll_mode: 0,
        }),
        sound_material: 0,
        collision: astrolabe::model::collision::CollisionProperties::Absent,
    };
    let model = Model {
        path: AssetPath::new("level:/models/shared")?,
        name: "Shared triangle".into(),
        positions: vec![[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]],
        normals: vec![[0., 0., 1.]; 3],
        sprite_elements: Vec::new(),
        submeshes: vec![Submesh {
            source_index: None,
            material: AssetRef::from_path(material.path.clone()),
            triangles: vec![[0, 1, 2]],
            texcoords: vec![[0., 0.], [1., 0.], [0., 1.]],
            uv_indices: vec![[0, 1, 2]],
            face_normals: vec![[0., 0., 1.]],
            vertex_indices: vec![0, 1, 2],
        }],
        vertex_materials: vec![],
    };
    let mut instances = Vec::new();
    for (index, name) in ["first", "second", "red", "green"].iter().enumerate() {
        let mut instance =
            ModelInstance::new(AssetPath::new(format!("level:/instances/{name}"))?, &model);
        instance.transforms.push(SourceTransform {
            translation: [index as f32 + 1., 0., 0.],
            ..SourceTransform::default()
        });
        instance.lod_distance = index as f32;
        if index >= 2 {
            let color = if index == 2 {
                [128, 0, 0, 0]
            } else {
                [0, 128, 0, 0]
            };
            instance.radiosity = vec![color; 3];
        }
        instances.push(instance);
    }
    Ok((model, material, instances))
}

#[test]
fn source_fragments_do_not_overwrite_instances_from_another_package_role() -> Result<()> {
    let (model, material, mut instances) = fixture()?;
    instances[1].path = AssetPath::new("fix:/instances/first")?;
    let temp = tempfile::tempdir()?;
    let input = temp.path().join("rete");
    let output = temp.path().join("source");
    persist(&input, &model, &material, &instances[..2])?;
    let (mut store, package) = Store::open(input)?;
    let report = openspace::export(&mut store, &package.roots, &output)?;
    assert!(
        report
            .files
            .contains(&"fragments/level/instances/first.bin".into())
    );
    assert!(
        report
            .files
            .contains(&"fragments/fix/instances/first.bin".into())
    );
    assert_ne!(
        fs::read(output.join("fragments/level/instances/first.bin"))?,
        fs::read(output.join("fragments/fix/instances/first.bin"))?
    );
    let expected = write_model_fragment(&model, &instances[0], &[material], &[], 0x10000000)?;
    assert_eq!(
        fs::read(output.join("fragments/level/instances/first.bin"))?,
        expected.bytes
    );
    Ok(())
}

fn persist(
    root: &Path,
    model: &Model,
    material: &Material,
    instances: &[ModelInstance],
) -> Result<()> {
    fn insert<T: Content + Clone>(
        source: &mut Collection,
        inventory: &mut Vec<ContentRoot>,
        path: &AssetPath,
        value: &T,
    ) -> Result<()> {
        source.insert(&AssetRef::from_path(path.clone()), value.clone())?;
        inventory.push(ContentRoot {
            kind: T::KIND.into(),
            reference: path.clone(),
        });
        Ok(())
    }
    let mut source = Collection::default();
    let mut inventory = Vec::new();
    insert(&mut source, &mut inventory, &model.path, model)?;
    insert(&mut source, &mut inventory, &material.path, material)?;
    for instance in instances {
        insert(&mut source, &mut inventory, &instance.path, instance)?;
    }
    let package = Package {
        selection: Selection::hype("instances", "EN")?,
        roots: inventory[2..].to_vec(),
        diagnostics: vec![],
    };
    rete::save_package(&mut source, &inventory, &package, root)
}

#[test]
fn instance_edits_preserve_shared_geometry_in_rete_and_source_fragments() -> Result<()> {
    let (model, material, instances) = fixture()?;
    let temp = tempfile::tempdir()?;
    let input = temp.path().join("rete");
    persist(&input, &model, &material, &instances[2..])?;
    let (mut store, package) = Store::open(&input)?;
    let first: Arc<ModelInstance> = store.load(&AssetRef::from_path(instances[2].path.clone()))?;
    let second: Arc<ModelInstance> = store.load(&AssetRef::from_path(instances[3].path.clone()))?;
    let shared = store.load(&first.model)?;
    assert!(Arc::ptr_eq(&shared, &store.load(&second.model)?));
    assert_eq!(*first, instances[2]);
    assert_eq!(*second, instances[3]);

    let first_bytes = rete::encode(first.as_ref())?;
    let mut edited = second.as_ref().clone();
    edited.radiosity[0] = [-12, 400, 0, 0];
    edited.transforms[0].translation[0] = 20.;
    store.save(&AssetRef::from_path(edited.path.clone()), &edited)?;
    assert_eq!(
        rete::encode(store.load(&first.model)?.as_ref())?,
        rete::encode(&model)?
    );
    assert_eq!(
        rete::encode(
            store
                .load(&AssetRef::<ModelInstance>::from_path(first.path.clone()))?
                .as_ref()
        )?,
        first_bytes
    );

    let mut geometry = shared.as_ref().clone();
    geometry.positions[1][0] = 4.;
    store.save(&first.model, &geometry)?;
    let updated = store.load(&second.model)?;
    assert!(Arc::ptr_eq(&updated, &store.load(&first.model)?));
    assert_eq!(updated.positions[1][0], 4.);
    drop(store);
    let (mut store, _) = Store::open(&input)?;
    let output = temp.path().join("source");
    openspace::export(&mut store, &package.roots, &output)?;
    let material = [material];
    let a = write_model_fragment(&geometry, &first, &material, &[], 0x10000000)?;
    let b = write_model_fragment(&geometry, &edited, &material, &[], 0x10000000)?;
    assert_eq!(
        fs::read(output.join("fragments/level/instances/red.bin"))?,
        write_model_fragment(
            &geometry,
            &first,
            &material,
            &[],
            fragment_base(&output, "red")?
        )?
        .bytes
    );
    assert_eq!(
        fs::read(output.join("fragments/level/instances/green.bin"))?,
        write_model_fragment(
            &geometry,
            &edited,
            &material,
            &[],
            fragment_base(&output, "green")?
        )?
        .bytes
    );
    assert!(
        fragment_base(&output, "green")? >= fragment_base(&output, "red")? + a.bytes.len() as u32
    );
    for range in a
        .ranges
        .iter()
        .filter(|r| r.name == "positions" || r.name.starts_with("submeshes/"))
    {
        let other = b.ranges.iter().find(|r| r.name == range.name).unwrap();
        assert_eq!(
            &a.bytes[range.offset..range.offset + range.length],
            &b.bytes[other.offset..other.offset + other.length]
        );
    }
    let colors = b
        .ranges
        .iter()
        .find(|r| r.name == "radiosity/colors")
        .unwrap();
    assert_eq!(
        &b.bytes[colors.offset..colors.offset + 4],
        &[244, 255, 144, 1]
    );
    let distance = b.ranges.iter().find(|r| r.name == "lod_distance").unwrap();
    assert_eq!(
        &b.bytes[distance.offset..distance.offset + 4],
        &3f32.to_le_bytes()
    );
    Ok(())
}

#[test]
#[ignore = "requires Godot; included in the complete tools/test.py --local suite"]
fn godot_shares_plain_meshes_and_keeps_instance_lighting_separate() -> Result<()> {
    let (model, material, instances) = fixture()?;
    let temp = tempfile::tempdir()?;
    let input = temp.path().join("rete");
    let output = temp.path().join("godot");
    persist(&input, &model, &material, &instances)?;
    let (mut store, package) = Store::open(input)?;
    let engine = std::env::var("ASTROLABE_GODOT").unwrap_or_else(|_| "godot".into());
    let manifest = godot::export(
        &mut store,
        &package.roots,
        package.selection,
        &output,
        &engine,
    )?;
    assert_eq!(
        manifest
            .files
            .keys()
            .filter(|p| p.ends_with(".mesh.tres"))
            .count(),
        3
    );
    let script = r#"extends SceneTree
func require(ok: bool, reason: String) -> bool:
    if not ok:
        push_error(reason)
        quit(1)
    return ok
func _initialize():
    var nodes = []
    for name in ["first", "second", "red", "green"]:
        var scene = load("res://generated/hype/levels/instances/instances/" + name + ".tscn") as PackedScene
        if not require(scene != null, "missing instance " + name): return
        nodes.append(scene.instantiate())
    if not require(nodes[0].mesh == nodes[1].mesh, "plain instances lost shared mesh identity"): return
    if not require(nodes[2].mesh != nodes[3].mesh, "lighting variants overwrote each other"): return
    for i in nodes.size():
        if not require(nodes[i].position.is_equal_approx(Vector3(i + 1, 0, 0)), "instance transform lost"): return
    var red = nodes[2].mesh.surface_get_arrays(0)[Mesh.ARRAY_COLOR]
    var green = nodes[3].mesh.surface_get_arrays(0)[Mesh.ARRAY_COLOR]
    if not require(red[0].is_equal_approx(Color(1, 0, 0, 1)), "red lighting lost"): return
    if not require(green[0].is_equal_approx(Color(0, 1, 0, 1)), "green lighting lost"): return
    for node in nodes:
        node.free()
    quit(0)
"#;
    fs::write(output.join("inspect_instances.gd"), script)?;
    godot::run_godot(
        &engine,
        &output,
        &["--headless", "--script", "res://inspect_instances.gd"],
        Duration::from_secs(30),
    )?;
    Ok(())
}

fn fragment_base(output: &std::path::Path, name: &str) -> Result<u32> {
    let framed = fs::read(output.join(format!("fragments/level/instances/{name}.sna")))?;
    Ok(u32::from_le_bytes(framed[2..6].try_into()?))
}
