use anyhow::Result;
use astrolabe::{
    model::{
        AssetPath, Selection,
        animation::*,
        io::{Content, Load, Save},
    },
    openspace::level::{read_transform, write_transform},
    rete,
};

#[test]
fn compact_transforms_all_forms_shear_edits_and_pool_identity() -> Result<()> {
    let quaternion = [28377i16, 16384, 0, 0];
    let cases = [
        (0x11u16, vec![512, -1024, 1536]),
        (0x22, quaternion.to_vec()),
        (0x33, [vec![512, -1024, 1536], quaternion.to_vec()].concat()),
        (
            0x77,
            [vec![512, -1024, 1536], quaternion.to_vec(), vec![384]].concat(),
        ),
        (
            0x7b,
            [
                vec![512, -1024, 1536],
                quaternion.to_vec(),
                vec![256, 384, 512],
            ]
            .concat(),
        ),
        (
            0x7f,
            [
                vec![512, -1024, 1536],
                quaternion.to_vec(),
                vec![256, 16, -32, 384, 48, 512],
            ]
            .concat(),
        ),
    ];
    let mut entries = Vec::new();
    for (i, (form, values)) in cases.into_iter().enumerate() {
        let bytes: Vec<_> = form
            .to_le_bytes()
            .into_iter()
            .chain(values.into_iter().flat_map(i16::to_le_bytes))
            .collect();
        let value = read_transform(&bytes)?;
        assert_eq!(
            write_transform(&value)?.unwrap(),
            bytes,
            "compact form {form:x} reconstruction"
        );
        entries.push(TransformEntry {
            id: i as u32,
            source_index: Some(i as u32 * 2 + 10),
            value,
        });
    }
    let TransformValue::Components {
        translation,
        stretch,
        ..
    } = &entries[5].value
    else {
        panic!()
    };
    assert_eq!(*translation, [1., -2., 3.]);
    assert_eq!(*stretch, [1., 0.0625, -0.125, 1.5, 0.1875, 2.]);
    assert_ne!(
        entries[5].value.affine().basis[0][2],
        0.,
        "symmetric off-axis scale was discarded"
    );
    let mut duplicate = entries[2].clone();
    duplicate.id = 6;
    duplicate.source_index = Some(99);
    entries.push(duplicate);
    let pool = TransformPool {
        path: AssetPath::new("fix:/transforms/test")?,
        entries,
    };
    pool.validate()?;
    let temp = tempfile::tempdir()?;
    let reference = astrolabe::model::AssetRef::from_path(pool.path.clone());
    let selection = Selection::hype("animation-test", "en")?;
    let mut store = rete::Store::new(temp.path(), selection.clone())?;
    store.save(&reference, &pool)?;
    drop(store);
    let mut store = rete::Store::new(temp.path(), selection)?;
    let reopened: std::sync::Arc<TransformPool> = store.load(&reference)?;
    assert_eq!(*reopened, pool);
    assert_eq!(reopened.entries[2].value, reopened.entries[6].value);
    assert_ne!(
        reopened.entries[2].source_index,
        reopened.entries[6].source_index
    );
    let mut edited = pool.entries[0].value.clone();
    let TransformValue::Components { translation, .. } = &mut edited else {
        panic!()
    };
    translation[0] = 100.;
    let bytes = write_transform(&edited)?.unwrap();
    assert_eq!(bytes.len(), 88);
    assert_eq!(read_transform(&bytes)?.affine().translation[0], 100.);
    assert!(write_transform(&TransformValue::default())?.is_none());
    // Zoom is unsigned in the original engine, unlike Raymap's signed interpretation.
    let mut unsigned = vec![0x77, 0];
    unsigned.extend(
        [0i16, 0, 0, 32767, 0, 0, 0]
            .into_iter()
            .flat_map(i16::to_le_bytes),
    );
    unsigned.extend(40000u16.to_le_bytes());
    let large = read_transform(&unsigned)?;
    assert_eq!(large.affine().basis[0][0], 156.25);
    assert_eq!(write_transform(&large)?.unwrap(), unsigned);
    Ok(())
}

#[test]
#[ignore = "requires Godot; included by tools/test.py --local"]
fn godot_sampled_animation_reparents_switches_hides_and_uses_state_timing() -> Result<()> {
    use astrolabe::{
        godot,
        model::{
            AssetRef, character::*, content::Collection, geometry::*, io::ContentRoot, scene::*,
        },
    };
    let mut source = Collection::default();
    let material = Material {
        path: AssetPath::new("fix:/materials/plain")?,
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
        path: AssetPath::new("fix:/models/triangle")?,
        name: "triangle".into(),
        positions: vec![[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]],
        normals: vec![],
        vertex_materials: vec![],
        sprite_elements: Vec::new(),
        submeshes: vec![Submesh {
            source_index: None,
            material: AssetRef::from_path(material.path.clone()),
            triangles: vec![[0, 1, 2]],
            texcoords: vec![[0., 0.], [1., 0.], [0., 1.]],
            uv_indices: vec![[0, 1, 2]],
            face_normals: vec![],
            vertex_indices: vec![0, 1, 2],
        }],
    };
    let instance = ModelInstance::new(AssetPath::new("fix:/objects/triangle")?, &model);
    let mut configuration = ObjectList {
        reset_parts: None,
        unknown: [0; 2],
        path: AssetPath::new("fix:/configurations/body")?,
        parts: vec![
            ObjectPart {
                action: None,
                kind: ObjectPartKind::Model,
                instance: Some(AssetRef::from_path(instance.path.clone())),
                scale: None,
                unknown: [0; 10],
            },
            ObjectPart {
                action: None,
                kind: ObjectPartKind::Model,
                instance: Some(AssetRef::from_path(instance.path.clone())),
                scale: Some([2.; 3]),
                unknown: [0; 10],
            },
        ],
    };
    configuration.parts.resize(
        301,
        ObjectPart {
            action: None,
            kind: ObjectPartKind::Model,
            instance: None,
            scale: None,
            unknown: [0; 10],
        },
    );
    configuration.parts[300] = configuration.parts[0].clone();
    configuration.reset_parts = Some(configuration.parts.clone());
    configuration.reset_parts.as_mut().unwrap()[300].scale = Some([3., 4., 5.]);
    let pool = TransformPool {
        path: AssetPath::new("fix:/transforms/test")?,
        entries: vec![TransformEntry {
            id: 7,
            source_index: Some(7),
            value: TransformValue::Components {
                translation: [1., 2., 3.],
                quaternion: [1., 0., 0., 0.],
                stretch: [1., 0., 0., 1., 0., 1.],
            },
        }],
    };
    let action = AnimationAction {
        path: AssetPath::new("fix:/animation-actions/attachment")?,
        operation: AnimationOperation::Attachment { value: 0 },
        unknown: [0; 7],
    };
    let mut clip = AnimationClip {
        cues: vec![
            AnimationCue {
                action: AssetRef::from_path(action.path.clone()),
                frame: 0,
                channel: 1,
                configuration_slot: 300,
            },
            AnimationCue {
                action: AssetRef::from_path(action.path.clone()),
                frame: 2,
                channel: 0,
                configuration_slot: 301,
            },
        ],
        morph_count: 0,
        path: AssetPath::new("fix:/animations/pose")?,
        channel_count: 2,
        frame_rate: 20,
        speed: Default::default(),
        unknown: vec![0; 6],
        samples: vec![
            ChannelSample {
                transform: None,
                object: None,
                channel: Some(0),
                visibility: 1,
                opacity: 255,
                kind: ChannelKind::Object,
                unknown: [0; 6],
            },
            ChannelSample {
                transform: Some(TransformLink {
                    pool: AssetRef::from_path(pool.path.clone()),
                    entry: 7,
                }),
                object: Some(300),
                channel: Some(1),
                visibility: 1,
                opacity: 255,
                kind: ChannelKind::Object,
                unknown: [0; 6],
            },
            ChannelSample {
                transform: None,
                object: Some(1),
                channel: Some(0),
                visibility: 1,
                opacity: 255,
                kind: ChannelKind::Object,
                unknown: [0; 6],
            },
            ChannelSample {
                transform: None,
                object: Some(300),
                channel: Some(1),
                visibility: 1,
                opacity: 255,
                kind: ChannelKind::Hidden,
                unknown: [0; 6],
            },
            ChannelSample {
                transform: Some(TransformLink {
                    pool: AssetRef::from_path(pool.path.clone()),
                    entry: 7,
                }),
                object: Some(0),
                channel: Some(0),
                visibility: 1,
                opacity: 255,
                kind: ChannelKind::Object,
                unknown: [0; 6],
            },
            ChannelSample {
                transform: None,
                object: Some(1),
                channel: Some(1),
                visibility: 1,
                opacity: 255,
                kind: ChannelKind::Object,
                unknown: [0; 6],
            },
        ],
        frames: vec![
            AnimationFrame {
                samples: vec![0, 1],
                hierarchy: vec![[1, 0]],
                motion_rotation: None,
                motion_translation: None,
            },
            AnimationFrame {
                samples: vec![2, 3],
                hierarchy: vec![[0, 1]],
                motion_rotation: None,
                motion_translation: None,
            },
            AnimationFrame {
                samples: vec![5, 4],
                hierarchy: vec![],
                motion_rotation: None,
                motion_translation: None,
            },
        ],
    };
    clip.validate()?;
    clip.frames[0].hierarchy.push([0, 1]);
    assert!(clip.validate().is_err());
    clip.frames[0].hierarchy.pop();
    let bounds = astrolabe::model::collision::Collision {
        path: AssetPath::new("fix:/collision/family-bounds/test")?,
        geometry: astrolabe::model::collision::CollisionGeometry::Mesh(
            astrolabe::model::collision::CollisionMesh {
                positions: vec![[1., 2., 3.]],
                normals: vec![],
                vertex_materials: vec![],
                elements: vec![astrolabe::model::collision::CollisionElement::Spheres(
                    vec![astrolabe::model::collision::CollisionSphere {
                        center: 0,
                        radius: 2.,
                        material: None,
                    }],
                )],
            },
        ),
    };
    let mechanics = astrolabe::model::mechanics::Mechanics {
        path: AssetPath::new("fix:/mechanics/test")?,
        data: astrolabe::model::mechanics::MechanicsData::Mass { mass: 17. },
    };
    let family = Family {
        object_type: 0,
        bounding_geometry: Some(AssetRef::from_path(bounds.path.clone())),
        path: AssetPath::new("fix:/families/test")?,
        name: "Test".into(),
        states: vec![
            CharacterState {
                mechanics: Some(AssetRef::from_path(mechanics.path.clone())),
                id: 0,
                animation: Some(AssetRef::from_path(clip.path.clone())),
                speed: 2,
                next: Some(5),
                transitions: vec![],
                prohibited: vec![],
                unknown: [0; 7],
            },
            CharacterState {
                mechanics: None,
                id: 5,
                animation: Some(AssetRef::from_path(clip.path.clone())),
                speed: 4,
                next: None,
                transitions: vec![],
                prohibited: vec![],
                unknown: [0; 7],
            },
        ],
        configurations: vec![AssetRef::from_path(configuration.path.clone())],
        default_configuration: None,
        animation_bank: 0,
        unknown: [0; 3],
    };
    let perso = Perso {
        path: AssetPath::new("level:/persos/test")?,
        name: "Test".into(),
        family: AssetRef::from_path(family.path.clone()),
        initial_state: Some(0),
        current_state: Some(0),
        secondary_state: None,
        configuration: Some(AssetRef::from_path(configuration.path.clone())),
        initial_configuration: None,
        object_types: [0; 3],
    };
    let scene = Scene {
        path: AssetPath::new("level:/scene")?,
        name: "AnimationTest".into(),
        nodes: vec![SceneNode {
            properties: Default::default(),
            id: "world".into(),
            name: "world".into(),
            parent: None,
            kind: NodeKind::World,
            transform: Some(SourceTransform {
                translation: [5., 0., 0.],
                ..Default::default()
            }),
            instance: None,
            collision: None,
            perso: Some(AssetRef::from_path(perso.path.clone())),
            draw_mask: u32::MAX,
            sector: None,
            light: None,
        }],
    };
    macro_rules! put {
        ($value:ident) => {
            source.insert(&AssetRef::from_path($value.path.clone()), $value.clone())?
        };
    }
    put!(material);
    put!(model);
    put!(instance);
    put!(configuration);
    put!(pool);
    put!(clip);
    put!(action);
    put!(family);
    put!(bounds);
    put!(mechanics);
    put!(perso);
    put!(scene);
    // Pooled transforms reconstruct once; every sample points to that generated allocation.
    let emitted = astrolabe::openspace::level::write_transform_pool(&pool, 0x30000000)?;
    assert_eq!(&emitted.bytes[..4], &3u32.to_le_bytes());
    let target = emitted.targets[&7];
    let fragment = astrolabe::openspace::level::write_animation(
        &clip,
        &std::collections::BTreeMap::from([(pool.path.clone(), emitted.targets)]),
        &astrolabe::openspace::level::write_animation_actions(&[action], 0x50000000)?.targets,
        0x20000000,
    )?;
    let sample = fragment
        .ranges
        .iter()
        .find(|r| r.name == "samples/1")
        .unwrap();
    assert_eq!(
        &fragment.bytes[sample.offset..sample.offset + 4],
        &target.address.to_le_bytes()
    );
    assert_eq!(
        &fragment.bytes[sample.offset + 4..sample.offset + 16],
        &[44, 1, 1, 0, 0, 0, 1, 255, 0, 0, 0, 0]
    );
    let temp = tempfile::tempdir()?;
    let engine = std::env::var("ASTROLABE_GODOT").unwrap_or_else(|_| "godot".into());
    godot::export(
        &mut source,
        &[ContentRoot {
            kind: Scene::KIND.into(),
            reference: scene.path,
        }],
        Selection::hype("animation-test", "en")?,
        temp.path(),
        &engine,
    )?;
    std::fs::write(
        temp.path().join("animation_check.gd"),
        r#"extends SceneTree
func require(ok: bool, message: String) -> bool:
    if not ok:
        push_error(message)
        quit(1)
    return ok
func _initialize():
    call_deferred("check")
func check():
    var scene = load("res://generated/hype/levels/animation-test/scene.tscn").instantiate()
    root.add_child(scene)
    var actor = scene.get_node("world/Character")
    actor.playing = false
    var cues: Array = []
    actor.cue_triggered.connect(func(cue, action):
        require(actor.frame_index == int(cue.frame), "cue listener observed the wrong pose")
        cues.append([int(cue.frame),int(cue.channel),int(cue.configuration_slot),action.path]))
    actor.select_state(0)
    if not require(actor.current_mechanics.data.Mass.mass == 17, "state mechanics reference not projected"): return
    actor.set_bounds_visible(true)
    var bound = actor.bounds_debug.get_child(0)
    if not require(bound is MeshInstance3D and bound.mesh is SphereMesh and bound.mesh.radius == 2 and bound.position.is_equal_approx(Vector3(1,3,-2)), "family bound geometry differs"): return
    if not require(actor.bounds_debug.find_children("*", "CollisionObject3D", true, false).is_empty(), "debug bound must not create physics"): return
    actor.set_bounds_visible(false)
    if not require(not actor.bounds_debug.visible, "bound visibility toggle failed"): return
    if not require(cues.size() == 1 and cues[0].slice(0,3) == [0,1,300], "state entry cue/column/slot"): return
    cues.clear()
    if not require(actor.current_state == 0 and actor.frame_index == 0, "initial state/frame"): return
    if not require(actor.channels[1].get_parent() == actor.channels[0], "initial channel parent"): return
    if not require(actor.channels[1].get_node("Part").is_visible_in_tree(), "empty parent hides visible child"): return
    if not require(actor.channels[1].global_position.is_equal_approx(Vector3(6,3,-2)), "source-local transform conversion"): return
    actor.advance(0.5)
    if not require(actor.frame_index == 1, "state speed must override clip default speed"): return
    if not require(actor.channels[0].get_parent() == actor.channels[1], "changed hierarchy"): return
    if not require(actor.channels[1].get_node_or_null("Part") == null, "hidden part still present"): return
    if not require(actor.channels[0].get_node("Part").scale.is_equal_approx(Vector3(2,2,2)), "object switch lost configuration scale"): return
    actor.advance(0.5)
    if not require(actor.frame_index == 2 and actor.channels[0].get_parent() == actor and actor.channels[1].get_parent() == actor, "removed parents must reset"): return
    actor.advance(0.5)
    if not require(actor.frame_index == 0, "animation wrap"): return
    if not require(cues.size() == 2 and cues[0][0] == 2 and cues[1][0] == 0 and cues[0][3] == cues[1][3], "loop cue order and shared action identity"): return
    cues.clear()
    actor.advance(3.0)
    if not require(cues.size() == 4 and cues.map(func(c):return c[0]) == [2,0,2,0], "long tick must preserve complete-loop cues"): return
    actor.apply_frame(2)
    if not require(cues.size() == 4, "visual inspection must not trigger sounds/actions"): return
    if not require(actor.select_state(5), "stable state ID selection"): return
    if not require(actor.current_mechanics.is_empty(), "state without mechanics retained another state's card"): return
    actor.advance(0.25)
    if not require(actor.current_state == 5 and actor.frame_index == 1, "selected state timing"): return
    if not require(not actor.select_state(99) and actor.current_state == 5, "invalid state changed playback"): return
    actor.select_state(0)
    cues.clear()
    actor.cue_triggered.connect(func(cue, _action):
        if int(cue.frame) == 2: actor.select_state(5))
    actor.advance(2.0)
    if not require(actor.current_state == 5 and actor.frame_index == 0 and actor.elapsed == 0.0 and cues.map(func(c):return c[0]) == [2,0], "cue-driven transition must cancel the previous clip tick"): return
    actor.select_state(0)
    var restored_frame = actor.frame_index
    var restored_cues = cues.size()
    if not require(actor.restore_configuration(), "configuration snapshot not exported"): return
    if not require(actor.channels[1].get_node("Part").scale.is_equal_approx(Vector3(3,5,4)), "reset table edit did not change the displayed part"): return
    if not require(actor.frame_index == restored_frame and cues.size() == restored_cues, "restoring objects must not advance time or replay cues"): return
    actor.configuration[300].scale[0] = 99
    if not require(actor.restore_configuration() and actor.configuration[300].scale[0] == 3, "mutable object table changed the saved snapshot"): return
    scene.free()
    quit(0)
"#,
    )?;
    godot::run_godot(
        &engine,
        temp.path(),
        &["--headless", "--script", "res://animation_check.gd"],
        std::time::Duration::from_secs(30),
    )?;
    Ok(())
}
