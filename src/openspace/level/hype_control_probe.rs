//! Source-only checks for the player values and logical controls used next.
use super::*;

#[test]
#[ignore = "requires local manoir data; included by tools/test.py --local"]
fn source_hype_idle_rule_preserves_deferred_behavior_entry() -> Result<()> {
    let game = std::env::var_os("ASTROLABE_GAME_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| "disc".into());
    let s = SourceSession::open(&game, &Selection::hype("manoir", "en")?)?;
    let perso = s.required(s.required(s.root(0x2ae0)?, 4)?, 4)?;
    let model = s.required(s.required(s.required(perso, 16)?, 0)?, 0)?;
    let sets = [source_set(&s, model, 0)?, source_set(&s, model, 4)?];
    let fixture_path = AssetPath::new("level:/ai/hype-idle-rule-fixture")?;
    let bindings = behavior_bindings(&fixture_path, &sets)?;
    let source = sets[0].as_ref().unwrap().behaviors[4].rules[0]
        .as_ref()
        .unwrap();
    let mut codec = AiScriptCodec::default();
    for (i, bytes) in source.bytes.as_chunks::<8>().0.iter().enumerate() {
        let node: NodeWire = wire(bytes)?;
        if node.kind == 32 {
            let target = bindings[&s.required(add(source.address, i * 8), 0)?].clone();
            assert_eq!(target.group, BehaviorGroup::Normal);
            assert_eq!(target.behavior, 41);
            codec.behaviors.insert(i, target.clone());
            codec.behavior_handles.push((target, node.parameter));
        }
    }
    let mut rule = codec.read(&source.bytes)?;
    assert_eq!(codec.write(&rule)?, source.bytes);
    let Statement::If {
        condition,
        then,
        otherwise,
    } = &mut rule.statements[0]
    else {
        panic!("idle rule must branch");
    };
    assert!(otherwise.is_none());
    assert_eq!(
        *condition,
        Expression::Compare {
            operation: Comparison::Equal,
            left: Box::new(Expression::Variable(56)),
            right: Box::new(Expression::Integer(1)),
        }
    );
    assert!(matches!(
        &then[..],
        [Statement::ChangeBehavior {
            pause_after: true,
            ..
        }]
    ));
    // Only compact the variable number after checking the untouched complete
    // rule's reconstruction. The destination marker below is a test observer,
    // not an implementation of Hype's behavior 41 or a playable Brain.
    let Expression::Compare { left, .. } = condition else {
        unreachable!()
    };
    **left = Expression::Variable(0);
    let layout = designer_layout(&s, model)?;
    let info = &layout.fields[56];
    let (definition, _) = layout.definition.as_ref().unwrap();
    let default = DesignerValueCodec {
        kind: info.kind,
        ..Default::default()
    }
    .read(s.bytes(add(s.required(*definition, 0)?, info.offset as usize), 1)?)?;
    assert_eq!(default, DesignerValue::Boolean(false));
    let fixture = AiModel {
        path: fixture_path,
        name: "Hype complete idle rule 0 fixture".into(),
        normal: Some(BehaviorSet {
            behaviors: vec![
                Behavior {
                    id: 4,
                    rules: vec![Some(rule)],
                    schedule: None,
                    unknown: [0; 3],
                },
                Behavior {
                    id: 41,
                    rules: vec![],
                    schedule: Some(Script {
                        statements: vec![
                            Statement::Assign {
                                variable: 1,
                                value: Expression::Integer(41),
                            },
                            Statement::Wait {
                                milliseconds: Expression::Integer(100000),
                            },
                        ],
                    }),
                    unknown: [0; 3],
                },
            ],
            default_behavior: Some(4),
            action_capacity: 1,
            unknown: [0; 3],
        }),
        reflex: None,
        variables: vec![
            DesignerVariable {
                default,
                save: info.save,
                initialize: info.initialize,
            },
            DesignerVariable {
                default: DesignerValue::Integer(0),
                save: 0,
                initialize: 0,
            },
        ],
        designer_unknown: [0; 3],
        unknown: [0; 4],
    };
    let reopened = crate::rete::decode::<AiModel>(&crate::rete::encode(&fixture)?)?;
    assert_eq!(
        crate::godot::ai::compile(&fixture, &[])?,
        crate::godot::ai::compile(&reopened, &[])?
    );
    let temp = tempfile::tempdir()?;
    crate::godot::support::initialize(temp.path())?;
    std::fs::write(
        temp.path().join("idle.gd"),
        crate::godot::ai::compile(&reopened, &[])?,
    )?;
    std::fs::write(
        temp.path().join("check.gd"),
        r#"extends SceneTree
func _initialize() -> void:
    call_deferred("check")
func check() -> void:
    var actor := Node3D.new()
    root.add_child(actor)
    var brain = load("res://idle.gd").new()
    brain.automatic = false
    brain.current_normal = 4
    actor.add_child(brain)
    brain.main_actor = actor
    brain.sound_emitter = actor
    assert(brain.start())
    assert(brain.current_normal == 4 and brain.dsg_1 == 0)
    brain.dsg_0 = true
    brain.advance(0.01)
    assert(brain.current_normal == 41 and brain.dsg_1 == 0)
    brain.advance(0.01)
    assert(brain.dsg_1 == 41)
    actor.free()
    print("HYPE_IDLE_RULE_OK: complete source rule, Rete, deferred entry into behavior 41")
    quit()
"#,
    )?;
    let engine = std::env::var("ASTROLABE_GODOT").unwrap_or_else(|_| "godot".into());
    let evidence = crate::godot::run_godot(
        &engine,
        temp.path(),
        &["--headless", "--script", "res://check.gd"],
        std::time::Duration::from_secs(15),
    )?;
    assert!(evidence.contains("HYPE_IDLE_RULE_OK"));
    println!("{evidence}");
    Ok(())
}

#[test]
#[ignore = "requires local manoir game data; included by tools/test.py --local"]
fn source_hype_input_operands_resolve_to_named_actions() -> Result<()> {
    let game = std::env::var_os("ASTROLABE_GAME_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| "disc".into());
    let s = SourceSession::open(&game, &Selection::hype("manoir", "en")?)?;
    let actor = s.required(s.root(0x2ae0)?, 4)?;
    let perso = s.required(actor, 4)?;
    let mind = s.required(s.required(perso, 16)?, 0)?;
    let model = s.required(mind, 0)?;
    let sets = [source_set(&s, model, 0)?, source_set(&s, model, 4)?];
    assert_eq!(sets[0].as_ref().unwrap().behaviors.len(), 107);
    assert_eq!(sets[1].as_ref().unwrap().behaviors.len(), 8);
    let mut actions = BTreeSet::new();
    for set in sets.iter().flatten() {
        for behavior in &set.behaviors {
            for script in behavior
                .rules
                .iter()
                .flatten()
                .chain(behavior.schedule.iter())
            {
                for (index, bytes) in script.bytes.as_chunks::<8>().0.iter().enumerate() {
                    let node: NodeWire = wire(bytes)?;
                    if node.kind == 14 {
                        actions.insert(input_action(&s, add(script.address, index * 8))?);
                    }
                }
            }
        }
    }
    for expected in [
        "Button_Up",
        "Button_Down",
        "Button_Left",
        "Button_Right",
        "Button_SpeedUp",
        "Button_Jump",
        "Button_Hit",
        "Button_GetSword",
        "Button_GetCrossbow",
        "Button_Magic",
    ] {
        assert!(
            actions.contains(expected),
            "missing Hype control {expected}: {actions:?}"
        );
    }
    println!(
        "HYPE_INPUT_SOURCE_OK: {} named actions, 107 normal + 8 reflex behaviors",
        actions.len()
    );
    Ok(())
}

#[test]
#[ignore = "requires local manoir game data; included by tools/test.py --local"]
fn source_hype_numeric_designer_values_reconstruct_through_rete() -> Result<()> {
    let game = std::env::var_os("ASTROLABE_GAME_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| "disc".into());
    let s = SourceSession::open(&game, &Selection::hype("manoir", "en")?)?;
    let perso = s.required(s.required(s.root(0x2ae0)?, 4)?, 4)?;
    let mind = s.required(s.required(perso, 16)?, 0)?;
    let model = s.required(mind, 0)?;
    let layout = designer_layout(&s, model)?;
    let (definition, header) = layout.definition.as_ref().unwrap();
    assert_eq!(layout.fields.len(), 177);
    // The player memory stores the first Boolean at its end, not offset zero.
    assert_eq!(layout.fields[0].offset, 2337);
    assert_eq!(layout.fields[133].offset, 0);
    let mut buffers = vec![s.required(*definition, 0)?];
    if let Some(memory) = s.pointer(mind, 12)? {
        buffers.extend(
            [s.pointer(memory, 4)?, s.pointer(memory, 8)?]
                .into_iter()
                .flatten(),
        );
    }
    for address in &buffers {
        let bytes = s.bytes(*address, header.length as usize)?;
        let mut variables = Vec::new();
        let mut fragments = Vec::new();
        for info in &layout.fields {
            // This fixture covers numeric memory. Actor/action reference coverage is distinct from these numeric fields.
            if !matches!(info.kind, 0..=7 | 12 | 20..=22) {
                continue;
            }
            let codec = DesignerValueCodec {
                kind: info.kind,
                ..Default::default()
            };
            let tail = &bytes[info.offset as usize..];
            let extent = codec.extent(tail)?;
            let original = &tail[..extent];
            let default = codec.read(original)?;
            assert_eq!(codec.write(&default)?, original);
            fragments.push((info.kind, original));
            variables.push(DesignerVariable {
                default,
                save: info.save,
                initialize: info.initialize,
            });
        }
        assert_eq!(variables.len(), 164);
        assert_eq!(
            variables
                .iter()
                .filter(|v| matches!(v.default, DesignerValue::IntegerArray(_)))
                .count(),
            2
        );
        assert_eq!(
            variables
                .iter()
                .filter(|v| matches!(v.default, DesignerValue::Vector(_)))
                .count(),
            8
        );
        // A useful model fixture containing the supported memory fragments; this
        // does not claim that Hype's full program or reference memory is imported.
        let fixture = AiModel {
            path: AssetPath::new("level:/ai/numeric-memory-fixture")?,
            name: "Hype numeric memory fixture".into(),
            normal: None,
            reflex: None,
            variables,
            designer_unknown: header.unknown,
            unknown: [0; 4],
        };
        let reopened = crate::rete::decode::<AiModel>(&crate::rete::encode(&fixture)?)?;
        for ((kind, original), variable) in fragments.iter().zip(&reopened.variables) {
            assert_eq!(
                DesignerValueCodec {
                    kind: *kind,
                    ..Default::default()
                }
                .write(&variable.default)?,
                *original
            );
        }
        let mut edited = reopened.clone();
        let array = edited
            .variables
            .iter_mut()
            .find_map(|v| match &mut v.default {
                DesignerValue::IntegerArray(a) => Some(a),
                _ => None,
            })
            .unwrap();
        let previous = array.values[0];
        array.values[0] = previous.wrapping_add(1);
        let edited = crate::rete::decode::<AiModel>(&crate::rete::encode(&edited)?)?;
        let value = &edited
            .variables
            .iter()
            .find(|v| matches!(v.default, DesignerValue::IntegerArray(_)))
            .unwrap()
            .default;
        let codec = DesignerValueCodec {
            kind: 22,
            ..Default::default()
        };
        let bytes = codec.write(value)?;
        assert_eq!(u32_at(&bytes, 8)? as i32, previous.wrapping_add(1));
        assert_eq!(codec.read(&bytes)?, *value);
    }
    println!(
        "HYPE_NUMERIC_MEMORY_OK: 164 numeric fields per {} source memories, 8 vectors, 2 Montreal integer arrays, Rete and byte reconstruction",
        buffers.len()
    );
    Ok(())
}

#[test]
#[ignore = "requires local manoir data; included by tools/test.py --local"]
fn source_hype_action_arrays_keep_family_ownership_and_reconstruct() -> Result<()> {
    let game = std::env::var_os("ASTROLABE_GAME_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| "disc".into());
    let s = SourceSession::open(&game, &Selection::hype("manoir", "en")?)?;
    let perso = s.required(s.required(s.root(0x2ae0)?, 4)?, 4)?;
    let family_address = s.required(s.required(perso, 0)?, 20)?;
    let family: FamilyWire = wire(s.bytes(family_address, 56)?)?;
    assert_eq!(family.states.count, 389);
    let family_ref = AssetRef::new(format!("fix:/families/{}", family.index))?;
    let mut states = BTreeMap::new();
    let mut at = s.pointer(family_address, 16)?;
    for state in 0..family.states.count {
        let a = at.context("short Hype state list")?;
        states.insert(
            a,
            StateRef {
                family: family_ref.clone(),
                state,
            },
        );
        let _: CharacterStateWire = wire(s.bytes(a, 64)?)?;
        at = s.pointer(a, 0)?;
    }
    let mind = s.required(s.required(perso, 16)?, 0)?;
    let model = s.required(mind, 0)?;
    let layout = designer_layout(&s, model)?;
    let (definition, header) = layout.definition.as_ref().unwrap();
    let mem = s.required(mind, 12)?;
    let buffers = [
        s.required(*definition, 0)?,
        s.required(mem, 4)?,
        s.required(mem, 8)?,
    ];
    let mut element_count = 0;
    let mut populated = 0;
    let mut array_defaults = Vec::new();
    for address in buffers {
        let mut variables = Vec::new();
        let mut fragments = Vec::new();
        for info in layout.fields.iter().filter(|i| i.kind == 25) {
            let mut codec = DesignerValueCodec {
                kind: 25,
                ..Default::default()
            };
            let site = add(address, info.offset as usize);
            let extent =
                codec.extent(s.bytes(site, header.length as usize - info.offset as usize)?)?;
            let bytes = s.bytes(site, extent)?;
            for offset in (8..extent).step_by(12) {
                let target = state_reference(&s, &states, add(site, offset))?;
                let raw = u32_at(bytes, offset)?;
                codec.actions.insert(offset, target.clone());
                if !codec.action_handles.iter().any(|(r, _)| *r == target) {
                    codec.action_handles.push((target, raw));
                }
                element_count += 1;
                populated += usize::from(codec.actions[&offset].is_some());
            }
            let value = codec.read(bytes)?;
            assert!(
                value
                    .state_references()
                    .iter()
                    .all(|r| r.family.path == family_ref.path)
            );
            variables.push(DesignerVariable {
                default: value,
                save: info.save,
                initialize: info.initialize,
            });
            fragments.push((codec, bytes.to_vec()));
        }
        assert_eq!(variables.len(), 7);
        array_defaults = variables.clone();
        let fixture = AiModel {
            path: AssetPath::new("level:/ai/action-memory-fixture")?,
            name: "Hype action memory".into(),
            normal: None,
            reflex: None,
            variables,
            designer_unknown: header.unknown,
            unknown: [0; 4],
        };
        let reopened = crate::rete::decode::<AiModel>(&crate::rete::encode(&fixture)?)?;
        for (variable, (codec, bytes)) in reopened.variables.iter().zip(&fragments) {
            assert_eq!(codec.write(&variable.default)?, *bytes);
        }
        let DesignerValue::ActionArray(array) = &reopened.variables[6].default else {
            panic!("expected action array");
        };
        assert_eq!(array.values.len(), 18);
    }
    let sets = [source_set(&s, model, 0)?, source_set(&s, model, 4)?];
    // Execute the actual top-level array assignments from Hype's initializer.
    // Decode/reconstruct each untouched five-node subtree before remapping only
    // its variable number into this seven-array fixture. This is an initializer
    // fragment test, not a replacement for importing Hype's complete Brain.
    let initializer = sets[0].as_ref().unwrap().behaviors[0]
        .schedule
        .as_ref()
        .unwrap();
    let variable_ids: Vec<_> = layout
        .fields
        .iter()
        .enumerate()
        .filter(|(_, info)| info.kind == 25)
        .map(|(id, _)| id as u16)
        .collect();
    assert_eq!(variable_ids, (158..=164).collect::<Vec<_>>());
    let mut initialization = Vec::new();
    let nodes: Vec<NodeWire> = initializer
        .bytes
        .as_chunks::<8>()
        .0
        .iter()
        .map(|bytes| wire(bytes))
        .collect::<Result<_>>()?;
    for (i, group) in nodes.windows(5).enumerate() {
        if group[0].kind != 2
            || group[0].parameter != 11
            || group[1].kind != 2
            || group[1].parameter != 26
            || group[2].kind != 11
            || !variable_ids
                .iter()
                .any(|id| u32::from(*id) == group[2].parameter)
        {
            continue;
        }
        assert_eq!(group[0].indent, 1);
        assert_eq!(group[2].kind, 11);
        assert_eq!(group[3].kind, 12);
        assert_eq!(group[4].kind, 25);
        let action = state_reference(&s, &states, add(initializer.address, (i + 4) * 8))?;
        let codec = AiScriptCodec {
            actions: BTreeMap::from([(4, action.clone())]),
            action_handles: vec![(action, group[4].parameter)],
            ..Default::default()
        };
        let mut bytes = initializer.bytes[i * 8..(i + 5) * 8].to_vec();
        bytes.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 8]);
        let script = codec.read(&bytes)?;
        assert_eq!(codec.write(&script)?, bytes);
        let mut statement = script.statements.into_iter().next().unwrap();
        let Statement::ArrayAssign {
            array: Expression::Variable(variable),
            ..
        } = &mut statement
        else {
            panic!("source initializer must assign an array element")
        };
        *variable = variable_ids
            .iter()
            .position(|id| id == variable)
            .context("initializer array outside designer declarations")? as u16;
        initialization.push(statement);
    }
    assert_eq!(initialization.len(), 102);
    initialization.push(Statement::Wait {
        milliseconds: Expression::Integer(100000),
    });
    let fixture = AiModel {
        path: AssetPath::new("level:/ai/hype-array-initializer")?,
        name: "Hype array initializer fragment".into(),
        normal: Some(BehaviorSet {
            behaviors: vec![Behavior {
                id: 0,
                rules: vec![],
                schedule: Some(Script {
                    statements: initialization,
                }),
                unknown: [0; 3],
            }],
            default_behavior: Some(0),
            action_capacity: 1,
            unknown: [0; 3],
        }),
        reflex: None,
        variables: array_defaults,
        designer_unknown: [0; 3],
        unknown: [0; 4],
    };
    let fixture = crate::rete::decode::<AiModel>(&crate::rete::encode(&fixture)?)?;
    let imported = super::super::import_scene(&game, &Selection::hype("manoir", "en")?)?;
    let family = imported
        .families
        .iter()
        .find(|f| f.path == family_ref.path)
        .context("imported player family missing")?;
    assert_eq!(family.states.len(), 389);
    let temp = tempfile::tempdir()?;
    crate::godot::support::initialize(temp.path())?;
    std::fs::write(
        temp.path().join("initializer.gd"),
        crate::godot::ai::compile_for_family(&fixture, &[], Some(family))?,
    )?;
    std::fs::write(
        temp.path().join("check.gd"),
        r#"extends SceneTree
func _initialize() -> void:
    call_deferred("check")
func check() -> void:
    var actor := Node3D.new()
    root.add_child(actor)
    var brain = load("res://initializer.gd").new()
    brain.automatic = false
    brain.current_normal = 0
    actor.add_child(brain)
    brain.main_actor = actor
    brain.sound_emitter = actor
    assert(Array(brain.dsg_6).all(func(v): return v == -1))
    assert(brain.start())
    # Independently read through Hype MCP after loading the untouched English
    # save in manoir, run 20260914-014307. Live pointers were resolved back to
    # the 389-member family state list; no memory writes were used.
    var original: Array = [
        [162,163,164,245,243,251,253,255,248,246,254,244,269,267,268,264,265,266],
        [91,65,204,205,193,196,228,229,242,241,222,215],
        [90,64,206,207,197,195,224,226,238,240,217,219],
        [167,166,165,230,231,232,252,247,250,249,257,256,258,259,260,261,262,262],
        [88,45,200,201,192,194,216,220,235,236,218,221],
        [89,46,202,203,198,199,233,227,237,239,225,223],
        [48,48,47,31,31,32,92,92,33,27,27,36,63,111,82,62,112,85]
    ]
    var count := 0
    for i in 7:
        var values: PackedInt32Array = brain.get("dsg_%d" % i)
        assert(Array(values) == original[i])
        count += values.size()
    assert(count == 102)
    assert(brain.dsg_6[5] == 32)
    assert(brain.dsg_6[11] == 36)
    var second = load("res://initializer.gd").new()
    second.automatic = false
    actor.add_child(second)
    assert(second.dsg_6[5] == -1)
    brain.stop()
    actor.queue_free()
    await process_frame
    print("HYPE_ARRAY_INITIALIZER_OK: 102 source assignments, native arrays, walk states 32/36")
    quit()
"#,
    )?;
    let engine = std::env::var("ASTROLABE_GODOT").unwrap_or_else(|_| "godot".into());
    let evidence = crate::godot::run_godot(
        &engine,
        temp.path(),
        &["--headless", "--script", "res://check.gd"],
        std::time::Duration::from_secs(20),
    )?;
    assert!(evidence.contains("HYPE_ARRAY_INITIALIZER_OK"));
    println!("{evidence}");
    assert_eq!(element_count, 306);
    assert_eq!(
        populated, 0,
        "source arrays are populated by initialization"
    );
    Ok(())
}
