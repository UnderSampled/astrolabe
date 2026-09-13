use anyhow::Result;
use astrolabe::{
    godot,
    model::{
        AssetPath, AssetRef, Selection,
        collision::CollisionProperties,
        content::Collection,
        geometry::*,
        io::{Content, ContentRoot, Load, Save},
        sprite::*,
    },
    openspace, rete,
};

fn fixture() -> Result<(Model, Sprite, Material)> {
    let material = Material {
        path: AssetPath::new("fix:/materials/sprite")?,
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
        sound_material: 7,
        collision: CollisionProperties::Absent,
    };
    let sprite = Sprite {
        path: AssetPath::new("fix:/sprites/shared")?,
        unknown: 0,
        views: vec![
            SpriteView {
                mode: 1,
                threshold: -1.,
                half_size: [0.25, 0.5],
                displacement: [0.1, 0.2],
                material: Some(AssetRef::from_path(material.path.clone())),
            },
            SpriteView {
                mode: 5,
                threshold: 0.,
                half_size: [0.75, 1.],
                displacement: [0.; 2],
                material: Some(AssetRef::from_path(material.path.clone())),
            },
        ],
    };
    let placement = IndexedSprite {
        center: 0,
        unknown: 17,
        sprite: AssetRef::from_path(sprite.path.clone()),
        size: [2., 3.],
        constraint: [0.; 3],
        uv_origin_extent: [[0.; 2]; 2],
    };
    let model = Model {
        path: AssetPath::new("level:/models/sprites")?,
        name: "Sprites".into(),
        positions: vec![[1., 2., 3.], [4., 5., 6.]],
        normals: vec![[0., 1., 0.]; 2],
        submeshes: vec![],
        vertex_materials: vec![],
        sprite_elements: vec![SpriteElement {
            source_index: Some(0),
            unknown: 23,
            sprites: vec![
                placement.clone(),
                IndexedSprite {
                    center: 1,
                    ..placement
                },
            ],
        }],
    };
    Ok((model, sprite, material))
}
#[test]
fn shared_sprite_definition_edits_survive_rete_and_source_reconstruction() -> Result<()> {
    let (model, sprite, material) = fixture()?;
    let temp = tempfile::tempdir()?;
    let selection = Selection::hype("sprite-test", "en")?;
    let mut store = rete::Store::new(temp.path(), selection.clone())?;
    store.save(&AssetRef::from_path(model.path.clone()), &model)?;
    store.save(&AssetRef::from_path(sprite.path.clone()), &sprite)?;
    store.save(&AssetRef::from_path(material.path.clone()), &material)?;
    drop(store);
    let mut store = rete::Store::new(temp.path(), selection)?;
    let persisted = store.load(&AssetRef::<Model>::from_path(model.path.clone()))?;
    assert_eq!(*persisted, model);
    let a = store.load(&persisted.sprite_elements[0].sprites[0].sprite)?;
    let b = store.load(&persisted.sprite_elements[0].sprites[1].sprite)?;
    assert!(std::sync::Arc::ptr_eq(&a, &b));
    assert_eq!(*a, sprite);
    let instance = ModelInstance::new(AssetPath::new("level:/instances/sprites")?, &model);
    let first = openspace::level::write_model_fragment(
        &model,
        &instance,
        std::slice::from_ref(&material),
        std::slice::from_ref(&sprite),
        0x1000,
    )?;
    let range = |name: &str| first.ranges.iter().find(|r| r.name == name).unwrap();
    let g = range("geometry");
    let types = range("element_types");
    assert_eq!(
        &first.bytes[g.offset + 20..g.offset + 22],
        &1u16.to_le_bytes()
    );
    assert_eq!(
        &first.bytes[types.offset..types.offset + types.length],
        &3u16.to_le_bytes()
    );
    let entries = range("sprite_elements/0/entries");
    assert_eq!(entries.length, 88);
    assert_eq!(
        &first.bytes[entries.offset..entries.offset + 4],
        &[0, 0, 17, 0]
    );
    assert_eq!(
        &first.bytes[entries.offset + 4..entries.offset + 8],
        &first.bytes[entries.offset + 48..entries.offset + 52]
    );
    let mut edited = sprite.clone();
    edited.views[0].half_size[0] = 2.;
    let changed =
        openspace::level::write_model_fragment(&model, &instance, &[material], &[edited], 0x1000)?;
    assert_ne!(first.bytes, changed.bytes);
    let size = range("sprites/sprites/shared/sizes");
    assert_eq!(
        f32::from_le_bytes(changed.bytes[size.offset..size.offset + 4].try_into()?),
        2.
    );
    let mut invalid = model.clone();
    invalid.sprite_elements[0].sprites[0].center = 2;
    assert!(invalid.validate().is_err());
    Ok(())
}

#[test]
#[ignore = "requires Godot; included by tools/test.py --local"]
fn native_sprite_placement_size_material_and_view_visibility() -> Result<()> {
    let (model, sprite, material) = fixture()?;
    let mut source = Collection::default();
    source.insert(&AssetRef::from_path(model.path.clone()), model.clone())?;
    source.insert(&AssetRef::from_path(sprite.path.clone()), sprite)?;
    source.insert(&AssetRef::from_path(material.path.clone()), material)?;
    let temp = tempfile::tempdir()?;
    let engine = std::env::var("ASTROLABE_GODOT").unwrap_or_else(|_| "godot".into());
    godot::export(
        &mut source,
        &[ContentRoot {
            kind: Model::KIND.into(),
            reference: model.path,
        }],
        Selection::hype("sprite-test", "en")?,
        temp.path(),
        &engine,
    )?;
    std::fs::write(
        temp.path().join("check.gd"),
        r#"extends SceneTree
func require(ok: bool, message: String) -> bool:
    if not ok:
        push_error(message)
        quit(1)
    return ok
func _initialize(): call_deferred("check")
func check():
    var model = load("res://generated/hype/levels/sprite-test/models/sprites.tscn").instantiate()
    root.add_child(model)
    var sprite = model.get_node("Sprite_0_0")
    var first = sprite.get_node("View_0")
    if not require(sprite.position == Vector3(1,3,-2), "sprite source center conversion"): return
    if not require(first.mesh.size == Vector2(1,3), "definition and placement size composition"): return
    if not require(first.mesh.material.billboard_mode == 1 and first.mesh.material.billboard_keep_scale, "scaled billboard projection"): return
    var camera = Camera3D.new()
    root.add_child(camera)
    camera.position = Vector3(1,3,-6)
    camera.make_current()
    sprite._process(0.0)
    if not require(first.visible and not sprite.get_node("View_1").visible, "front view selection"): return
    camera.position = Vector3(1,3,2)
    sprite._process(0.0)
    if not require(not first.visible and not sprite.get_node("View_1").visible, "hidden sprite view mode"): return
    model.free()
    camera.free()
    quit(0)
"#,
    )?;
    godot::run_godot(
        &engine,
        temp.path(),
        &["--headless", "--script", "res://check.gd"],
        std::time::Duration::from_secs(30),
    )?;
    Ok(())
}
