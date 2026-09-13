use anyhow::Result;
use astrolabe::{
    model::{
        AssetPath, AssetRef, Selection,
        geometry::SourceTransform,
        io::{Content, Load, Save},
        light::*,
    },
    openspace::level::{read_light, write_light},
    rete,
};
fn fixture() -> Result<Light> {
    Ok(Light {
        path: AssetPath::new("level:/lights/lamp")?,
        name: "lamp".into(),
        kind: 2,
        on: 1,
        cast_shadows: 0,
        rotating: 0,
        pulsing: 0,
        far: 5.,
        near: 1.,
        angles: [0.; 2],
        rotation_step: 0.,
        pulse_step: 0.,
        transform: SourceTransform {
            translation: [1., 2., 3.],
            ..Default::default()
        },
        color: [0.4, -0.2, 0.1, 0.8],
        painting: 0,
        creates_shadows: 0,
        send: 1,
        intensity_min: 1.,
        unresolved_dimmer: false,
        unknown: LightUnknown {
            header: 7,
            after_transform: [0; 4],
            after_painting: 11,
            before_send: 2,
            after_send: [3, 4],
            name_tail: vec![0, 9],
        },
    })
}
#[test]
fn light_signed_color_switches_unknowns_and_source_free_edits() -> Result<()> {
    let value = fixture()?;
    let bytes = write_light(&value)?;
    assert_eq!(bytes.len(), 200);
    assert_eq!(&bytes[148..152], &0.4f32.to_le_bytes());
    assert_eq!(&bytes[152..156], &(-0.2f32).to_le_bytes());
    assert_eq!(bytes[189], 1);
    assert_eq!(read_light(&bytes, value.path.clone())?, value);
    let temp = tempfile::tempdir()?;
    let mut store = rete::Store::new(temp.path(), Selection::hype("lights", "en")?)?;
    let r = AssetRef::from_path(value.path.clone());
    store.save(&r, &value)?;
    drop(store);
    let mut store = rete::Store::new(temp.path(), Selection::hype("lights", "en")?)?;
    let mut edited: Light = (*store.load(&r)?).clone();
    edited.color[1] = 0.6;
    edited.send = 0;
    edited.far = 8.;
    let output = write_light(&edited)?;
    assert_eq!(&output[152..156], &0.6f32.to_le_bytes());
    assert_eq!(output[0], 1);
    assert_eq!(output[189], 0);
    assert_ne!(output, bytes);
    edited.unresolved_dimmer = true;
    assert!(write_light(&edited).is_err());
    edited.color[0] = f32::NAN;
    assert!(edited.validate().is_err());
    Ok(())
}
#[test]
#[ignore = "requires Godot; included by tools/test.py --local"]
fn godot_signed_lights_and_sector_sharing() -> Result<()> {
    use astrolabe::{
        godot,
        model::{content::Collection, io::ContentRoot, scene::*},
    };
    let light = fixture()?;
    let reference = AssetRef::from_path(light.path.clone());
    let mut source = Collection::default();
    source.insert(&reference, light)?;
    let scene = Scene {
        path: AssetPath::new("level:/scene")?,
        name: "Lights".into(),
        nodes: vec![SceneNode {
            properties: Default::default(),
            id: "world".into(),
            name: "world".into(),
            parent: None,
            kind: NodeKind::Sector,
            transform: None,
            instance: None,
            perso: None,
            collision: None,
            draw_mask: u32::MAX,
            light: Some(reference.clone()),
            sector: Some(Sector {
                name: "Room".into(),
                virtual_sector: 0,
                activation: u32::MAX,
                static_lights: vec![reference.clone()],
                dynamic_lights: vec![reference],
                ..Default::default()
            }),
        }],
    };
    source.insert(&AssetRef::from_path(scene.path.clone()), scene.clone())?;
    let temp = tempfile::tempdir()?;
    let engine = std::env::var("ASTROLABE_GODOT").unwrap_or_else(|_| "godot".into());
    godot::export(
        &mut source,
        &[ContentRoot {
            kind: Scene::KIND.into(),
            reference: scene.path,
        }],
        Selection::hype("lights", "en")?,
        temp.path(),
        &engine,
    )?;
    std::fs::write(
        temp.path().join("check.gd"),
        r#"extends SceneTree
func _initialize():
    var scene = load("res://generated/hype/levels/lights/scene.tscn").instantiate()
    var lights = scene.find_children("*", "OmniLight3D", true, false)
    assert(lights.size() == 2, "shared light duplicated across memberships")
    var positive = scene.get_node("Light_0/Positive")
    var negative = scene.get_node("Light_0/Negative")
    assert(positive.light_cull_mask == 2 and negative.light_negative)
    assert(positive.omni_range == 5.0)
    assert(negative.light_color.is_equal_approx(Color(0,0.2,0.0)))
    assert(scene.get_node("Light_0").position.is_equal_approx(Vector3(1,3,-2)))
    assert(scene.get_node("world").has_meta("source_sector"))
    scene.free()
    quit(0)
"#,
    )?;
    godot::run_godot(
        &engine,
        temp.path(),
        &["--headless", "--script", "res://check.gd"],
        std::time::Duration::from_secs(15),
    )?;
    Ok(())
}
