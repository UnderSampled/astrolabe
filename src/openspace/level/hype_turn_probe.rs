//! Complete source rules exercised with Hype's exported character and animations.
use super::*;
use crate::model::{
    io::{ContentRoot, Load},
    scene::Scene,
};

#[test]
#[ignore = "requires local manoir data and Godot; included by tools/test.py --local"]
fn source_hype_turn_and_walk_rules_drive_native_character() -> Result<()> {
    let game = std::env::var_os("ASTROLABE_GAME_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| "disc".into());
    let selection = Selection::hype("manoir", "en")?;
    let s = SourceSession::open(&game, &selection)?;
    let perso_address = s.required(s.required(s.root(0x2ae0)?, 4)?, 4)?;
    let family_address = s.required(s.required(perso_address, 0)?, 20)?;
    let wire_family: FamilyWire = wire(s.bytes(family_address, 56)?)?;
    let family_ref = AssetRef::new(format!("fix:/families/{}", wire_family.index))?;
    let mut states = BTreeMap::new();
    let mut at = s.pointer(family_address, 16)?;
    for state in 0..wire_family.states.count {
        let a = at.context("short Hype state list")?;
        states.insert(
            a,
            StateRef {
                family: family_ref.clone(),
                state,
            },
        );
        at = s.pointer(a, 0)?;
    }
    assert_eq!(states.len(), 389);
    let mind = s.required(s.required(perso_address, 16)?, 0)?;
    let model_address = s.required(mind, 0)?;
    let set = source_set(&s, model_address, 0)?.unwrap();
    let sources = [(4, 2, 184), (5, 1, 50)].map(|(b, r, count)| {
        let source = set.behaviors[b].rules[r].as_ref().unwrap();
        assert_eq!(source.bytes.len() / 8, count);
        (b as u32, source)
    });
    let variable_ids: BTreeSet<_> = sources
        .iter()
        .flat_map(|(_, source)| source.bytes.as_chunks::<8>().0.iter())
        .map(|bytes| wire::<NodeWire>(bytes))
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .filter(|n| n.kind == 11)
        .map(|n| n.parameter as usize)
        .collect();
    let variable_ids: Vec<_> = variable_ids.into_iter().collect();
    assert_eq!(variable_ids, [10, 19, 52, 59, 61, 107, 132, 147]);
    let mut behaviors = Vec::new();
    for (id, source) in sources {
        let mut codec = AiScriptCodec::default();
        for (i, bytes) in source.bytes.as_chunks::<8>().0.iter().enumerate() {
            let node: NodeWire = wire(bytes)?;
            match node.kind {
                14 => {
                    let name = input_action(&s, add(source.address, i * 8))?;
                    codec.buttons.insert(i, name.clone());
                    codec.button_handles.push((name, node.parameter));
                }
                25 => {
                    let action = state_reference(&s, &states, add(source.address, i * 8))?;
                    codec.actions.insert(i, action.clone());
                    codec.action_handles.push((action, node.parameter));
                }
                _ => {}
            }
        }
        let original = codec.read(&source.bytes)?;
        assert_eq!(codec.write(&original)?, source.bytes);
        // Only compact variable numbers after reconstructing the complete,
        // untouched rule. No decisions or calls are replaced in this fixture.
        let mut compact = source.bytes.clone();
        for bytes in compact.as_chunks_mut::<8>().0 {
            if bytes[7] == 11 {
                let source_id = u32_at(bytes, 0)? as usize;
                let compact_id =
                    variable_ids.iter().position(|id| *id == source_id).unwrap() as u32;
                bytes[..4].copy_from_slice(&compact_id.to_le_bytes());
            }
        }
        println!("Hype normal {id} source inputs: {:?}", codec.button_handles);
        behaviors.push(Behavior {
            id,
            rules: vec![Some(codec.read(&compact)?)],
            schedule: None,
            unknown: [0; 3],
        });
    }
    let layout = designer_layout(&s, model_address)?;
    let (_, header) = layout.definition.as_ref().unwrap();
    let initial = s.required(s.required(mind, 12)?, 4)?;
    let memory = s.bytes(initial, header.length as usize)?;
    let variables = variable_ids
        .iter()
        .map(|id| {
            let info = &layout.fields[*id];
            let codec = DesignerValueCodec {
                kind: info.kind,
                ..Default::default()
            };
            let tail = &memory[info.offset as usize..];
            let bytes = &tail[..codec.extent(tail)?];
            let default = codec.read(bytes)?;
            assert_eq!(codec.write(&default)?, bytes);
            Ok(DesignerVariable {
                default,
                save: info.save,
                initialize: info.initialize,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let fixture = AiModel {
        path: AssetPath::new("level:/ai/hype-turn-walk-rules")?,
        name: "Hype complete turning and walk-transition rules".into(),
        normal: Some(BehaviorSet {
            behaviors,
            default_behavior: Some(4),
            action_capacity: 0,
            unknown: [0; 3],
        }),
        reflex: None,
        variables,
        designer_unknown: [0; 3],
        unknown: [0; 4],
    };
    let reopened = crate::rete::decode::<AiModel>(&crate::rete::encode(&fixture)?)?;
    let mut imported = crate::openspace::content::import_scene(&game, selection.clone())?;
    let family = imported.load(&family_ref)?;
    let script = crate::godot::ai::compile_for_family(&fixture, &[], Some(&family))?;
    assert_eq!(
        script,
        crate::godot::ai::compile_for_family(&reopened, &[], Some(&family))?
    );
    let mut edited = reopened.clone();
    let speed = variable_ids.iter().position(|id| *id == 107).unwrap();
    assert_eq!(
        edited.variables[speed].default,
        DesignerValue::UnsignedByte(160)
    );
    edited.variables[speed].default = DesignerValue::UnsignedByte(80);
    let edited = crate::rete::decode::<AiModel>(&crate::rete::encode(&edited)?)?;
    let root = AssetRef::<Scene>::new("level:/scene")?;
    let mut scene = imported.load(&root)?.as_ref().clone();
    // Native asset fixture: retain Hype's actual resources and collider, using
    // an explicit origin placement. It is not a replacement player controller.
    let mut player = scene
        .nodes
        .iter()
        .find(|n| {
            n.perso
                .as_ref()
                .is_some_and(|p| Some(p) == scene.main_actor.as_ref())
        })
        .unwrap()
        .clone();
    player.parent = None;
    player.id = "hype".into();
    player.transform = None;
    player.properties = Default::default();
    scene.nodes = vec![player];
    imported.assets.insert(&root, scene)?;
    let temp = tempfile::tempdir()?;
    let project = std::env::var_os("ASTROLABE_HYPE_RULE_PROJECT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| temp.path().join("godot"));
    let engine = std::env::var("ASTROLABE_GODOT").unwrap_or_else(|_| "godot".into());
    crate::godot::support::initialize(&project)?;
    let manifest = crate::godot::export(
        &mut imported,
        &[ContentRoot {
            kind: Scene::KIND.into(),
            reference: root.path,
        }],
        selection,
        &project,
        &engine,
    )?;
    let player_path = manifest
        .files
        .keys()
        .find(|p| p.contains("/persos/Hype__") && p.ends_with(".tscn"))
        .context("exported Hype missing")?;
    std::fs::write(project.join("rules.gd"), script)?;
    std::fs::write(
        project.join("rules-edited.gd"),
        crate::godot::ai::compile_for_family(&edited, &[], Some(&family))?,
    )?;
    let mut check = include_str!("hype_turn_check.gd").to_owned();
    for (compact, original) in variable_ids.iter().enumerate() {
        check = check.replace(&format!("@v{original}@"), &format!("dsg_{compact}"));
    }
    for (name, offset) in [
        ("left_slow", 182072),
        ("right_slow", 174592),
        ("left_fast", 2768848),
        ("right_fast", 2768780),
        ("idle", 276640),
        ("walk_start", 568436),
        ("walk", 150044),
        ("walk_start_other", 1915332),
        ("walk_other", 143828),
    ] {
        let state = states
            .iter()
            .find(|(a, _)| a.owner == Owner::Fix && a.key == BlockKey(5, 0) && a.offset == offset)
            .context("rule action missing from Hype family")?
            .1
            .state;
        check = check.replace(&format!("@{name}@"), &state.to_string());
        println!("Hype rule action {name}: {state}");
    }
    std::fs::write(project.join("check.gd"), check)?;
    let evidence = crate::godot::run_godot(
        &engine,
        &project,
        &[
            "--headless",
            "--script",
            "res://check.gd",
            "--",
            player_path,
        ],
        std::time::Duration::from_secs(30),
    )?;
    assert!(evidence.contains("HYPE_NATIVE_RULES_OK"));
    println!("{evidence}");
    Ok(())
}
