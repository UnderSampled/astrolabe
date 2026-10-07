//! Native motion extraction, physics ownership and Rete edits through the exporter.
use anyhow::Result;
use astrolabe::{
    godot,
    model::{
        AssetPath, AssetRef, Selection,
        ai::*,
        animation::*,
        character::*,
        collision::*,
        content::Collection,
        io::{Content, ContentRoot, Load, Save},
        scene::{NodeKind, Scene, SceneNode},
    },
    rete,
};
use std::{fs, time::Duration};

#[test]
fn native_motion_survives_rete_and_drives_character_contact() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let selection = Selection::hype("motion", "en")?;
    let clip = AnimationClip {
        path: AssetPath::new("fix:/animations/stride")?,
        channel_count: 1,
        frame_rate: 99, // Character state playback rate must win.
        speed: Default::default(),
        samples: vec![ChannelSample {
            transform: None,
            object: None,
            channel: Some(0),
            visibility: 1,
            opacity: 255,
            kind: ChannelKind::Object,
            unknown: [0; 6],
        }],
        frames: [0.125, 0.375, 0.25]
            .into_iter()
            .map(|distance| AnimationFrame {
                samples: vec![0],
                hierarchy: vec![],
                motion_rotation: None,
                motion_translation: Some([0., distance, 0.]),
            })
            .collect(),
        cues: vec![],
        morph_count: 0,
        unknown: vec![0; 6],
    };
    let mut family = Family {
        path: AssetPath::new("fix:/families/mover")?,
        name: "Mover".into(),
        object_type: 0,
        bounding_geometry: None,
        states: [10, 20, 0]
            .into_iter()
            .enumerate()
            .map(|(id, speed)| CharacterState {
                id: id as u32,
                animation: Some(AssetRef::from_path(clip.path.clone())),
                mechanics: None,
                speed,
                next: None,
                transitions: vec![],
                prohibited: vec![],
                repeats: 0,
                transition_lock: 0,
                unknown: [0; 5],
            })
            .collect(),
        configurations: vec![],
        default_configuration: None,
        animation_bank: 0,
        unknown: [0; 3],
    };
    for id in 3..6 {
        let mut state = family.states[0].clone();
        state.id = id;
        family.states.push(state);
    }
    family.states[0].transitions = vec![
        StateTransition {
            target: 1,
            via: 3,
            linking_type: 0,
            unknown: [0; 3],
        },
        StateTransition {
            target: 1,
            via: 5,
            linking_type: 0,
            unknown: [0; 3],
        },
    ];
    family.states[0].prohibited = vec![1, 5]; // Explicit transition wins for 1.
    family.states[3].repeats = 1;
    family.states[3].next = Some(1);
    family.states[4].transition_lock = 1;
    let collider = Collision {
        path: AssetPath::new("fix:/collision/movement")?,
        geometry: CollisionGeometry::Mesh(CollisionMesh {
            positions: vec![[0., 0., 0.25]],
            normals: vec![],
            vertex_materials: vec![],
            elements: vec![CollisionElement::Spheres(vec![CollisionSphere {
                center: 0,
                radius: 0.25,
                material: None,
            }])],
        }),
    };
    let profile = CollisionProfile {
        path: AssetPath::new("fix:/collision-profiles/mover")?,
        geometry: Some(CollisionGeometrySlots {
            slots: vec![Some(AssetRef::from_path(collider.path.clone()))],
            unknown: [0; 4],
        }),
        activations: Some(
            (0..3)
                .map(|state| CollisionActivation {
                    state,
                    selection: Some(if state == 2 { 1 } else { 0 }),
                    unknown: [0; 2],
                })
                .collect(),
        ),
        selections: vec![
            CollisionSelection {
                id: 0,
                zones: vec![CollisionZone {
                    index: 0,
                    unknown: [0; 2],
                }],
                unknown: [0; 4],
            },
            CollisionSelection {
                id: 1,
                zones: vec![],
                unknown: [0; 4],
            },
        ],
    };
    let mut groups: [CharacterCollisionGroup; 4] = Default::default();
    groups[2] = CharacterCollisionGroup {
        profile: Some(AssetRef::from_path(profile.path.clone())),
        active: Some(0),
    };
    let action = |state| {
        Expression::Action(Some(StateRef {
            family: AssetRef::from_path(family.path.clone()),
            state,
        }))
    };
    let ai = AiModel {
        path: AssetPath::new("level:/ai/motion")?,
        name: "Action helpers".into(),
        normal: Some(BehaviorSet {
            behaviors: vec![Behavior {
                id: 0,
                rules: vec![],
                schedule: Some(Script {
                    statements: vec![Statement::If {
                        condition: Expression::Button {
                            action: "Button_Hit".into(),
                            query: ButtonQuery::JustPressed,
                        },
                        then: vec![
                            Statement::Assign {
                                variable: 2,
                                value: Expression::Integer(0),
                            },
                            Statement::ExecuteAction(action(1)),
                            Statement::Assign {
                                variable: 2,
                                value: Expression::Integer(2),
                            },
                        ],
                        otherwise: Some(vec![Statement::If {
                            condition: Expression::Button {
                                action: "Button_Up".into(),
                                query: ButtonQuery::Pressed,
                            },
                            then: vec![Statement::ChangeAction(Expression::ArrayElement {
                                array: Box::new(Expression::Variable(0)),
                                index: Box::new(Expression::Variable(1)),
                            })],
                            otherwise: Some(vec![Statement::ChangeAction(action(0))]),
                        }]),
                    }],
                }),
                unknown: [0; 3],
            }],
            default_behavior: Some(0),
            action_capacity: 1,
            unknown: [0; 3],
        }),
        reflex: None,
        variables: vec![
            DesignerValue::ActionArray(DesignerArray::new(
                (0..2)
                    .map(|state| {
                        Some(StateRef {
                            family: AssetRef::from_path(family.path.clone()),
                            state,
                        })
                    })
                    .collect(),
            )),
            DesignerValue::Integer(1),
            DesignerValue::Integer(0),
        ]
        .into_iter()
        .map(|default| DesignerVariable {
            default,
            save: 0,
            initialize: 0,
        })
        .collect(),
        designer_unknown: [0; 3],
        unknown: [0; 4],
    };
    let perso = Perso {
        path: AssetPath::new("level:/persos/mover")?,
        name: "Mover".into(),
        family: AssetRef::from_path(family.path.clone()),
        initial_state: Some(0),
        current_state: Some(0),
        secondary_state: None,
        configuration: None,
        initial_configuration: None,
        object_types: [0; 3],
        hit_points: None,
        camera: None,
        routes: vec![],
        brain: Some(Brain {
            model: AssetRef::from_path(ai.path.clone()),
            normal: Some(BehaviorSelection {
                current: Some(0),
                previous: None,
                initial: Some(0),
            }),
            reflex: None,
            variables: ai.variables.iter().map(|v| v.default.clone()).collect(),
            initial_variables: None,
            unknown: 0,
            mind_unknown: [0; 4],
        }),
        particles: None,
        collisions: Some(CharacterCollisions {
            groups,
            unknown: [0; 4],
        }),
    };
    let mut source = Collection::default();
    let scene = Scene {
        path: AssetPath::new("level:/scene")?,
        name: "Motion".into(),
        main_actor: Some(AssetRef::from_path(perso.path.clone())),
        armor: None,
        global_actors: vec![],
        nodes: vec![SceneNode {
            properties: Default::default(),
            id: "mover".into(),
            name: "Mover".into(),
            parent: None,
            kind: NodeKind::Perso,
            transform: Some(Default::default()),
            instance: None,
            perso: Some(AssetRef::from_path(perso.path.clone())),
            collision: None,
            draw_mask: 1,
            sector: None,
            light: None,
        }],
    };
    source.insert(&AssetRef::from_path(scene.path.clone()), scene.clone())?;
    source.insert(&AssetRef::from_path(ai.path.clone()), ai.clone())?;
    source.insert(&AssetRef::from_path(clip.path.clone()), clip.clone())?;
    source.insert(&AssetRef::from_path(family.path.clone()), family.clone())?;
    source.insert(&AssetRef::from_path(perso.path.clone()), perso.clone())?;
    source.insert(
        &AssetRef::from_path(collider.path.clone()),
        collider.clone(),
    )?;
    source.insert(&AssetRef::from_path(profile.path.clone()), profile.clone())?;
    let roots = vec![ContentRoot {
        kind: Scene::KIND.into(),
        reference: scene.path.clone(),
    }];
    let engine = std::env::var("ASTROLABE_GODOT").unwrap_or_else(|_| "godot".into());
    let project = temp.path().join("direct");
    let direct = godot::export(&mut source, &roots, selection.clone(), &project, &engine)?;
    let scene_path = direct
        .files
        .keys()
        .find(|p| p.contains("/persos/") && p.ends_with(".tscn"))
        .unwrap();
    let mut store = rete::Store::new(temp.path().join("rete"), selection.clone())?;
    store.save(&AssetRef::from_path(scene.path.clone()), &scene)?;
    store.save(&AssetRef::from_path(ai.path.clone()), &ai)?;
    store.save(&AssetRef::from_path(clip.path.clone()), &clip)?;
    store.save(&AssetRef::from_path(family.path.clone()), &family)?;
    store.save(&AssetRef::from_path(perso.path.clone()), &perso)?;
    store.save(&AssetRef::from_path(collider.path.clone()), &collider)?;
    store.save(&AssetRef::from_path(profile.path.clone()), &profile)?;
    drop(store);
    let mut store = rete::Store::new(temp.path().join("rete"), selection.clone())?;
    let reloaded = godot::export(
        &mut store,
        &roots,
        selection,
        &temp.path().join("reloaded"),
        &engine,
    )?;
    for (path, digest) in &direct.files {
        if reloaded.files.get(path) != Some(digest) && path.ends_with(".tscn") {
            let a = fs::read_to_string(project.join(path))?;
            let b = fs::read_to_string(temp.path().join("reloaded").join(path))?;
            for (i, (a, b)) in a.lines().zip(b.lines()).enumerate() {
                if a != b {
                    println!("ROUTE_DIFF {path}:{}\n  direct: {a}\n  rete:   {b}", i + 1);
                }
            }
        }
    }
    assert_eq!(direct.files, reloaded.files);
    fs::write(
        project.join("check_motion.gd"),
        include_bytes!("check_motion.gd"),
    )?;
    let evidence = godot::run_godot(
        &engine,
        &project,
        &[
            "--headless",
            "--script",
            "res://check_motion.gd",
            "--",
            scene_path,
        ],
        Duration::from_secs(20),
    )?;
    assert!(evidence.contains("NATIVE_MOTION_OK"), "{evidence}");
    println!("{evidence}");
    let mut changed_brain = perso.clone();
    changed_brain.brain.as_mut().unwrap().variables[1] = DesignerValue::Integer(0);
    store.save(
        &AssetRef::from_path(changed_brain.path.clone()),
        &changed_brain,
    )?;
    let mut edited = clip.clone();
    edited.frames[1].motion_translation = Some([0., 0.75, 0.]);
    store.save(&AssetRef::from_path(edited.path.clone()), &edited)?;
    drop(store);
    let mut store = rete::Store::new(temp.path().join("rete"), Selection::hype("motion", "en")?)?;
    assert_eq!(
        *store.load(&AssetRef::<AnimationClip>::from_path(clip.path))?,
        edited
    );
    let edited_project = temp.path().join("edited");
    godot::export(
        &mut store,
        &roots,
        Selection::hype("motion", "en")?,
        &edited_project,
        &engine,
    )?;
    fs::write(
        edited_project.join("check_motion.gd"),
        include_bytes!("check_motion.gd"),
    )?;
    let evidence = godot::run_godot(
        &engine,
        &edited_project,
        &[
            "--headless",
            "--script",
            "res://check_motion.gd",
            "--",
            scene_path,
            "edited",
        ],
        Duration::from_secs(20),
    )?;
    assert!(evidence.contains("NATIVE_MOTION_EDIT_OK"), "{evidence}");
    println!("{evidence}");
    Ok(())
}
