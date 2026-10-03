//! Native route progress/contact and the explicitly deferred wind call.
use anyhow::{Context, Result, ensure};
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
        route::*,
        scene::*,
    },
    openspace::{ai::AiScriptCodec, codec::FormatCodec},
    rete,
};
use std::{
    fs,
    path::PathBuf,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

#[test]
fn compiled_routes_use_native_contact_and_actor_local_progress() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let selection = Selection::hype("routes", "en")?;
    let project = temp.path().join("project");
    let clip = AnimationClip {
        path: AssetPath::new("level:/animations/walk")?,
        channel_count: 1,
        frame_rate: 10,
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
        frames: (0..2)
            .map(|_| AnimationFrame {
                samples: vec![0],
                hierarchy: vec![],
                motion_rotation: None,
                motion_translation: Some([0., -0.15, 0.]),
            })
            .collect(),
        cues: vec![],
        morph_count: 0,
        unknown: vec![0; 6],
    };
    let idle = AnimationClip {
        path: AssetPath::new("level:/animations/idle")?,
        frames: (0..2)
            .map(|_| AnimationFrame {
                samples: vec![0],
                hierarchy: vec![],
                motion_rotation: None,
                motion_translation: None,
            })
            .collect(),
        ..clip.clone()
    };
    let family = Family {
        path: AssetPath::new("level:/families/walker")?,
        name: "Walker".into(),
        object_type: 0,
        bounding_geometry: None,
        states: [&clip, &idle]
            .into_iter()
            .enumerate()
            .map(|(i, clip)| CharacterState {
                id: i as u32,
                animation: Some(AssetRef::from_path(clip.path.clone())),
                mechanics: None,
                speed: 10,
                repeats: 0,
                transition_lock: 0,
                next: Some(i as u32),
                transitions: vec![],
                prohibited: vec![],
                unknown: [0; 5],
            })
            .collect(),
        configurations: vec![],
        default_configuration: None,
        animation_bank: 0,
        unknown: [0; 3],
    };
    let collision = Collision {
        path: AssetPath::new("level:/collision/walker")?,
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
        path: AssetPath::new("level:/collision-profiles/walker")?,
        geometry: Some(CollisionGeometrySlots {
            slots: vec![Some(AssetRef::from_path(collision.path.clone()))],
            unknown: [0; 4],
        }),
        activations: None,
        selections: vec![CollisionSelection {
            id: 0,
            zones: vec![CollisionZone {
                index: 0,
                unknown: [0; 2],
            }],
            unknown: [0; 4],
        }],
    };
    let identity = |allocation| SourceObjectIdentity {
        role: SourceRole::Level,
        block: [5, 1],
        allocation,
    };
    let start = Waypoint {
        path: AssetPath::new("level:/waypoints/start")?,
        source: identity(1),
        position: [0.; 3],
        radius: 0.25,
        relative_actor: None,
        unknown: [0; 4],
        tail: [0; 4],
    };
    let mut end = Waypoint {
        path: AssetPath::new("level:/waypoints/end")?,
        source: identity(2),
        position: [2., 0., 0.],
        ..start.clone()
    };
    let route = Route {
        path: AssetPath::new("level:/routes/walk")?,
        source: identity(3),
        points: vec![
            AssetRef::from_path(start.path.clone()),
            AssetRef::from_path(end.path.clone()),
        ],
        links: vec![Some(RouteLink {
            source: identity(4),
            geometry: Some(identity(5)),
            static_endpoint: Some(AssetRef::from_path(end.path.clone())),
            connection: 0,
            dynamic_kind: 0,
            sample: 0,
            used: 0,
            length: 2.,
            unknown: [0; 2],
            dynamics: vec![0; 52],
        })],
        current: 0,
        previous: 255,
        unknown: 0,
    };
    let wind = |x| {
        Statement::AddWind(Expression::Vector(Box::new([
            Expression::Real(x),
            Expression::Real(0.),
            Expression::Real(0.),
        ])))
    };
    let script = Script {
        statements: vec![
            wind(0.),
            wind(9.),
            Statement::Assign {
                variable: 1,
                value: Expression::Integer(7),
            },
            Statement::ChangeAction(Expression::Integer(0)),
            Statement::ResetRoute(0),
            Statement::GoOverRoute(0),
            Statement::Assign {
                variable: 0,
                value: Expression::Integer(1),
            },
            Statement::ChangeAction(Expression::Integer(1)),
            Statement::Wait {
                milliseconds: Expression::Integer(100000),
            },
        ],
    };
    let codec = AiScriptCodec::default();
    assert_eq!(codec.read(&codec.write(&script)?)?, script);
    let ai = AiModel {
        path: AssetPath::new("level:/ai/walker")?,
        name: "Route".into(),
        normal: Some(BehaviorSet {
            behaviors: vec![Behavior {
                id: 0,
                rules: vec![],
                schedule: Some(script),
                unknown: [0; 3],
            }],
            default_behavior: Some(0),
            action_capacity: 1,
            unknown: [0; 3],
        }),
        reflex: None,
        variables: (0..2)
            .map(|_| DesignerVariable {
                default: DesignerValue::Integer(0),
                save: 0,
                initialize: 0,
            })
            .collect(),
        designer_unknown: [0; 3],
        unknown: [0; 4],
    };
    let mut groups: [CharacterCollisionGroup; 4] = Default::default();
    groups[2] = CharacterCollisionGroup {
        profile: Some(AssetRef::from_path(profile.path.clone())),
        active: Some(0),
    };
    let perso = Perso {
        path: AssetPath::new("level:/persos/walker")?,
        name: "Walker".into(),
        family: AssetRef::from_path(family.path.clone()),
        initial_state: Some(0),
        current_state: Some(0),
        secondary_state: None,
        configuration: None,
        initial_configuration: None,
        object_types: [0; 3],
        has_camera: false,
        routes: vec![RouteBinding {
            route: Some(AssetRef::from_path(route.path.clone())),
            progress: [0, 0, 255, 255],
        }],
        brain: Some(Brain {
            model: AssetRef::from_path(ai.path.clone()),
            variables: vec![DesignerValue::Integer(0); 2],
            normal: Some(BehaviorSelection {
                current: Some(0),
                previous: Some(0),
                initial: Some(0),
            }),
            reflex: None,
            initial_variables: None,
            unknown: 0,
            mind_unknown: [0; 4],
        }),
        collisions: Some(CharacterCollisions {
            groups,
            unknown: [0; 4],
        }),
    };
    let mut scene = Scene {
        path: AssetPath::new("level:/scene")?,
        name: "Routes".into(),
        main_actor: Some(AssetRef::from_path(perso.path.clone())),
        nodes: vec![SceneNode {
            id: "walker".into(),
            name: "Walker".into(),
            parent: None,
            kind: NodeKind::Perso,
            instance: None,
            perso: Some(AssetRef::from_path(perso.path.clone())),
            transform: Default::default(),
            properties: Default::default(),
            collision: None,
            sector: None,
            light: None,
            draw_mask: 1,
        }],
        global_actors: vec![],
    };
    let static_box = |name: &str, min, max| -> Result<Collision> {
        Ok(Collision {
            path: AssetPath::new(format!("level:/collision/{name}"))?,
            geometry: CollisionGeometry::Mesh(CollisionMesh {
                positions: vec![min, max],
                normals: vec![],
                vertex_materials: vec![],
                elements: vec![CollisionElement::Boxes(vec![CollisionBox {
                    min: 0,
                    max: 1,
                    material: None,
                }])],
            }),
        })
    };
    let floor = static_box("floor", [-10., -10., -1.], [10., 10., 0.])?;
    let wall = static_box("listed-wall", [0.9, -5., 0.], [1.1, 5., 4.])?;
    let display_wall = static_box("display-wall", [1.5, -5., 0.], [1.7, 5., 4.])?;
    let mut node = |id: &str, kind, collision: Option<&Collision>, sector| {
        scene.nodes.push(SceneNode {
            id: id.into(),
            name: id.rsplit('/').next().unwrap().into(),
            parent: id.rsplit_once('/').map(|(parent, _)| parent.into()),
            kind,
            instance: None,
            perso: None,
            transform: None,
            properties: Default::default(),
            collision: collision.map(|value| AssetRef::from_path(value.path.clone())),
            sector,
            light: None,
            draw_mask: u32::MAX,
        });
    };
    let sector = |name: &str, virtual_sector, collision| Sector {
        name: name.into(),
        virtual_sector,
        activation: u32::MAX,
        environments: vec![],
        static_lights: vec![],
        dynamic_lights: vec![],
        boundary: None,
        persos: vec![],
        graphics: vec![],
        optional_graphics: vec![],
        collision,
        activity: vec![],
        sound: vec![],
        unknown: [0; 3],
    };
    node(
        "contact",
        NodeKind::Sector,
        None,
        Some(sector("contact", 0, vec!["listed".into()])),
    );
    node("contact/floor", NodeKind::Object, Some(&floor), None);
    node(
        "listed",
        NodeKind::Sector,
        None,
        Some(sector("listed", 1, vec![])),
    );
    node("listed/wall", NodeKind::Object, Some(&wall), None);
    node(
        "display",
        NodeKind::Sector,
        None,
        Some(sector("display", 1, vec![])),
    );
    node("display/wall", NodeKind::Object, Some(&display_wall), None);
    let mut source = Collection::default();
    macro_rules! put {
        ($v:expr) => {
            source.insert(&AssetRef::from_path($v.path.clone()), $v.clone())?
        };
    }
    put!(clip);
    put!(idle);
    put!(family);
    put!(collision);
    put!(profile);
    put!(start);
    put!(end);
    put!(route);
    put!(ai);
    put!(perso);
    put!(floor);
    put!(wall);
    put!(display_wall);
    put!(scene);
    let roots = [ContentRoot {
        kind: Scene::KIND.into(),
        reference: scene.path.clone(),
    }];
    let engine = std::env::var("ASTROLABE_GODOT").unwrap_or_else(|_| "godot".into());
    // Save/reopen deliberately edited useful geometry before native conversion.
    let mut store = rete::Store::new(temp.path().join("rete"), selection.clone())?;
    store.save(&AssetRef::from_path(end.path.clone()), &end)?;
    end.position[0] = 3.;
    store.save(&AssetRef::from_path(end.path.clone()), &end)?;
    let mut reopened = rete::Store::new(temp.path().join("rete"), selection.clone())?;
    let edited = reopened.load(&AssetRef::<Waypoint>::from_path(end.path.clone()))?;
    ensure!(edited.position == [3., 0., 0.], "Rete waypoint edit lost");
    source.insert(
        &AssetRef::from_path(edited.path.clone()),
        edited.as_ref().clone(),
    )?;
    godot::export(&mut source, &roots, selection, &project, &engine)?;
    fs::write(
        project.join("check_routes.gd"),
        include_bytes!("check_routes.gd"),
    )?;
    let evidence = godot::run_godot(
        &engine,
        &project,
        &["--headless", "--script", "res://check_routes.gd"],
        Duration::from_secs(20),
    )?;
    ensure!(
        evidence.contains("NATIVE_ROUTES_WIND_DEBT_OK"),
        "{evidence}"
    );
    println!("{evidence}");
    Ok(())
}

#[test]
#[ignore = "requires local Hype and Godot; opening actor slice, not complete intro acceptance"]
fn exported_gogoud_opening_walk_reaches_native_action_sequence() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let game = std::env::var_os("ASTROLABE_GAME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| "disc".into());
    let engine = std::env::var("ASTROLABE_GODOT").unwrap_or_else(|_| "godot".into());
    // Parse the component harness before the expensive source/Rete/full-scene
    // conversion. This stage runs no scene or gameplay and needs no game data.
    let syntax_project = temp.path().join("syntax");
    fs::create_dir(&syntax_project)?;
    fs::write(syntax_project.join("project.godot"), "config_version=5\n")?;
    fs::write(
        syntax_project.join("check_gogoud_opening.gd"),
        include_bytes!("check_gogoud_opening.gd"),
    )?;
    godot::run_godot(
        &engine,
        &syntax_project,
        &[
            "--headless",
            "--log-file",
            "godot-parser.log",
            "--check-only",
            "--script",
            "res://check_gogoud_opening.gd",
        ],
        Duration::from_secs(10),
    )?;
    let syntax_log = syntax_project.join("godot-parser.log");
    ensure!(
        fs::metadata(&syntax_log)?.len() <= 1024 * 1024,
        "Godot syntax log exceeded 1MiB diagnostic limit"
    );
    let syntax_output = fs::read_to_string(&syntax_log)?;
    ensure!(
        !syntax_output.contains("SCRIPT ERROR:") && !syntax_output.contains("ERROR:"),
        "Godot syntax log reported errors: {syntax_output}"
    );
    let selection = Selection::hype("manoir", "en")?;
    let mut imported = astrolabe::openspace::content::import_scene(&game, selection.clone())?;
    let scene = imported.load(&AssetRef::<Scene>::new("level:/scene")?)?;
    let garden = scene
        .nodes
        .iter()
        .find(|node| node.id == "world/1/9")
        .and_then(|node| node.sector.as_ref())
        .context("source garden sector missing")?;
    ensure!(
        garden.name == "sctjardinvirt"
            && garden.virtual_sector == 1
            && !scene.nodes.iter().any(|node| {
                node.sector
                    .as_ref()
                    .is_some_and(|sector| sector.collision.iter().any(|id| id == "world/1/9"))
            }),
        "opening fence collision eligibility differs from source"
    );
    let perso = imported.load(&AssetRef::<Perso>::new("level:/persos/world/0/5")?)?;
    ensure!(
        perso.routes.len() == 7 && perso.routes[1..4].iter().all(|b| b.route.is_none()),
        "Gogoud route slots differ"
    );
    let route = imported.load(
        perso.routes[6]
            .route
            .as_ref()
            .context("opening slot empty")?,
    )?;
    let end = imported.load(&route.points[1])?;
    ensure!(
        end.position == [-18.166048, 149.11961, 32.88652] && end.radius == 1.,
        "opening geometry differs"
    );
    let hype = imported.load(&AssetRef::<AiModel>::new("fix:/ai/models/28")?)?;
    let init = hype
        .normal
        .as_ref()
        .unwrap()
        .behaviors
        .iter()
        .find(|b| b.id == 0)
        .unwrap()
        .schedule
        .as_ref()
        .unwrap();
    let wind = init
        .statements
        .iter()
        .position(|s| matches!(s, Statement::AddWind(_)))
        .context("source wind not typed")?;
    ensure!(
        matches!(&init.statements[wind+1], Statement::Unsupported(f) if f.start == 14),
        "next initialization gate was hidden"
    );
    let rete_root = temp.path().join("rete");
    astrolabe::app::save_rete(&mut imported, &rete_root)?;
    let (mut reopened, package) = rete::Store::open(&rete_root)?;
    ensure!(
        *reopened.load(&AssetRef::<Perso>::from_path(perso.path.clone()))? == *perso,
        "actor route bindings lost through Rete"
    );
    ensure!(
        *reopened.load(&AssetRef::<Route>::from_path(route.path.clone()))? == *route,
        "shared route lost through Rete"
    );
    let project = temp.path().join("project");
    godot::export(&mut reopened, &package.roots, selection, &project, &engine)?;
    fs::write(
        project.join("check_gogoud_opening.gd"),
        include_bytes!("check_gogoud_opening.gd"),
    )?;
    let evidence_dir = std::env::var_os("ASTROLABE_EVIDENCE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| "TestResults".into());
    fs::create_dir_all(&evidence_dir)?;
    let id = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let evidence = evidence_dir
        .canonicalize()?
        .join(format!("gogoud-native-opening-{id}.json"));
    let output = godot::run_godot(
        &engine,
        &project,
        &[
            "--headless",
            "--script",
            "res://check_gogoud_opening.gd",
            "--",
            evidence.to_str().context("non-UTF8 evidence")?,
        ],
        Duration::from_secs(35),
    )?;
    fs::write(evidence.with_extension("log"), &output)?;
    ensure!(output.contains("GOGOUD_OPENING_SLICE_OK"), "{output}");
    println!("Gogoud opening component evidence: {}", evidence.display());
    Ok(())
}
