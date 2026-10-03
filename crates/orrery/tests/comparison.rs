use anyhow::Result;
use astrolabe::{
    model::{
        AssetPath, AssetRef, Selection,
        character::{CharacterState, Family, Perso},
        io::Save,
        scene::{NodeKind, Scene, SceneNode},
    },
    openspace::level::live::{ActionDefinitionProjection, ActorObservation},
    rete::Store,
};
use orrery::comparison::{Outcome, compare_main_actor, load_main_actor};

fn fixture() -> Result<(Scene, Perso, Family, ActorObservation)> {
    let family = Family {
        path: AssetPath::new("fix:/families/shared")?,
        name: "not an identity".into(),
        object_type: 7,
        bounding_geometry: None,
        states: vec![CharacterState {
            id: 0,
            animation: None,
            mechanics: None,
            speed: 24,
            repeats: 2,
            transition_lock: 1,
            next: Some(0),
            transitions: vec![],
            prohibited: vec![],
            unknown: [0; 5],
        }],
        configurations: vec![],
        default_configuration: None,
        animation_bank: 0,
        unknown: [0; 3],
    };
    let actor = Perso {
        path: AssetPath::new("level:/persos/player")?,
        name: "a different display name".into(),
        family: AssetRef::from_path(family.path.clone()),
        initial_state: Some(0),
        current_state: Some(0),
        secondary_state: None,
        configuration: None,
        initial_configuration: None,
        object_types: [7, 3, 9],
        camera: None,
        routes: vec![],
        brain: None,
        collisions: None,
    };
    let scene = Scene {
        path: AssetPath::new("level:/scene")?,
        name: "fixture".into(),
        main_actor: Some(AssetRef::from_path(actor.path.clone())),
        global_actors: vec![],
        nodes: vec![SceneNode {
            properties: Default::default(),
            id: "explicit-instance".into(),
            name: "unrelated name".into(),
            parent: None,
            kind: NodeKind::Perso,
            transform: None,
            instance: None,
            perso: Some(AssetRef::from_path(actor.path.clone())),
            collision: None,
            draw_mask: 0,
            sector: None,
            light: None,
        }],
    };
    // Independent observation fixture: source-local addresses have no relation
    // to the semantic asset paths, and runtime selections need not match files.
    let observed = ActorObservation {
        node: "live:1000".into(),
        perso: "live:2000".into(),
        data3d: "live:3000".into(),
        family: "live:4000".into(),
        object_types: [7, 3, 9],
        family_type: 7,
        action_members: vec!["live:5000".into()],
        action_definitions: vec![ActionDefinitionProjection {
            id: 0,
            speed: 24,
            repeats: 2,
            transition_lock: 1,
            next: Some(0),
        }],
        initial_action: None,
        current_action: None,
        secondary_action: None,
        global_transform: Default::default(),
    };
    Ok((scene, actor, family, observed))
}

#[test]
fn reopened_rete_comparison_catches_edits_missing_actions_and_ambiguous_role() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let selection = Selection::hype("fixture", "en")?;
    let (scene, actor, mut family, mut observed) = fixture()?;
    let scene_ref = AssetRef::from_path(scene.path.clone());
    let actor_ref = AssetRef::from_path(actor.path.clone());
    let family_ref = AssetRef::from_path(family.path.clone());
    {
        let mut writer = Store::new(temp.path(), selection.clone())?;
        writer.save(&scene_ref, &scene)?;
        writer.save(&actor_ref, &actor)?;
        writer.save(&family_ref, &family)?;
    }
    let mut reopened = Store::new(temp.path(), selection.clone())?;
    let expected = load_main_actor(&mut reopened, &[0])?;
    let matched = compare_main_actor(&expected, &observed);
    assert!(matched.selected_invariants_match);
    assert!(!matched.behavior_parity);
    assert_eq!(matched.counts[&Outcome::Matched], 8);
    assert_eq!(matched.counts[&Outcome::Unavailable], 3);
    assert_eq!(matched.counts[&Outcome::Unsupported], 1);
    drop(reopened);
    // Deliberate persisted edit, with the entire prior reader/cache gone.
    family.states[0].speed = 25;
    {
        Store::new(temp.path(), selection.clone())?.save(&family_ref, &family)?;
    }
    let mut reopened = Store::new(temp.path(), selection.clone())?;
    let edited = load_main_actor(&mut reopened, &[0])?;
    let mismatch = compare_main_actor(&edited, &observed);
    assert!(!mismatch.selected_invariants_match);
    let differences: Vec<_> = mismatch
        .fields
        .iter()
        .filter(|f| f.outcome == Outcome::Different)
        .collect();
    assert_eq!(differences.len(), 1);
    assert_eq!(differences[0].path, "family.actions[0].speed");
    assert_eq!(differences[0].expected, 25);
    assert_eq!(differences[0].actual, 24);
    observed.action_definitions.clear();
    let absent = compare_main_actor(&expected, &observed);
    assert!(!absent.selected_invariants_match);
    assert_eq!(absent.counts[&Outcome::Unmatched], 1);
    drop(reopened);
    let mut ambiguous = scene;
    let mut duplicate = ambiguous.nodes[0].clone();
    duplicate.id = "duplicate-instance".into();
    ambiguous.nodes.push(duplicate);
    assert!(
        Store::new(temp.path(), selection)?
            .save(&scene_ref, &ambiguous)
            .unwrap_err()
            .to_string()
            .contains("main actor must resolve to one scene perso")
    );
    Ok(())
}
