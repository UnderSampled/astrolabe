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
    configuration.parts[299] = configuration.parts[0].clone();
    configuration.parts[299].scale = Some([4., 5., 6.]);
    configuration.parts[300] = configuration.parts[0].clone();
    configuration.reset_parts = Some(configuration.parts.clone());
    configuration.reset_parts.as_mut().unwrap()[300].scale = Some([3., 4., 5.]);
    let pool = TransformPool {
        path: AssetPath::new("fix:/transforms/test")?,
        entries: vec![
            TransformEntry {
                id: 7,
                source_index: Some(7),
                value: TransformValue::Components {
                    translation: [1., 2., 3.],
                    quaternion: [1., 0., 0., 0.],
                    stretch: [1., 0., 0., 1., 0., 1.],
                },
            },
            TransformEntry {
                id: 8,
                source_index: Some(8),
                value: TransformValue::Affine(SourceTransform {
                    translation: [10., 0., 0.],
                    basis: [[2., 0., 0.], [0.5, 1., 0.], [0., 0., 1.]],
                    ..Default::default()
                }),
            },
        ],
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
    let mut fractional_clip = clip.clone();
    fractional_clip.path = AssetPath::new("fix:/animations/fractional")?;
    fractional_clip.frames = clip.frames.iter().cycle().take(29).cloned().collect();
    fractional_clip.cues[1].frame = 14;
    let mut affine_clip = clip.clone();
    affine_clip.path = AssetPath::new("fix:/animations/affine")?;
    affine_clip.channel_count = 3;
    affine_clip.cues.clear();
    let mut auxiliary = affine_clip.samples[1].clone();
    auxiliary.kind = ChannelKind::Light;
    auxiliary.object = None;
    affine_clip.samples.push(auxiliary);
    let mut parent = affine_clip.samples[0].clone();
    parent.transform = Some(TransformLink {
        pool: AssetRef::from_path(pool.path.clone()),
        entry: 8,
    });
    affine_clip.samples.push(parent);
    affine_clip.samples[1].opacity = 127;
    affine_clip.frames = vec![
        AnimationFrame {
            samples: vec![7, 1, 6],
            hierarchy: vec![[1, 0]],
            motion_rotation: None,
            motion_translation: None,
        },
        AnimationFrame {
            samples: vec![6, 1, 7],
            hierarchy: vec![[1, 2]],
            motion_rotation: None,
            motion_translation: None,
        },
    ];
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
                mechanics: None,
                id: 10,
                animation: Some(AssetRef::from_path(fractional_clip.path.clone())),
                speed: 30,
                next: None,
                transitions: vec![],
                prohibited: vec![],
                repeats: 0,
                transition_lock: 0,
                unknown: [0; 5],
            },
            CharacterState {
                mechanics: None,
                id: 9,
                animation: Some(AssetRef::from_path(affine_clip.path.clone())),
                speed: 2,
                next: None,
                transitions: vec![],
                prohibited: vec![],
                repeats: 0,
                transition_lock: 0,
                unknown: [0; 5],
            },
            CharacterState {
                mechanics: Some(AssetRef::from_path(mechanics.path.clone())),
                id: 0,
                animation: Some(AssetRef::from_path(clip.path.clone())),
                speed: 2,
                next: Some(5),
                transitions: vec![],
                prohibited: vec![],
                repeats: 0,
                transition_lock: 0,
                unknown: [0; 5],
            },
            CharacterState {
                mechanics: None,
                id: 8,
                animation: Some(AssetRef::from_path(clip.path.clone())),
                speed: 0,
                next: None,
                transitions: vec![],
                prohibited: vec![],
                repeats: 0,
                transition_lock: 0,
                unknown: [0; 5],
            },
            CharacterState {
                mechanics: None,
                id: 5,
                animation: Some(AssetRef::from_path(clip.path.clone())),
                speed: 4,
                next: None,
                transitions: vec![],
                prohibited: vec![],
                repeats: 0,
                transition_lock: 0,
                unknown: [0; 5],
            },
        ],
        configurations: vec![AssetRef::from_path(configuration.path.clone())],
        default_configuration: None,
        animation_bank: 0,
        unknown: [0; 3],
    };
    let perso = Perso {
        shadow: None,
        path: AssetPath::new("level:/persos/test")?,
        name: "Test".into(),
        family: AssetRef::from_path(family.path.clone()),
        initial_state: Some(0),
        current_state: Some(0),
        secondary_state: None,
        configuration: Some(AssetRef::from_path(configuration.path.clone())),
        initial_configuration: None,
        object_types: [0; 3],
        hit_points: None,
        inventory_stock: None,
        motion_binding_present: false,
        picking_bounds: None,
        activation: None,
        custom_bits: 0,
        capabilities: None,
        microphone: None,
        camera: None,
        routes: vec![],
        brain: None,
        particles: None,
        collisions: None,
    };
    let scene = Scene {
        main_actor: None,
        armor: None,
        inventory: None,
        map_names: None,
        hud: None,
        loading: None,
        global_actors: vec![],
        always_active: Vec::new(),
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
    put!(affine_clip);
    put!(fractional_clip);
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
    let roots = [ContentRoot {
        kind: Scene::KIND.into(),
        reference: scene.path,
    }];
    let first = godot::export(
        &mut source,
        &roots,
        Selection::hype("animation-test", "en")?,
        temp.path(),
        &engine,
    )?;
    let repeated = tempfile::tempdir()?;
    let second = godot::export(
        &mut source,
        &roots,
        Selection::hype("animation-test", "en")?,
        repeated.path(),
        &engine,
    )?;
    assert_eq!(
        first.files, second.files,
        "native output changed between builds"
    );
    assert!(first.files.keys().any(|p| p.ends_with("animations.res")));
    assert!(!first.files.keys().any(|p| {
        p.rsplit('/')
            .next()
            .is_some_and(|name| name.starts_with("state_") && name.ends_with(".res"))
    }));
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
    var packed = load("res://generated/hype/levels/animation-test/level.tscn") as PackedScene
    var scene = packed.instantiate()
    var actor = scene.get_node("world/Test")
    var player = actor.get_node("AnimationPlayer") as AnimationPlayer
    var channel0 = actor.get_node("Channels/channel_0")
    var channel1 = actor.get_node("Channels/channel_1")
    var initial = channel1.get_node("part_300")
    var saved_count = actor.find_children("*", "", true, false).size()
    if not require(initial.visible and not channel0.get_node("part_1").visible, "saved initial configuration missing before ready"): return
    if not require(channel1.position.is_equal_approx(Vector3(1,3,-2)), "saved initial pose missing before ready"): return
    if not require(actor.current_mechanics.mass == 17, "saved typed mechanics missing"): return
    if not require(player.get_animation("state_0") is Animation and player.get_animation_library("") is AnimationLibrary, "native library missing"): return
    if not require(player.get_animation_library("").resource_path.ends_with(".res"), "dense native animation library must use a binary resource"): return
    for animation_name in player.get_animation_list():
        var animation = player.get_animation(animation_name)
        for track in animation.get_track_count():
            var path = animation.track_get_path(track)
            if not require(actor.has_node(NodePath(path.get_concatenated_names())), "unresolved native track: " + str(path)): return
    actor.autoplay = false
    root.add_child(scene)
    if not require(saved_count == actor.find_children("*", "", true, false).size(), "ready constructed missing character content"): return
    var shared_catalog: Resource = null
    for channel in [channel0, channel1]:
        for bank in channel.get_children():
            if not str(bank.name).begins_with("bank_"): continue
            if not require(bank.get_child_count() == 0, "initial load instantiated parked payloads"): return
            var catalog = bank.get("catalog") as Resource
            if not require(catalog != null and catalog.get("payload_scenes").size() == 301, "native payload catalogue missing"): return
            if not bool(bank.get_meta("source_restored")):
                if shared_catalog == null: shared_catalog = catalog
                elif not require(catalog == shared_catalog, "channel catalogues were duplicated"): return
    if not require(channel1.global_position.is_equal_approx(Vector3(6,3,-2)), "source placement"): return
    actor.set_bounds_visible(true)
    var bound = actor.get_node("FamilyBounds/Bound_0")
    if not require(bound.mesh is SphereMesh and bound.mesh.radius == 2 and bound.position.is_equal_approx(Vector3(1,3,-2)), "persisted family bounds"): return
    if not require(actor.get_node("FamilyBounds").find_children("*", "CollisionObject3D", true, false).is_empty(), "bounds became physics"): return
    actor.set_bounds_visible(false)
    var cues: Array = []
    var observed_poses: Array = []
    actor.cue_triggered.connect(func(action, frame, channel, slot):
        cues.append([frame,channel,slot,action.source_action])
        observed_poses.append([actor.frame_index, channel0.position, channel1.position]))
    actor.select_state(0)
    player.pause()
    if not require(cues.size() == 1 and cues[0].slice(0,3) == [0,1,300], "state entry native cue"): return
    var released := Node3D.new()
    scene.add_child(released)
    actor.generated_object = released
    actor.select_state(0)
    if not require(released.global_transform.is_equal_approx(channel1.global_transform) and actor.generated_object == null, "release cue did not copy its native channel world transform"): return
    actor.generated_object = released
    actor.advance(1.001)
    if not require(released.global_transform.is_equal_approx(channel1.global_transform) and actor.generated_object == null, "release cue lost frame-column mapping after channel swap"): return
    released.free()
    actor.select_state(0)
    cues.clear()
    player.seek(0.5, true, true)
    if not require(actor.frame_index == 1 and actor.current_state == 0, "state rate overrides clip fps"): return
    if not require(not initial.visible and channel0.get_node("part_1").visible, "native object visibility switch"): return
    if not require(channel0.get_node("part_1").get_child(0).scale.is_equal_approx(Vector3(2,2,2)), "configuration payload scale"): return
    player.seek(1.0, true, true)
    if not require(channel0.position.is_equal_approx(Vector3(1,3,-2)) and channel1.position.is_zero_approx(), "logical identity follows swapped columns"): return
    player.seek(0.0, true, true)
    if not require(initial.visible and not channel0.get_node("part_1").visible and channel1.position.is_equal_approx(Vector3(1,3,-2)), "backward seek did not restore pose/configuration"): return
    if not require(cues.is_empty(), "editor seek dispatched cue side effects"): return
    # Slot299 is never sampled. Proc82 must acquire its native scene before
    # returning, keep the animation address/time, and reuse it on warm swaps.
    if not require(actor.source_swap_configured_parts(301, 300), "unused payload swap"): return
    var acquired = channel1.get_node("part_300").get_child(0)
    if not require(acquired.original_slot == 299 and acquired.scale.is_equal_approx(Vector3(4,6,5)), "unused native payload identity/scale"): return
    var swap_count = actor.find_children("*", "", true, false).size()
    if not require(actor.source_swap_configured_parts(301, 300) and actor.source_swap_configured_parts(301, 300), "warm payload swap"): return
    if not require(channel1.get_node("part_300").get_child(0) == acquired and actor.find_children("*", "", true, false).size() == swap_count, "warm swap duplicated native content"): return
    if not require(actor.source_swap_configured_parts(301, 300), "restore normal source order"): return
    if not require(cues.is_empty() and actor.frame_index == 0, "swap advanced animation or replayed cues"): return
    if not require(actor.restore_configuration(), "reset library missing"): return
    var playback_count = actor.find_children("*", "", true, false).size()
    if not require(not initial.visible and channel1.get_node("reset_part_300").visible and channel1.get_node("reset_part_300").get_child(0).scale.is_equal_approx(Vector3(3,5,4)), "native reset configuration edit"): return
    if not require(cues.is_empty() and actor.frame_index == 0, "configuration restore replayed cues or changed time: " + str(cues) + " frame " + str(actor.frame_index)): return
    actor.select_state(5)
    player.advance(0.25)
    if not require(actor.current_state == 5 and actor.frame_index == 1 and actor.current_mechanics == null, "native state selection timing/mechanics"): return
    if not require(not actor.select_state(99) and actor.current_state == 5, "invalid state changed player"): return
    actor.select_state(0)
    cues.clear()
    player.advance(0.5)
    player.advance(0.5)
    player.advance(0.5)
    if not require(actor.frame_index == 0 and cues.map(func(c):return c[0]) == [2,0], "native looping cue order"): return
    cues.clear()
    observed_poses.clear()
    actor.advance(4.6)
    if not require(cues.map(func(c):return c[0]) == [2,0,2,0,2,0], "large tick skipped complete loops: " + str(cues)): return
    for i in cues.size():
        var frame = cues[i][0]
        if not require(observed_poses[i][0] == frame, "cue observed wrong playback time: " + str(observed_poses)): return
        if not require(observed_poses[i][1 if frame == 2 else 2].is_equal_approx(Vector3(1,3,-2)), "cue observed previous pose: " + str(observed_poses)): return
    actor.select_state(8)
    cues.clear()
    actor.advance(9.0)
    if not require(actor.frame_index == 0 and not actor.playing and cues.is_empty(), "zero-speed source state advanced"): return
    actor.select_state(0)
    var change_state = func(_action, frame, _channel, _slot):
        if frame == 2: actor.select_state(5)
    actor.cue_triggered.connect(change_state)
    cues.clear()
    actor.advance(5.0)
    if not require(actor.current_state == 5 and actor.frame_index == 0 and cues.map(func(c):return c[0]) == [2,0], "cue callback did not cancel old tick: " + str(cues)): return
    actor.cue_triggered.disconnect(change_state)
    actor.select_state(10)
    cues.clear()
    actor.advance(26.0/30.0)
    actor.advance(3.0/30.0)
    if not require(actor.frame_index == 0 and cues.map(func(c):return c[0]) == [14,0], "fractional frame-rate loop boundary stranded playback: " + str(cues)): return
    actor.select_state(9)
    player.pause()
    var repeated = actor.get_node("Channels/channel_257")
    if not require(channel1.position.is_equal_approx(Vector3(13,3,-2)), "native parent composition"): return
    if not require(channel1.basis.is_equal_approx(Basis(Vector3(2,0,0),Vector3(0,1,0),Vector3(-0.5,0,1))), "native affine shear was decomposed: " + str(channel1.basis)): return
    if not require(is_equal_approx(channel1.get_node("reset_part_300").transparency, 1.0 - 127.0/255.0), "native opacity missing"): return
    if not require(repeated.position.is_equal_approx(Vector3(1,3,-2)) and repeated.get_meta("source_channel") == 1 and repeated.get_meta("source_channel_occurrence") == 1, "duplicate logical channel lost"): return
    player.seek(0.5, true, true)
    if not require(repeated.position.is_equal_approx(Vector3(13,3,-2)) and repeated.get_node("reset_part_300").visible and not channel1.get_node("reset_part_300").visible, "duplicate occurrence object selection"): return
    player.seek(0.0, true, true)
    if not require(channel1.get_node("reset_part_300").visible and not repeated.get_node("reset_part_300").visible, "duplicate occurrence backward seek"): return
    if not require(playback_count == actor.find_children("*", "", true, false).size(), "native playback rebuilt warm parts"): return
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
